#!/usr/bin/env python3
"""Development-only CPU availability probe; never a product inference endpoint.

Original source conversion is separate. This child reads only the registered
profile artifact and denies network syscalls before native loading/generation.
"""
import ctypes as C
import errno
import hashlib
import json
import os
from pathlib import Path
import pwd
import socket
import stat
import sys
import time

from model_conversion import source_lock


def offline():
    path = Path(os.environ["AIOS_PROFILE_SECCOMP"]).resolve(strict=True)
    if not str(path).startswith("/nix/store/") or path.resolve() != path:
        raise ValueError("untrusted qualification seccomp library")
    api = C.CDLL(str(path), use_errno=True)
    api.seccomp_init.argtypes = [C.c_uint32]; api.seccomp_init.restype = C.c_void_p
    api.seccomp_syscall_resolve_name.argtypes = [C.c_char_p]; api.seccomp_syscall_resolve_name.restype = C.c_int
    api.seccomp_rule_add.argtypes = [C.c_void_p, C.c_uint32, C.c_int, C.c_uint]
    api.seccomp_load.argtypes = [C.c_void_p]
    api.seccomp_release.argtypes = [C.c_void_p]
    context = api.seccomp_init(0x7fff0000)
    if not context:
        raise RuntimeError("seccomp allocation failed")
    try:
        for name in ("socket", "socketpair", "connect", "bind", "listen", "accept", "accept4", "sendto", "sendmsg", "recvfrom", "recvmsg"):
            number = api.seccomp_syscall_resolve_name(name.encode())
            if number < 0 or api.seccomp_rule_add(context, 0x00050000 | errno.EPERM, number, 0):
                raise RuntimeError("offline rule unavailable")
        libc = C.CDLL(None, use_errno=True)
        if libc.prctl(38, 1, 0, 0, 0) or api.seccomp_load(context):
            raise RuntimeError("offline filter failed")
    finally:
        api.seccomp_release(context)
    try:
        socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    except PermissionError as error:
        if error.errno == errno.EPERM:
            return
        raise
    raise RuntimeError("network was not denied")


def native(runtime):
    api = C.CDLL(str(runtime / "lib/libaios-llama-bridge.so"))
    specifications = {
        "aios_abi_version": (C.c_uint32, []), "aios_runtime_revision": (C.c_char_p, []),
        "aios_cpu_backend_check": (C.c_int, []), "aios_model_open": (C.c_int, [C.c_char_p, C.POINTER(C.c_void_p)]),
        "aios_model_free": (None, [C.c_void_p]), "aios_cancel_new": (C.c_void_p, []),
        "aios_cancel_set": (None, [C.c_void_p]), "aios_cancel_free": (None, [C.c_void_p]),
        "aios_model_template": (C.c_int, [C.c_void_p, C.c_void_p, C.c_size_t, C.POINTER(C.c_size_t)]),
        "aios_chat_format": (C.c_int, [C.c_void_p, C.c_char_p, C.c_char_p, C.c_void_p, C.c_size_t, C.POINTER(C.c_size_t)]),
        "aios_context_new": (C.c_int, [C.c_void_p, C.c_uint32, C.c_uint32, C.c_void_p, C.POINTER(C.c_void_p)]),
        "aios_context_free": (None, [C.c_void_p]),
        "aios_context_prompt": (C.c_int, [C.c_void_p, C.c_char_p, C.c_char_p, C.c_uint32, C.POINTER(C.c_uint32)]),
        "aios_context_next": (C.c_int, [C.c_void_p, C.c_void_p, C.c_size_t, C.POINTER(C.c_size_t)]),
    }
    for name, (returns, arguments) in specifications.items():
        function = getattr(api, name); function.restype = returns; function.argtypes = arguments
    return api


