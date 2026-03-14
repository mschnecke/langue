# Implementation Plan: Package Manager Distribution (Homebrew & Chocolatey)

> Generated from: `docs/PRD-package-manager-distribution.md`
> Date: 2026-03-14

## 1. Overview

This plan adds Homebrew (macOS) and Chocolatey (Windows) package manager support for Pisum Langue. The implementation creates the necessary package definitions, a Homebrew template cask, and wires everything into the existing `release.yml` workflow so that every release automatically publishes updated packages.

The reference implementation at `/Users/mschnecke/workspace/github-global-hotkey` provides proven patterns for all components. The current `release.yml` already has `update-homebrew` and `update-chocolatey` jobs stubbed out — this plan fills in the missing package files and fixes issues in the workflow.

## 2. Architecture & Design

### Package Distribution Flow

```
release.yml (workflow_dispatch)
  │
  ├─ bump-version ──► Updates package.json, Cargo.toml, tauri.conf.json, pisum-langue.nuspec
  │
  ├─ create-release ──► Creates draft GitHub Release
  │
  ├─ build-tauri ──► Builds .pkg (macOS) and .msi (Windows), uploads to release
  │
  ├─ publish-release ──► Marks release as non-draft
  │
  ├─ update-homebrew ──► repository-dispatch → mschnecke/homebrew-pisum-langue
  │                        └─ Tap workflow: downloads .pkg, computes SHA256, updates cask, commits
  │
  └─ update-chocolatey ──► Downloads .msi, computes SHA256, updates install script + nuspec,
                            packs .nupkg, pushes to MyGet
```

### Repository Structure

```
Main repo (mschnecke/langue):
  packages/
  ├── homebrew/
  │   └── pisum-langue.rb        ← Template cask (placeholder version/SHA)
  └── chocolatey/
      ├── pisum-langue.nuspec    ← Package metadata
      └── tools/
          ├── chocolateyinstall.ps1    ← Install script
          └── chocolateyuninstall.ps1  ← Uninstall script

Tap repo (mschnecke/homebrew-pisum-langue):
  Casks/
  └── pisum-langue.rb            ← Live cask (auto-updated by workflow)
  .github/workflows/
  └── update-cask.yml            ← Receives dispatch, updates cask
```

## 3. Phases & Milestones

### Phase 1: Chocolatey Package Files
**Goal:** Create the Chocolatey package definition files in the main repo.
**Deliverable:** `packages/chocolatey/` directory with nuspec, install, and uninstall scripts ready for CI consumption.

### Phase 2: Homebrew Cask Template
**Goal:** Create the template Homebrew cask in the main repo.
**Deliverable:** `packages/homebrew/pisum-langue.rb` with placeholder values.

### Phase 3: Release Workflow Fixes
**Goal:** Fix the existing `release.yml` to correctly reference files and update the nuspec during version bumps.
**Deliverable:** Working `bump-version` and `update-homebrew` jobs with correct paths and repository name.

### Phase 4: Homebrew Tap Repository
**Goal:** Set up the `mschnecke/homebrew-pisum-langue` repository with cask and update workflow.
**Deliverable:** Tap repo with `Casks/pisum-langue.rb` and `update-cask.yml` workflow.

### Phase 5: Release Body Update
**Goal:** Fix the release body installation instructions to match the PRD's prescribed commands.
**Deliverable:** Updated release body template in `release.yml` with correct Homebrew and Chocolatey commands.

## 4. Files Overview

### Files to Create
| File Path | Purpose |
|-----------|---------|
| `packages/chocolatey/pisum-langue.nuspec` | Chocolatey package metadata |
| `packages/chocolatey/tools/chocolateyinstall.ps1` | Downloads and installs MSI from GitHub Releases |
| `packages/chocolatey/tools/chocolateyuninstall.ps1` | Uninstalls MSI via registry lookup |
| `packages/homebrew/pisum-langue.rb` | Template Homebrew cask with placeholder version/SHA256 |

### Files in Tap Repository (mschnecke/homebrew-pisum-langue)
| File Path | Purpose |
|-----------|---------|
| `Casks/pisum-langue.rb` | Live Homebrew cask formula |
| `.github/workflows/update-cask.yml` | Receives dispatch, updates cask with new version |

### Files to Modify
| File Path | What Changes |
|-----------|-------------|
| `.github/workflows/release.yml` | Fix homebrew repo name, add nuspec to bump-version, update release body |

## 5. Task Breakdown

### Phase 1: Chocolatey Package Files

#### Task 1.1: Create `packages/chocolatey/pisum-langue.nuspec`

- **Files to create:**
  - `packages/chocolatey/pisum-langue.nuspec` — Package metadata adapted from reference repo
