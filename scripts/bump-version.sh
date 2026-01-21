#!/bin/bash
# Usage: ./scripts/bump-version.sh 0.2.0

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

VERSION=$1
ROOT=$(git rev-parse --show-toplevel)

echo "Bumping version to $VERSION..."

# Rust (root Cargo.toml)
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$ROOT/Cargo.toml"

# Python Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$ROOT/packages/python/Cargo.toml"

# Python pyproject.toml
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$ROOT/packages/python/pyproject.toml"

# Python __init__.py
sed -i '' "s/__version__ = \".*\"/__version__ = \"$VERSION\"/" "$ROOT/packages/python/python/chonkie_core/__init__.py"

# WASM Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$ROOT/packages/wasm/Cargo.toml"

# npm package.json
sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" "$ROOT/packages/wasm/package.json"

echo "Updated versions:"
echo "  - Cargo.toml: $(grep '^version' "$ROOT/Cargo.toml" | head -1)"
echo "  - packages/python/Cargo.toml: $(grep '^version' "$ROOT/packages/python/Cargo.toml")"
echo "  - packages/python/pyproject.toml: $(grep '^version' "$ROOT/packages/python/pyproject.toml")"
echo "  - packages/python/__init__.py: $(grep '__version__' "$ROOT/packages/python/python/chonkie_core/__init__.py")"
echo "  - packages/wasm/Cargo.toml: $(grep '^version' "$ROOT/packages/wasm/Cargo.toml")"
echo "  - packages/wasm/package.json: $(grep '\"version\"' "$ROOT/packages/wasm/package.json")"
