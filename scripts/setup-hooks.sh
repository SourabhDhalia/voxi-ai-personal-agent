#!/bin/bash
# Install Voxi git hooks
# Usage: ./scripts/setup-hooks.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
HOOKS_DIR="${REPO_DIR}/.git/hooks"

if [ ! -d "${HOOKS_DIR}" ]; then
  echo "Error: Not a git repository."
  exit 1
fi

ln -sf "${SCRIPT_DIR}/pre-commit" \
  "${HOOKS_DIR}/pre-commit"
chmod +x "${SCRIPT_DIR}/pre-commit"

echo "Git hooks installed:"
echo "  pre-commit -> scripts/pre-commit"
echo ""
echo "To uninstall: rm .git/hooks/pre-commit"