- **Implementation details:**
  - Adapt from `/Users/mschnecke/workspace/github-global-hotkey/packages/chocolatey/global-hotkey.nuspec`
  - Key fields:
    ```xml
    <id>pisum-langue</id>
    <version>0.1.7</version>
    <title>Pisum Langue</title>
    <authors>Pisum Langue Team</authors>
    <owners>mschnecke</owners>
    <projectUrl>https://github.com/mschnecke/langue</projectUrl>
    <description>AI-driven transcription utility. Hold a hotkey to record speech, release to transcribe and paste.</description>
    <tags>transcription dictation speech-to-text ai hotkey</tags>
    <releaseNotes>https://github.com/mschnecke/langue/releases</releaseNotes>
    <packageSourceUrl>https://github.com/mschnecke/langue/tree/main/packages/chocolatey</packageSourceUrl>
    ```
- **Dependencies:** None
- **Acceptance criteria:** Valid nuspec XML that `choco pack` can parse

#### Task 1.2: Create `packages/chocolatey/tools/chocolateyinstall.ps1`

- **Files to create:**
  - `packages/chocolatey/tools/chocolateyinstall.ps1` — Install script
- **Implementation details:**
  - Adapt from reference repo's `chocolateyinstall.ps1`
  - Key values:
    ```powershell
    $packageName = 'pisum-langue'
    $packageArgs = @{
      packageName    = $packageName
      fileType       = 'msi'
      url64bit       = 'https://github.com/mschnecke/langue/releases/download/v0.1.7/Pisum.Langue_0.1.7_x64_en-US.msi'
      softwareName   = 'Pisum Langue*'
      checksum64     = 'REPLACE_WITH_ACTUAL_CHECKSUM'
      checksumType64 = 'sha256'
      silentArgs     = '/qn /norestart'
      validExitCodes = @(0, 3010, 1641)
    }
    Install-ChocolateyPackage @packageArgs
    ```
  - The `url64bit` and `checksum64` are placeholders — they get replaced by the `update-chocolatey` job in CI
- **Dependencies:** None
- **Acceptance criteria:** Script follows `Install-ChocolateyPackage` pattern with SHA256 checksum verification

#### Task 1.3: Create `packages/chocolatey/tools/chocolateyuninstall.ps1`

- **Files to create:**
  - `packages/chocolatey/tools/chocolateyuninstall.ps1` — Uninstall script
- **Implementation details:**
  - Adapt from reference repo's `chocolateyuninstall.ps1`
  - Searches registry at:
    - `HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*`
    - `HKLM:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*`
  - Matches `softwareName` = `'Pisum Langue*'`
  - Uses `Uninstall-ChocolateyPackage` with the MSI product code from registry
  - Silent args: `/qn /norestart`
- **Dependencies:** None
- **Acceptance criteria:** Script correctly looks up and removes the MSI via registry ProductCode

### Phase 2: Homebrew Cask Template

#### Task 2.1: Create `packages/homebrew/pisum-langue.rb`

- **Files to create:**
  - `packages/homebrew/pisum-langue.rb` — Template cask formula
- **Implementation details:**
  - Adapt from reference repo's `packages/homebrew/global-hotkey.rb`
  - aarch64 only (no Intel/universal — per PRD non-goals)
  - Key structure:
    ```ruby
    cask "pisum-langue" do
      version "0.1.7"
      sha256 "REPLACE_WITH_ACTUAL_CHECKSUM"

      url "https://github.com/mschnecke/langue/releases/download/v#{version}/Pisum.Langue_#{version}_aarch64.pkg"
      name "Pisum Langue"
      desc "AI-driven transcription utility"
      homepage "https://github.com/mschnecke/langue"

      depends_on macos: ">= :catalina"

      pkg "Pisum.Langue_#{version}_aarch64.pkg"

      uninstall pkgutil: "com.pisum.langue.app"

      zap trash: [
        "~/Library/Application Support/com.pisum.langue",
        "~/Library/Caches/com.pisum.langue",
        "~/Library/Preferences/com.pisum.langue.plist",
        "~/Library/LaunchAgents/com.pisum.langue.plist",
      ]
    end
    ```
  - Version and SHA256 are placeholders in this template — the tap repo holds the live version
- **Dependencies:** None
- **Acceptance criteria:** Valid Ruby cask syntax with `zap` stanza covering all four paths from PRD requirement 4

### Phase 3: Release Workflow Fixes

#### Task 3.1: Fix Homebrew tap repository name in `release.yml`

- **Files to modify:**
  - `.github/workflows/release.yml` — Line 283
