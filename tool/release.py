import hashlib
import shutil
import sys
import tarfile
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

from tool._subprocess import run_with_return
from tool.chromium_patches import (
    ConfigError,
    chromium_src_dir,
    load_series_config,
    repo_root,
    resolve_depot_tools_path,
)

RELEASE_OUT_DIR = Path("out") / "Release"
RELEASE_PACKAGE_NAME = "cbf-chrome-macos"
RELEASE_TARGETS = ("chrome", "cbf_bridge")
RELEASE_BUILD_COMMAND = "autoninja -C out/Release chrome cbf_bridge"
REQUIRED_GN_ARGS = {
    "is_debug": "false",
    "dcheck_always_on": "false",
    "is_component_build": "false",
    "is_official_build": "true",
    "proprietary_codecs": "false",
    "ffmpeg_branding": '"Chromium"',
}
OPTIONAL_GN_ARGS = ("target_os",)
REQUIRED_PRESENT_GN_ARGS = ("target_cpu",)


@dataclass(frozen=True)
class ReleasePaths:
    root: Path
    chromium_src: Path
    out_dir: Path
    release_dir: Path
    version_dir: Path
    archive_path: Path
    args_gn: Path
    chromium_app: Path
    bridge_dylib: Path
    cbf_license: Path
    third_party_licenses: Path
    source_info: Path


@dataclass(frozen=True)
class ReleaseMetadata:
    version: str
    build_date_utc: str
    cbf_commit: str
    chromium_commit: str
    patch_queue_state: str
    working_tree_clean: bool
    selected_gn_args: dict[str, str]
    args_gn_sha256: str
    archive_name: str
    artifact_sha256: dict[str, str]


def resolve_release_paths(version: str) -> ReleasePaths:
    root = repo_root()
    chromium_src = chromium_src_dir(root)
    out_dir = chromium_src / RELEASE_OUT_DIR
    release_dir = root / "dist" / "release"
    version_dir = release_dir / version
    archive_path = release_dir / f"{RELEASE_PACKAGE_NAME}-{version}.tar.gz"

    return ReleasePaths(
        root=root,
        chromium_src=chromium_src,
        out_dir=out_dir,
        release_dir=release_dir,
        version_dir=version_dir,
        archive_path=archive_path,
        args_gn=out_dir / "args.gn",
        chromium_app=out_dir / "Chromium.app",
        bridge_dylib=out_dir / "libcbf_bridge.dylib",
        cbf_license=version_dir / "CBF_LICENSE.txt",
        third_party_licenses=version_dir / "THIRD_PARTY_LICENSES.txt",
        source_info=version_dir / "SOURCE_INFO.txt",
    )


def current_release_version(root: Path, *, tag: str | None = None) -> str:
    if tag is not None:
        result = run_with_return(
            ["git", "rev-parse", "--verify", "--quiet", f"refs/tags/{tag}"],
            cwd=root,
            check=False,
        )
        if result.returncode != 0:
            raise ConfigError(f"Release tag does not exist: {tag}")
        return tag

    result = run_with_return(["git", "tag", "--points-at", "HEAD"], cwd=root)
    tags = [line.strip() for line in result.stdout.splitlines() if line.strip()]

    if not tags:
        raise ConfigError("HEAD is not tagged. Create a release tag before packaging.")
    if len(tags) > 1:
        raise ConfigError(
            "HEAD has multiple tags. Keep exactly one release tag on HEAD for deterministic packaging."
        )

    return tags[0]


def check_required_tools(root: Path) -> None:
    missing: list[str] = []
    for cmd in ("uv", "tar"):
        if shutil.which(cmd) is None:
            missing.append(cmd)

    has_python = shutil.which("python3") or shutil.which("python")
    if has_python is None and not Path(sys.executable).is_file():
        missing.append("python")

    has_hash = shutil.which("shasum") or shutil.which("sha256sum")
    if has_hash is None:
        missing.append("shasum-or-sha256sum")

    chromium_src = chromium_src_dir(root)
    if not chromium_src.is_dir():
        missing.append("chromium/src")

    depot_tools = resolve_depot_tools_path(root)
    if not depot_tools.is_dir():
        missing.append("depot_tools")

    if missing:
        missing_text = ", ".join(missing)
        raise ConfigError(f"Missing release prerequisites: {missing_text}")


