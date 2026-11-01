# Project layout

`daily` is a Rust command-line application. `src/main.rs` defines its command
tree and implements download, reminder, file, and time operations. Cargo
produces the `daily` executable and installs it with `cargo install --path .`.

The `website/` directory contains a static Astro site with installation,
command, and documentation pages. `.github/dependabot.yml` checks Cargo, npm,
and GitHub Actions dependencies weekly.
