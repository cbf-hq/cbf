from __future__ import annotations

import argparse
import difflib
import subprocess
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
CRATE_ROOT = Path(__file__).resolve().parents[1]
SRC_DIR = CRATE_ROOT / "src"
INCLUDE_DIR = CRATE_ROOT / "include"

FFI_WRAPPER_HEADER = INCLUDE_DIR / "cbf_bridge_ffi_bindgen_wrapper.h"
BRIDGE_WRAPPER_HEADER = INCLUDE_DIR / "cbf_bridge_bindgen_wrapper.h"

FFI_GENERATED = SRC_DIR / "ffi_generated.rs"
BRIDGE_API_GENERATED = SRC_DIR / "bridge_api_generated.rs"

FFI_ALLOW_ATTR = """#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    improper_ctypes
)]"""

BRIDGE_API_ALLOW_ATTR = """#![allow(
    non_camel_case_types,
    unsafe_op_in_unsafe_fn,
    clippy::missing_safety_doc,
    clippy::too_many_arguments
)]"""


def _insert_inner_attr_after_first_line(text: str, attr: str) -> str:
    if attr in text:
        return text

    first_line, sep, rest = text.partition("\n")
    if not sep:
        return f"{first_line}\n\n{attr}\n"
    normalized_rest = rest.lstrip("\n")
    return f"{first_line}\n\n{attr}\n\n{normalized_rest}"


def _run_bindgen(args: list[str]) -> str:
    result = subprocess.run(
        args,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "bindgen generation failed")
    return result.stdout


def generate_ffi_types(*, output_path: Path | None = None) -> Path:
    output = output_path or FFI_GENERATED
    text = _run_bindgen(
        [
            "bindgen",
            str(FFI_WRAPPER_HEADER),
            "--allowlist-type",
            "Cbf.*",
            "--allowlist-var",
            "k.*|Cbf.*",
            "--allowlist-function",
            "^$",
            "--with-derive-default",
            "--no-layout-tests",
            "--formatter",
            "rustfmt",
            "--",
            "-Ichromium/src",
        ]
    )
    text = _insert_inner_attr_after_first_line(text, FFI_ALLOW_ATTR)
    output.write_text(text)
    return output


def generate_bridge_api(*, output_path: Path | None = None) -> Path:
    output = output_path or BRIDGE_API_GENERATED
    text = _run_bindgen(
        [
            "bindgen",
            str(BRIDGE_WRAPPER_HEADER),
            "--dynamic-loading",
            "cbf_bridge",
            "--dynamic-link-require-all",
            "--allowlist-function",
            "cbf_bridge_.*",
            "--blocklist-type",
            "Cbf.*",
            "--raw-line",
            "use super::*;",
            "--no-layout-tests",
            "--formatter",
            "rustfmt",
            "--",
            "-Ichromium/src",
        ]
    )
    text = _insert_inner_attr_after_first_line(text, BRIDGE_API_ALLOW_ATTR)
    output.write_text(text)
    return output


def _verify_file(expected: Path, generated: Path) -> list[str]:
    expected_text = expected.read_text()
    generated_text = generated.read_text()
    if expected_text == generated_text:
        return []
    return list(
        difflib.unified_diff(
            expected_text.splitlines(),
            generated_text.splitlines(),
            fromfile=str(expected),
            tofile=str(generated),
            lineterm="",
        )
    )


def verify() -> int:
    with tempfile.TemporaryDirectory(prefix="cbf-ffi-") as temp_dir:
        temp_root = Path(temp_dir)
        ffi_temp = temp_root / "ffi_generated.rs"
        bridge_temp = temp_root / "bridge_api_generated.rs"
        generate_ffi_types(output_path=ffi_temp)
        generate_bridge_api(output_path=bridge_temp)

        diffs = _verify_file(FFI_GENERATED, ffi_temp) + _verify_file(
            BRIDGE_API_GENERATED, bridge_temp
        )
        if diffs:
            print("\n".join(diffs))
            return 1
    print("FFI generated files are up to date.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate cbf-chrome-sys FFI bindings.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("generate", help="Regenerate both generated Rust binding files.")
    subparsers.add_parser("verify", help="Check that generated Rust binding files are up to date.")

    args = parser.parse_args()
    if args.command == "generate":
        generate_ffi_types()
        generate_bridge_api()
        print(f"Wrote {FFI_GENERATED.relative_to(REPO_ROOT)}")
        print(f"Wrote {BRIDGE_API_GENERATED.relative_to(REPO_ROOT)}")
        return 0
    if args.command == "verify":
        return verify()
    raise AssertionError(f"unexpected command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