def read_git_status(cwd: Path) -> str:
    return run_with_return(["git", "status", "--porcelain=v1"], cwd=cwd).stdout.strip()


def ensure_clean_worktrees(root: Path, chromium_src: Path, allow_dirty: bool) -> bool:
    root_clean = not read_git_status(root)
    chromium_clean = not read_git_status(chromium_src)
    clean = root_clean and chromium_clean
    if not clean and not allow_dirty:
        raise ConfigError(
            "Working tree is dirty. Commit or stash changes in the repo root and chromium/src, or rerun with --allow-dirty."
        )
    return clean


def parse_args_gn(args_gn_path: Path) -> dict[str, str]:
    if not args_gn_path.is_file():
        raise ConfigError(f"Release args.gn not found: {args_gn_path}")

    parsed: dict[str, str] = {}
    for raw_line in args_gn_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        parsed[key.strip()] = value.strip()

    return parsed


def validate_release_gn_args(parsed_args: dict[str, str]) -> dict[str, str]:
    selected: dict[str, str] = {}
    for key, expected in REQUIRED_GN_ARGS.items():
        actual = parsed_args.get(key)
        if actual is None:
            raise ConfigError(f"Required GN arg is missing from out/Release/args.gn: {key}")
        if actual != expected:
            raise ConfigError(
                f"Release GN arg mismatch for {key}: expected {expected}, found {actual}"
            )
        selected[key] = actual

    for key in REQUIRED_PRESENT_GN_ARGS:
        actual = parsed_args.get(key)
        if actual is None:
            raise ConfigError(f"Required GN arg is missing from out/Release/args.gn: {key}")
        selected[key] = actual

    for key in OPTIONAL_GN_ARGS:
        value = parsed_args.get(key)
        if value is not None:
            selected[key] = value

    return selected


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_tree(path: Path) -> str:
    digest = hashlib.sha256()
    for entry in sorted(p for p in path.rglob("*") if p.is_file()):
        digest.update(entry.relative_to(path).as_posix().encode("utf-8"))
        digest.update(b"\0")
        with entry.open("rb") as fh:
            for chunk in iter(lambda: fh.read(1024 * 1024), b""):
                digest.update(chunk)
    return digest.hexdigest()


def git_rev_parse(cwd: Path, rev: str) -> str:
    result = run_with_return(["git", "rev-parse", rev], cwd=cwd)
    return result.stdout.strip()


def render_source_info(metadata: ReleaseMetadata) -> str:
    selected_gn_lines = "\n".join(
        f"- {key}={value}" for key, value in metadata.selected_gn_args.items()
    )
    artifact_lines = "\n".join(
        f"- {name} | path={name} | sha256={digest}"
        for name, digest in metadata.artifact_sha256.items()
    )

    return (
        "CBF Release Source Information\n\n"
        "Format:\n"
        "- Source-Info-Format-Version: 1\n\n"
        "Package:\n"
        f"- Name: {RELEASE_PACKAGE_NAME}\n"
        f"- Version: {metadata.version}\n"
        f"- Build-Date-UTC: {metadata.build_date_utc}\n"
        "- Host-Platform: macOS\n"
        f"- Archive-Name: {metadata.archive_name}\n\n"
        "Source Revisions:\n"
        f"- CBF-Commit: {metadata.cbf_commit}\n"
        f"- Chromium-Commit: {metadata.chromium_commit}\n"
        f"- Patch-Queue-State: {metadata.patch_queue_state}\n"
        f"- Working-Tree-Clean: {str(metadata.working_tree_clean).lower()}\n\n"
        "Build:\n"
        "- Output-Directory: chromium/src/out/Release\n"
        f"- Targets: {', '.join(RELEASE_TARGETS)}\n"
        f"- Build-Command: {RELEASE_BUILD_COMMAND}\n\n"
        "Selected GN Args:\n"
        f"{selected_gn_lines}\n\n"
        "Args Traceability:\n"
        f"- args_gn_sha256: {metadata.args_gn_sha256}\n\n"
        "Artifacts:\n"
        f"{artifact_lines}\n\n"
        "License Bundle:\n"
        "- CBF-License-File: CBF_LICENSE.txt\n"
        "- Third-Party-License-File: THIRD_PARTY_LICENSES.txt\n"
        "- Third-Party-Licenses-Source: tools/licenses/licenses.py license_file --format txt\n"
        "- Chromium-Third-Party-Notices-File: CHROMIUM_THIRD_PARTY_NOTICES.html\n"
        "- Chromium-Third-Party-Notices-Included: false\n"
    )


