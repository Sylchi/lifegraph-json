#!/bin/bash
# Release script for lifegraph-json
#
# Automates the release process:
# 1. Validates current state
# 2. Updates version in Cargo.toml
# 3. Updates CHANGELOG
# 4. Creates git tag
# 5. Pushes to trigger CI release workflow
#
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 1.0.150

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== lifegraph-json Release Script ==="
echo ""

# Check arguments
if [[ -z "$1" ]]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 1.0.150"
    exit 1
fi

VERSION="$1"

# Validate version format
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Invalid version format. Use semantic versioning (e.g., 1.0.150)${NC}"
    exit 1
fi

# Check we're on main branch
BRANCH=$(git branch --show-current)
if [[ "$BRANCH" != "main" ]]; then
    echo -e "${YELLOW}Warning: You're on branch '$BRANCH', not 'main'${NC}"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check working directory is clean
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${RED}Error: Working directory has uncommitted changes${NC}"
    echo "Please commit or stash changes before releasing."
    exit 1
fi

echo "Releasing version $VERSION..."
echo ""

# Step 1: Update Cargo.toml
echo "Step 1: Updating Cargo.toml..."
sed -i "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" Cargo.toml
echo "✓ Cargo.toml updated"

# Step 2: Update CHANGELOG
echo ""
echo "Step 2: Updating CHANGELOG..."
DATE=$(date +%Y-%m-%d)
if grep -q "## \[Unreleased\]" CHANGELOG.md; then
    # Replace [Unreleased] with version
    sed -i "s/## \[Unreleased\]/## [Unreleased]\n\n## [$VERSION] - $DATE/" CHANGELOG.md
else
    # Add new version section at top after first line
    sed -i "2i\\
\\
## [$VERSION] - $DATE\\
" CHANGELOG.md
fi
echo "✓ CHANGELOG updated"

# Step 3: Run tests
echo ""
echo "Step 3: Running tests..."
if ! cargo test --quiet; then
    echo -e "${RED}Error: Tests failed${NC}"
    exit 1
fi
echo "✓ Tests passed"

# Step 4: Build release
echo ""
echo "Step 4: Building release..."
if ! cargo build --release --quiet; then
    echo -e "${RED}Error: Release build failed${NC}"
    exit 1
fi
echo "✓ Release build successful"

# Step 5: Commit changes
echo ""
echo "Step 5: Committing changes..."
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release version $VERSION"
echo "✓ Changes committed"

# Step 6: Create tag
echo ""
echo "Step 6: Creating git tag..."
git tag -a "v$VERSION" -m "Release version $VERSION"
echo "✓ Tag created: v$VERSION"

# Step 7: Push
echo ""
echo "Step 7: Pushing to remote..."
echo -e "${YELLOW}This will push commits and tags to the remote repository.${NC}"
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted. You can manually push with:"
    echo "  git push && git push --tags"
    exit 1
fi

git push && git push --tags
echo "✓ Pushed to remote"

echo ""
echo -e "${GREEN}=== Release Complete ===${NC}"
echo ""
echo "Next steps:"
echo "1. CI will automatically publish to crates.io"
echo "2. GitHub release will be created"
echo "3. Monitor the Actions tab for progress"
echo ""
echo "If something goes wrong, you can:"
echo "  - Delete the tag: git tag -d v$VERSION && git push origin :refs/tags/v$VERSION"
echo "  - Reset the commit: git reset --hard HEAD~1"
echo ""
