#!/usr/bin/env python3
"""Observe protected native guard intake from fixed initial-image instrumentation."""
import hashlib
import json
from pathlib import Path
import snapshot
from installed_runtime_smoke import protected_preflight


def main():
    identity=snapshot.identity()
    proof,digest=protected_preflight(guard=True)
    guard=Path("/run/current-system/sw/bin/aios-guard").resolve(strict=True)
    if proof["uid"]!=0 or proof["effective_uid"]!=0 or proof["boot_id"]!=identity["boot_id"] or proof["installation_uuid"]!=identity["installation_uuid"]:
        raise ValueError("guard proof belongs to another target or caller")
    if proof["authorization_verified"] is not False or proof["activation_performed"] is not False:
        raise ValueError("guard intake claimed authorization or effects")
    if proof.get("executable")!=str(guard) or proof.get("executable_sha256")!=hashlib.sha256(guard.read_bytes()).hexdigest():
        raise ValueError("guard executable differs from protected proof")
    result=proof["native_result"]
    if proof["native_artifact_intake_verified"] is True:
        evidence=result["evidence"]
        wanted={key:identity[key] for key in ("installation_uuid","dmi_uuid","machine_id","boot_id","disk_serial","management_channel")}
        wanted["role"]=identity["guest_role"]
        if evidence["identity"]!=wanted or evidence["running"]["path"]!=identity["current_system"] or evidence["guard"]["sha256"]!=proof["executable_sha256"]:
            raise ValueError("native artifact intake target/guard differs")
        if evidence["boot"]["path"]!=evidence["boot_selection"]["closure"] or evidence["boot"]["kernel_sha256"]!=evidence["boot_selection"]["kernel_sha256"] or evidence["boot"]["initrd_sha256"]!=evidence["boot_selection"]["initrd_sha256"]:
            raise ValueError("boot-selected closure and EFI payloads disagree")
        if result["authorization_verified"] is not False or result["activation_performed"] is not False or result["runtime_adapter_available"] is not False:
            raise ValueError("native guard intake bypassed runtime gate")
    if snapshot.identity()!=identity:
        raise ValueError("guard target changed after observation")
    print("AIOS_INSTALLED_GUARD "+json.dumps({"schema_version":1,"evidence_kind":"actual-installed-root-guard-intake-observation",
        "target_identity":identity,"proof":proof,"proof_sha256":digest,"activation_performed":False,
        "guard_survival_verified":False,"authenticated_heartbeat_verified":False,
        "limitations":["Protected root artifact intake is not authorization, plan registration or an armed guard.",
                        "Live activation, independent survival, API/action health and recovery remain unqualified."]},sort_keys=True))
    if proof["native_artifact_intake_verified"] is not True or proof["upstream_exit"]!=0:
        raise RuntimeError("installed native guard artifact intake failed")


if __name__=="__main__":
    main()
