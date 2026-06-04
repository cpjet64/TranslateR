#!/bin/sh
set -eu

if command -v apt-get >/dev/null 2>&1; then
  if command -v sudo >/dev/null 2>&1; then
    SUDO=sudo
  else
    SUDO=
  fi

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
fi

if command -v rustup >/dev/null 2>&1; then
  rustup default stable
  rustup component add rustfmt
fi
