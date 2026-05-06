# Windows Editor UI Automation

This suite runs end-to-end desktop automation against the native editor window and validates input-focus behavior from emitted automation events.

## Scope

The suite validates:

1. Viewport gizmo hotkeys fire when the viewport is focused.
2. Gizmo hotkeys are blocked while text input is active.
3. Text input owner transitions are emitted and observable.

## Prerequisites

1. Windows.
2. Python 3.10+.
3. Rust toolchain.
4. An active interactive desktop session (not headless, RDP minimized, or locked).
5. Installed Python dependencies:

```powershell
pip install -r editor/automation/windows_ui/requirements.txt
```

## Run

From the repository root:

```powershell
python editor/automation/windows_ui/run_focus_suite.py
```

Options:

1. `--skip-build` skips `cargo build -p rengine-editor`.
2. `--timeout <seconds>` controls window wait timeout.
3. `--keep-open` leaves the editor window running after checks.

## GitHub Actions (Self-Hosted)

Workflow file: `.github/workflows/editor-windows-ui-automation.yml`

Runner requirements:

1. Labels include `self-hosted`, `windows`, and `rengine-ui`.
2. Active interactive desktop session is available while the job runs.
3. Environment variable `RENGINE_UI_DESKTOP_READY=1` is set on the runner host.

The workflow is manual (`workflow_dispatch`) so desktop timing can be controlled.

## Event Source

The editor writes automation events to the path in `RENGINE_EDITOR_AUTOMATION_LOG`.
The Python suite sets this variable to a temporary file for each run.