- **Implementation details:**
  - Change `repository: mschnecke/homebrew-lange` → `repository: mschnecke/homebrew-pisum-langue`
  - This is a typo in the existing workflow
- **Dependencies:** None
- **Acceptance criteria:** Repository dispatch targets the correct tap repo

#### Task 3.2: Add nuspec version update to `bump-version` job

- **Files to modify:**
  - `.github/workflows/release.yml` — `bump-version` job, after the tauri.conf.json update (around line 85)
- **Implementation details:**
  - Add sed command to update the nuspec version:
    ```bash
    # Update Chocolatey nuspec version
    sed -i "s/<version>.*<\/version>/<version>$NEW_VERSION<\/version>/" packages/chocolatey/pisum-langue.nuspec
    ```
  - Add `packages/chocolatey/pisum-langue.nuspec` to the `git add` command on line 93
- **Dependencies:** Task 1.1 (nuspec must exist)
- **Acceptance criteria:** Version bump updates nuspec alongside package.json, Cargo.toml, and tauri.conf.json

#### Task 3.3: Update release body installation instructions

- **Files to modify:**
  - `.github/workflows/release.yml` — Release body template (lines 126–163)
- **Implementation details:**
  - Update Homebrew section to match PRD requirement 6:
    ```markdown
    **macOS (Homebrew) - Recommended:**
    ```bash
    brew tap mschnecke/pisum-langue
    brew install --cask pisum-langue
    ```
    ```
  - Update Chocolatey section to match PRD requirement 11:
    ```markdown
    **Windows (Chocolatey):**
    ```powershell
    choco install pisum-langue --source https://www.myget.org/F/mschnecke/api/v3/index.json
    ```
    ```
  - Current body uses `brew tap mschnecke/langue` and `brew install --cask langue` — these need to match the actual tap name
  - Current body uses `choco source add` pattern — PRD specifies v3 endpoint with `--source` flag inline
- **Dependencies:** None
- **Acceptance criteria:** Release body shows correct install commands matching PRD requirements 6 and 11

### Phase 4: Homebrew Tap Repository

#### Task 4.1: Create `Casks/pisum-langue.rb` in tap repository

- **Files to create (in tap repo):**
  - `Casks/pisum-langue.rb` — Live cask formula (copy of template from Task 2.1)
- **Implementation details:**
  - Same content as `packages/homebrew/pisum-langue.rb` from the main repo
  - This is the file Homebrew will read when users run `brew install --cask pisum-langue`
- **Dependencies:** None
- **Acceptance criteria:** `brew tap mschnecke/pisum-langue` succeeds and shows the cask

#### Task 4.2: Create `update-cask.yml` workflow in tap repository

- **Files to create (in tap repo):**
  - `.github/workflows/update-cask.yml` — Dispatch event handler
- **Implementation details:**
  - Triggered by `repository_dispatch` with `event-type: update-cask`
  - Receives version from `client-payload.version`
  - Steps:
    1. Checkout the tap repo
    2. Download the `.pkg` from GitHub Releases:
       ```bash
       curl -L -o pisum-langue.pkg \
         "https://github.com/mschnecke/langue/releases/download/v${VERSION}/Pisum.Langue_${VERSION}_aarch64.pkg"
       ```
    3. Compute SHA256:
       ```bash
       SHA256=$(shasum -a 256 pisum-langue.pkg | awk '{print $1}')
       ```
    4. Update `Casks/pisum-langue.rb` with new version and SHA256 using sed
    5. Commit and push the change
  - Reference: follow the same pattern from the `global-hotkey` tap repo workflow
- **Dependencies:** Task 4.1
- **Acceptance criteria:** Dispatch event triggers workflow that successfully updates the cask file and commits

### Phase 5: Release Body Update

> **Note:** This phase is already covered by Task 3.3. It's listed as a separate phase for clarity since the PRD's open question #3 explicitly calls it out, but the actual work is a single edit within Task 3.3.

## 6. Data Model Changes

No data model changes are required. This feature is entirely CI/CD and packaging.

## 7. API Changes

No API changes are required. This feature does not affect the Tauri IPC layer or any runtime behavior.

## 8. Dependencies & Risks

### External Dependencies
| Dependency | Purpose | Risk |
|-----------|---------|------|
| MyGet.org | Chocolatey package hosting | Service availability; API key must be configured as `MYGET_API_KEY` secret |
| `peter-evans/repository-dispatch@v2` | Triggers tap repo workflow | GitHub Action must remain available |
| `HOMEBREW_TAP_TOKEN` secret | PAT with `actions:write` scope on tap repo | Must be created and kept valid |

