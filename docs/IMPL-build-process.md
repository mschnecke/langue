# Implementation Plan: Build Process with GitHub Actions

> Generated from: `docs/PRD-build-process.md`
> Date: 2026-03-14

## 1. Overview

This feature introduces GitHub Actions CI/CD pipelines for Pisum Langue, a Tauri 2 desktop application. Two workflows will be created:

1. **CI Workflow** — Validates that the app builds on macOS and Windows for every PR and push to `main`.
2. **Release Workflow** — Builds platform installers and publishes them as GitHub Releases, triggered by `v*` tags or manual dispatch with version bumping.

The project currently has zero CI/CD infrastructure. The reference project (`github-global-hotkey`) provides proven patterns for Tauri builds, macOS `.pkg` packaging, and the draft-then-publish release flow.

## 2. Architecture & Design

### Workflow Structure

```
.github/
└── workflows/
    ├── ci.yml          # PR/push build verification
    └── release.yml     # Tag/manual release pipeline
```

### CI Workflow Flow

```
push to main / PR → checkout → setup Node 20 + Rust stable (cached)
  → install platform deps (Opus on macOS)
  → npm ci → npm run build
  → tauri build (matrix: macOS aarch64 + Windows)
  → upload artifacts (7-day retention)
```

### Release Workflow Flow

```
workflow_dispatch or v* tag push
  ↓
[bump-version] (dispatch only) → update 3 files + Cargo.lock → commit → tag → push
  ↓
[create-release] → create draft GitHub Release
  ↓
[build-tauri] (matrix: macOS + Windows) → build → upload assets to draft release
  ↓
[publish-release] → un-draft the release
```

### macOS Post-Build Packaging

The `.app` bundle produced by `tauri-apps/tauri-action` needs a post-build script to create a `.pkg` installer (consistent with the reference project). This script uses `pkgbuild` to wrap the `.app` into an installable `.pkg`.

## 3. Phases & Milestones

### Phase 1: CI Workflow
**Goal:** Every PR and push to `main` is automatically built on both platforms.
**Deliverable:** Green/red build status on PRs; build artifacts available for 7 days.

### Phase 2: Release Workflow
**Goal:** Maintainers can produce and publish platform installers via tag push or manual dispatch.
**Deliverable:** GitHub Releases with macOS `.app`/`.pkg` and Windows MSI installers.

## 4. Files Overview

### Files to Create
| File Path | Purpose |
|-----------|---------|
| `.github/workflows/ci.yml` | CI build verification workflow |
| `.github/workflows/release.yml` | Release build and publish workflow |
| `scripts/create-macos-pkg.sh` | Post-build script to create `.pkg` from `.app` bundle |

### Files to Modify
| File Path | What Changes |
|-----------|-------------|
| None | No existing files need modification for the workflow files themselves. Version bumps are handled dynamically by the release workflow at runtime. |

## 5. Task Breakdown

### Phase 1: CI Workflow

#### Task 1.1: Create the CI workflow file

- **Files to create:**
  - `.github/workflows/ci.yml` — Full CI pipeline definition
- **Implementation details:**
  - Trigger configuration:
    ```yaml
    on:
      push:
        branches: [main]
      pull_request:
        branches: [main]
    ```
  - Matrix strategy for `macos-latest` and `windows-latest`
  - Setup steps:
    - `actions/checkout@v4`
    - `actions/setup-node@v4` with `node-version-file: '.nvmrc'` and `cache: 'npm'`
    - `dtolnay/rust-toolchain@stable` (with `targets: aarch64-apple-darwin` on macOS)
    - `Swatinem/rust-cache@v2` with `workspaces: 'src-tauri -> target'`
  - Platform-specific dependency step (macOS only):
    ```yaml
    - name: Install Opus (macOS)
      if: matrix.os == 'macos-latest'
      run: brew install opus
    ```
  - Build steps:
    ```yaml
    - run: npm ci
    - run: npm run build
    - name: Build Tauri app
      uses: tauri-apps/tauri-action@v0
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        args: >-
          ${{ matrix.os == 'macos-latest' && '--target aarch64-apple-darwin --bundles app' || '--bundles msi' }}
    ```
  - macOS post-build `.pkg` creation step (runs `scripts/create-macos-pkg.sh`)
  - Upload artifacts with `actions/upload-artifact@v4` and `retention-days: 7`
