import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

from .config import load_config
from .doctor import host_report
from .errors import DevctlError, ExitCode
from . import guest, provision, vm


class Parser(argparse.ArgumentParser):
    def error(self, message):
        raise DevctlError(ExitCode.INVALID_INPUT, "INVALID_ARGUMENT", message)


def parser() -> Parser:
    root = Parser(prog="devctl", description="Host tooling for verified AIOS guests")
    root.add_argument("--workspace", type=Path, default=Path(__file__).resolve().parents[2])
    # --json is handled before parsing so every command accepts it in any position.
    root.add_argument("--json", action="store_true", help="emit a structured result")
    commands = root.add_subparsers(dest="command", required=True)
    doctor = commands.add_parser("doctor", help="read-only prerequisite and target discovery")
    scope = doctor.add_mutually_exclusive_group(required=True)
    scope.add_argument("--host", action="store_true")
    scope.add_argument("--guest", action="store_true")
    vm = commands.add_parser("vm").add_subparsers(dest="operation", required=True)
    create = vm.add_parser("create").add_mutually_exclusive_group()
    create.add_argument("--authorize-provision", metavar="GUEST_UUID")
    create.add_argument("--refresh-seed", action="store_true")
    start = vm.add_parser("start")
    start.add_argument("--bootstrap", action="store_true")
    start.add_argument("--display", choices=("gtk", "none"), default="gtk")
    console = vm.add_parser("console").add_mutually_exclusive_group()
    console.add_argument("--capture", action="store_true")
    console.add_argument("--bootstrap-run", action="store_true")
    console.add_argument("--bootstrap-recover-unformatted", action="store_true")
    console.add_argument("--bootstrap-finish", action="store_true")
    console.add_argument("--bootstrap-audit", action="store_true")
    console.add_argument("--bootstrap-repair-access", action="store_true")
    console.add_argument("--bootstrap-inspect", action="store_true")
    console.add_argument("--follow", type=int, metavar="SECONDS")
    vm.add_parser("stop")
    for action in ("snapshot", "restore"):
        vm.add_parser(action).add_argument("--name", required=True)
    enrollment = commands.add_parser("enroll").add_mutually_exclusive_group()
    enrollment.add_argument("--pin-console-only", action="store_true")
    enrollment.add_argument("--trust-file", metavar="LOCAL_CONSOLE_JSON")
    commands.add_parser("sync")
    commands.add_parser("build").add_argument("--target", choices=("packages", "system"), required=True)
    commands.add_parser("test").add_argument("--suite", choices=("unit", "integration", "desktop"), required=True)
    commands.add_parser("benchmark").add_argument("--profile", choices=("normal", "low", "high"), required=True)
    commands.add_parser("deploy").add_argument("--mode", choices=("test", "commit"), required=True)
    logs = commands.add_parser("logs")
    logs.add_argument("--unit", required=True)
    logs.add_argument("--user")
    commands.add_parser("artifacts").add_subparsers(dest="operation", required=True).add_parser("pull")
    return root


def dispatch(args) -> tuple[ExitCode, dict]:
    if args.command == "doctor" and args.host:
        data = host_report(load_config(args.workspace))
        return (ExitCode.UNMET_PREREQUISITE if data["missing_prerequisites"] else ExitCode.SUCCESS), data
    if args.command == "doctor" and args.guest:
        return guest.doctor(load_config(args.workspace))
    if args.command == "enroll":
        if args.pin_console_only:
            config = load_config(args.workspace)
            if config.values["provider"] != "qemu":
                raise DevctlError(ExitCode.UNSUPPORTED_CAPABILITY, "UNSUPPORTED_CAPABILITY", "Automatic console pinning requires the verified QEMU bootstrap")
            trust = guest.pin_console(config)
            return ExitCode.SUCCESS, {"state": "console-trust-pinned", "identity_verified": False, "host_key_fingerprint": trust["host_key_fingerprint"]}
        return guest.enroll(load_config(args.workspace), args.trust_file)
    if args.command == "vm" and args.operation in ("create", "start", "console", "stop"):
        config = load_config(args.workspace)
        if config.values["provider"] != "qemu":
            raise DevctlError(ExitCode.UNSUPPORTED_CAPABILITY, "UNSUPPORTED_CAPABILITY", "External provider has no verified power/provisioning adapter")
        if args.operation == "create":
            if args.refresh_seed:
                return provision.refresh_seed(config)
            return provision.create(config, args.authorize_provision)
        if args.operation == "start":
            return vm.lifecycle(config, "start", display=args.display, bootstrap=args.bootstrap)
        if args.operation == "console" and args.follow is not None:
            if not 1 <= args.follow <= 3600:
                raise DevctlError(ExitCode.INVALID_INPUT, "INVALID_ARGUMENT", "Console follow duration must be 1..3600 seconds")
            return vm.follow_console(config, args.follow)
        if args.operation == "console" and args.capture:
            return vm.console(config, capture=True)
        if args.operation == "console" and (args.bootstrap_run or args.bootstrap_inspect or args.bootstrap_recover_unformatted or args.bootstrap_finish or args.bootstrap_audit or args.bootstrap_repair_access):
            with provision.operation_lock(config.root):
                return vm.console(config, capture=args.capture, bootstrap_run=args.bootstrap_run, bootstrap_inspect=args.bootstrap_inspect, bootstrap_recover=args.bootstrap_recover_unformatted, bootstrap_finish=args.bootstrap_finish, bootstrap_audit=args.bootstrap_audit, bootstrap_repair=args.bootstrap_repair_access)
        return vm.lifecycle(config, args.operation)
    # No unverified SSH, guest shell or host mutation fallback is available.
    raise DevctlError(
        ExitCode.UNSUPPORTED_CAPABILITY, "UNSUPPORTED_CAPABILITY",
        "This operation is not implemented. Guest enrollment and target verification must precede guest execution; VM mutations require the protected VM provider.",
    )


def main(argv=None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    use_json = "--json" in argv
    args = None
    data = None
    error = None
    try:
        args = parser().parse_args([item for item in argv if item != "--json"])
        code, data = dispatch(args)
    except DevctlError as failure:
        code = failure.exit_code
        error = {"code": failure.code, "message": str(failure), **failure.details}
    except TimeoutError:
        code = ExitCode.TIMEOUT
        error = {"code": "TIMEOUT", "message": "Host operation timed out"}
    except (OSError, ValueError) as failure:
        code = ExitCode.OPERATION_FAILURE
        error = {"code": "OPERATION_FAILED", "message": f"Host operation failed: {type(failure).__name__}"}
    result = {
        "schema_version": 1,
        "command": " ".join(filter(None, (getattr(args, "command", None), getattr(args, "operation", None)))),
        "target": "host" if args and (args.command == "vm" or args.command == "doctor" and args.host) else "guest",
        "release_digest": None, "artifact_path": data.get("artifact_path") if data else None,
        "observed_at": datetime.now(timezone.utc).isoformat(),
        "exit_status": int(code), "status": "ok" if code == ExitCode.SUCCESS else "error",
        "data": data, "error": error,
    }
    if use_json:
        print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    else:
        print(f"{result['command'] or 'devctl'}: target={result['target']} exit={int(code)} release=unknown artifact=none")
        if error:
            print(f"{error['code']}: {error['message']}")
        elif data:
            print(json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True))
    return int(code)
