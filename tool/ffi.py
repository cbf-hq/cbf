from __future__ import annotations

import ast
import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# Temporary #64 migration helper. Keep this module until #65 completes so the
# generated FFI rollout can be checked against the pre-generation snapshots.

BINDGEN_WRAPPER_HEADER = "crates/cbf-chrome-sys/include/cbf_bridge_bindgen_wrapper.h"
BRIDGE_API_GENERATED = "crates/cbf-chrome-sys/src/bridge_api_generated.rs"
BRIDGE_API_ALLOW_ATTR = """#![allow(
    non_camel_case_types,
    unsafe_op_in_unsafe_fn,
    clippy::missing_safety_doc,
    clippy::too_many_arguments
)]"""


@dataclass
class SnapshotSource:
    ref: str | None
    repo_root: Path

    def read_text(self, relative_path: str) -> str | None:
        path = self.repo_root / relative_path
        if self.ref is None:
            if not path.exists():
                return None
            return path.read_text()

        git_cwd = self.repo_root
        git_path = relative_path
        chromium_prefix = "chromium/src/"
        if relative_path.startswith(chromium_prefix):
            git_cwd = self.repo_root / chromium_prefix.rstrip("/")
            git_path = relative_path.removeprefix(chromium_prefix)

        result = subprocess.run(
            ["git", "show", f"{self.ref}:{git_path}"],
            cwd=git_cwd,
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode != 0:
            return None
        return result.stdout


def create_ffi_snapshot(*, repo_root: Path, ref: str | None) -> dict[str, Any]:
    source = SnapshotSource(ref=ref, repo_root=repo_root)
    c_header = _require_text(
        source.read_text("chromium/src/chrome/browser/cbf/bridge/cbf_bridge_ffi.h"),
        "chromium/src/chrome/browser/cbf/bridge/cbf_bridge_ffi.h",
    )
    c_exports = _require_text(
        source.read_text("chromium/src/chrome/browser/cbf/bridge/cbf_bridge.h"),
        "chromium/src/chrome/browser/cbf/bridge/cbf_bridge.h",
    )
    rust_wrapper = _require_text(
        source.read_text("crates/cbf-chrome-sys/src/ffi.rs"),
        "crates/cbf-chrome-sys/src/ffi.rs",
    )
    rust_generated = source.read_text("crates/cbf-chrome-sys/src/ffi_generated.rs")
    rust_bridge_api = source.read_text(BRIDGE_API_GENERATED)

    rust = parse_rust_ffi(
        wrapper_text=rust_wrapper,
        generated_text=rust_generated,
        bridge_api_text=rust_bridge_api,
    )
    c = parse_c_ffi(
        ffi_header_text=c_header,
        exports_header_text=c_exports,
    )

    return {
        "ref": ref,
        "c": c,
        "rust": rust,
    }


def write_snapshot(
    *,
    repo_root: Path,
    ref: str | None,
    output_dir: Path,
) -> Path:
    snapshot = create_ffi_snapshot(repo_root=repo_root, ref=ref)
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "ffi-snapshot.json"
    output_path.write_text(json.dumps(snapshot, indent=2, sort_keys=True) + "\n")
    return output_path


def generate_bridge_api(*, repo_root: Path, output_path: Path | None = None) -> Path:
    wrapper_path = repo_root / BINDGEN_WRAPPER_HEADER
    output = output_path or (repo_root / BRIDGE_API_GENERATED)
    result = subprocess.run(
        [
            "bindgen",
            str(wrapper_path),
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
        ],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "bindgen bridge generation failed")

    output_text = result.stdout
    if BRIDGE_API_ALLOW_ATTR not in output_text:
        output_text = _insert_inner_attr_after_first_line(
            output_text, BRIDGE_API_ALLOW_ATTR
        )

    output.write_text(output_text)
    return output


def compare_snapshots(
    *,
    baseline_path: Path,
    candidate_path: Path,
    allowlist_path: Path | None,
    normalize: bool,
) -> tuple[list[str], dict[str, Any], dict[str, Any]]:
    baseline = json.loads(baseline_path.read_text())
    candidate = json.loads(candidate_path.read_text())
    allowlist = json.loads(allowlist_path.read_text()) if allowlist_path else {}

    if normalize:
        baseline = normalize_snapshot(baseline, allowlist)
        candidate = normalize_snapshot(candidate, allowlist)

    differences = diff_snapshots(baseline, candidate)
    return differences, baseline, candidate


def normalize_snapshot(
    snapshot: dict[str, Any], allowlist: dict[str, Any]
) -> dict[str, Any]:
    normalized = json.loads(json.dumps(snapshot))

    _drop_items(
        normalized["c"]["functions"], set(allowlist.get("remove_c_functions", []))
    )
    _drop_items(
        normalized["rust"]["functions"], set(allowlist.get("remove_rust_functions", []))
    )
    _drop_items(
        normalized["c"]["constants"], set(allowlist.get("remove_c_constants", []))
    )
    _drop_items(
        normalized["rust"]["constants"],
        set(allowlist.get("remove_rust_constants", [])),
    )
    _drop_items(
        normalized["c"]["opaque_handles"],
        set(allowlist.get("remove_c_opaque_handles", [])),
    )
    _drop_items(
        normalized["rust"]["opaque_handles"],
        set(allowlist.get("remove_rust_opaque_handles", [])),
    )

    _rename_items(normalized["c"]["functions"], allowlist.get("rename_c_functions", {}))
    _rename_items(
        normalized["rust"]["functions"], allowlist.get("rename_rust_functions", {})
    )
    _rename_items(normalized["c"]["constants"], allowlist.get("rename_c_constants", {}))
    _rename_items(
        normalized["rust"]["constants"], allowlist.get("rename_rust_constants", {})
    )

    _normalize_fields(
        normalized["c"]["structs"],
        allowlist.get("rename_c_fields", {}),
        set(allowlist.get("remove_c_fields", [])),
    )
    _normalize_fields(
        normalized["rust"]["structs"],
        allowlist.get("rename_rust_fields", {}),
        set(allowlist.get("remove_rust_fields", [])),
    )

    return normalized


def diff_snapshots(baseline: dict[str, Any], candidate: dict[str, Any]) -> list[str]:
    differences: list[str] = []
    if baseline["c"]["functions"] != candidate["c"]["functions"]:
        differences.append("C exported functions differ.")
    if baseline["rust"]["functions"] != candidate["rust"]["functions"]:
        differences.append("Rust extern functions differ.")
    if baseline["c"]["constants"] != candidate["c"]["constants"]:
        differences.append("C constants differ.")
    if baseline["rust"]["constants"] != candidate["rust"]["constants"]:
        differences.append("Rust constants differ.")
    if baseline["c"]["structs"] != candidate["c"]["structs"]:
        differences.append("C struct layouts differ.")
    if baseline["rust"]["structs"] != candidate["rust"]["structs"]:
        differences.append("Rust struct layouts differ.")
    if baseline["c"]["opaque_handles"] != candidate["c"]["opaque_handles"]:
        differences.append("C opaque handles differ.")
    if baseline["rust"]["opaque_handles"] != candidate["rust"]["opaque_handles"]:
        differences.append("Rust opaque handles differ.")
    if baseline["rust"]["c_void_usages"] != candidate["rust"]["c_void_usages"]:
        differences.append("Rust c_void usages differ.")
    return differences


def parse_c_ffi(*, ffi_header_text: str, exports_header_text: str) -> dict[str, Any]:
    constants = parse_c_enums(ffi_header_text)
    structs = parse_c_structs(ffi_header_text)
    functions = parse_c_exported_functions(exports_header_text)
    opaque_handles = parse_c_opaque_handles(ffi_header_text)
    return {
        "constants": constants,
        "structs": structs,
        "functions": functions,
        "opaque_handles": opaque_handles,
    }


def parse_rust_ffi(
    *,
    wrapper_text: str,
    generated_text: str | None,
    bridge_api_text: str | None,
) -> dict[str, Any]:
    texts = [wrapper_text]
    if generated_text:
        texts.append(generated_text)
    if bridge_api_text:
        texts.append(bridge_api_text)
    combined_text = "\n".join(texts)
    constants = parse_rust_constants("\n".join(texts))
    structs = parse_rust_structs("\n".join(texts))
    functions = parse_rust_functions("\n".join(texts))
    opaque_handles = parse_rust_opaque_handles(wrapper_text)
    c_void_matches = re.findall(
        r"(?:std::)?ffi::c_void|std::ffi::c_void", combined_text
    )
    c_void_usages = ["std::ffi::c_void"] if c_void_matches else []
    return {
        "constants": constants,
        "structs": structs,
        "functions": functions,
        "opaque_handles": opaque_handles,
        "c_void_usages": c_void_usages,
    }


def parse_c_enums(text: str) -> dict[str, int]:
    enum_pattern = re.compile(r"enum\s+\w+\s*\{(?P<body>.*?)\};", re.S)
    constants: dict[str, int] = {}
    for match in enum_pattern.finditer(text):
        body = _strip_c_comments(match.group("body"))
        for entry in body.split(","):
            item = entry.strip()
            if not item:
                continue
            name, expr = [part.strip() for part in item.split("=", 1)]
            constants[name] = _eval_numeric_expression(expr, constants)
    return dict(sorted(constants.items()))


def parse_c_structs(text: str) -> dict[str, list[dict[str, str]]]:
    struct_pattern = re.compile(
        r"typedef\s+struct\s+\w+\s*\{(?P<body>.*?)\}\s+(?P<alias>\w+)\s*;",
        re.S,
    )
    structs: dict[str, list[dict[str, str]]] = {}
    for match in struct_pattern.finditer(text):
        name = match.group("alias")
        body = _strip_c_comments(match.group("body"))
        fields = []
        for declaration in body.split(";"):
            field = declaration.strip()
            if not field:
                continue
            fields.append(_parse_c_field(field))
        structs[name] = fields
    return dict(sorted(structs.items()))


def parse_c_exported_functions(text: str) -> dict[str, str]:
    function_pattern = re.compile(
        r"CBF_BRIDGE_EXPORT\s+(?P<signature>[^;]+?cbf_bridge_[^(]+\([^;]*?\))\s*;",
        re.S,
    )
    functions: dict[str, str] = {}
    for match in function_pattern.finditer(text):
        signature = " ".join(match.group("signature").split())
        name_match = re.search(r"\b(cbf_bridge_[^(]+)\(", signature)
        if not name_match:
            continue
        functions[name_match.group(1)] = signature
    return dict(sorted(functions.items()))


def parse_c_opaque_handles(text: str) -> dict[str, str]:
    pattern = re.compile(r"typedef\s+struct\s+(\w+)\s+(\w+)\s*;")
    handles = {}
    for source_name, alias in pattern.findall(text):
        if source_name == alias:
            handles[alias] = f"typedef struct {source_name} {alias};"
    return dict(sorted(handles.items()))


def parse_rust_constants(text: str) -> dict[str, int]:
    pattern = re.compile(r"pub const (\w+): [^=]+ = (.+?);")
    constants: dict[str, int] = {}
    for name, expr in pattern.findall(text):
        constants[name] = _eval_numeric_expression(expr.strip(), constants)
    return dict(sorted(constants.items()))


def parse_rust_structs(text: str) -> dict[str, list[dict[str, str]]]:
    pattern = re.compile(r"pub struct (\w+)\s*\{(?P<body>.*?)\}", re.S)
    structs: dict[str, list[dict[str, str]]] = {}
    for name, body in pattern.findall(text):
        fields = []
        for line in body.splitlines():
            line = line.strip().rstrip(",")
            if not line.startswith("pub "):
                continue
            if line.startswith("pub(crate)"):
                continue
            field = line.removeprefix("pub ").strip()
            field_name, field_type = [part.strip() for part in field.split(":", 1)]
            fields.append({"name": field_name, "type": field_type})
        structs[name] = fields
    return dict(sorted(structs.items()))


def parse_rust_functions(text: str) -> dict[str, str]:
    field_pattern = re.compile(
        r"pub (?P<name>cbf_bridge_[^:]+):\s*unsafe extern \"C\" fn\((?P<params>.*?)\)"
        r"(?:\s*->\s*(?P<ret>[^,]+))?,",
        re.S,
    )
    functions: dict[str, str] = {}
    for match in field_pattern.finditer(text):
        name = match.group("name")
        params = " ".join(match.group("params").split())
        ret = (
            f" -> {' '.join(match.group('ret').split())}" if match.group("ret") else ""
        )
        functions[name] = f"{name}({params}){ret}"

    if functions:
        return dict(sorted(functions.items()))

    pattern = re.compile(
        r"pub fn (cbf_bridge_[^(]+)\((?P<params>.*?)\)(?: -> (?P<ret>[^;]+))?;", re.S
    )
    functions: dict[str, str] = {}
    for match in pattern.finditer(text):
        name = match.group(1)
        params = " ".join(match.group("params").split())
        ret = (
            f" -> {' '.join(match.group('ret').split())}" if match.group("ret") else ""
        )
        functions[name] = f"{name}({params}){ret}"
    return dict(sorted(functions.items()))


def parse_rust_opaque_handles(text: str) -> dict[str, str]:
    pattern = re.compile(r"pub struct (\w+)\s*\{\s*_private:\s*\[u8;\s*0\],\s*\}", re.S)
    handles = {}
    for name in pattern.findall(text):
        handles[name] = "opaque"
    return dict(sorted(handles.items()))


def _drop_items(mapping: dict[str, Any], names: set[str]) -> None:
    for name in names:
        mapping.pop(name, None)


def _rename_items(mapping: dict[str, Any], rename_map: dict[str, str]) -> None:
    for old_name, new_name in rename_map.items():
        if old_name not in mapping:
            continue
        value = mapping.pop(old_name)
        if isinstance(value, str):
            value = value.replace(old_name, new_name)
        mapping[new_name] = value


def _normalize_fields(
    structs: dict[str, list[dict[str, str]]],
    rename_map: dict[str, str],
    remove_set: set[str],
) -> None:
    for struct_name, fields in structs.items():
        new_fields = []
        seen: set[tuple[str, str]] = set()
        for field in fields:
            key = f"{struct_name}.{field['name']}"
            if key in remove_set:
                continue
            if key in rename_map:
                field = {"name": rename_map[key], "type": field["type"]}
            dedupe_key = (field["name"], field["type"])
            if dedupe_key in seen:
                continue
            seen.add(dedupe_key)
            new_fields.append(field)
        structs[struct_name] = new_fields


def _parse_c_field(field: str) -> dict[str, str]:
    normalized = " ".join(field.split())
    field_name = normalized.split()[-1]
    field_name = field_name.lstrip("*")
    field_type = normalized[: normalized.rfind(field_name)].strip()
    return {"name": field_name, "type": field_type}


def _eval_numeric_expression(expr: str, environment: dict[str, int]) -> int:
    parsed = ast.parse(expr, mode="eval")
    return _eval_ast(parsed.body, environment)


def _eval_ast(node: ast.AST, environment: dict[str, int]) -> int:
    if isinstance(node, ast.Constant):
        if isinstance(node.value, bool):
            return int(node.value)
        if isinstance(node.value, int):
            return node.value
    if isinstance(node, ast.Name):
        return environment[node.id]
    if isinstance(node, ast.UnaryOp) and isinstance(node.op, (ast.USub, ast.UAdd)):
        value = _eval_ast(node.operand, environment)
        return -value if isinstance(node.op, ast.USub) else value
    if isinstance(node, ast.BinOp) and isinstance(
        node.op,
        (ast.Add, ast.Sub, ast.BitOr, ast.BitAnd, ast.LShift, ast.RShift),
    ):
        left = _eval_ast(node.left, environment)
        right = _eval_ast(node.right, environment)
        if isinstance(node.op, ast.Add):
            return left + right
        if isinstance(node.op, ast.Sub):
            return left - right
        if isinstance(node.op, ast.BitOr):
            return left | right
        if isinstance(node.op, ast.BitAnd):
            return left & right
        if isinstance(node.op, ast.LShift):
            return left << right
        return left >> right
    raise ValueError(f"unsupported numeric expression: {ast.dump(node)}")


def _strip_c_comments(text: str) -> str:
    text = re.sub(r"//.*", "", text)
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return text


def _require_text(text: str | None, path: str) -> str:
    if text is None:
        raise FileNotFoundError(path)
    return text


def _insert_inner_attr_after_first_line(text: str, attr: str) -> str:
    first_line, sep, rest = text.partition("\n")
    if not sep:
        return f"{first_line}\n\n{attr}\n"
    return f"{first_line}\n\n{attr}\n{rest}"
