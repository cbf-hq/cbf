import os
import re
import tomllib
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path

from tool._subprocess import CmdError, run, run_passthrough, run_with_return

_SERIES_NAME_RE = re.compile(r"^[a-zA-Z0-9_.-]+$")


@dataclass(frozen=True)
class PatchSeriesConfig:
    name: str
    patch_dir: Path
    base_commit: str | None
    out_dir: Path


class ConfigError(RuntimeError):
    pass


def repo_root(start: Path | None = None) -> Path:
    here = (start or Path.cwd()).resolve()
    for parent in [here, *here.parents]:
        if (parent / "pyproject.toml").is_file():
            return parent
    raise ConfigError("Could not locate repo root (pyproject.toml not found).")


def chromium_src_dir(root: Path) -> Path:
    return root / "chromium" / "src"


def series_dir(root: Path, series: str) -> Path:
    if not _SERIES_NAME_RE.match(series):
        raise ConfigError(f"Invalid series name: {series!r}")
    return root / "chromium" / "patches" / series


def load_series_config(root: Path, series: str) -> PatchSeriesConfig:
    patch_dir = series_dir(root, series)
    if not patch_dir.is_dir():
        raise ConfigError(f"Patch directory not found: {patch_dir}")

    series_toml = patch_dir / "series.toml"
    base_commit: str | None = None
    out_dir = chromium_src_dir(root) / "out" / "Default"

    if series_toml.is_file():
        raw = tomllib.loads(series_toml.read_text(encoding="utf-8"))
        base_commit_raw = raw.get("base_commit")

        if isinstance(base_commit_raw, str) and base_commit_raw.strip():
            base_commit = base_commit_raw.strip()

        out_dir_raw = raw.get("out_dir")
        if isinstance(out_dir_raw, str) and out_dir_raw.strip():
            out_dir = chromium_src_dir(root) / out_dir_raw.strip()

    return PatchSeriesConfig(
        name=series,
        patch_dir=patch_dir,
        base_commit=base_commit,
        out_dir=out_dir,
    )


def list_patches(patch_dir: Path) -> list[Path]:
    patches = sorted(patch_dir.glob("000*.patch"))
    if not patches:
        raise ConfigError(f"No patch files found under: {patch_dir}")

    return patches


def ensure_clean(chromium_src: Path) -> None:
    status = run_with_return(
        ["git", "status", "--porcelain=v1"], cwd=chromium_src
    ).stdout
    if status.strip():
        raise ConfigError(
            "chromium/src has local modifications. Please stash/reset before applying patches."
        )


def checkout_work_branch(chromium_src: Path, branch: str, base_commit: str) -> None:
    run(["git", "checkout", "-B", branch, base_commit], cwd=chromium_src)


def git_am(chromium_src: Path, patch_files: Sequence[Path]) -> None:
    for patch in patch_files:
        try:
            run(["git", "am", str(patch)], cwd=chromium_src)
        except CmdError:
            # Best-effort cleanup, so the user can retry safely.
            try:
                run(["git", "am", "--abort"], cwd=chromium_src, check=False)
            finally:
                raise


def export_series(
    *,
    root: Path,
    series: str,
    base_commit: str | None,
) -> int:
    chromium_src = chromium_src_dir(root)
    cfg = load_series_config(root, series)

    chosen_base = base_commit or cfg.base_commit
    if chosen_base is None:
        raise ConfigError(
            "Base commit is not specified. Provide --base or set base_commit in series.toml."
        )

    # Clean up old patches first
    for p in cfg.patch_dir.glob("000*.patch"):
        p.unlink()

    run(
        [
            "git",
            "format-patch",
            "--output-directory",
            str(cfg.patch_dir),
            f"{chosen_base}..HEAD",
        ],
        cwd=chromium_src,
    )

    return len(list(cfg.patch_dir.glob("000*.patch")))


def apply_series(
    *,
    root: Path,
    series: str,
    base_commit: str | None,
    branch: str,
) -> list[Path]:
    chromium_src = chromium_src_dir(root)
    cfg = load_series_config(root, series)

    chosen_base = base_commit or cfg.base_commit
    if chosen_base is None:
        raise ConfigError(
            "Base commit is not specified. Provide --base or set base_commit in series.toml."
        )

    ensure_clean(chromium_src)
    checkout_work_branch(chromium_src, branch, chosen_base)

    patches = list_patches(cfg.patch_dir)
    git_am(chromium_src, patches)

    return patches