### Risks & Mitigations
| Risk | Impact | Mitigation |
|------|--------|-----------|
| MyGet push fails during release | Chocolatey package not updated | Workflow failure notification; can manually re-run job |
| Homebrew dispatch doesn't trigger | Cask not updated | Check PAT permissions; workflow logs on tap repo |
| SHA256 mismatch if assets re-uploaded | Install fails for users | Ensure `publish-release` completes before package jobs download artifacts |
| Tap repo naming conflict | `brew tap` fails | Verify `homebrew-pisum-langue` follows Homebrew naming convention |

### Assumptions
- The `mschnecke/homebrew-pisum-langue` repository already exists
- MyGet account and API key are already configured (per PRD open question #2)
- `HOMEBREW_TAP_TOKEN` PAT will be created with appropriate scope

## 9. Testing Strategy

### Manual Verification (Post-First-Release)
- Trigger a release via `workflow_dispatch` and verify:
  - `bump-version` updates `pisum-langue.nuspec` alongside other version files
  - `update-homebrew` dispatches to the correct tap repo
  - `update-chocolatey` successfully packs and pushes to MyGet
  - Tap repo's `update-cask.yml` runs, updates cask, and commits

### End-to-End Install Tests
- **macOS:** `brew tap mschnecke/pisum-langue && brew install --cask pisum-langue` installs successfully
- **Windows:** `choco install pisum-langue --source https://www.myget.org/F/mschnecke/api/v3/index.json` installs successfully
- **Upgrade:** After a second release, `brew upgrade pisum-langue` and `choco upgrade pisum-langue` pull the new version

### Pre-Release Checks
- `choco pack` in `packages/chocolatey/` produces a valid `.nupkg` locally
- `pisum-langue.rb` cask syntax is valid (can lint with `brew audit --cask`)

## 10. Requirement Traceability

### Functional Requirements

| PRD Ref | Requirement Summary | Task(s) | Notes |
|---------|-------------------|---------|-------|
| #1 | Separate `homebrew-pisum-langue` tap repo | — | Already created |
| #2 | Cask formula at `Casks/pisum-langue.rb` installing `.pkg` | 4.1 | |
| #3 | Cask targets aarch64 only, `depends_on macos: ">= :catalina"` | 2.1, 4.1 | |
| #4 | Cask includes `zap` stanza for app data cleanup | 2.1, 4.1 | All four paths included |
| #5 | Template cask at `packages/homebrew/pisum-langue.rb` | 2.1 | |
| #6 | Install via `brew tap` + `brew install --cask` | 4.1, 3.3 | Instructions in release body |
| #7 | Chocolatey nuspec at `packages/chocolatey/pisum-langue.nuspec` | 1.1 | |
| #8 | Install script with `Install-ChocolateyPackage` + SHA256 | 1.2 | |
| #9 | Uninstall script via registry lookup | 1.3 | |
| #10 | Publish to MyGet NuGet feed | Existing workflow | Already in `release.yml` lines 330-337 |
| #11 | Install via `choco install pisum-langue --source ...` | 1.1, 3.3 | Instructions in release body |
| #12 | `update-homebrew` job triggers dispatch to tap repo | 3.1 | Fix repo name typo |
| #13 | Tap workflow receives dispatch, updates cask | 4.2 | |
| #14 | `update-chocolatey` job downloads MSI, updates files, packs, pushes | Existing workflow | Already in `release.yml` lines 287-337 |
| #15 | `bump-version` updates nuspec version | 3.2 | |
| #16 | `HOMEBREW_TAP_TOKEN` secret configured | — (manual) | Needs `actions:write` on tap repo |
| #17 | `MYGET_API_KEY` secret configured | Existing | Already referenced in workflow |

### User Stories

| PRD Ref | User Story Summary | Implementing Tasks | Fully Covered? |
|---------|-------------------|-------------------|----------------|
| US-1 | macOS user installs via Homebrew | 2.1, 4.1, 4.2 | Yes |
| US-2 | Windows user installs via Chocolatey | 1.1, 1.2, 1.3 | Yes |
| US-3 | Maintainer: auto-publish on release | 3.1, 3.2, 4.2 | Yes |
| US-4 | User upgrades via package manager | 4.2 (Homebrew), existing workflow (Chocolatey) | Yes |

### Success Metrics

| Metric | How the Plan Addresses It |
|--------|--------------------------|
| `brew install --cask pisum-langue` works | Tap repo with cask (Phase 4) + template (Phase 2) |
| `choco install pisum-langue` works | Chocolatey package files (Phase 1) + existing workflow |
| Release auto-updates both packages | Workflow fixes (Phase 3) + tap workflow (Phase 4) |
| `brew upgrade` / `choco upgrade` works | Cask/nuspec version updates on each release |
