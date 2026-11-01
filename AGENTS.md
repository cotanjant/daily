# Instructions for coding agents

This repository contains a Rust CLI and an Astro website. Read `README.md` and
the relevant pages in `docs/` before changing commands or user-facing behavior.
Keep this file for contributor instructions; the README, docs, and website are
for people using `daily`.

## Project conventions

- **DO NOT FORGET TO UPDATE DOCUMENTATION AFTER ADDING NEW FEATURE.** Update the
  README and relevant usage or architecture pages with the feature.
- **AFTER UPDATING DOCUMENTATION, UPDATE THE WEBSITE.** Keep its user-facing
  pages consistent with the README and docs.
- **YOU HAVE TO COMMIT AFTER ADDING A NEW FEATURE.** Use a focused commit with a
  Conventional Commit subject. Examples: `feat(download): show MB progress`,
  `fix(files): preserve destination on failed move`, and `docs: explain reminder setup`.
- Put CLI commands in the existing Clap tree in `src/main.rs`. Keep names and
  flags concise, and document them in `docs/usage.md`.
- Ask before overwriting files. In noninteractive runs, require `--yes` when a
  destination already exists.
- Keep progress and status output off stdout when it is redirected. Commands
  may still print requested results and report errors on stderr.
- Keep `Cargo.toml` and `Cargo.lock` in sync when Rust dependencies or package
  versions change. Keep `website/package.json` and its lockfile in sync when
  website dependencies change.
- Preserve unrelated user files and generated downloads.

## Feature workflow

Use this workflow for new commands and user-facing behavior. For bug fixes and
refactors, inspect callers and update docs when behavior or documented limits
change.

1. Inspect the working tree, command handlers, docs, and manifests. Preserve
   unrelated changes and generated files.
2. Define the command's arguments, defaults, output, errors, and platform
   requirements. Use safe defaults for overwriting or deleting files.
3. Make the smallest complete change in the existing project structure. Add a
   dependency only when it improves correctness or maintenance.
4. Handle invalid input, errors, cancellation, noninteractive use, and platform
   limits. Document prompts, file writes, desktop-session needs, and external
   OS commands.
5. Update the README and relevant pages in `docs/`, then update matching
   website pages. Keep command examples consistent.
6. Format and run checks requested for the task. Do not add or run tests unless
   the user asks to test or verify the implementation. State which platforms
   were checked.
7. Review `git diff --check` and the final diff. Commit each new feature with a
   concise Conventional Commit subject and a body that explains important
   behavior. Keep unrelated changes in separate commits.
