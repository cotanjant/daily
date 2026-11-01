# Instructions for coding agents

This repository is a mixed Python/Rust library. Read `README.md` and the
relevant pages in `docs/` before making public API changes.

## Project conventions

- **DO NOT FORGET TO UPDATE DOCUMENTATION AFTER ADDING NEW FEATURE.** Update the
  README and the relevant usage or architecture docs in the same change.
- **AFTER UPDATING DOCUMENTATION, UPDATE THE WEBSITE.** Keep website pages
  aligned with any README or docs changes.
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

1. Inspect the working tree, relevant code, public exports, docs, and manifests.
   Preserve unrelated user changes and generated files.
2. Define the Python API: name, arguments, defaults, return values, and errors.
   Use safe defaults for destructive actions and keep Rust details out of the
   public interface.
3. Choose the implementation layer. Put portable native helpers in
   `src/lib.rs`; put Python and OS orchestration in `python/daily/`. Add Rust
   dependencies only when they improve correctness or maintenance, and keep
   `Cargo.toml` and `Cargo.lock` in sync.
4. Handle validation, failures, cancellation, noninteractive use, and platform
   constraints. Keep terminal output quiet when stdout is not a TTY unless the
   API explicitly requests it.
5. Export new public helpers through `python/daily/__init__.py`.
6. Update the README and relevant files in `docs/`, then update the matching
   `website/` pages. Keep examples consistent across all three.
7. Format and run the checks requested for the task. Do not add or run tests
   unless the user asks to test or verify the implementation. State which
   platforms were actually checked.
8. Review `git diff --check` and the final diff. Commit each new feature with a
   focused Conventional Commit subject, such as
   `feat(download): show MB progress`; describe important behavior in the
   commit body. Keep unrelated changes in separate commits.

For each new helper, document whether it prompts, writes files, needs a desktop
session, or relies on an external OS command. Do not silently change existing
argument behavior.