- **Dependencies:** None
- **Acceptance criteria:**
  - Pushing to `main` or opening a PR against `main` triggers builds on both platforms
  - Build failures produce red status checks
  - Artifacts are downloadable for 7 days

#### Task 1.2: Create the macOS `.pkg` packaging script

- **Files to create:**
  - `scripts/create-macos-pkg.sh` — Shell script to create `.pkg` from `.app`
- **Implementation details:**
  - Script must be executable (`chmod +x`)
  - Uses `pkgbuild` (available on macOS runners by default)
  - Key logic:
    ```bash
    #!/bin/bash
    set -euo pipefail

    APP_NAME="PisumLangue"
    APP_PATH="src-tauri/target/aarch64-apple-darwin/release/bundle/macos/${APP_NAME}.app"
    PKG_OUTPUT="src-tauri/target/aarch64-apple-darwin/release/bundle/macos/${APP_NAME}.pkg"
    IDENTIFIER="com.pisumlangue.app"

    pkgbuild \
      --component "$APP_PATH" \
      --install-location "/Applications" \
      --identifier "$IDENTIFIER" \
      "$PKG_OUTPUT"

    echo "Created: $PKG_OUTPUT"
    ```
  - Follow the reference project's approach for paths and naming
- **Dependencies:** None (can be done in parallel with Task 1.1)
- **Acceptance criteria:**
  - Script produces a valid `.pkg` file from the `.app` bundle
  - CI workflow successfully runs this script on macOS

### Phase 2: Release Workflow

#### Task 2.1: Create the release workflow file

- **Files to create:**
  - `.github/workflows/release.yml` — Full release pipeline definition
- **Implementation details:**
  - Permissions:
    ```yaml
    permissions:
      contents: write
    ```
  - Trigger configuration:
    ```yaml
    on:
      push:
        tags: ['v*']
      workflow_dispatch:
        inputs:
          version:
            description: 'Version bump type (patch/minor/major) or exact version (e.g. 0.2.0)'
            required: true
            default: 'patch'
    ```

  - **Job 1: `bump-version`** (only on `workflow_dispatch`):
    - Runs on `ubuntu-latest`
    - Condition: `if: github.event_name == 'workflow_dispatch'`
    - Checkout with token for push access
    - Parse version input (bump type or exact version)
    - Read current version from `package.json`
    - Compute new version using semver logic (shell-based, no external tools — consistent with reference project)
    - Update version in three files:
      - `package.json` — update `"version"` field via `jq` or `sed`
      - `src-tauri/Cargo.toml` — update `version = "..."` in `[package]`
      - `src-tauri/tauri.conf.json` — update `"version"` field via `jq`
    - Install Rust toolchain and run `cargo generate-lockfile` in `src-tauri/` to update `Cargo.lock`
    - Configure git user:
      ```yaml
      git config user.name "github-actions[bot]"
      git config user.email "github-actions[bot]@users.noreply.github.com"
      ```
    - Commit all changed files (including `Cargo.lock`), create tag, push both
    - Output the new version and tag name for downstream jobs

  - **Job 2: `create-release`**:
    - Needs: `bump-version` (but use `if: always()` logic to also run on tag push)
    - Condition: succeeds or was skipped (tag push path)
    - Determine version from tag ref or bump-version output
    - Create draft release using `softprops/action-gh-release@v2`:
      ```yaml
      - uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ needs.bump-version.outputs.tag || github.ref_name }}
          name: ${{ needs.bump-version.outputs.version || github.ref_name }}
          draft: true
          generate_release_notes: true
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      ```

  - **Job 3: `build-tauri`** (matrix: macOS + Windows):
    - Needs: `create-release`
    - Same setup as CI workflow (Node 20, Rust stable, caching, Opus on macOS)
    - Build with `tauri-apps/tauri-action@v0`
    - macOS: `--target aarch64-apple-darwin --bundles app`, then run `scripts/create-macos-pkg.sh`
    - Windows: `--bundles msi`
    - Upload installers to the draft release using `softprops/action-gh-release@v2`:
      ```yaml
      - uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ needs.create-release.outputs.tag }}
          files: |
            src-tauri/target/aarch64-apple-darwin/release/bundle/macos/*.pkg
            src-tauri/target/aarch64-apple-darwin/release/bundle/macos/*.app
            src-tauri/target/release/bundle/msi/*.msi
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      ```
    - Use conditional glob patterns per platform (macOS paths on macOS, MSI path on Windows)

  - **Job 4: `publish-release`**:
    - Needs: `build-tauri`
    - Runs on `ubuntu-latest`
    - Un-draft the release using `actions/github-script@v7`:
      ```yaml
      - uses: actions/github-script@v7
        with:
          script: |
            const { data: releases } = await github.rest.repos.listReleases({
              owner: context.repo.owner,
              repo: context.repo.repo,
            });
            const draft = releases.find(r => r.draft && r.tag_name === '${{ needs.create-release.outputs.tag }}');
            if (draft) {
              await github.rest.repos.updateRelease({
                owner: context.repo.owner,
                repo: context.repo.repo,
                release_id: draft.id,
                draft: false,
              });
            }
      ```

