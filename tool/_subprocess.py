import os
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class CmdResult:
    argv: list[str]
    cwd: Path
    returncode: int
    stdout: str
    stderr: str


class CmdError(RuntimeError):
    def __init__(self, result: CmdResult) -> None:
        msg = (
            "Command failed\n"
            f"  cwd: {result.cwd}\n"
            f"  argv: {result.argv}\n"
            f"  exit: {result.returncode}\n"
        )
        if result.stdout.strip():
            msg += f"\nstdout:\n{result.stdout.rstrip()}\n"
        if result.stderr.strip():
            msg += f"\nstderr:\n{result.stderr.rstrip()}\n"
        super().__init__(msg)
        self.result = result


def run_with_return(
    argv: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    check: bool = True,
) -> CmdResult:
    merged_env = os.environ.copy()
    if env is not None:
        merged_env.update(env)

    completed = subprocess.run(
        argv,
        cwd=str(cwd),
        env=merged_env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    result = CmdResult(
        argv=argv,
        cwd=cwd,
        returncode=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )

    if check and result.returncode != 0:
        raise CmdError(result)

    return result


def run(
    argv: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    check: bool = False,
) -> None:
    merged_env = os.environ.copy()
    if env is not None:
        merged_env.update(env)

    subprocess.run(argv, cwd=str(cwd), env=merged_env, check=check)


def run_passthrough(
    argv: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
) -> int:
    merged_env = os.environ.copy()
    if env is not None:
        merged_env.update(env)

    completed = subprocess.run(argv, cwd=str(cwd), env=merged_env)
    return completed.returncode
