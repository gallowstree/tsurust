# Proposal 002: CI/CD Pipeline with GitHub Actions

**Status:** Draft
**Author:** Claude
**Date:** 2026-01-18
**Estimated Effort:** 2-3 days

---

## Summary

Set up a comprehensive CI/CD pipeline using GitHub Actions that automatically tests, builds, and publishes Tsurust on every PR, push to main, and release tag. The pipeline will build native clients for macOS, Linux, and Windows, build the WASM client, create Docker images, and push them to GitHub Container Registry.

---

## Motivation

Currently, there is **no automated testing or building**. This causes:

1. **Untested merges** - Bugs can slip into main branch
2. **Manual release process** - Building for all platforms is tedious
3. **Inconsistent builds** - Different machines produce different results
4. **No binary distribution** - Users must compile from source
5. **Docker images outdated** - Must manually build and push

---

## Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        TRIGGER                                   │
│  PR opened/updated │ Push to main │ Tag pushed (v*)             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      TEST & LINT                                 │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │
│  │ clippy  │  │ rustfmt │  │  tests  │  │  audit  │            │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (only if tests pass)
┌─────────────────────────────────────────────────────────────────┐
│                    BUILD ARTIFACTS                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Linux x64    │  │ macOS x64    │  │ macOS ARM    │          │
│  │ Linux ARM    │  │ Windows x64  │  │ WASM Client  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (only on main/tags)
┌─────────────────────────────────────────────────────────────────┐
│                   DOCKER & PUBLISH                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Docker Build │  │ Push to GHCR │  │ GitHub       │          │
│  │ Server+Client│  │              │  │ Release      │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

---

## Workflow Files

### 1. Main CI Workflow (`.github/workflows/ci.yml`)

```yaml
name: CI

on:
  push:
    branches: [main, master]
    tags: ['v*']
  pull_request:
    branches: [main, master]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  # ============================================================
  # LINT & FORMAT CHECK
  # ============================================================
  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          components: clippy, rustfmt

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

  # ============================================================
  # SECURITY AUDIT
  # ============================================================
  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install cargo-audit
        run: cargo install cargo-audit
      - name: Run audit
        run: cargo audit

  # ============================================================
  # TESTS
  # ============================================================
  test:
    name: Test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Run tests
        run: cargo test --workspace --verbose

  # ============================================================
  # WASM BUILD CHECK
  # ============================================================
  wasm-check:
    name: WASM Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Check WASM build
        run: cargo check --package client-egui --target wasm32-unknown-unknown

  # ============================================================
  # BUILD NATIVE BINARIES (only on main/tags)
  # ============================================================
  build-native:
    name: Build ${{ matrix.target }}
    needs: [lint, test, wasm-check]
    if: github.event_name == 'push'
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          # Linux
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: tsurust-linux-x64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            artifact: tsurust-linux-arm64

          # macOS
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: tsurust-macos-x64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: tsurust-macos-arm64

          # Windows
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: tsurust-windows-x64

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools (Linux ARM)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Build server
        run: cargo build --release --bin server --target ${{ matrix.target }}

      - name: Build client
        run: cargo build --release --bin client-egui_bin --target ${{ matrix.target }}

      - name: Package artifacts (Unix)
        if: runner.os != 'Windows'
        run: |
          mkdir -p dist
          cp target/${{ matrix.target }}/release/server dist/
          cp target/${{ matrix.target }}/release/client-egui_bin dist/tsurust-client
          chmod +x dist/*
          tar -czvf ${{ matrix.artifact }}.tar.gz -C dist .

      - name: Package artifacts (Windows)
        if: runner.os == 'Windows'
        run: |
          mkdir dist
          copy target\${{ matrix.target }}\release\server.exe dist\
          copy target\${{ matrix.target }}\release\client-egui_bin.exe dist\tsurust-client.exe
          Compress-Archive -Path dist\* -DestinationPath ${{ matrix.artifact }}.zip

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: |
            ${{ matrix.artifact }}.tar.gz
            ${{ matrix.artifact }}.zip
          if-no-files-found: ignore

  # ============================================================
  # BUILD WASM CLIENT (only on main/tags)
  # ============================================================
  build-wasm:
    name: Build WASM
    needs: [lint, test, wasm-check]
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Install Trunk
        run: cargo install trunk

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Build WASM client
        run: |
          cd client-egui
          trunk build --release

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: tsurust-wasm
          path: client-egui/dist/

  # ============================================================
  # BUILD & PUSH DOCKER IMAGES (only on main/tags)
  # ============================================================
  docker:
    name: Docker (${{ matrix.image }})
    needs: [lint, test]
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    strategy:
      matrix:
        include:
          - image: server
            dockerfile: server/Dockerfile
          - image: client
            dockerfile: client-egui/Dockerfile
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Login to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/${{ github.repository }}/${{ matrix.image }}
          tags: |
            type=ref,event=branch
            type=ref,event=pr
            type=semver,pattern={{version}}
            type=semver,pattern={{major}}.{{minor}}
            type=sha

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          file: ${{ matrix.dockerfile }}
          push: ${{ github.event_name != 'pull_request' }}
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  # ============================================================
  # CREATE GITHUB RELEASE (only on tags)
  # ============================================================
  release:
    name: Create Release
    needs: [build-native, build-wasm, docker]
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            artifacts/**/*.tar.gz
            artifacts/**/*.zip
          generate_release_notes: true
          draft: false
          prerelease: ${{ contains(github.ref, '-') }}
```

