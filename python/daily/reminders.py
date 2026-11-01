"""Persistent desktop reminders backed by each operating system's scheduler."""

from __future__ import annotations

import datetime as dt
import json
import os
import plistlib
import shutil
import subprocess
import sys
import time
import uuid
from pathlib import Path
from typing import Any


def _data_dir() -> Path:
    if sys.platform == "win32":
        root = Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData/Local"))
    elif sys.platform == "darwin":
        root = Path.home() / "Library/Application Support"
    else:
        root = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share"))
    result = root / "daily"
    result.mkdir(parents=True, exist_ok=True)
    return result


def _store_path() -> Path:
    return _data_dir() / "reminders.json"


def _read() -> dict[str, dict[str, Any]]:
    path = _store_path()
    if not path.exists():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise RuntimeError(f"could not read reminder store {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise RuntimeError(f"invalid reminder store: {path}")
    return data


def _write(data: dict[str, dict[str, Any]]) -> None:
    path = _store_path()
    temporary = path.with_suffix(".json.tmp")
    temporary.write_text(json.dumps(data, indent=2), encoding="utf-8")
    temporary.replace(path)


def _run(command: list[str], *, description: str) -> None:
    try:
        result = subprocess.run(command, capture_output=True, text=True, check=False)
    except OSError as exc:
        raise RuntimeError(f"{description}: {exc}") from exc
    if result.returncode:
        message = result.stderr.strip() or result.stdout.strip() or "command failed"
        raise RuntimeError(f"{description}: {message}")


def _unit_name(reminder_id: str) -> str:
    return f"daily-reminder-{reminder_id}"


def _schedule(reminder_id: str, when: dt.datetime) -> None:
    worker = [sys.executable, "-m", "daily._notify", reminder_id]
    if sys.platform.startswith("linux"):
        if shutil.which("systemctl") is None:
            raise RuntimeError("persistent Linux reminders require systemd and systemctl")
        unit_dir = Path.home() / ".config/systemd/user"
        unit_dir.mkdir(parents=True, exist_ok=True)
        name = _unit_name(reminder_id)
        # systemd accepts a quoted executable path in ExecStart.
        executable = '"' + str(Path(worker[0]).resolve()).replace("\\", "\\\\").replace('"', '\\"') + '"'
        service = "\n".join(
            ["[Unit]", "Description=Daily reminder", "", "[Service]", "Type=oneshot",
             f"ExecStart={executable} -m daily._notify {reminder_id}", ""]
        )
        calendar = when.astimezone(dt.timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
        timer = "\n".join(
            ["[Unit]", "Description=Daily reminder timer", "", "[Timer]",
             f"OnCalendar={calendar}", "Persistent=true", f"Unit={name}.service", "",
             "[Install]", "WantedBy=timers.target", ""]
        )
        (unit_dir / f"{name}.service").write_text(service, encoding="utf-8")
        (unit_dir / f"{name}.timer").write_text(timer, encoding="utf-8")
        _run(["systemctl", "--user", "daemon-reload"], description="reloading user systemd units")
        _run(["systemctl", "--user", "enable", "--now", f"{name}.timer"], description="scheduling reminder")
    elif sys.platform == "darwin":
        label = _unit_name(reminder_id)
        launch_dir = Path.home() / "Library/LaunchAgents"
        launch_dir.mkdir(parents=True, exist_ok=True)
        local = when.astimezone()
        plist = {
            "Label": label,
            "ProgramArguments": worker,
            "StartCalendarInterval": {
                "Year": local.year, "Month": local.month, "Day": local.day,
                "Hour": local.hour, "Minute": local.minute, "Second": local.second,
            },
            "RunAtLoad": False,
        }
        path = launch_dir / f"{label}.plist"
        path.write_bytes(plistlib.dumps(plist))
        _run(["launchctl", "load", str(path)], description="scheduling reminder")
    elif sys.platform == "win32":
        local = when.astimezone()
        task_name = f"\\{_unit_name(reminder_id)}"
        schedule_time = local.replace(second=0, microsecond=0)
        if local > schedule_time:
            schedule_time += dt.timedelta(minutes=1)
        command = subprocess.list2cmdline(worker)
        _run(
            ["schtasks", "/Create", "/TN", task_name, "/SC", "ONCE", "/SD",
             schedule_time.strftime("%m/%d/%Y"), "/ST", schedule_time.strftime("%H:%M"),
             "/TR", command, "/F", "/IT"],
            description="scheduling reminder",
        )
    else:
        raise RuntimeError(f"persistent reminders are not supported on {sys.platform}")


def _unschedule(reminder_id: str) -> None:
    name = _unit_name(reminder_id)
    if sys.platform.startswith("linux"):
        subprocess.run(["systemctl", "--user", "disable", "--now", f"{name}.timer"], capture_output=True, check=False)
        unit_dir = Path.home() / ".config/systemd/user"
        for suffix in ("timer", "service"):
            (unit_dir / f"{name}.{suffix}").unlink(missing_ok=True)
        _run(["systemctl", "--user", "daemon-reload"], description="reloading user systemd units")
    elif sys.platform == "darwin":
        path = Path.home() / "Library/LaunchAgents" / f"{name}.plist"
        if path.exists():
            subprocess.run(["launchctl", "unload", str(path)], capture_output=True, check=False)
            path.unlink(missing_ok=True)
    elif sys.platform == "win32":
        subprocess.run(["schtasks", "/Delete", "/TN", f"\\{name}", "/F"], capture_output=True, check=False)

def remind_me(message: str, when: dt.datetime) -> str:
    """Schedule a persistent desktop notification and return its reminder ID.

    ``when`` must be a timezone-aware datetime. The reminder is delivered after
    the current Python process exits, when the user's desktop session is active.
    """
    if not isinstance(message, str) or not message.strip():
        raise ValueError("message must be a non-empty string")
    if not isinstance(when, dt.datetime) or when.tzinfo is None or when.utcoffset() is None:
        raise ValueError("when must be a timezone-aware datetime")
    if when <= dt.datetime.now(dt.timezone.utc):
        raise ValueError("when must be in the future")
    reminder_id = uuid.uuid4().hex
    record = {"id": reminder_id, "message": message, "when": when.isoformat()}
    data = _read()
    data[reminder_id] = record
    _write(data)
    try:
        _schedule(reminder_id, when)
    except Exception:
        try:
            _unschedule(reminder_id)
        except Exception:
            pass
        data.pop(reminder_id, None)
        _write(data)
        raise
    return reminder_id


def list_reminders() -> list[dict[str, Any]]:
    """Return pending reminders, ordered by scheduled time."""
    return sorted(_read().values(), key=lambda item: item["when"])


def cancel_reminder(reminder_id: str) -> bool:
    """Cancel a pending reminder; return False if the ID was not found."""
    data = _read()
    if reminder_id not in data:
        return False
    _unschedule(reminder_id)
    del data[reminder_id]
    _write(data)
    return True


def _deliver(reminder_id: str) -> None:
    data = _read()
    reminder = data.get(reminder_id)
    if reminder is None:
        return
    when = dt.datetime.fromisoformat(reminder["when"])
    while True:
        remaining = (when - dt.datetime.now(when.tzinfo)).total_seconds()
        if remaining <= 0:
            break
        time.sleep(min(remaining, 30))
    title = "Reminder"
    message = reminder["message"]
    if sys.platform.startswith("linux"):
        _run(["notify-send", title, message], description="showing notification")
    elif sys.platform == "darwin":
        script = f"display notification {json.dumps(message)} with title {json.dumps(title)}"
        _run(["osascript", "-e", script], description="showing notification")
    elif sys.platform == "win32":
        escaped = message.replace("'", "''")
        script = (
            "Add-Type -AssemblyName System.Windows.Forms; "
            "$n=New-Object System.Windows.Forms.NotifyIcon; "
            "$n.Icon=[System.Drawing.SystemIcons]::Information; "
            f"$n.BalloonTipTitle='Reminder'; $n.BalloonTipText='{escaped}'; "
            "$n.Visible=$true; $n.ShowBalloonTip(10000); Start-Sleep -Seconds 11; $n.Dispose()"
        )
        _run(["powershell", "-NoProfile", "-WindowStyle", "Hidden", "-Command", script], description="showing notification")
    data.pop(reminder_id, None)
    _write(data)
    name = _unit_name(reminder_id)
    if sys.platform.startswith("linux"):
        unit_dir = Path.home() / ".config/systemd/user"
        subprocess.run(["systemctl", "--user", "disable", f"{name}.timer"], capture_output=True, check=False)
        for suffix in ("timer", "service"):
            (unit_dir / f"{name}.{suffix}").unlink(missing_ok=True)
        subprocess.run(["systemctl", "--user", "daemon-reload"], capture_output=True, check=False)
    elif sys.platform == "darwin":
        path = Path.home() / "Library/LaunchAgents" / f"{name}.plist"
        subprocess.run(["launchctl", "unload", str(path)], capture_output=True, check=False)
        path.unlink(missing_ok=True)
    elif sys.platform == "win32":
        subprocess.run(["schtasks", "/Delete", "/TN", f"\\{name}", "/F"], capture_output=True, check=False)
