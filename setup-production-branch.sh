#!/bin/bash
set -e

echo "=== Tsurust Production Branch Setup ==="
echo ""

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "Error: Not in a git repository"
    exit 1
fi

# Check if production branch already exists
if git show-ref --verify --quiet refs/heads/production; then
    echo "✓ Production branch already exists"
    git checkout production
    echo ""
    echo "Production branch is ready!"
    echo "Current branch: $(git branch --show-current)"
    exit 0
fi

echo "Creating production branch from current state..."
echo ""

# Get current branch
CURRENT_BRANCH=$(git branch --show-current)
echo "Current branch: $CURRENT_BRANCH"
echo ""

# Ensure we're up to date
echo "Fetching latest changes..."
git fetch origin

# Create production branch from main (or current branch)
echo "Creating production branch..."
if git show-ref --verify --quiet refs/remotes/origin/main; then
    # Create from origin/main
    git checkout -b production origin/main
    echo "✓ Created production branch from origin/main"
elif git show-ref --verify --quiet refs/heads/main; then
    # Create from local main
    git checkout main
    git checkout -b production
    echo "✓ Created production branch from local main"
else
    # Create from current branch
    git checkout -b production
    echo "✓ Created production branch from $CURRENT_BRANCH"
fi

echo ""
echo "Production branch created successfully!"
echo ""

# Push to remote
read -p "Push production branch to remote? (y/n) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git push -u origin production
    echo "✓ Production branch pushed to remote"
    echo ""
    echo "=== Next Steps ==="
    echo "1. Go to GitHub: Settings → Branches"
    echo "2. Add branch protection rule for 'production'"
    echo "3. Enable: Require pull request reviews"
    echo "4. Enable: Require status checks to pass"
    echo ""
fi

echo "=== Production Branch Workflow ==="
echo ""
echo "Development:"
echo "  git checkout main"
echo "  git commit -m 'Add feature'"
echo "  git push origin main"
echo ""
echo "Deploy to production:"
echo "  git checkout production"
echo "  git merge main"
echo "  git push origin production  # Triggers GitHub Pages deployment"
echo ""
echo "Done! 🚀"
