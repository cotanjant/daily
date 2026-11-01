# daily

`daily` is a small collection of everyday command-line tools written in Rust.
One executable provides commands for downloading files, setting reminders,
managing files, and working with dates and durations.

## Install

Install Rust, then build and install the executable from this checkout:

```bash
cargo install --path .
```

Or run commands without installing:

```bash
cargo run -- --help
```

## Commands

```bash
daily download https://example.com/report.pdf report.pdf
daily remind "Take a break" --at 2026-11-01T18:30:00+03:00
daily file copy notes.txt notes-backup.txt
daily file mkdir output/reports
daily time duration 3665
```

Use `daily --help` or `daily <command> --help` for options. Downloads show a
colored progress bar, percentage, size in MB, and speed in MB/s in interactive
terminals. Existing files prompt before replacement; pass `--yes` to replace
without prompting. Interrupted downloads remove the partial file and report
that the connection should be checked before retrying.

`daily remind` waits in the foreground until the requested RFC 3339 time. Keep
the process running. It uses `notify-send` on Linux and `osascript` on macOS;
Windows support uses PowerShell. See the [usage guide](docs/usage.md) for
platform details.

## Website

The Astro site in `website/` introduces the commands and links to their docs.
Run it with `cd website && npm install && npm run dev`; build it with
`npm run build`.

## CI and releases

Pull requests and pushes to `main` run formatting and compile checks and build
release binaries for Linux, macOS, and Windows. Push a version tag such as
`v0.1.0` to build the same binaries and create a GitHub Release with the
platform executables attached. Keep the tag version in sync with `Cargo.toml`:

```bash
git tag v0.1.0
git push origin v0.1.0
```
