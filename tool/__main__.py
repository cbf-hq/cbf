from pathlib import Path
import sys

if __package__ in (None, ""):
    # `uv run tool` can execute this file as a script, where package-relative
    # imports are unavailable. Add the repo root to sys.path as a fallback.
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from tool.cli import main
else:
    from .cli import main


if __name__ == "__main__":
    raise SystemExit(main())