def check(code):
    if code:
        raise RuntimeError("native compatibility operation failed: " + str(code))


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in {"low", "high"} or os.geteuid() == 0 or "ID=nixos" not in Path("/etc/os-release").read_text():
        raise ValueError("registered development profile required")
    profile = sys.argv[1]
    release = Path(__file__).resolve().parents[2]
    lock = source_lock(release, profile)
    directory = Path(pwd.getpwuid(os.geteuid()).pw_dir) / ".aios-models" / (lock["repository"].split("/")[1].lower() + "-" + lock["revision"])
    if directory.resolve(strict=True) != directory or directory.stat().st_uid != os.geteuid() or directory.stat().st_mode & 0o077:
        raise ValueError("unsafe profile directory")
    receipt = json.loads((directory / "conversion.json").read_text())
    runtime = Path(os.environ["AIOS_LLAMA_BRIDGE"])
    if receipt["source"] != lock or receipt["profile"] != profile or receipt["runtime_path"] != str(runtime) or runtime.resolve() != runtime or not str(runtime).startswith("/nix/store/"):
        raise ValueError("profile conversion identity changed")
    path = directory / "model-q4_k_m.gguf"
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as file:
        before = os.fstat(file.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_uid != os.geteuid() or before.st_nlink != 1 or before.st_mode & 0o222 or before.st_size != receipt["artifact"]["bytes"]:
            raise ValueError("unsafe profile artifact")
        digest_value = hashlib.file_digest(file, "sha256").hexdigest()
        if digest_value != receipt["artifact"]["sha256"]:
            raise ValueError("profile artifact hash changed")
        file.seek(0)
        offline()
        api = native(runtime)
        if api.aios_abi_version() != 1 or api.aios_runtime_revision().decode() != lock["runtime"]["revision"]:
            raise ValueError("native runtime changed")
        check(api.aios_cpu_backend_check())
        model, context, cancel = C.c_void_p(), C.c_void_p(), None
        try:
            started = time.monotonic()
            check(api.aios_model_open(("/proc/self/fd/" + str(file.fileno())).encode(), C.byref(model)))
            load_ms = round((time.monotonic() - started) * 1000, 3)
            buffer = C.create_string_buffer(32768); written = C.c_size_t()
            check(api.aios_model_template(model, buffer, len(buffer), C.byref(written)))
            if written.value > len(buffer) or hashlib.sha256(buffer.raw[:written.value]).hexdigest() != receipt["chat_template_sha256"]:
                raise ValueError("profile chat template differs")
            check(api.aios_chat_format(model, b"Return only the requested JSON object. Perform no actions.", ("Return {\"profile\":\"" + profile + "\"}.").encode(), buffer, len(buffer), C.byref(written)))
            if written.value > len(buffer):
                raise ValueError("profile prompt exceeded bound")
            prompt = buffer.raw[:written.value]
            cancel = api.aios_cancel_new()
            if not cancel:
                raise RuntimeError("native cancellation allocation failed")
            check(api.aios_context_new(model, 1024, 4, cancel, C.byref(context)))
            grammar = ('root ::= ' + json.dumps(json.dumps({"profile": profile}, separators=(",", ":"))) + "\n").encode()
            input_tokens = C.c_uint32()
            check(api.aios_context_prompt(context, prompt, grammar, 512, C.byref(input_tokens)))
            result = bytearray(); output_tokens = 0; deadline = time.monotonic() + 60
            for _ in range(32):
                if time.monotonic() >= deadline:
                    raise TimeoutError("profile generation deadline")
                token = C.create_string_buffer(1024); count = C.c_size_t()
                code = api.aios_context_next(context, token, len(token), C.byref(count))
                if code == 6:
                    break
                check(code)
                if count.value > len(token):
                    raise ValueError("native token output exceeded bound")
                result.extend(token.raw[:count.value]); output_tokens += 1
            else:
                raise ValueError("profile output did not finish within bound")
            if json.loads(result) != {"profile": profile}:
                raise ValueError("profile grammar output differs")
            # Completion consumes readiness; cancellation is checked on a fresh
            # active context rather than misclassifying the completed one.
            api.aios_context_free(context)
            context = C.c_void_p()
            check(api.aios_context_new(model, 1024, 4, cancel, C.byref(context)))
            cancelled_input = C.c_uint32()
            check(api.aios_context_prompt(context, prompt, grammar, 512, C.byref(cancelled_input)))
            api.aios_cancel_set(cancel)
            if api.aios_context_next(context, token, len(token), C.byref(count)) != 5:
                raise ValueError("profile cancellation failed")
            after = os.fstat(file.fileno())
            if (before.st_dev, before.st_ino, before.st_size, before.st_ctime_ns) != (after.st_dev, after.st_ino, after.st_size, after.st_ctime_ns):
                raise ValueError("profile artifact changed during load")
            print("AIOS_PROFILE_COMPATIBILITY=" + json.dumps({"schema_version": 1, "profile": profile,
                "repository": lock["repository"], "source_revision": lock["revision"], "artifact": receipt["artifact"],
                "chat_template_sha256": receipt["chat_template_sha256"], "converter_sha256": receipt["converter_sha256"],
                "quantizer_sha256": receipt["quantizer_sha256"], "runtime_revision": lock["runtime"]["revision"],
                "load_ms": load_ms, "input_tokens": input_tokens.value, "output_tokens": output_tokens,
                "output": json.loads(result), "actual_cpu_load": True, "grammar_validated": True, "cancellation_checked": True,
                "network_syscalls_denied": True, "compatibility_available": True, "product_profile_enabled": False,
                "quality_qualified": False, "performance_qualified": False,
                "evidence_kind": "actual-development-profile-cpu-availability",
                "limitations": ["Bounded 1024-context/4-thread compatibility check; not the full profile quality/resource/performance gate.",
                                "Python drives this explicit development probe only; production inference remains the Rust/C ABI package."]}, sort_keys=True), flush=True)
        finally:
            if context.value: api.aios_context_free(context)
            if cancel: api.aios_cancel_free(cancel)
            if model.value: api.aios_model_free(model)


if __name__ == "__main__":
    main()
