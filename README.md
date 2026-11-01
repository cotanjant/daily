# daily

`daily` is a small collection of everyday command-line tools written in Rust.
One executable provides commands for downloading files, setting reminders,
managing files, and working with dates and durations.

## Install

Download a prebuilt executable from the
[latest GitHub release](https://github.com/cotanjant/daily/releases/latest), or
install Rust and Cargo to build from source:

The release binaries target Linux x86_64, macOS on Apple silicon, and Windows
x86_64. On Linux and macOS, run `chmod +x daily-*` on the downloaded binary
and place it in a directory on your `PATH`.

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
daily remind "Take a break" --at "YYYY-MM-DDTHH:MM:SS+03:00"
daily file copy notes.txt notes-backup.txt
daily file mkdir output/reports
daily time duration 3665
```

Replace the reminder timestamp with a future RFC 3339 time. Use `daily --help`
or `daily <command> --help` for options. Interactive downloads show transferred
size and speed. When the server reports the total size, the display also shows
a progress bar and percentage. Existing files prompt before replacement; pass
`--yes` to replace without prompting. A failed transfer leaves an existing
destination intact and removes its temporary file when possible.

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
release binaries for Linux x86_64, macOS on Apple silicon, and Windows x86_64. A
pushed `v*` tag builds the binaries and creates a GitHub Release. Set the release
version in `Cargo.toml`, run `cargo check` to update `Cargo.lock`, and use that
version in the tag.
Replace the example version below with the version being released; do not reuse
a published tag.

```bash
git tag v0.1.1
git push origin v0.1.1
```
