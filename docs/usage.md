# Usage guide

The package imports as `daily`. Build and install it into the active Python
environment with `maturin develop` from the repository root; setup steps are in
the [README](../README.md).

## Download a file

```python
from pathlib import Path
import daily

result = daily.download_url(
    "https://example.com/report.pdf",
    Path("downloads/report.pdf"),
)
if result is None:
    print("Download cancelled")
```

When the destination already exists and stdin/stdout are terminals, `daily`
prints a colored prompt with the full destination path. Answer `y` to replace
the file or `n` to cancel. If the process is noninteractive, the default raises
`FileExistsError` instead of waiting for input. Set `overwrite=True` to replace
automatically or `overwrite=False` to refuse automatically.

The colored progress bar is only printed to an interactive terminal. It shows
a percentage, transferred size in MB, and average speed in MB/s when the server
provides a content length. Without a content length, it shows the received size
and average speed. In scripts that redirect output, the download remains
quiet.

Downloads are written to a temporary file and moved into place only after a
successful transfer. If the connection is interrupted or the response ends
before its advertised size, the temporary file is removed. The raised
`ConnectionError` explains that cleanup occurred and recommends checking the
connection before retrying.

## Reminders

```python
from datetime import datetime, timedelta, timezone
import daily

reminder_id = daily.remind_me(
    "Take a break",
    datetime.now(timezone.utc) + timedelta(minutes=30),
)

for reminder in daily.list_reminders():
    print(reminder["id"], reminder["when"], reminder["message"])

daily.cancel_reminder(reminder_id)
```

Reminders are persisted and delegated to the current user's OS scheduler. The
user's Python environment must remain installed so the scheduled worker can
run. Linux requires a systemd user session and `notify-send`; macOS uses
LaunchAgents and Notification Center; Windows uses Task Scheduler and a desktop
notification, with up to one minute of scheduler delay. Delivery requires an
available desktop session.

## Files and time

```python
from pathlib import Path
import daily

daily.copy_file(Path("input.txt"), Path("backup.txt"))
daily.move_file(Path("backup.txt"), Path("archive/backup.txt"))
daily.rename_file(Path("archive/backup.txt"), Path("archive/input.txt"))
daily.create_directory(Path("output/reports"), parents=True, exist_ok=True)

print(daily.format_time("2026-11-01T12:30:00+00:00", "%Y-%m-%d %H:%M"))
print(daily.parse_time("2026-11-01 12:30", "%Y-%m-%d %H:%M"))
print(daily.format_duration(3665))  # 1h 1m 5s
```

`parse_time` accepts a naive datetime string and interprets it as UTC. Its
`format` argument uses chrono/strftime directives. File functions return the
destination path on success and raise Python exceptions for filesystem errors.
