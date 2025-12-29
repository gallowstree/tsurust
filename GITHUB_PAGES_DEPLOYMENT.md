# GitHub Pages Deployment Guide

This guide explains how to deploy the Tsurust WASM client to GitHub Pages using Trunk and GitHub Actions.

## Quick Start

1. **Enable GitHub Pages in your repository**
2. **Configure WebSocket server URL**
3. **Push to trigger deployment**

Your client will be live at: `https://<username>.github.io/tsurust/`

---

## Setup Instructions

### 1. Enable GitHub Pages

1. Go to your repository on GitHub
2. Navigate to **Settings** → **Pages**
3. Under **Source**, select:
   - **Branch**: `gh-pages`
   - **Folder**: `/ (root)`
4. Click **Save**

**Note:** The `gh-pages` branch will be created automatically on first deployment.

### 2. Configure WebSocket Server URL

You have **three options** for configuring the production WebSocket server URL:

#### **Option A: Repository Variable (Recommended)**

Set a repository variable that the workflow will use:

1. Go to **Settings** → **Secrets and variables** → **Actions** → **Variables**
2. Click **New repository variable**
3. Name: `WS_SERVER_URL`
4. Value: Your production WebSocket URL (e.g., `wss://api.yourdomain.com`)
5. Click **Add variable**

#### **Option B: Manual Workflow Trigger**

Manually trigger deployment with a custom URL:

1. Go to **Actions** → **Deploy to GitHub Pages**
2. Click **Run workflow**
3. Enter your WebSocket Server URL
4. Click **Run workflow**

#### **Option C: Edit Workflow File**

Hardcode the URL in `.github/workflows/deploy-pages.yml`:

```yaml
- name: Configure production WebSocket URL
  run: |
    WS_URL="wss://your-production-server.com"  # Change this
```

### 3. Deploy

**Automatic Deployment (on push to main):**
```bash
git add .
git commit -m "Update client"
git push origin main
```

**Manual Deployment:**
1. Go to **Actions** → **Deploy to GitHub Pages**
2. Click **Run workflow**
3. Select branch and configure URL if needed
4. Click **Run workflow**

---

## Branch Strategy

### Current Setup: Deploy from `main`

The workflow is configured to deploy automatically when you push to the `main` branch.

```yaml
on:
  push:
    branches:
      - main  # Auto-deploy on push to main
```

### Option: Use Dedicated Production Branch

For more control, create a `production` branch for deployments:

**1. Update workflow to deploy from `production` branch:**

Edit `.github/workflows/deploy-pages.yml`:
```yaml
on:
  push:
    branches:
      - production  # Change from 'main' to 'production'
```

**2. Create and push production branch:**
```bash
# Create production branch from main
git checkout main
git pull
git checkout -b production
git push -u origin production
```

**3. Deployment workflow:**
```bash
# Development on main
git checkout main
# ... make changes ...
git commit -m "Add new feature"
git push origin main

# Deploy to production
git checkout production
git merge main
git push origin production  # This triggers deployment
```

### Option: Use Release Tags

Deploy only on tagged releases:

**Update workflow:**
```yaml
on:
  push:
    tags:
      - 'v*.*.*'  # Deploy on version tags (v1.0.0, v2.1.3, etc.)
```

**Deploy a release:**
```bash
git tag v1.0.0
git push origin v1.0.0  # This triggers deployment
```

---

## Environment Configuration

### Development (Local)

```bash
# Install Trunk
cargo install --locked trunk

# Serve locally
cd client-egui
trunk serve --open

# Access at http://localhost:8080
# Uses default config: ws://127.0.0.1:8080
```

### Staging (GitHub Pages with test server)

Configure a staging server URL:

1. Create a staging branch
2. Set `WS_SERVER_URL` variable to staging server
3. Deploy from staging branch

### Production (GitHub Pages with prod server)

1. Set `WS_SERVER_URL` to production server (e.g., `wss://api.yourdomain.com`)
2. Deploy from `main` or `production` branch
3. Access at `https://<username>.github.io/tsurust/`

---

## Custom Domain (Optional)

To use a custom domain instead of `<username>.github.io/tsurust`:

### 1. Set up DNS

Add CNAME record pointing to `<username>.github.io`:
```
CNAME    tsurust    <username>.github.io
```

### 2. Configure in repository

