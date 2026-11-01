# Instructions for coding agents

This repository is a mixed Python/Rust library. Read `README.md` and the
relevant pages in `docs/` before making public API changes.

## Project conventions

- **DO NOT FORGET TO UPDATE DOCUMENTATION AFTER ADDING NEW FEATURE.** Update the
  README and the relevant usage or architecture docs in the same change.
- **YOU HAVE TO COMMIT AFTER ADDING A NEW FEATURE.** Include the feature and its
  documentation in a focused commit.
- Use clear Conventional Commit subjects such as `feat(download): show MB
  progress` or `docs: explain reminder setup`. Keep the subject concise and
  imperative; use the body to explain important behavior or reasoning. Avoid
  vague subjects such as `updates` or `misc fixes`, and keep unrelated changes
  in separate commits.
- Python imports the package as `daily`; the Rust extension is `daily._daily`.
- Rust/PyO3 code belongs in `src/lib.rs`. Python orchestration belongs under
  `python/daily/`. Re-export public helpers from `python/daily/__init__.py`.
- Keep Python-facing APIs Pythonic and document argument defaults, return
  values, cancellation, and errors.
- Use safe defaults for overwriting files and other destructive actions.
- Keep terminal output out of redirected/noninteractive runs unless explicitly
  requested by the API.
- Update `README.md` and `docs/usage.md` whenever public behavior changes.
- Update `website/` pages when installation, API, or user-facing behavior
  changes; keep the site copy aligned with the README and usage guide.
- Keep `Cargo.toml` and `Cargo.lock` in sync when Rust dependencies change.
- Keep `website/package.json` and its lockfile in sync when website dependencies
  change.
- Do not delete or overwrite unrelated user files or generated download files.

## Feature workflow

Follow [docs/feature-workflow.md](docs/feature-workflow.md): inspect the tree,
define the public Python behavior, select the Rust or Python implementation
layer, implement errors and edge cases, expose the API, update docs, format and
run the relevant requested checks, then review the diff. Do not add or run
tests unless the user asks to test or verify the implementation. Be explicit
about which platforms were actually verified.