- **Dependencies:** Task 1.2 (uses the same `.pkg` script)
- **Acceptance criteria:**
  - Pushing a `v*` tag triggers the full release pipeline (skipping bump-version)
  - Manual dispatch with "patch" bumps version, commits, tags, and triggers release
  - Manual dispatch with exact version (e.g., "0.3.0") sets that version exactly
  - Draft release is created, assets uploaded, then published
  - Both macOS and Windows installers appear as release assets

## 6. Data Model Changes

No data model changes required. This feature only adds CI/CD configuration files.

## 7. API Changes

No API changes required. This feature only adds CI/CD configuration files.

## 8. Dependencies & Risks

### External Dependencies (GitHub Actions)
| Action | Version | Purpose |
|--------|---------|---------|
| `actions/checkout` | v4 | Repository checkout |
| `actions/setup-node` | v4 | Node.js setup with caching |
| `dtolnay/rust-toolchain` | stable | Rust toolchain installation |
| `Swatinem/rust-cache` | v2 | Rust build caching |
| `tauri-apps/tauri-action` | v0 | Tauri application building |
| `actions/upload-artifact` | v4 | CI artifact upload |
| `softprops/action-gh-release` | v2 | GitHub Release creation/asset upload |
| `actions/github-script` | v7 | Release un-drafting |

### Risks & Mitigations
| Risk | Mitigation |
|------|-----------|
| macOS runner architecture mismatch (x86 vs ARM) | Explicitly specify `--target aarch64-apple-darwin`; GitHub's `macos-latest` runs on ARM (M-series) |
| Opus installation failure on macOS | Homebrew is pre-installed on GitHub macOS runners; `brew install opus` is reliable |
| Version bump race condition on concurrent dispatches | Low risk — manual dispatch is infrequent; sequential `bump-version` job with git push will fail on conflict |
| `tauri-apps/tauri-action@v0` breaking changes | Pin to `v0` as per PRD; monitor for Tauri 2 compatibility |
| Release asset upload from matrix jobs to same release | `softprops/action-gh-release` supports concurrent uploads to the same release |

### Assumptions
- GitHub-hosted runners have sufficient resources for Tauri builds
- `macos-latest` runners are ARM-based (Apple Silicon), making `aarch64-apple-darwin` the native target
- The `GITHUB_TOKEN` has sufficient permissions for release creation when `permissions: contents: write` is set

## 9. Testing Strategy

Since this feature is entirely CI/CD configuration (no application code changes), testing is manual/observational:

- **CI Workflow verification:**
  - Open a PR against `main` → verify both macOS and Windows jobs run and pass
  - Push to `main` → verify CI triggers
  - Introduce a deliberate build error in a PR → verify the pipeline fails