1. Go to **Settings** → **Pages**
2. Under **Custom domain**, enter: `tsurust.yourdomain.com`
3. Click **Save**
4. Wait for DNS check to complete

### 3. Update workflow (optional)

Set `CUSTOM_DOMAIN` variable in repository:
```
Name: CUSTOM_DOMAIN
Value: tsurust.yourdomain.com
```

The workflow will automatically create a `CNAME` file in the deployment.

---

## Troubleshooting

### Deployment Failed

**Check workflow logs:**
1. Go to **Actions**
2. Click on the failed workflow run
3. Expand failed steps to see error messages

**Common issues:**
- **Trunk build failed**: Check Rust/WASM compilation errors
- **Permission denied**: Ensure `contents: write` permission in workflow
- **Pages not enabled**: Enable GitHub Pages in repository settings

### Client Loads but Can't Connect

**Check WebSocket URL:**
1. Open browser console (F12)
2. Look for connection errors
3. Verify `WS_SERVER_URL` matches your server

**Update WebSocket URL:**
- Edit repository variable `WS_SERVER_URL`
- Re-run deployment workflow

### Changes Not Visible

**GitHub Pages caching:**
- Wait 2-5 minutes for GitHub Pages to update
- Hard refresh browser (Ctrl+Shift+R / Cmd+Shift+R)
- Check deployment timestamp in Actions

---

## Monitoring Deployments

### View Deployment Status

1. Go to **Actions** tab
2. See list of workflow runs
3. Green checkmark = successful deployment
4. Red X = failed deployment

### Deployment URL

After successful deployment, access your app at:
```
https://<username>.github.io/tsurust/
```

**Example:**
- Username: `gallowstree`
- URL: `https://gallowstree.github.io/tsurust/`

---

## Cost and Limits

### GitHub Pages Limits

- **Storage**: 1 GB
- **Bandwidth**: 100 GB/month
- **Build time**: 10 minutes per build
- **Builds**: 10 per hour

### What This Means for Tsurust

- ✅ **Client WASM**: ~3-5 MB (well under limit)
- ✅ **Bandwidth**: Typical game session uses ~1-5 MB
- ✅ **Cost**: **Free** (within GitHub free tier)

**Note:** These limits apply to the static WASM client only. Your WebSocket server needs separate hosting (AWS, GCP, etc.).

---

## CI/CD Best Practices

### 1. Protect Production Branch

If using a dedicated production branch:

1. Go to **Settings** → **Branches**
2. Click **Add rule**
3. Branch name pattern: `production`
4. Enable:
   - ✅ Require pull request reviews
   - ✅ Require status checks to pass
   - ✅ Require branches to be up to date

### 2. Add Status Badge

Show deployment status in README:

```markdown
[![Deploy to GitHub Pages](https://github.com/<username>/tsurust/actions/workflows/deploy-pages.yml/badge.svg)](https://github.com/<username>/tsurust/actions/workflows/deploy-pages.yml)
```

### 3. Automated Testing

Add tests before deployment:

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --workspace

  build-and-deploy:
    needs: test  # Only deploy if tests pass
    runs-on: ubuntu-latest
    # ... deployment steps
```

---

## Server Deployment

Remember: GitHub Pages only hosts the **client**. You still need to deploy the **server** separately.

### Recommended Server Setup

1. **Deploy server to cloud** (AWS, GCP, Azure, DigitalOcean)
2. **Use SSL/TLS** for production (`wss://` not `ws://`)
3. **Configure CORS** to allow requests from GitHub Pages domain
4. **Set up monitoring** and health checks

See `DEPLOYMENT.md` for server containerization and deployment guides.

---

## Quick Reference

| Task | Command |
|------|---------|
| **Local development** | `trunk serve --open` |
| **Build locally** | `trunk build --release` |
| **Deploy (auto)** | Push to `main` branch |
| **Deploy (manual)** | Actions → Run workflow |
| **View deployment** | `https://<username>.github.io/tsurust/` |
| **Check logs** | Actions → Latest workflow run |

---

## Next Steps

1. ✅ Enable GitHub Pages
2. ✅ Set `WS_SERVER_URL` variable
3. ✅ Push to trigger first deployment
4. 🚀 Deploy your WebSocket server to cloud
5. 🎮 Share your game with the world!
