# Command guide

`daily` is one Rust executable with subcommands. Build it with `cargo install
--path .`, or run it from the repository with `cargo run -- <command>`.

## Download

```bash
daily download https://example.com/report.pdf downloads/report.pdf
daily download https://example.com/report.pdf report.pdf --yes
```

In an interactive terminal, the command shows transferred MB and sampled speed
in MB/s. When the server reports a nonzero total size, it also shows a colored
progress bar and percentage. It hides progress output when stdout is redirected.
Existing destinations prompt for `y` or `n`; `--yes` replaces without
prompting. In a noninteractive process an existing file is preserved and the
command exits with an error unless `--yes` is set.

Downloads stream to a uniquely named temporary file beside the destination.
Network failures and truncated transfers remove the partial file and suggest
checking the connection. If cleanup fails, the error gives the partial file's
path. The destination is replaced only after the transfer completes.

## Remind

```bash
daily remind "Take a break"
```

Without `--at`, the command opens a date and time picker in the terminal. It
starts five minutes from the current local time. Use the left and right arrow
keys to choose the year, month, day, hour, or minute wheel; use up and down to
change its value. Press Enter to set the reminder, or Esc or `q` to cancel. The
year wheel covers the current year through ten years ahead. Times use the local
timezone; nonexistent times during a timezone change are rejected.

For scripts or a noninteractive process, pass a future RFC 3339 timestamp with
an explicit timezone:

```bash
daily remind "Take a break" --at "2030-01-01T09:00:00+03:00"
```

The command waits in the foreground, so leave it running until the notification
appears. Linux requires `notify-send`; macOS uses `osascript`; Windows uses
PowerShell. An active desktop notification session is required. Closing the
process cancels the reminder.

## Files

```bash
daily file copy source.txt backup.txt
daily file move backup.txt archive/backup.txt
daily file rename archive/backup.txt archive/source.txt
daily file mkdir output/reports
```

Copy, move, and rename prompt before replacing an existing file. Add `--yes` to
replace without prompting. If replacement fails, the old destination is kept;
if Windows cannot restore it, the error reports the backup path. Copy supports
different filesystems. Move and rename require the source and destination to
be on the same filesystem, and the two paths must be different. `mkdir` creates
missing parents.

## Time

```bash
daily time format 2026-11-01T12:30:00+00:00 "%Y-%m-%d %H:%M"
daily time parse "2026-11-01 12:30" "%Y-%m-%d %H:%M"
daily time duration 3665
```

`format` accepts RFC 3339 and chrono/strftime directives. `parse` interprets a
naive datetime as UTC. `duration` prints a compact value such as `1h 1m 5s`.
