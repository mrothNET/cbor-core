#! /bin/sh
set -eu

IMAGE_NAME=localhost/cbor-core-ci
CACHE_DIR=.cache

if [ "$(sed -n '1p' Cargo.toml 2>/dev/null)" != '[package]' ] \
  || [ "$(sed -n '2p' Cargo.toml 2>/dev/null)" != 'name = "cbor-core"' ]; then
  echo "Error: must be run from the cbor-core project root directory" >&2
  exit 1
fi

if [ "${1:-}" = "--clean" ]; then
  rm -rf "$CACHE_DIR/cargo-amd64" "$CACHE_DIR/cargo-i386" \
         "$CACHE_DIR/target-amd64" "$CACHE_DIR/target-i386"
  rmdir "$CACHE_DIR" 2>/dev/null || true
  exit 0
fi

if [ "${1:-}" = "--build" ]; then
  podman build -f Dockerfile.ci --pull --platform linux/amd64 -t $IMAGE_NAME:amd64 .
  podman build -f Dockerfile.ci --pull --platform linux/386   -t $IMAGE_NAME:i386  .
  exit 0
fi

run_ci() {
  mkdir -p "$CACHE_DIR/cargo-$1" "$CACHE_DIR/target-$1"
  podman run --rm -t --platform "linux/$1" \
    -v "$PWD:/work" -w /work \
    -v "$PWD/$CACHE_DIR/cargo-$1:/usr/local/cargo/registry" \
    -v "$PWD/$CACHE_DIR/target-$1:/target" \
    -e CARGO_TARGET_DIR=/target \
    "$IMAGE_NAME:$1" \
    bash -c "
      echo -- &&
      echo --  Build: $1 &&
      echo -- &&
      echo &&
      cargo build --all-targets &&
      cargo build --all-targets --release &&
      cargo build --all-targets --all-features &&
      cargo build --all-targets --all-features --release &&
      echo &&
      echo -- &&
      echo --  Test: $1 &&
      echo -- &&
      echo &&
      cargo test --all-features --quiet &&
      cargo test --all-features --quiet --release &&
      echo &&
      echo -- &&
      echo --  Clippy: $1 &&
      echo -- &&
      echo &&
      cargo clippy --all-targets --all-features -- -D warnings
    "
}

run_ci amd64
run_ci i386
