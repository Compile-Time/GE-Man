#!/usr/bin/env bash

TARGET="x86_64-unknown-linux-gnu"
RELEASE_DIR="release"

if [ -d release ]; then
  rm -r "$RELEASE_DIR"
fi
mkdir "$RELEASE_DIR"
cargo build --release --target "$TARGET"

VERSION="$(grep -Eo '[0-9]\.[0-9]\.[0-9]' Cargo.toml | head -n 1)"
BINARY="geman-$VERSION-$TARGET"
cp target/x86_64-unknown-linux-gnu/release/geman "$RELEASE_DIR/$BINARY"

ARCHIVE="$BINARY.tar.gz"
tar -czf "$RELEASE_DIR/$BINARY".tar.gz "$RELEASE_DIR/$BINARY"

cd "$RELEASE_DIR" || exit
sha512sum "$ARCHIVE" >"$BINARY.sha512sum"
