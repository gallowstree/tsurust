# Tsurust Production Branch Setup Script for Windows
# Run this in PowerShell

Write-Host "=== Tsurust Production Branch Setup ===" -ForegroundColor Cyan
Write-Host ""

# Check if we're in a git repository
try {
    git rev-parse --git-dir 2>&1 | Out-Null
} catch {
    Write-Host "Error: Not in a git repository" -ForegroundColor Red
    exit 1
}

# Check if production branch already exists
$productionExists = git show-ref --verify refs/heads/production 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Production branch already exists" -ForegroundColor Green
    git checkout production
    Write-Host ""
    Write-Host "Production branch is ready!" -ForegroundColor Green
    $currentBranch = git branch --show-current
    Write-Host "Current branch: $currentBranch"
    exit 0
}

Write-Host "Creating production branch from current state..."
Write-Host ""

# Get current branch
$currentBranch = git branch --show-current
Write-Host "Current branch: $currentBranch"
Write-Host ""

# Ensure we're up to date
Write-Host "Fetching latest changes..."
git fetch origin

# Create production branch from main (or current branch)
Write-Host "Creating production branch..."

$originMainExists = git show-ref --verify refs/remotes/origin/main 2>&1
$localMainExists = git show-ref --verify refs/heads/main 2>&1

if ($LASTEXITCODE -eq 0 -and $originMainExists) {
    # Create from origin/main
    git checkout -b production origin/main
    Write-Host "✓ Created production branch from origin/main" -ForegroundColor Green
} elseif ($localMainExists) {
    # Create from local main
    git checkout main
    git checkout -b production
    Write-Host "✓ Created production branch from local main" -ForegroundColor Green
} else {
    # Create from current branch
    git checkout -b production
    Write-Host "✓ Created production branch from $currentBranch" -ForegroundColor Green
}

Write-Host ""
Write-Host "Production branch created successfully!" -ForegroundColor Green
Write-Host ""

# Push to remote
$response = Read-Host "Push production branch to remote? (y/n)"
if ($response -eq "y" -or $response -eq "Y") {
    git push -u origin production
    Write-Host "✓ Production branch pushed to remote" -ForegroundColor Green
    Write-Host ""
    Write-Host "=== Next Steps ===" -ForegroundColor Yellow
    Write-Host "1. Go to GitHub: Settings → Branches"
    Write-Host "2. Add branch protection rule for 'production'"
    Write-Host "3. Enable: Require pull request reviews"
    Write-Host "4. Enable: Require status checks to pass"
    Write-Host ""
}

Write-Host "=== Production Branch Workflow ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Development:" -ForegroundColor Yellow
Write-Host "  git checkout main"
Write-Host "  git commit -m 'Add feature'"
Write-Host "  git push origin main"
Write-Host ""
Write-Host "Deploy to production:" -ForegroundColor Yellow
Write-Host "  git checkout production"
Write-Host "  git merge main"
Write-Host "  git push origin production  # Triggers GitHub Pages deployment"
Write-Host ""
Write-Host "Done! 🚀" -ForegroundColor Green
