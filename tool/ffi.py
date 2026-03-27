from __future__ import annotations

import subprocess
import sys
from pathlib import Path

GENERATOR_SCRIPT = (
    Path(__file__).resolve().parents[1]
    / "crates"
    / "cbf-chrome-sys"
    / "tools"
    / "generate_ffi.py"
)


def _run_generator(*args: str) -> int:
    result = subprocess.run(
        [sys.executable, str(GENERATOR_SCRIPT), *args],
        check=False,
    )
    return result.returncode


def generate_bindings() -> int:
    return _run_generator("generate")


def verify_bindings() -> int:
    return _run_generator("verify")
