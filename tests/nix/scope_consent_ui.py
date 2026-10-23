"""Fixed synthetic assistive-technology driver, absent from product packages.

This inspects/clicks the real Qt button in the fixture account's desktop. It
does not write to the broker's private confirmation socket or mint a decision.
"""
import json
import os
from pathlib import Path
import sys
import time

import pyatspi


def dialog():
    native = Path('/etc/aios/scope-consent-native').read_text().strip()
    deadline = time.monotonic() + 45
    while time.monotonic() < deadline:
        desktop = pyatspi.Registry.getDesktop(0)
        for app in desktop:
            try:
                pid = app.get_process_id()
                if Path(f'/proc/{pid}/exe').resolve(strict=True) != Path(native):
                    continue
                frames = [node for node in app if node.name == 'Minnerite file root permission']
                if len(frames) == 1:
                    return pid, frames[0]
            except (OSError, RuntimeError):
                continue
        time.sleep(0.1)
    raise RuntimeError('pinned native file-root dialog did not appear')


def inspect_and_act(mode):
    assert os.geteuid() == 1000
    assert mode in ('inspect', 'allow')
    pid, frame = dialog()
    nodes, pending = [], [frame]
    while pending:
        node = pending.pop()
        nodes.append(node)
        assert len(nodes) <= 128
        pending.extend(list(node))
    text = '\n'.join(node.name or '' for node in nodes)
    assert '/home/alice/Documents' in text
    assert '/home/alice/Downloads' not in text
    assert 'Access classes: content' in text
    assert 'five minutes' in text
    buttons = [node for node in nodes if node.getRole() == pyatspi.ROLE_PUSH_BUTTON
               and node.name == 'Enroll these file roots']
    assert len(buttons) == 1
    assert buttons[0].getState().contains(pyatspi.STATE_ENABLED)
    if mode == 'allow':
        action = buttons[0].queryAction()
        assert 0 < action.nActions <= 16
        names = [action.getName(index) for index in range(action.nActions)]
        # Qt also exposes actions such as SetFocus. Select the named press
        # action rather than assuming the button has only one action.
        press = [index for index, name in enumerate(names) if name.casefold() == 'press']
        assert len(press) == 1, names
        assert action.doAction(press[0])
    print(json.dumps({'native_pid': pid, 'exact_root_visible': True,
                     'content_only_visible': True, 'mode': mode}, sort_keys=True))


if __name__ == '__main__':
    assert len(sys.argv) == 2
    inspect_and_act(sys.argv[1])
