#!/usr/bin/env python3
"""Capture the running Cactus Launcher window into docs/screenshots/.

Usage:
    python3 scripts/screenshot.py <name>          # e.g. home, instance, browse, tuneup
    python3 scripts/screenshot.py <name> --crop   # trim the window's drop shadow

Navigate the app to the view you want first, then run this — a desktop window
can't be auto-navigated, so capture one view per run. Requires macOS and Screen
Recording permission for your terminal (System Settings → Privacy & Security →
Screen Recording) or the capture will be blank.
"""
import subprocess
import sys
from pathlib import Path

try:
    import Quartz
except ImportError:
    sys.exit("PyObjC/Quartz not available. Install with: python3 -m pip install --user pyobjc-framework-Quartz")

OUT_DIR = Path(__file__).resolve().parent.parent / "docs" / "screenshots"


def find_window():
    """Return the (id, w, h) of the largest on-screen Cactus Launcher window."""
    wins = Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
        Quartz.kCGNullWindowID,
    )
    candidates = []
    for w in wins:
        owner = str(w.get("kCGWindowOwnerName") or "")
        name = str(w.get("kCGWindowName") or "")
        if "cactus" in owner.lower() or "cactus" in name.lower():
            bounds = w.get("kCGWindowBounds", {})
            area = bounds.get("Width", 0) * bounds.get("Height", 0)
            candidates.append((area, w.get("kCGWindowNumber"), int(bounds.get("Width", 0)), int(bounds.get("Height", 0))))
    if not candidates:
        return None
    _, win_id, width, height = max(candidates)
    return win_id, width, height


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    crop = "--crop" in sys.argv
    if not args:
        sys.exit("Usage: python3 scripts/screenshot.py <name> [--crop]")
    name = args[0]

    found = find_window()
    if not found:
        sys.exit(
            "No Cactus Launcher window found on screen.\n"
            "Make sure the app is running (bun run tauri dev), visible, and on the current Space."
        )
    win_id, width, height = found

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    out = OUT_DIR / f"{name}.png"

    # -l <id>: capture just that window · -o: no shadow · -x: no capture sound
    cmd = ["screencapture", f"-l{win_id}", "-o", "-x", str(out)]
    if crop:
        cmd.remove("-o")  # keep the shadow off anyway; -o already omits it
    subprocess.run(cmd, check=True)
    print(f"Captured {width}x{height} window → {out.relative_to(OUT_DIR.parent.parent)}")


if __name__ == "__main__":
    main()