def collect_metadata(paths: ReleasePaths, *, allow_dirty: bool, series: str) -> ReleaseMetadata:
    cfg = load_series_config(paths.root, series)
    working_tree_clean = ensure_clean_worktrees(
        paths.root, paths.chromium_src, allow_dirty=allow_dirty
    )
    parsed_args = parse_args_gn(paths.args_gn)
    selected_gn_args = validate_release_gn_args(parsed_args)

    if not paths.chromium_app.is_dir():
        raise ConfigError(f"Release artifact not found: {paths.chromium_app}")
    if not paths.bridge_dylib.is_file():
        raise ConfigError(f"Release artifact not found: {paths.bridge_dylib}")

    artifact_sha256 = {
        "Chromium.app": sha256_tree(paths.chromium_app),
        "libcbf_bridge.dylib": sha256_file(paths.bridge_dylib),
    }

    return ReleaseMetadata(
        version=paths.version_dir.name,
        build_date_utc=datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
        cbf_commit=git_rev_parse(paths.root, "HEAD"),
        chromium_commit=git_rev_parse(paths.chromium_src, "HEAD"),
        patch_queue_state=cfg.base_commit or "unknown",
        working_tree_clean=working_tree_clean,
        selected_gn_args=selected_gn_args,
        args_gn_sha256=sha256_file(paths.args_gn),
        archive_name=paths.archive_path.name,
        artifact_sha256=artifact_sha256,
    )


def release_check(*, allow_dirty: bool, series: str, tag: str | None = None) -> ReleasePaths:
    root = repo_root()
    check_required_tools(root)
    version = current_release_version(root, tag=tag)
    paths = resolve_release_paths(version)
    ensure_clean_worktrees(root, paths.chromium_src, allow_dirty=allow_dirty)
    validate_release_gn_args(parse_args_gn(paths.args_gn))
    return paths


def write_release_licenses(*, series: str, tag: str | None = None) -> ReleasePaths:
    root = repo_root()
    version = current_release_version(root, tag=tag)
    paths = resolve_release_paths(version)
    paths.version_dir.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(root / "LICENSE", paths.cbf_license)

    result = run_with_return(
        [
            sys.executable,
            "tools/licenses/licenses.py",
            "license_file",
            "--format",
            "txt",
            str(paths.third_party_licenses),
        ],
        cwd=paths.chromium_src,
    )
    if not paths.third_party_licenses.is_file():
        raise ConfigError(
            f"Chromium license generator did not create output: {paths.third_party_licenses}"
        )
    return paths


def write_source_info(
    *, allow_dirty: bool, series: str, tag: str | None = None
) -> ReleasePaths:
    root = repo_root()
    version = current_release_version(root, tag=tag)
    paths = resolve_release_paths(version)
    paths.version_dir.mkdir(parents=True, exist_ok=True)
    metadata = collect_metadata(paths, allow_dirty=allow_dirty, series=series)
    paths.source_info.write_text(render_source_info(metadata), encoding="utf-8")
    return paths


def create_release_archive(
    *, allow_dirty: bool, series: str, tag: str | None = None
) -> ReleasePaths:
    root = repo_root()
    version = current_release_version(root, tag=tag)
    paths = resolve_release_paths(version)
    metadata = collect_metadata(paths, allow_dirty=allow_dirty, series=series)
    paths.version_dir.mkdir(parents=True, exist_ok=True)
    paths.source_info.write_text(render_source_info(metadata), encoding="utf-8")

    required_files = (
        paths.cbf_license,
        paths.third_party_licenses,
        paths.source_info,
    )
    for required in required_files:
        if not required.is_file():
            raise ConfigError(f"Required release file is missing: {required}")

    paths.release_dir.mkdir(parents=True, exist_ok=True)

    with tarfile.open(paths.archive_path, "w:gz") as archive:
        archive.add(paths.chromium_app, arcname="Chromium.app")
        archive.add(paths.bridge_dylib, arcname="libcbf_bridge.dylib")
        archive.add(paths.cbf_license, arcname="CBF_LICENSE.txt")
        archive.add(paths.third_party_licenses, arcname="THIRD_PARTY_LICENSES.txt")
        archive.add(paths.source_info, arcname="SOURCE_INFO.txt")

    return paths
