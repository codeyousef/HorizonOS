#!/usr/bin/env python3
"""Measure the installed 600-second idle unload without a replacement daemon."""
import json
import os
from pathlib import Path
import time

from installed_model_smoke import properties
from model_service_smoke import Client


def main():
    if os.geteuid() == 0 or Path('/etc/aios/model-test-profile').read_text().strip() != 'installed-normal-cpu-model-v1':
        raise RuntimeError('requires the actual model image and normal user')
    client = Client('/run/aios/model.sock')
    try:
        response = client.call({'kind': 'generate', 'generation': {
            'profile': 'normal', 'system_prompt': 'Return an answer JSON object using only the observation. Cite ev_idle. Perform no actions.',
            'user_prompt': 'Observation ev_idle: this operating system is NixOS. What OS does ev_idle report?',
            'response_mode': 'final_answer', 'allowed_tools': [], 'evidence_ids': ['ev_idle'], 'deadline_ms': 90000}})
        if response['error']:
            raise RuntimeError('idle qualification generation rejected')
        result = client.wait(response['data']['generation_id'])
        if result['state'] != 'completed' or result['output']['evidence_ids'] != ['ev_idle'] or 'NixOS' not in result['output']['text']:
            raise RuntimeError('idle qualification has no actual successful generation')
        began = time.monotonic()
        unit = properties('aios-model.service', ['MainPID', 'ControlGroup', 'NRestarts'])
        cgroup = unit['ControlGroup']
        if not cgroup.startswith('/system.slice/') or '..' in Path(cgroup).parts:
            raise RuntimeError('unexpected model cgroup')
        directory = Path('/sys/fs/cgroup') / cgroup.lstrip('/')
        samples = []
        while True:
            status = client.call({'kind': 'get_status'})
            if status['error'] or status['data']['idle_unload_seconds'] != 600 or status['data']['busy'] or status['data']['own_queued']:
                raise RuntimeError('idle observation is not the configured quiescent service')
            elapsed = time.monotonic() - began
            samples.append({'elapsed_seconds': round(elapsed, 3), 'loaded': status['data']['loaded'],
                            'cgroup_memory_current_bytes': int((directory / 'memory.current').read_text()),
                            'cgroup_memory_peak_bytes': int((directory / 'memory.peak').read_text())})
            if not status['data']['loaded']:
                # Completion polling may lag the worker's idle clock by 150 ms.
                if elapsed < 599 or elapsed > 615:
                    raise RuntimeError('actual idle unload outside the 600-second observation bound')
                break
            if elapsed > 615:
                raise RuntimeError('actual model did not unload at 600 seconds')
            time.sleep(2)
        if properties('aios-model.service', list(unit)) != unit:
            raise RuntimeError('model restarted or changed while observing idle unload')
        print('AIOS_INSTALLED_MODEL_IDLE_VERIFIED=' + json.dumps({
            'schema_version': 1, 'evidence_kind': 'actual-installed-model-elapsed-600-second-idle-unload',
            'uid': os.geteuid(), 'unit_identity': unit, 'answer': result, 'samples': samples,
            'idle_unload_observed_seconds': samples[-1]['elapsed_seconds'], 'mutation_performed': False,
            'limitations': ['Cgroup memory samples include file cache and are not process PSS or a workload peak qualification.',
                            'No service restart or other model operation is permitted during this measurement.']}), flush=True)
    finally:
        client.socket.close()


if __name__ == '__main__':
    main()