def clean_series(
    *,
    root: Path,
    series: str,
    base_commit: str | None,
) -> None:
    chromium_src = chromium_src_dir(root)
    cfg = load_series_config(root, series)

    chosen_base = base_commit or cfg.base_commit
    if chosen_base is None:
        raise ConfigError(
            "Base commit is not specified. Provide --base or set base_commit in series.toml."
        )

    run(["git", "reset", "--hard", chosen_base], cwd=chromium_src)
    run(["git", "clean", "-fd"], cwd=chromium_src)


def commit_series(
    *,
    root: Path,
    series: str,
    message: str,
    amend: bool,
    stage_all: bool,
) -> None:
    if not message.strip():
        raise ConfigError("Commit message must not be empty.")

    # Validate the requested series exists for consistent patch workflow.
    load_series_config(root, series)
    chromium_src = chromium_src_dir(root)

    status = run_with_return(
        ["git", "status", "--porcelain=v1"],
        cwd=chromium_src,
    ).stdout
    if not status.strip():
        raise ConfigError("No changes to commit in chromium/src.")

    argv = ["git", "commit"]
    if stage_all:
        argv.append("--all")
    if amend:
        argv.append("--amend")
    argv.extend(("--message", message))
    run_with_return(argv, cwd=chromium_src)


def run_git_in_chromium_src(*, root: Path, args: Sequence[str]) -> int:
    chromium_src = chromium_src_dir(root)
    return run_passthrough(["git", *list(args)], cwd=chromium_src)


def gn_gen_check(
    *,
    chromium_src: Path,
    out_dir: Path,
    env: dict[str, str] | None = None,
) -> None:
    run_with_return(["gn", "gen", str(out_dir), "--check"], cwd=chromium_src, env=env)


def autoninja(
    *,
    chromium_src: Path,
    out_dir: Path,
    targets: Iterable[str],
    env: dict[str, str] | None = None,
) -> None:
    argv = ["autoninja", "-C", str(out_dir), *list(targets)]
    run(argv, cwd=chromium_src, env=env)


def detect_mac_app_binary(out_dir: Path) -> Path | None:
    candidates = [
        out_dir / "Chromium.app" / "Contents" / "MacOS" / "Chromium",
        out_dir / "Chrome.app" / "Contents" / "MacOS" / "Chrome",
    ]

    for c in candidates:
        if c.is_file():
            return c

    return None


def run_chromium(
    *,
    chromium_src: Path,
    out_dir: Path,
    enable_features: str | None,
    cbf_ipc_channel: str | None,
    extra_args: Sequence[str],
    env: dict[str, str] | None,
) -> None:
    bin_path = detect_mac_app_binary(out_dir)
    if bin_path is None:
        raise ConfigError(
            "Could not find Chromium app binary. Build first, or pass a custom path via --bin (not implemented)."
        )

    argv: list[str] = [str(bin_path)]

    if enable_features is not None and enable_features.strip():
        argv.append(f"--enable-features={enable_features.strip()}")

    argv.extend(("--enable-logging=stderr", "--log-file=/tmp/chromium_debug.log"))
    if cbf_ipc_channel is not None and cbf_ipc_channel.strip():
        argv.append(f"--cbf-ipc-channel={cbf_ipc_channel.strip()}")
    argv.extend(extra_args)

    # Preserve PATH etc, but allow overrides.
    run(argv, cwd=chromium_src, env=env)


def resolve_depot_tools_path(root: Path, depot_tools: str | None = None) -> Path:
    if depot_tools is not None and depot_tools.strip():
        return Path(depot_tools.strip()).expanduser().resolve()

    env_depot_tools = os.environ.get("CBF_DEPOT_TOOLS_PATH")
    if env_depot_tools is not None and env_depot_tools.strip():
        return Path(env_depot_tools.strip()).expanduser().resolve()

    return (root / "depot_tools").resolve()


def tool_env_with_depot_tools(
    root: Path, depot_tools: str | None = None
) -> dict[str, str]:
    depot = resolve_depot_tools_path(root, depot_tools)
    path = os.environ.get("PATH", "")
    return {"PATH": f"{depot}{os.pathsep}{path}"}
