#!/bin/sh
set -eu

if command -v apt-get >/dev/null 2>&1; then
  if [ "$(id -u)" -eq 0 ]; then
    SUDO=
  elif command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
    SUDO=sudo
  else
    SUDO=skip
  fi

  if [ "$SUDO" != "skip" ]; then
    $SUDO apt-get update
    $SUDO apt-get install -y \
      build-essential \
      pkg-config \
      libgtk-3-dev \
      libx11-dev \
      libxcb1-dev \
      libxcb-render0-dev \
      libxcb-shape0-dev \
      libxcb-xfixes0-dev \
      libxkbcommon-dev \
      libssl-dev
  else
    echo "Skipping apt package setup because passwordless sudo is unavailable."
  fi
fi

if command -v rustup >/dev/null 2>&1; then
  rustup default stable
  rustup component add rustfmt
fi

if [ "${INSTALL_COVERAGE_TOOLS:-}" = "1" ]; then
  if command -v rustup >/dev/null 2>&1; then
    rustup component add llvm-tools-preview
  fi

  if ! cargo llvm-cov --version >/dev/null 2>&1; then
    cargo install cargo-llvm-cov --locked
  fi
fi
