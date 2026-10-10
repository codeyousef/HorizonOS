#!/usr/bin/env python3
"""Remove only reproducible caches owned by completed development jobs."""
import os
from pathlib import Path
import shutil
import stat
import uuid

import jobs


CACHE_NAMES = ("cargo-target", "cargo-home", "cache")


def safe_cache(path: Path, uid: int) -> bool:
    try:
        root = path.lstat()
    except FileNotFoundError:
        return False
    if not stat.S_ISDIR(root.st_mode) or stat.S_ISLNK(root.st_mode) or root.st_uid != uid:
        raise RuntimeError("unsafe terminal job cache root")
    for directory, directories, files in os.walk(path, topdown=False, followlinks=False):
        current = Path(directory)
        info = current.lstat()
        if not stat.S_ISDIR(info.st_mode) or stat.S_ISLNK(info.st_mode) or info.st_uid != uid:
            raise RuntimeError("unsafe terminal job cache directory")
        for name in directories + files:
            child = current / name
            child_info = child.lstat()
            if stat.S_ISLNK(child_info.st_mode) or child_info.st_uid != uid:
                raise RuntimeError("unsafe terminal job cache entry")
    return True


def available() -> int:
    value = os.statvfs("/nix/store")
    return value.f_bavail * value.f_frsize


def main():
    uid = os.geteuid()
    if uid == 0:
        raise RuntimeError("terminal job cache recovery must run as the development user")
    root = jobs.job_root()
    before = available()
    removed = []
    for directory in sorted(root.iterdir()):
        if not directory.is_dir() or directory.is_symlink():
            continue
        try:
            uuid.UUID(directory.name)
            report = jobs.read_record(directory)
        except (OSError, ValueError, KeyError):
            raise RuntimeError("unclassifiable development job blocks cache recovery")
        if report.get("state") not in jobs.TERMINAL or jobs.process_matches(report):
            continue
        for name in CACHE_NAMES:
            candidate = directory / name
            if safe_cache(candidate, uid):
                shutil.rmtree(candidate)
                removed.append(f"{directory.name}/{name}")
    after = available()
    if after < before:
        raise RuntimeError("available storage decreased during terminal cache recovery")
    print(f"AIOS_TERMINAL_JOB_CACHE_RECOVERY=before:{before},after:{after},removed:{len(removed)}")


if __name__ == "__main__":
    main()
