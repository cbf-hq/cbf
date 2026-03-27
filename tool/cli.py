from collections.abc import Callable, Sequence
from pathlib import Path
from typing import Any

import click

from tool.ffi import generate_bindings, verify_bindings
from tool.chromium_patches import (
    ConfigError,
    apply_series,
    autoninja,
    chromium_src_dir,
    clean_series,
    commit_series,
    export_series,
    gn_gen_check,
    load_series_config,
    repo_root,
    run_chromium,
    run_git_in_chromium_src,
    tool_env_with_depot_tools,
)
from tool.release import (
    create_release_archive,
    release_check,
    write_release_licenses,
    write_source_info,
)

type CommandFunc = Callable[..., int]


def _common_series_options[FS: Callable[..., Any]](func: FS) -> FS:
    return click.option(
        "--series",
        default="cbf",
        show_default=True,
        help="Patch series name under chromium/patches/<series>",
    )(func)


def _common_depot_tools_option[FS: Callable[..., Any]](func: FS) -> FS:
    return click.option(
        "--depot-tools",
        default=None,
        help=(
            "Path to depot_tools directory. "
            "Priority: --depot-tools > CBF_DEPOT_TOOLS_PATH > ./depot_tools"
        ),
    )(func)


def _common_release_tag_option[FS: Callable[..., Any]](func: FS) -> FS:
    return click.option(
        "--tag",
        default=None,
        help=(
            "Release tag to package. "
            "Defaults to the single tag pointing at HEAD."
        ),
    )(func)


def cmd_chromium_apply(*, series: str, base: str | None, branch: str | None) -> int:
    root = repo_root()
    patches = apply_series(
        root=root,
        series=series,
        base_commit=base,
        branch=branch,
    )
    if branch is None:
        click.echo(f"Applied {len(patches)} patches to the current checkout.")
    else:
        click.echo(f"Applied {len(patches)} patches to branch {branch!r}.")

    return 0


def cmd_chromium_export(*, series: str, base: str | None) -> int:
    root = repo_root()
    count = export_series(
        root=root,
        series=series,
        base_commit=base,
    )
    click.echo(f"Exported {count} patches to chromium/patches/{series}.")

    return 0


def cmd_chromium_clean(*, series: str, base: str | None) -> int:
    root = repo_root()
    clean_series(
        root=root,
        series=series,
        base_commit=base,
    )
    click.echo("Cleaned chromium/src to base commit.")

    return 0


def cmd_chromium_commit(
    *,
    series: str,
    message: str,
    amend: bool,
    stage_all: bool,
) -> int:
    root = repo_root()
    commit_series(
        root=root,
        series=series,
        message=message,
        amend=amend,
        stage_all=stage_all,
    )
    click.echo("Commit OK.")
    return 0


def cmd_chromium_git(*, args: Sequence[str]) -> int:
    root = repo_root()
    return run_git_in_chromium_src(root=root, args=args)


def cmd_chromium_verify(*, series: str, depot_tools: str | None) -> int:
    root = repo_root()
    cfg = load_series_config(root, series)
    chromium_src = chromium_src_dir(root)
    env = tool_env_with_depot_tools(root, depot_tools)

    gn_gen_check(chromium_src=chromium_src, out_dir=cfg.out_dir, env=env)
    click.echo(f"gn gen --check OK: {cfg.out_dir}")

    return 0


def cmd_chromium_build(
    *,
    series: str,
    depot_tools: str | None,
    targets: Sequence[str],
    out_dir: str | None,
) -> int:
    root = repo_root()
    cfg = load_series_config(root, series)
    chromium_src = chromium_src_dir(root)
    out_dir_path = _resolve_out_dir(chromium_src, out_dir, cfg.out_dir)
    env = tool_env_with_depot_tools(root, depot_tools)

    autoninja(
        chromium_src=chromium_src,
        out_dir=out_dir_path,
        targets=targets,
        env=env,
    )
    click.echo(f"Build OK: {out_dir_path}")

    return 0


