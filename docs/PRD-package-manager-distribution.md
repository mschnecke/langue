# PRD: Package Manager Distribution (Homebrew & Chocolatey)

## 1. Introduction/Overview

Pisum Langue is currently distributed only via GitHub Releases as direct downloads (.pkg for macOS, .msi for Windows). Users must manually find, download, and install updates. This PRD covers adding Homebrew (macOS) and Chocolatey (Windows) package manager support, and wiring both into the existing release workflow so that every release automatically publishes updated packages.

## 2. Goals

- Enable macOS users to install and update Pisum Langue via `brew install --cask pisum-langue`
- Enable Windows users to install and update Pisum Langue via `choco install pisum-langue`
- Automate package publishing so every GitHub Release triggers Homebrew cask and Chocolatey package updates with zero manual steps
- Follow the same proven patterns established in the `global-hotkey` reference repo

## 3. User Stories

- As a **macOS user**, I want to install Pisum Langue with `brew install --cask pisum-langue` so that I can use my familiar package manager and get updates through `brew upgrade`.
- As a **Windows user**, I want to install Pisum Langue with `choco install pisum-langue` so that I can manage it alongside my other Chocolatey packages.
- As a **maintainer**, I want package manager updates to happen automatically on release so that I don't have to manually update formulas, checksums, or package specs.
- As a **user upgrading**, I want `brew upgrade` or `choco upgrade` to give me the latest version so that I stay current without visiting GitHub.

## 4. Functional Requirements

### Homebrew Cask

1. A separate GitHub repository `mschnecke/homebrew-pisum-langue` must be created to serve as the Homebrew tap.
2. The tap repository must contain a cask formula at `Casks/pisum-langue.rb` that installs the `.pkg` artifact from GitHub Releases.
3. The cask must target Apple Silicon (aarch64) only, with `depends_on macos: ">= :catalina"`.
4. The cask must include a `zap` stanza that removes application data from `~/Library/Application Support/com.pisum.langue`, `~/Library/Caches/com.pisum.langue`, `~/Library/Preferences/com.pisum.langue.plist`, and `~/Library/LaunchAgents/com.pisum.langue.plist`.
5. A template cask file must exist in the main repo at `packages/homebrew/pisum-langue.rb` with placeholder values for version and SHA256.
6. Users must be able to install with: `brew tap mschnecke/pisum-langue && brew install --cask pisum-langue`.

### Chocolatey Package

7. A Chocolatey package definition must exist at `packages/chocolatey/pisum-langue.nuspec` containing package metadata (id, version, title, authors, project URL, description, tags).
8. An install script must exist at `packages/chocolatey/tools/chocolateyinstall.ps1` that downloads and silently installs the `.msi` from GitHub Releases using `Install-ChocolateyPackage` with SHA256 checksum verification.
9. An uninstall script must exist at `packages/chocolatey/tools/chocolateyuninstall.ps1` that finds and removes the MSI via Windows Registry lookup.
10. The Chocolatey package must be published to the MyGet NuGet feed at `https://www.myget.org/F/mschnecke/api/v3/index.json`.
11. Users must be able to install with: `choco install pisum-langue --source https://www.myget.org/F/mschnecke/api/v3/index.json`.

### Release Workflow Integration

12. The existing `release.yml` must be updated to include an `update-homebrew` job that triggers a repository dispatch to `mschnecke/homebrew-pisum-langue` with the new version, passing the version string in the payload.
13. The Homebrew tap repository must have a workflow that receives the dispatch event, downloads the `.pkg` artifact, computes the SHA256 checksum, updates the cask formula with the new version and hash, and commits the change.
14. The existing `release.yml` must be updated to include an `update-chocolatey` job that:
    - Runs on `windows-latest` after the `publish-release` job
    - Downloads the built `.msi` artifact from the GitHub Release
    - Computes the SHA256 checksum
    - Updates `chocolateyinstall.ps1` with the new download URL and checksum
    - Updates `pisum-langue.nuspec` with the new version
    - Runs `choco pack` to create the `.nupkg`
    - Pushes the package to MyGet using `choco push` with the `MYGET_API_KEY` secret
15. The `bump-version` job in `release.yml` must also update the version in `packages/chocolatey/pisum-langue.nuspec` alongside the existing version files (package.json, Cargo.toml, tauri.conf.json).

### Secrets & Configuration

16. A `HOMEBREW_TAP_TOKEN` GitHub secret must be configured with a personal access token that has permission to trigger workflows on the `mschnecke/homebrew-pisum-langue` repository.
17. A `MYGET_API_KEY` GitHub secret must be configured for publishing Chocolatey packages to the MyGet feed.

## 5. Non-Goals (Out of Scope)

- Not included: Publishing to the official Chocolatey Community Repository (chocolatey.org) — MyGet is sufficient for now
- Not included: Intel (x64) macOS builds or universal binaries — only aarch64 is supported
- Not included: Linux package managers (apt, snap, flatpak, AUR)
- Not included: Auto-update mechanisms within the app itself (Tauri updater plugin)
- Not included: Code signing for macOS or Windows installers
- Not included: Creating a Homebrew formula (non-cask) — the app is a GUI application, so a cask is appropriate

## 6. Design Considerations

- The Homebrew tap naming convention follows `homebrew-{name}` so that `brew tap mschnecke/pisum-langue` maps to the `mschnecke/homebrew-pisum-langue` repo.
- The Chocolatey package ID should be `pisum-langue` (lowercase, hyphenated) to follow Chocolatey naming conventions.
- Installation instructions in the GitHub Release body (already generated by `release.yml`) should be updated to include the Homebrew and Chocolatey commands.

## 7. Technical Considerations

- **Reference implementation**: The `global-hotkey` repo (`/Users/mschnecke/workspace/github-global-hotkey`) has a working implementation of this exact pattern — use it as the primary reference for workflow structure, scripts, and package definitions.
- **Workflow job dependencies**: `update-homebrew` and `update-chocolatey` must depend on `publish-release` to ensure artifacts are publicly available before package managers reference them.
- **SHA256 checksums**: Both Homebrew and Chocolatey require SHA256 checksums of the installer artifacts. These must be computed in CI after downloading the built artifact — never hardcoded.
- **MyGet feed**: The existing MyGet account (`mschnecke`) and feed should be reused. The Chocolatey install source URL for users is the v3 endpoint.
- **Homebrew tap dispatch**: The dispatch event should include the version in the payload (e.g., `{ "version": "0.1.8" }`) so the tap workflow knows which release to pull.
- **Asset naming**: macOS pkg is `Pisum.Langue_{version}_aarch64.pkg`, Windows MSI is `Pisum.Langue_{version}_x64_en-US.msi` — these names are determined by the existing Tauri build config and `create-macos-pkg.sh`.

## 8. Success Metrics

- `brew tap mschnecke/pisum-langue && brew install --cask pisum-langue` successfully installs the latest version on macOS (Apple Silicon)
- `choco install pisum-langue --source https://www.myget.org/F/mschnecke/api/v3/index.json` successfully installs the latest version on Windows
- A new release triggered via `release.yml` automatically updates both the Homebrew cask and Chocolatey package within the same workflow run, with no manual intervention
- `brew upgrade pisum-langue` and `choco upgrade pisum-langue` correctly pull the new version after a release

## 9. Open Questions

- [x] Does the `mschnecke/homebrew-pisum-langue` tap repository already exist, or does it need to be created? -> already created
- [x] Is the MyGet account and API key already configured, or do they need to be set up? -> already configured
- [x] Should the GitHub Release body template be updated in this effort, or handled separately? -> the GitHub Release body template should be updated
