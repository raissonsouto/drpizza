# Agent Operating Guide

This file defines how AI coding agents should operate in this repository.

## Scope

- Repository: `drpizza`
- Product: terminal CLI for Dr. Pizza ordering and account flows
- Primary language: Rust 2021
- Main libraries: `clap`, `tokio`, `reqwest`, `serde`

## Repository Map

- `src/`: CLI application code
- `src/models/`: domain models (`menu`, `order`, `unit`, `user`)
- `README.md`: installation and high-level usage
- `DOCS.md`: command and flag reference
- `.github/workflows/ci.yml`: required CI checks
- `.github/workflows/release.yml`: tagged release workflow (`v*`)

## Agent Responsibilities

- Implement fixes and enhancements with minimal, focused changes.
- Preserve existing CLI compatibility for:
  - `pedir`
  - `menu`
  - `unidades`
  - `status` and `pedido`
  - `pedidos`
  - `perfil`
  - `enderecos`
- Update docs when behavior, flags, or user flow changes.
- Avoid unrelated refactors unless explicitly requested.

## Definition Of Done

- Code compiles.
- Formatting passes: `cargo fmt --check`.
- Lint passes with no warnings: `cargo clippy -- -D warnings`.
- Relevant documentation is updated.
- Final handoff includes summary, changed files, and validation commands.

## Working Rules

### Always

- Read relevant files before editing.
- Keep patches small and directly tied to the task.
- Reuse existing project patterns and naming style.
- Explain assumptions and residual risks in final output.

### Never

- Commit, push, tag, or release unless explicitly requested.
- Remove files or break compatibility without explicit approval.
- Change release pipeline behavior without task-level justification.

## Code Standards

### Style

- `cargo fmt` is the formatting source of truth.
- Final code must be clean under clippy with `-D warnings`.

### Naming

- Prefer descriptive, domain-aligned names.
- Avoid opaque abbreviations in public APIs and shared types.

### Error Handling

- Return errors with actionable context.
- Avoid `unwrap` and `expect` in normal execution paths.

### Logging

- Keep verbose API logging behind the `dev` feature.

### Performance

- Avoid redundant network calls.
- Preserve cache behavior (`~/.drpizza_menu_cache.json`) unless change is explicit.

## Validation Commands

- Required:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
- Recommended:
  - `cargo build --release`
  - `make build`
- Debug flow:
  - `make debug`
  - `./target/debug/drpizza --help`

## Testing Notes

- There is no dedicated unit/integration test suite currently.
- If adding tests, keep them deterministic and narrow in scope.

## Dependencies And Tooling

- Dependency manager: Cargo (`Cargo.toml`, `Cargo.lock`).
- Toolchain: Rust stable (`rust-version = "1.75"`).
- Build tools: Cargo and Make.
- External integration: Dr. Pizza API via `reqwest`.
- Local persistence:
  - `~/.drpizza`
  - `~/.drpizza_menu_cache.json`

## Security And Safety

- Never commit secrets, tokens, credentials, or personal data.
- Treat profile and address data as sensitive.
- Do not introduce writes outside expected local files without explicit need.
- Global install changes (for example `/usr/local/bin`) require explicit request.

## Documentation Policy

- Update `DOCS.md` for command, flag, and flow changes.
- Update `README.md` for setup or installation changes.
- In delivery summaries, include:
  - Problem
  - Solution
  - Compatibility impact
  - Validation performed

## Communication Expectations

- Be concise and technical.
- Surface tradeoffs only when decision-relevant.
- Ask clarifying questions only for real blockers or high-risk ambiguity.
- Final response must include:
  - What changed
  - Where it changed
  - How it was validated
  - Any remaining risks

## Task-Specific Notes

- Keep CLI UX text consistent with current project tone and language.
- Preserve aliases and flags unless explicitly asked to change them.
- For order-flow changes, review impact across:
  - `order.rs`
  - `orders.rs`
  - `menu.rs`
  - `units.rs`
  - `profile.rs`

## Completion Checklist

- [ ] Requirements implemented
- [ ] Formatting passes (`cargo fmt --check`)
- [ ] Lint passes (`cargo clippy -- -D warnings`)
- [ ] Build passes (`cargo build --release` or equivalent)
- [ ] Docs updated when applicable
- [ ] Final summary includes changes, validation, and residual risks
