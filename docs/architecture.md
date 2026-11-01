# Project layout

`daily` is a Rust command-line application. `src/main.rs` defines its command
tree and implements download, reminder, file, and time operations. Cargo builds
the `daily` executable; `cargo install --path .` installs it from a checkout.

The `website/` directory contains a static Astro site with installation,
command, and documentation pages. GitHub Actions checks formatting and
compilation, builds binaries for Linux x86_64, macOS on Apple silicon, and
Windows x86_64, then publishes them when a `v*` tag is pushed.
`.github/dependabot.yml` checks Cargo, npm, and GitHub Actions dependencies
weekly.
