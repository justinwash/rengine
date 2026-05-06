from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from pywinauto import Desktop, keyboard, mouse


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--keep-open", action="store_true")
    return parser.parse_args()


def run_cmd(command: list[str], cwd: Path) -> None:
    proc = subprocess.run(command, cwd=str(cwd), check=False)
    if proc.returncode != 0:
        raise RuntimeError(f"command failed: {' '.join(command)}")


def wait_for_window(title_regex: str, timeout: float):
    start = time.time()
    while time.time() - start < timeout:
        windows = Desktop(backend="win32").windows(title_re=title_regex)
        if windows:
            return windows[0]
        time.sleep(0.2)
    raise TimeoutError(f"window not found: {title_regex}")


def click_ratio(window, x_ratio: float, y_ratio: float) -> None:
    rect = window.rectangle()
    x = int(rect.left + rect.width() * x_ratio)
    y = int(rect.top + rect.height() * y_ratio)
    mouse.click(coords=(x, y))


def send_keys(window, keys: str) -> None:
    try:
        window.type_keys(keys, set_foreground=False)
    except Exception:
        keyboard.send_keys(keys, vk_packet=False)


def read_events(path: Path) -> list[str]:
    if not path.exists():
        return []
    return [line.strip() for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def assert_sequence(events: list[str]) -> None:
    if "shortcut:gizmo:scale" not in events:
        raise AssertionError("missing shortcut:gizmo:scale event")
    if "shortcut_blocked:KeyW:text_input" not in events:
        raise AssertionError("missing shortcut_blocked:KeyW:text_input event")
    if "shortcut:gizmo:rotate" not in events:
        raise AssertionError("missing shortcut:gizmo:rotate event")
    if "text_owner:inspector" not in events:
        raise AssertionError("missing text_owner:inspector event")
    if "text_owner:none" not in events:
        raise AssertionError("missing text_owner:none event")



def run_suite(repo_root: Path, skip_build: bool, timeout: float, keep_open: bool) -> int:
    if sys.platform != "win32":
        raise RuntimeError("windows desktop automation is only supported on win32")

    if not skip_build:
        run_cmd(["cargo", "build", "-p", "rengine-editor"], repo_root)

    exe = repo_root / "target" / "debug" / "rengine-editor.exe"
    if not exe.exists():
        raise FileNotFoundError(f"editor executable not found: {exe}")

    temp_dir = Path(tempfile.mkdtemp(prefix="rengine-ui-auto-"))
    log_path = temp_dir / "events.log"
    env = os.environ.copy()
    env["RENGINE_EDITOR_AUTOMATION_LOG"] = str(log_path)

    proc = subprocess.Popen([str(exe)], cwd=str(repo_root), env=env)
    try:
        window = wait_for_window(r".*Rengine Editor.*", timeout)
        try:
            window.restore()
            window.set_focus()
        except Exception:
            pass
        time.sleep(0.5)

        click_ratio(window, 0.50, 0.58)
        time.sleep(0.2)
        send_keys(window, "r")
        time.sleep(0.25)

        click_ratio(window, 0.88, 0.23)
        time.sleep(0.25)
        send_keys(window, "w")
        time.sleep(0.25)
        send_keys(window, "abc")
        time.sleep(0.25)

        click_ratio(window, 0.50, 0.58)
        time.sleep(0.25)
        send_keys(window, "e")
        time.sleep(0.5)

        events = read_events(log_path)
        assert_sequence(events)
        print("PASS: focus/shortcut automation checks passed")
        print("Events:")
        for event in events:
            print(f"  {event}")

        if keep_open:
            print("Editor is still open because --keep-open was passed")
            return 0

        send_keys(window, "%{F4}")
        proc.wait(timeout=10)
        return 0
    finally:
        if proc.poll() is None and not keep_open:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()


if __name__ == "__main__":
    args = parse_args()
    try:
        code = run_suite(args.repo_root, args.skip_build, args.timeout, args.keep_open)
    except Exception as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        sys.exit(1)
    sys.exit(code)