def _resolve_out_dir(chromium_src: Path, out_dir: str | None, default: Path) -> Path:
    if out_dir is None:
        return default

    candidate = Path(out_dir)
    if candidate.is_absolute():
        return candidate

    return chromium_src / candidate


def cmd_chromium_run(
    *,
    series: str,
    enable_features: str | None,
    cbf_ipc_channel: str | None,
    depot_tools: str | None,
    args: Sequence[str],
) -> int:
    root = repo_root()
    cfg = load_series_config(root, series)
    chromium_src = chromium_src_dir(root)
    env = tool_env_with_depot_tools(root, depot_tools)

    run_chromium(
        chromium_src=chromium_src,
        out_dir=cfg.out_dir,
        enable_features=enable_features,
        cbf_ipc_channel=cbf_ipc_channel,
        extra_args=args,
        env=env,
    )
    return 0


@click.group(help="CBF development tools.")
def cli() -> None:
    return None


@cli.group("release", help="Release packaging helpers.")
def release_group() -> None:
    return None


@cli.group("ffi", help="FFI generation helpers.")
def ffi_group() -> None:
    return None


@cli.command("apply", help="Apply patch series via git am.")
@_common_series_options
@click.option(
    "--base",
    default=None,
    help="Base commit to apply onto (overrides series.toml)",
)
@click.option(
    "--branch",
    default=None,
    help="Work branch name to reset/create (defaults to current checkout)",
)
def apply(*, series: str, base: str | None, branch: str | None) -> None:
    cmd_chromium_apply(series=series, base=base, branch=branch)


@cli.command("export", help="Export patch series via git format-patch.")
@_common_series_options
@click.option(
    "--base",
    default=None,
    help="Base commit to export from (overrides series.toml)",
)
def export(*, series: str, base: str | None) -> None:
    cmd_chromium_export(series=series, base=base)


@cli.command("clean", help="Clean chromium/src (reset and clean).")
@_common_series_options
@click.option(
    "--base",
    default=None,
    help="Base commit to clean to (overrides series.toml)",
)
def clean(*, series: str, base: str | None) -> None:
    cmd_chromium_clean(series=series, base=base)


@cli.command(
    "git",
    help="Run git in chromium/src.",
    context_settings={"ignore_unknown_options": True, "allow_extra_args": True},
)
@click.argument("args", nargs=-1, type=click.UNPROCESSED)
def git(args: tuple[str, ...]) -> int:
    return cmd_chromium_git(args=args)


@cli.command("commit", help="Commit chromium/src changes.")
@_common_series_options
@click.option(
    "-m",
    "--message",
    required=True,
    help="Commit message",
)
@click.option(
    "--amend",
    is_flag=True,
    help="Amend the previous commit",
)
@click.option(
    "-a",
    "--all",
    "stage_all",
    is_flag=True,
    help="Stage all tracked modifications before commit",
)
def commit(
    *,
    series: str,
    message: str,
    amend: bool,
    stage_all: bool,
) -> None:
    cmd_chromium_commit(
        series=series,
        message=message,
        amend=amend,
        stage_all=stage_all,
    )


@cli.command("verify", help="Run gn gen --check.")
@_common_series_options
@_common_depot_tools_option
def verify(*, series: str, depot_tools: str | None) -> None:
    cmd_chromium_verify(series=series, depot_tools=depot_tools)


@cli.command("build", help="Build chrome via autoninja.")
@_common_series_options
@click.option(
    "-t",
    "--target",
    "targets",
    multiple=True,
    default=("chrome",),
    show_default=True,
    help="Build target name (repeatable)",
)
@click.option(
    "--out-dir",
    default=None,
    help=(
        "Override GN output directory. Relative paths are resolved from "
        "chromium/src (e.g. out/Release)."
    ),
)
@_common_depot_tools_option
def build(
    *,
    series: str,
    targets: tuple[str, ...],
    out_dir: str | None,
    depot_tools: str | None,
) -> None:
    cmd_chromium_build(
        series=series,
        depot_tools=depot_tools,
        targets=targets,
        out_dir=out_dir,
    )


