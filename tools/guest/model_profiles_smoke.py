#!/usr/bin/env python3
"""Explicit optional-profile fetch/conversion, followed by offline child probes."""
from pathlib import Path
import subprocess
import sys
from model_conversion import convert


def main():
    release = Path(__file__).resolve().parents[2]
    if len(sys.argv) != 2 or sys.argv[1] not in {"low", "high"}:
        raise ValueError("one registered optional profile required")
    profile = sys.argv[1]
    convert(release, profile)
    subprocess.run([sys.executable, str(release / "tools/guest/model_profile_probe.py"), profile], check=True, timeout=180)


if __name__ == "__main__":
    main()
