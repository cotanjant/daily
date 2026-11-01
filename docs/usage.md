# Command guide

`daily` is one Rust executable with subcommands. Build it with `cargo install
--path .`, or run it from the repository with `cargo run -- <command>`.

## Download

```bash
daily download https://example.com/report.pdf downloads/report.pdf
daily download https://example.com/report.pdf report.pdf --yes
```

The command shows a colored progress bar, percentage, transferred MB, and
current sampled speed in MB/s in an interactive terminal. It stays quiet when
output is redirected. Existing destinations prompt for `y` or `n`; `--yes`
replaces without prompting. In a noninteractive process an existing file is
preserved and the command exits with an error unless `--yes` is set.

Downloads stream to a temporary sibling file. A failed or truncated transfer
removes that file and tells the user to check the connection and retry. On
success the completed file is moved into place.

## Remind

```bash
daily remind "Take a break" --at 2026-11-01T18:30:00+03:00
```

`--at` requires an RFC 3339 timestamp with a timezone and a future time. The
command waits in the foreground, so leave it running until the notification
appears. Linux requires `notify-send`; macOS uses `osascript`; Windows uses
PowerShell. Closing the process cancels the reminder.

## Files

```bash
daily file copy source.txt backup.txt
daily file move backup.txt archive/backup.txt
daily file rename archive/backup.txt archive/source.txt
daily file mkdir output/reports
```

Copy, move, and rename prompt before replacing an existing file. Add `--yes` to
replace without prompting. `mkdir` creates missing parents.

## Time

```bash
daily time format 2026-11-01T12:30:00+00:00 "%Y-%m-%d %H:%M"
daily time parse "2026-11-01 12:30" "%Y-%m-%d %H:%M"
daily time duration 3665
```

`format` accepts RFC 3339 and chrono/strftime directives. `parse` interprets a
naive datetime as UTC. `duration` prints a compact value such as `1h 1m 5s`.
