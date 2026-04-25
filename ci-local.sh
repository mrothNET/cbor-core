#! /bin/sh
set -eu

IMAGE_NAME=localhost/cbor-core-ci
CACHE_DIR=.cache

if [ "$(sed -n '1p' Cargo.toml 2>/dev/null)" != '[package]' ] \
  || [ "$(sed -n '2p' Cargo.toml 2>/dev/null)" != 'name = "cbor-core"' ]; then
  echo "Error: must be run from the cbor-core project root directory" >&2
  exit 1
fi

cmd_help() {
  cat <<EOF
Usage: $0 <command>

Commands:
  run     Run build, test, and clippy in CI containers (amd64 and i386)
  build   Build the CI container images
  clean   Remove cached cargo and target directories
  help    Show this help message (also --help, -h)
EOF
}

cmd_clean() {
  rm -rf "$CACHE_DIR/cargo-amd64" "$CACHE_DIR/cargo-i386" \
         "$CACHE_DIR/target-amd64" "$CACHE_DIR/target-i386"
  rmdir "$CACHE_DIR" 2>/dev/null || true
}

cmd_build() {
  podman build -f Dockerfile.ci --pull --platform linux/amd64 -t $IMAGE_NAME:amd64 .
  podman build -f Dockerfile.ci --pull --platform linux/386   -t $IMAGE_NAME:i386  .
}

cmd_run() {
  run_ci amd64
  run_ci i386
}

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

case "${1:-}" in
  help|--help|-h) cmd_help ;;
  clean)          cmd_clean ;;
  build)          cmd_build ;;
  run)            cmd_run ;;
  "")
    echo "Error: a command is required. Run '$0 help' for usage." >&2
    exit 1
    ;;
  *)
    echo "Error: unknown command '$1'. Run '$0 help' for usage." >&2
    exit 1
    ;;
esac