- **Release Workflow verification:**
  - Push a `v0.1.1` tag → verify full release pipeline runs (skip bump, build, publish)
  - Use `workflow_dispatch` with `patch` → verify version bumps from `0.1.0` to `0.1.1`, commits, tags, builds, and publishes
  - Use `workflow_dispatch` with exact version `0.2.0` → verify exact version is set
  - Verify all release assets are downloadable
  - Verify `.pkg` installer is valid on macOS
  - Verify MSI installer is valid on Windows

- **Edge cases:**
  - Version bump with dirty working tree (should not happen on `main`)
  - Multiple concurrent workflow runs (rely on sequential job ordering)

## 10. Requirement Traceability

### Functional Requirements

| PRD Ref | Requirement Summary | Task(s) | Notes |
|---------|-------------------|---------|-------|
| 4 #1 | CI triggers on push to `main` and PRs targeting `main` | 1.1 | `on.push.branches` + `on.pull_request.branches` |
| 4 #2 | CI runs on `windows-latest` and `macos-latest` | 1.1 | Matrix strategy |
| 4 #3 | Install Node 20 and Rust stable with caching | 1.1 | `setup-node` + `rust-toolchain` + `rust-cache` |
| 4 #4 | Install platform-specific deps (Opus on macOS) | 1.1 | Conditional `brew install opus` step |
| 4 #5 | Install frontend deps via `npm ci` | 1.1 | Explicit `npm ci` step |
| 4 #6 | Build frontend (`npm run build`) | 1.1 | Explicit `npm run build` step |
| 4 #7 | Build Tauri app with platform-specific bundles + `.pkg` on macOS | 1.1, 1.2 | `tauri-action` + `create-macos-pkg.sh` |
| 4 #8 | Pipeline fails on build step failure | 1.1 | Default GitHub Actions behavior (fail-fast) |
| 4 #9 | Release triggers on `v*` tags | 2.1 | `on.push.tags` |
| 4 #10 | Release supports `workflow_dispatch` with version bump input | 2.1 | `workflow_dispatch.inputs.version` |
| 4 #11 | Version bump updates 3 files, commits, and creates tag | 2.1 | `bump-version` job |
| 4 #12 | Release builds on both platforms in parallel | 2.1 | Matrix in `build-tauri` job |
| 4 #13 | Release uses same setup as CI | 2.1 | Identical setup steps |
| 4 #14 | Release builds `.app`/`.pkg` on macOS, MSI on Windows | 2.1, 1.2 | Same build + pkg script |
| 4 #15 | Release creates draft GitHub Release | 2.1 | `create-release` job with `draft: true` |
| 4 #16 | Upload installers as release assets | 2.1 | `softprops/action-gh-release` in `build-tauri` |
| 4 #17 | Publish (un-draft) after all builds succeed | 2.1 | `publish-release` job |
| 4 #18 | Cache npm and Rust dependencies | 1.1, 2.1 | `setup-node` cache + `rust-cache` |
| 4 #19 | CI artifacts retained 7 days | 1.1 | `retention-days: 7` |
| 4 #20 | `GITHUB_TOKEN` passed to build and release steps | 1.1, 2.1 | `env.GITHUB_TOKEN` on relevant steps |

### User Stories

| PRD Ref | User Story Summary | Implementing Tasks | Fully Covered? |
|---------|-------------------|-------------------|----------------|
| US-1 | Auto-build PRs on both platforms | 1.1, 1.2 | Yes |
| US-2 | Manual release trigger with version bump | 2.1 | Yes |
| US-3 | Tag-based release trigger | 2.1 | Yes |
| US-4 | Download installers from GitHub Releases | 2.1 | Yes |

### Success Metrics

| Metric | How the Plan Addresses It |
|--------|--------------------------|
| Green/red build status on push/PR | CI workflow (Task 1.1) runs on every push and PR to `main` |
| Complete platform installers via tag or dispatch | Release workflow (Task 2.1) supports both triggers |
| Downloadable installers from GitHub Releases | Release workflow uploads assets and publishes the release |
