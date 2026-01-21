# Proposal 003: CI/CD Pipeline Implementation

**Status:** Draft
**Author:** Claude
**Date:** 2026-01-19
**Related:** Proposal 002 (CI/CD Pipeline Design)

---

## Summary

This proposal outlines the implementation plan for the CI/CD pipeline defined in Proposal 002. It covers the specific files to be created, configuration decisions, and step-by-step implementation approach.

---

## Scope

### In Scope
- GitHub Actions workflow file (`.github/workflows/ci.yml`)
- Lint, test, and audit jobs for all PRs
- Native binary builds for 4 platforms (Linux x64, macOS x64/ARM, Windows x64)
- WASM client build with Trunk
- Docker image builds and push to GitHub Container Registry
- Automated GitHub Releases on version tags

### Out of Scope (Deferred)
- Linux ARM64 cross-compilation (requires additional setup)
- Branch protection rules (manual configuration in GitHub UI)
- Staging/production deployment automation
- Slack/Discord notifications

---

## Implementation Details

### File Structure

```
.github/
└── workflows/
    └── ci.yml          # Main CI/CD workflow
```

### Workflow Jobs

| Job | Trigger | Runner | Dependencies |
|-----|---------|--------|--------------|
| `lint` | All | ubuntu-latest | None |
| `audit` | All | ubuntu-latest | None |
| `test` | All | ubuntu/macos/windows | None |
| `wasm-check` | All | ubuntu-latest | None |
| `build-native` | Push to main/tags | Per-target | lint, test, wasm-check |
| `build-wasm` | Push to main/tags | ubuntu-latest | lint, test, wasm-check |
| `docker` | Push to main/tags | ubuntu-latest | lint, test |
| `release` | Tags only | ubuntu-latest | build-native, build-wasm, docker |

### Build Matrix

**Native Builds:**
| Target | Runner | Artifact Name |
|--------|--------|---------------|
| `x86_64-unknown-linux-gnu` | ubuntu-latest | tsurust-linux-x64 |
| `x86_64-apple-darwin` | macos-latest | tsurust-macos-x64 |
| `aarch64-apple-darwin` | macos-latest | tsurust-macos-arm64 |
| `x86_64-pc-windows-msvc` | windows-latest | tsurust-windows-x64 |

**Docker Images:**
| Image | Dockerfile | Registry |
|-------|------------|----------|
| server | server/Dockerfile | ghcr.io/{owner}/tsurust/server |
| client | client-egui/Dockerfile | ghcr.io/{owner}/tsurust/client |

### Key Configuration Decisions

1. **Rust Toolchain Action**: Using `dtolnay/rust-toolchain@stable`
   - More reliable than `actions-rs/toolchain` (deprecated)
   - Automatic caching of toolchain downloads

2. **Cargo Caching**: Using `Swatinem/rust-cache@v2`
   - Caches `~/.cargo` and `target/` directories
   - Significantly reduces build times on subsequent runs

3. **Docker Caching**: Using GitHub Actions cache (`type=gha`)
   - Leverages GitHub's built-in cache storage
   - No need for external registry for cache layers

4. **Release Action**: Using `softprops/action-gh-release@v1`
   - Auto-generates release notes from commits
   - Supports prerelease detection via semver

---

## Changes Required

### New Files

**`.github/workflows/ci.yml`** (~200 lines)
- Complete workflow definition
- All jobs and build matrix configurations

### No Existing Files Modified

The CI/CD implementation is additive only.

---

## Verification Plan

### Pre-Push Verification
1. Validate YAML syntax locally
2. Review all job dependencies and conditions
3. Verify Dockerfile paths match repository structure

### Post-Push Verification
1. **PR Test**: Create a test PR to verify lint/test/audit jobs
2. **Push Test**: Merge to main to verify build jobs trigger
3. **Tag Test**: Create `v0.0.1-test` tag to verify release flow
4. **Docker Test**: Verify images appear in GitHub Packages

### Expected Outcomes

| Event | Expected Jobs |
|-------|---------------|
| PR opened | lint, audit, test (3x), wasm-check |
| Push to main | All jobs including builds and docker |
| Tag v1.0.0 | All jobs + release creation |

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Workflow syntax errors | Medium | Low | Local YAML validation, incremental testing |
| Build failures on specific platforms | Medium | Medium | Test on all platforms before tagging |
| Docker build timeout | Low | Medium | Use build caching, optimize Dockerfiles |
| GHCR authentication issues | Low | High | Use built-in GITHUB_TOKEN, verify permissions |
| Trunk version incompatibility | Low | Medium | Pin Trunk version if needed |

---

## Resource Usage Estimate

**Per PR (parallel jobs):**
- lint: ~2 min
- audit: ~1 min
- test (3x parallel): ~5 min each
- wasm-check: ~3 min
- **Total wall time: ~5-7 min**

**Per main push (including builds):**
- Above jobs + builds: ~15-20 min total

**GitHub Actions minutes (monthly estimate):**
- Assuming 50 PRs × 20 min = 1000 min
- Assuming 30 main pushes × 40 min = 1200 min
- **Total: ~2200 min/month** (within free tier for public repos)

---

## Implementation Steps

### Step 1: Create Workflow File
- [ ] Create `.github/workflows/` directory
- [ ] Create `ci.yml` with all job definitions
- [ ] Validate YAML syntax

### Step 2: Initial Testing
- [ ] Push to a feature branch
- [ ] Create PR to trigger lint/test jobs
- [ ] Fix any failures

### Step 3: Build Testing
- [ ] Merge PR to main
- [ ] Verify native builds complete
- [ ] Verify WASM build completes
- [ ] Verify Docker images pushed to GHCR

### Step 4: Release Testing
- [ ] Create test tag (e.g., v0.0.1-test)
- [ ] Verify release created with artifacts
- [ ] Delete test release and tag

### Step 5: Documentation
- [ ] Update README with CI badge
- [ ] Document release process

---

## Success Criteria

- [ ] All jobs pass on a clean PR
- [ ] Native binaries build for all 4 targets
- [ ] WASM client builds successfully
- [ ] Docker images pushed to GHCR with correct tags
- [ ] Release created with downloadable artifacts
- [ ] Total PR check time < 10 minutes

---

## Approval

- [ ] Implementation plan reviewed
- [ ] Resource usage acceptable
- [ ] Ready to implement