@cli.command("run", help="Run Chromium with CBF flags.")
@_common_series_options
@click.option(
    "--enable-features",
    default=None,
    help="Value for --enable-features flag",
)
@click.option(
    "--cbf-ipc-channel",
    default=None,
    help="Value for --cbf-ipc-channel flag",
)
@_common_depot_tools_option
@click.argument("args", nargs=-1, type=click.UNPROCESSED)
def run(
    *,
    series: str,
    enable_features: str | None,
    cbf_ipc_channel: str | None,
    depot_tools: str | None,
    args: tuple[str, ...],
) -> None:
    cmd_chromium_run(
        series=series,
        enable_features=enable_features,
        cbf_ipc_channel=cbf_ipc_channel,
        depot_tools=depot_tools,
        args=args,
    )


@release_group.command("check", help="Validate release prerequisites.")
@_common_series_options
@_common_release_tag_option
@click.option(
    "--allow-dirty",
    is_flag=True,
    help="Allow packaging from a dirty worktree.",
)
def release_check_command(*, series: str, tag: str | None, allow_dirty: bool) -> None:
    paths = release_check(allow_dirty=allow_dirty, series=series, tag=tag)
    click.echo(f"Release prerequisites OK: {paths.out_dir}")


@release_group.command("licenses", help="Generate release license files.")
@_common_series_options
@_common_release_tag_option
def release_licenses_command(*, series: str, tag: str | None) -> None:
    paths = write_release_licenses(series=series, tag=tag)
    click.echo(f"Wrote release licenses under {paths.version_dir}")


@release_group.command("source-info", help="Generate SOURCE_INFO.txt.")
@_common_series_options
@_common_release_tag_option
@click.option(
    "--allow-dirty",
    is_flag=True,
    help="Allow metadata generation from a dirty worktree.",
)
def release_source_info_command(
    *, series: str, tag: str | None, allow_dirty: bool
) -> None:
    paths = write_source_info(allow_dirty=allow_dirty, series=series, tag=tag)
    click.echo(f"Wrote source info: {paths.source_info}")


@release_group.command("package", help="Assemble the release tar.gz archive.")
@_common_series_options
@_common_release_tag_option
@click.option(
    "--allow-dirty",
    is_flag=True,
    help="Allow packaging from a dirty worktree.",
)
def release_package_command(*, series: str, tag: str | None, allow_dirty: bool) -> None:
    paths = create_release_archive(allow_dirty=allow_dirty, series=series, tag=tag)
    click.echo(f"Created release archive: {paths.archive_path}")


@ffi_group.command("generate", help="Regenerate cbf-chrome-sys bindgen output.")
def ffi_generate_command() -> None:
    exit_code = generate_bindings()
    if exit_code != 0:
        raise click.ClickException("FFI generation failed.")


@ffi_group.command("verify", help="Check that checked-in bindgen output is up to date.")
def ffi_verify_command() -> None:
    exit_code = verify_bindings()
    if exit_code != 0:
        raise click.ClickException("FFI bindings are out of date.")


def main(argv: Sequence[str] | None = None) -> int:
    try:
        result = cli.main(
            args=list(argv) if argv is not None else None,
            prog_name="cbf-tool",
            standalone_mode=False,
        )
        if isinstance(result, int):
            return result
    except ConfigError as e:
        click.echo(f"error: {e}", err=True)
        return 2
    except click.Abort as e:
        if not isinstance(e.__cause__, KeyboardInterrupt):
            raise e
    except click.ClickException as e:
        e.show()
        return e.exit_code

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
