# PRD: Build Process with GitHub Actions

## 1. Introduction/Overview

Pisum Langue currently has no CI/CD pipeline. Developers must manually build and verify the application on each platform, which is error-prone and time-consuming. This feature introduces GitHub Actions workflows for continuous integration (CI) and automated release builds, ensuring every pull request is verified and releases are produced consistently for macOS and Windows.

## 2. Goals

- Automatically verify that the application builds successfully on every push and pull request
- Automate the creation of platform-specific installers (macOS `.app`/`.pkg`, Windows MSI) and publish them as GitHub Releases
- Support both manual (workflow_dispatch) and tag-based (`v*`) release triggers
- Keep the pipeline simple — no code signing, no package manager publishing for now

## 3. User Stories

- As a developer, I want every pull request to be automatically built so that I know the code compiles and bundles correctly on both platforms before merging.
- As a maintainer, I want to trigger a release manually with a version bump option so that I can control when new versions are published.
- As a maintainer, I want to push a `v*` tag to trigger a release build so that I have a simple git-based release workflow.
- As a user, I want to download platform-specific installers from GitHub Releases so that I can easily install the application.

## 4. Functional Requirements

### CI Workflow (`.github/workflows/ci.yml`)

1. The CI workflow must trigger on pushes to `main` and on pull requests targeting `main`.
2. The CI workflow must run on both `windows-latest` and `macos-latest` runners.
3. The CI workflow must install Node.js (version 20) and Rust (stable) with appropriate caching for both npm packages and Rust dependencies.
4. The CI workflow must install platform-specific dependencies (Opus via Homebrew on macOS).
5. The CI workflow must install frontend dependencies using `npm ci`.
6. The CI workflow must build the frontend (`npm run build`).
7. The CI workflow must build the full Tauri application using `tauri-apps/tauri-action` with platform-specific bundle targets (`.app` on macOS with `aarch64-apple-darwin` target, MSI on Windows). On macOS, a `.pkg` installer must also be produced from the `.app` bundle via a post-build packaging script (see reference project).
8. The CI workflow must fail the pipeline if any build step fails.

### Release Workflow (`.github/workflows/release.yml`)

9. The release workflow must trigger on pushes to tags matching `v*` (e.g., `v0.2.0`).
10. The release workflow must also support `workflow_dispatch` with an input to select version bump type (`patch`, `minor`, `major`) or specify an exact version string.
11. When triggered via `workflow_dispatch`, the release workflow must update the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, commit the changes, and create a git tag.
12. The release workflow must build the Tauri application on both `windows-latest` and `macos-latest` runners in parallel.
13. The release workflow must use the same Node.js, Rust, and dependency setup as the CI workflow.
14. The release workflow must build platform-specific installers: `.app` bundle and `.pkg` installer on macOS (aarch64-apple-darwin), and MSI on Windows.
15. The release workflow must create a GitHub Release as a draft with the version number as the release name.
16. The release workflow must upload the built installers as release assets.
17. The release workflow must publish (un-draft) the GitHub Release after all platform builds and uploads succeed.

### Shared Concerns

18. Both workflows must cache npm dependencies (via Node.js setup action) and Rust dependencies (via `Swatinem/rust-cache` with workspace set to `src-tauri`).
19. CI build artifacts must be retained for 7 days.
20. The `GITHUB_TOKEN` secret must be passed to the Tauri build action and release creation steps.

## 5. Non-Goals (Out of Scope)

- Not included: Code signing or notarization for macOS or Windows
- Not included: Package manager publishing (Homebrew, Chocolatey, etc.)
- Not included: Tauri auto-updater integration
- Not included: Linux builds
- Not included: Linting or type-checking steps (ESLint, Prettier, cargo fmt, clippy, svelte-check) — the pipeline only verifies that the build succeeds
- Not included: Automated testing (no test suite exists yet)

## 6. Technical Considerations

- **Tauri Action**: Use `tauri-apps/tauri-action@v0` for building, consistent with the reference project (`github-global-hotkey`)
- **Node.js version**: Pinned to 20 via `.nvmrc` at the repository root; CI workflows must use this version
- **Platform targets**: macOS uses `--target aarch64-apple-darwin --bundles app`, followed by a post-build script to produce a `.pkg` installer (consistent with reference project); Windows uses `--bundles msi`
- **macOS dependency**: The Opus library must be installed via `brew install opus` before building on macOS runners (hard requirement for `audiopus` crate)
- **Version sync**: When bumping versions via workflow_dispatch, three files must be updated in lockstep: `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`
- **Git actor**: Version bump commits should be authored by `github-actions[bot]` to distinguish automated commits from human ones
- **Release creation**: Use `softprops/action-gh-release@v2` for creating GitHub Releases
- **Draft-then-publish pattern**: Create the release as a draft first, upload assets from parallel matrix jobs, then un-draft via `actions/github-script@v7` in a separate finalization job (using `needs:` dependencies). Follow the reference project's job structure: `bump-version` → `create-release` → `build-tauri` (matrix) → `publish-release`
- **Workflow permissions**: The release workflow must declare `permissions: contents: write` to allow creating releases and pushing version bump commits
- **Version bump mechanics**: When triggered via `workflow_dispatch`, the version bump job commits to `main`, pushes the commit and tag using `github-actions[bot]` credentials. `Cargo.lock` must also be committed alongside version changes. Follow the reference project's implementation for semver parsing and multi-file updates
- **Concurrency**: No explicit concurrency groups required (consistent with reference project); sequential job ordering via `needs:` is sufficient
- **Opus dependency**: The Opus library (`brew install opus`) is a hard build requirement on macOS, not optional

## 7. Success Metrics

- Every push and PR to `main` produces a green/red build status within a reasonable time
- A maintainer can produce a complete set of platform installers (macOS + Windows) by either pushing a tag or using the manual workflow dispatch
- Built installers are downloadable from GitHub Releases and install correctly on their respective platforms

## 8. Open Questions

- [x] Should the CI workflow also run on pushes to development branches (e.g., feature branches), or only on PRs targeting `main`? -> only on PRs targeting `main`
- [x] Should the macOS build also produce a `.pkg` installer (like the reference project) in addition to the `.app` bundle? -> like the reference project
- [x] What is the desired release notes format — auto-generated from commits, or manually written? -> auto-generated from commits
