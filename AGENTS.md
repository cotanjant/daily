# Instructions for coding agents

This repository is a Rust command-line app collection. Read `README.md` and
the relevant pages in `docs/` before changing commands or user-facing behavior.

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
- Implement commands in `src/main.rs` with the existing Clap subcommand tree.
- Keep command names and flags concise; document them through Clap help and
  `docs/usage.md`.
- Use safe defaults for overwriting files and other destructive actions.
- Keep terminal output out of redirected/noninteractive runs unless explicitly
  requested by the command.
- Update `README.md` and `docs/usage.md` whenever command behavior changes.
- Update `website/` pages when installation, CLI, or user-facing behavior
  changes; keep the site copy aligned with the README and usage guide.
- Keep `Cargo.toml` and `Cargo.lock` in sync when Rust dependencies change.
- Keep `website/package.json` and its lockfile in sync when website dependencies
  change.
- Do not delete or overwrite unrelated user files or generated download files.

## Feature workflow

1. Inspect the working tree, command handlers, docs, and manifests.
   Preserve unrelated user changes and generated files.
2. Define the CLI command, arguments, defaults, output, and error behavior.
   Use safe defaults for destructive actions.
3. Implement the command in `src/main.rs`. Add Rust dependencies only when
   they improve correctness or maintenance, and keep `Cargo.toml` and
   `Cargo.lock` in sync.
4. Handle validation, failures, cancellation, noninteractive use, and platform
   constraints. Keep terminal output quiet when stdout is not a TTY unless the
   command explicitly requests it.
5. Update the README and relevant files in `docs/`, then update the matching
   `website/` pages. Keep examples consistent across all three.
6. Format and run the checks requested for the task. Do not add or run tests
   unless the user asks to test or verify the implementation. State which
   platforms were actually checked.
7. Review `git diff --check` and the final diff. Commit each new feature with a
   focused Conventional Commit subject, such as
   `feat(download): show MB progress`; describe important behavior in the
   commit body. Keep unrelated changes in separate commits.

For each new command, document whether it prompts, writes files, needs a
desktop session, or relies on an external OS command. Keep command behavior
consistent with its documented options.