---

## Docker Image Tags

Images will be published to `ghcr.io/<owner>/tsurust/`:

| Image | Tags |
|-------|------|
| `ghcr.io/<owner>/tsurust/server` | `main`, `v1.0.0`, `v1.0`, `sha-abc1234` |
| `ghcr.io/<owner>/tsurust/client` | `main`, `v1.0.0`, `v1.0`, `sha-abc1234` |

**Usage:**
```bash
# Pull latest from main
docker pull ghcr.io/gallowstree/tsurust/server:main
docker pull ghcr.io/gallowstree/tsurust/client:main

# Pull specific version
docker pull ghcr.io/gallowstree/tsurust/server:v1.0.0
```

---

## Release Process

### Creating a Release

```bash
# Tag the release
git tag v1.0.0
git push origin v1.0.0

# GitHub Actions will automatically:
# 1. Run all tests
# 2. Build binaries for all platforms
# 3. Build WASM client
# 4. Build and push Docker images
# 5. Create GitHub Release with all artifacts
```

### Release Artifacts

Each release will include:

| Artifact | Contents |
|----------|----------|
| `tsurust-linux-x64.tar.gz` | Server + Client (Linux x64) |
| `tsurust-linux-arm64.tar.gz` | Server + Client (Linux ARM64) |
| `tsurust-macos-x64.tar.gz` | Server + Client (macOS Intel) |
| `tsurust-macos-arm64.tar.gz` | Server + Client (macOS Apple Silicon) |
| `tsurust-windows-x64.zip` | Server + Client (Windows x64) |
| `tsurust-wasm.zip` | WASM client (dist folder) |

---

## Branch Protection Rules

Recommended settings for `main` branch:

- [x] Require pull request before merging
- [x] Require status checks to pass:
  - `lint`
  - `test (ubuntu-latest)`
  - `test (macos-latest)`
  - `test (windows-latest)`
  - `wasm-check`
  - `audit`
- [x] Require branches to be up to date
- [ ] Require signed commits (optional)
- [x] Do not allow bypassing the above settings

---

## Estimated CI Times

| Job | Estimated Time |
|-----|----------------|
| Lint | 1-2 min |
| Audit | 1 min |
| Test (per OS) | 3-5 min |
| WASM Check | 2-3 min |
| Build Native (per target) | 5-10 min |
| Build WASM | 3-5 min |
| Docker Build | 5-10 min |

**Total PR Check:** ~10-15 minutes (parallel jobs)
**Total Release Build:** ~20-30 minutes

---

## Cost Considerations

**GitHub Actions Free Tier:**
- 2,000 minutes/month for private repos
- Unlimited for public repos
- macOS runners use 10x minutes

**Estimated Usage (active development):**
- ~50 PR checks/month × 15 min = 750 min
- ~30 main pushes/month × 30 min = 900 min
- **Total:** ~1,650 min/month (within free tier)

---

## Implementation Steps

### Day 1: Basic CI
- [ ] Create `.github/workflows/ci.yml`
- [ ] Implement lint, test, audit jobs
- [ ] Test on a PR
- [ ] Set up branch protection

### Day 2: Build & Docker
- [ ] Add native build jobs
- [ ] Add WASM build job
- [ ] Add Docker build and push
- [ ] Test Docker image push

### Day 3: Release Automation
- [ ] Add release job
- [ ] Test with a tag push
- [ ] Document release process
- [ ] Final review

---

## Success Criteria

- [ ] All jobs pass on a clean PR
- [ ] Native binaries build for all 5 targets
- [ ] WASM client builds successfully
- [ ] Docker images pushed to GHCR
- [ ] Release creates downloadable artifacts
- [ ] Branch protection enforces CI checks
- [ ] Total PR check time < 15 minutes

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| macOS ARM cross-compilation | Build on macos-latest (has ARM support) |
| Slow Docker builds | Use GitHub Actions cache |
| Flaky tests | Add retry logic, fix flaky tests |
| Minutes quota exceeded | Monitor usage, optimize caching |
| GHCR rate limits | Use authenticated pulls |

---

## Future Enhancements

1. **Staging deployment** - Auto-deploy to staging on main push
2. **Production deployment** - Manual approval gate for prod
3. **Performance benchmarks** - Run and track benchmarks
4. **Code coverage** - Add coverage reports with codecov
5. **Changelog generation** - Auto-generate from PR titles
6. **Slack/Discord notifications** - Alert on failures

---

## Approval

- [ ] Workflow design approved
- [ ] Branch protection rules agreed
- [ ] Release process documented
- [ ] Ready to implement
