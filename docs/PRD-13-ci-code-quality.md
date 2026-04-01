# PRD: CI Code Quality Pipeline

**Issue**: [#13 — Improve CI action with linter actions etc](https://github.com/mschnecke/pisum-transcript/issues/13)
**Status**: Draft
**Date**: 2026-04-01

---

## Overview

Add a comprehensive code quality pipeline to the CI workflow, adopting the proven patterns from the `github-global-hotkey` reference repository. This includes ESLint and Prettier for the Svelte/TypeScript frontend, and `cargo fmt` + Clippy for the Rust backend, with quality checks gating the build step.

## Problem Statement

The current CI workflow (`ci.yml`) only performs builds — there are no automated code quality checks. This means:

- Formatting inconsistencies go undetected until code review
- Linting issues (unused vars, type errors, anti-patterns) are caught manually or not at all
- Rust code quality (Clippy warnings, formatting) is not enforced
- The existing `svelte-check` script is available but not run in CI
- No ESLint or Prettier configuration exists in the project at all

The reference repository (`github-global-hotkey`) already has a working setup with `lint-and-check` and `rust-check` jobs that gate the build. This PRD describes adopting that pattern.

## Goals & Success Metrics

| Goal                              | Success Metric                                                                         |
| --------------------------------- | -------------------------------------------------------------------------------------- |
| Automated frontend quality checks | ESLint, Prettier, and svelte-check run on every PR                                     |
| Automated Rust quality checks     | `cargo fmt --check` and `cargo clippy` run on every PR                                 |
| Gated builds                      | Build job only runs after lint/check jobs pass                                         |
| Developer experience              | Local scripts (`npm run lint`, `npm run format:check`) available for pre-PR validation |

## User Stories

1. **As a developer**, I want CI to catch formatting and lint issues on my PR so I can fix them before review.
2. **As a reviewer**, I want code quality enforced automatically so I can focus on logic and design during review.
3. **As a maintainer**, I want consistent code style across the entire codebase without manual enforcement.

## Functional Requirements

### FR-1: Add ESLint Configuration

- Install ESLint with Svelte and TypeScript plugins
- Create `eslint.config.js` (flat config format) with rules appropriate for Svelte 5 + TypeScript
- Add `npm run lint` script to `package.json`

### FR-2: Add Prettier Configuration

- Install Prettier with the Svelte plugin
- Create `.prettierrc` configuration file
- Create `.prettierignore` (exclude `build/`, `dist/`, `src-tauri/target/`, etc.)
- Add `npm run format:check` script to `package.json`
- Add `npm run format` script for auto-fixing

### FR-3: Add `lint-and-check` CI Job

A new job that runs on `ubuntu-latest` with the following steps:

1. Checkout code
2. Setup Node.js (using `.nvmrc`)
3. `npm ci`
4. Run ESLint (`npm run lint`)
5. Run Prettier check (`npm run format:check`)
6. Run Svelte check (`npm run check`)

### FR-4: Add `rust-check` CI Job

A new job that runs on `ubuntu-latest` with the following steps:

1. Checkout code
2. Install system dependencies (libwebkit2gtk-4.1-dev, libasound2-dev, etc.)
3. Setup Rust stable with `clippy` and `rustfmt` components
4. Cache Rust dependencies (`Swatinem/rust-cache@v2`)
5. Run `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
6. Run `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`

### FR-5: Gate Build on Quality Checks

- Add `needs: [lint-and-check, rust-check]` to the existing `build` job
- Build only proceeds if both quality jobs pass

### FR-6: CI Trigger Configuration

- Keep CI triggered on `pull_request` to `main` only (current behavior)
- Do not re-enable push triggers

## Non-Functional Requirements

### NFR-1: CI Performance

- Lint/check jobs should complete in under 3 minutes
- Use dependency caching (npm cache, Rust cache) to minimize install time
- Lint and Rust check jobs run in parallel

### NFR-2: Developer Experience

- All CI checks must be reproducible locally via npm scripts
- Clear error messages when checks fail

### NFR-3: Compatibility

- Node.js version sourced from `.nvmrc` (currently 24)
- Rust stable toolchain
- Ubuntu system dependencies match what the project requires for Tauri 2 builds

## Technical Considerations

### Reference Implementation

The `github-global-hotkey` repository CI serves as the template. Key differences to account for:

| Aspect       | Reference Repo        | Pisum Transcript                                           |
| ------------ | --------------------- | ---------------------------------------------------------- |
| Node version | Hardcoded `20`        | Uses `.nvmrc` (24) — keep using `node-version-file`        |
| CI triggers  | Push + PR             | PR only (per decision)                                     |
| System deps  | Includes `libxdo-dev` | May need `libopus-dev` additionally                        |
| Env vars     | None extra            | Has `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true` — keep this |

### Dependencies to Add

**npm devDependencies:**

- `eslint`
- `@eslint/js`
- `typescript-eslint`
- `eslint-plugin-svelte`
- `prettier`
- `prettier-plugin-svelte`
- `prettier-plugin-tailwindcss`

### Package.json Scripts to Add

```json
{
	"lint": "eslint .",
	"lint:fix": "eslint . --fix",
	"format:check": "prettier --check .",
	"format": "prettier --write ."
}
```

### System Dependencies for `rust-check` Job

```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libasound2-dev \
  libopus-dev
```

## Out of Scope

- **Unit test jobs** — No test framework is configured yet; testing will be a separate effort
- **Security scanning** (e.g., `cargo audit`, npm audit) — Can be added later
- **Auto-formatting PRs** — No bot or action to auto-fix formatting
- **Pre-commit hooks** (e.g., husky + lint-staged) — Local developer tooling is separate from CI
- **Push triggers on main** — Explicitly excluded per decision
- **Code coverage** — No tests exist to measure coverage

## Open Questions

1. **Opus system dependency**: Does the `rust-check` job on Ubuntu need `libopus-dev` installed, or is Opus only required for the macOS build? -> macOS only
2. **Existing code compliance**: The codebase likely has formatting/lint violations since no tooling existed. Should we fix all violations in the same PR, or add tooling first and fix violations in a follow-up? -> Fix all violations in the same PR
3. **Tailwind plugin ordering**: Should Prettier use `prettier-plugin-tailwindcss` for automatic class sorting? -> Yes, but only for CSS classes in Svelte components

## Implementation Milestones

| Phase | Description                                   |
| ----- | --------------------------------------------- |
| 1     | Add ESLint + Prettier configs and npm scripts |
| 2     | Fix existing codebase violations              |
| 3     | Add `lint-and-check` and `rust-check` CI jobs |
| 4     | Gate build job on quality checks              |
| 5     | Verify full pipeline on a test PR             |

## References

- [Issue #13](https://github.com/mschnecke/pisum-transcript/issues/13)
- Reference CI: `github-global-hotkey/.github/workflows/ci.yml`
