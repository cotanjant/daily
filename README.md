# daily

`daily` is a collection of ready-to-use Python helpers backed by Rust. It
currently includes downloads with terminal progress, persistent reminders,
common file operations, and date/time helpers.

## Install from this checkout

Python 3.10 or newer and a Rust toolchain are required. From the project root:

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install maturin
maturin develop
```

`maturin develop` builds the Rust extension and installs it into the active
environment. To build a wheel instead, run `maturin build --release`; install a
wheel from `target/wheels/` with `python -m pip install <wheel-path>`.

## Quick start

```python
from datetime import datetime, timedelta, timezone
from pathlib import Path

import daily

# Shows a colored progress bar in a terminal. If the file exists, asks before
# replacing it. Returns None if you answer no.
downloaded = daily.download_url(
    "https://example.com/archive.zip",
    Path("archive.zip"),
)

daily.create_directory(Path("output/reports"))

reminder_id = daily.remind_me(
    "Check the oven",
    datetime.now(timezone.utc) + timedelta(minutes=20),
)
print(daily.list_reminders())
daily.cancel_reminder(reminder_id)
```

## Available helpers

- `download_url(url, path, overwrite=None)` downloads HTTP or HTTPS resources.
  Interactive terminals show a colored progress bar, percentage, and
  transferred size in MB and average transfer speed in MB/s if the server
  reports the file size. If a network interruption occurs, the partial file is
  removed and the error recommends checking the connection and retrying.
  Existing destinations prompt for `y` or `n`;
  pass `overwrite=True` to replace without prompting, or `False` to decline
  without prompting. A declined prompt returns `None`.
- `remind_me(message, when)`, `list_reminders()`, and
  `cancel_reminder(reminder_id)` schedule, inspect, and cancel desktop
  notifications. `when` must be a future, timezone-aware `datetime`.
- `copy_file`, `move_file`, `rename_file`, and `create_directory` cover common
  file operations. Existing files are protected by default for copy, move, and
  rename; pass `overwrite=True` to replace them.
- `format_time`, `parse_time`, and `format_duration` provide simple formatting,
  parsing, and duration helpers.

Paths accept strings and `pathlib.Path` objects. See the [usage guide](docs/usage.md)
for function signatures, examples, and platform notes. The project architecture
and feature development workflow are in [docs/architecture.md](docs/architecture.md)
and [AGENTS.md](AGENTS.md) for the feature workflow.

YouTube downloads are not included. Supporting YouTube reliably requires a
dedicated downloader implementation and ongoing compatibility work.

## Website

The Astro site lives in `website/`. Run it locally with Node.js:

```bash
cd website
npm install
npm run dev
```

Build the static site with `npm run build`.
