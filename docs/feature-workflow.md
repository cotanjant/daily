# Adding a feature

Use this workflow when extending `daily`:

1. **Inspect the current package.** Read the relevant code, public exports,
   README, docs, and manifests. Check the working tree before editing so user
   changes and generated artifacts remain intact.
2. **Define the Python-facing behavior.** Choose a short Pythonic function
   name, argument types, defaults, return value, and error behavior. Keep Rust
   implementation details out of the Python API. For potentially destructive
   actions, make the default safe and document how callers opt in.
3. **Choose the implementation layer.** Put portable performance-sensitive
   operations in `src/lib.rs` and expose them with PyO3. Put OS orchestration,
   scheduler integration, and Python-specific behavior in `python/daily/`.
   Add an external Rust dependency only when it materially improves
   correctness or maintainability, and update `Cargo.toml` and `Cargo.lock`.
4. **Implement complete behavior.** Handle input validation, filesystem and
   network failures, cancellation, noninteractive use, and platform-specific
   constraints. Keep terminal UI silent when output is not a TTY unless the
   API explicitly requests otherwise.
5. **Expose the function.** Rust functions must be registered in the PyO3
   module and re-exported from `python/daily/__init__.py`. Pure Python public
   functions should also be re-exported there.
6. **Document the change.** Update the README summary and add details and
   runnable examples to `docs/usage.md`. If the architecture or development
   process changes, update the corresponding docs and this workflow.
7. **Format and verify.** Run `cargo fmt`, `cargo check`, and Python syntax
   compilation for changed modules. Build with `maturin develop` in the active
   environment when available. Add or run tests when the task or repository
   instructions request them; do not claim cross-platform behavior that has
   only been checked on one platform.
8. **Review the final diff.** Check `git diff --check`, inspect the changed
   files, and report public API changes, checks performed, and any platform or
   dependency limitation.

For a new public helper, include a minimal example and mention whether it
prompts, writes files, needs a desktop session, or relies on an external OS
command. Avoid silently changing the behavior of existing arguments.
