# Project architecture

`daily` is a mixed Python/Rust package:

- `src/lib.rs` implements Rust-backed utility functions and exposes them through
  PyO3 as `daily._daily`.
- `python/daily/__init__.py` defines the public `daily` namespace and re-exports
  Rust functions.
- `python/daily/reminders.py` handles persistent reminders through native OS
  schedulers and desktop notification tools. `python/daily/_notify.py` is the
  scheduled worker entry point.
- `Cargo.toml`, `pyproject.toml`, and `Cargo.lock` define the Rust crate, Python
  package, and pinned Rust dependency graph. Maturin builds the mixed project.
- `website/` contains the static Astro site, with project, install, API, and
  documentation pages. Dependabot checks the Cargo, Python, npm, and GitHub
  Actions dependency sources each week.
- `README.md` is the project overview; `docs/usage.md` explains public APIs.

Use Rust for synchronous, portable operations that benefit from the native
extension (downloads, filesystem operations, and date/time helpers). Use Python
for platform orchestration where standard-library process and OS integration
are a better fit, such as reminder scheduling. Keep user-facing functions
available from `import daily` regardless of which language implements them.
