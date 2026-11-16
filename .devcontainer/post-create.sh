#!/usr/bin/env bash
# Runs once inside the dev container, from the project folder, right after the
# container is created (see postCreateCommand in devcontainer.json).
set -euo pipefail

# Docker volumes can start out owned by root on some setups (for example
# rootless Podman). Make sure the folders mounted from volumes are writable.
for dir in "${CARGO_TARGET_DIR}" "${CARGO_HOME}/registry" "${HOME}/.config/git"; do
  if [ ! -w "$dir" ]; then
    sudo chown -R "$(id -u):$(id -g)" "$dir"
  fi
done

# The project folder is shared from your computer, so its owner can differ
# from the container's user (notably on Windows). Tell git that's expected.
workspace="$(pwd)"
if ! git config --global --get-all safe.directory | grep -qxF "$workspace"; then
  git config --global --add safe.directory "$workspace"
fi

# Download the crates now so the first build is quicker.
if ! cargo fetch --locked; then
  echo "Couldn't download crates right now; they'll download on the first build."
fi

echo
echo "Dev container ready. Tools:"
echo "  $(rustc --version)"
echo "  $(trunk --version)"
echo "  $(wasm-bindgen --version)"
echo "  $(wasm-opt --version)"
echo
echo "Start the site with the \"trunk serve\" task, then open http://localhost:${TRUNK_SERVE_PORT:-8080}"

if [ -z "$(git config --global user.name || true)" ]; then
  echo
  echo "To commit from inside the container, set your git name and email once"
  echo "(they're kept when the container is rebuilt):"
  echo "  git config --global user.name \"Your Name\""
  echo "  git config --global user.email \"you@example.com\""
fi
