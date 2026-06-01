#! /bin/sh
set -eu

IMAGE_NAME=localhost/cbor-core-ci
CACHE_DIR=.cache

PLATFORMS="amd64 i386"
RUST_VERSIONS="msrv stable nightly"

if [ "$(sed -n '1p' Cargo.toml 2>/dev/null)" != '[package]' ] \
  || [ "$(sed -n '2p' Cargo.toml 2>/dev/null)" != 'name = "cbor-core"' ]; then
  echo "Error: must be run from the cbor-core project root directory" >&2
  exit 1
fi

MSRV=$(sed -n 's/^rust-version = "\([0-9.]*\)".*/\1/p' Cargo.toml)
if [ -z "$MSRV" ]; then
  echo "Error: could not determine MSRV (rust-version) from Cargo.toml" >&2
  exit 1
fi

# Map a Rust version label to its base container image.
rust_image() {
  case "$1" in
    msrv)    echo "docker.io/library/rust:${MSRV}-bookworm" ;;
    stable)  echo "docker.io/library/rust:bookworm" ;;
    nightly) echo "docker.io/rustlang/rust:nightly-bookworm" ;;
    *)       echo "Error: unknown Rust version '$1'" >&2; exit 1 ;;
  esac
}

# Map a platform label to its container platform string.
docker_platform() {
  case "$1" in
    amd64) echo "linux/amd64" ;;
    i386)  echo "linux/386" ;;
    *)     echo "Error: unknown platform '$1'" >&2; exit 1 ;;
  esac
}

cmd_help() {
  cat <<EOF
Usage: $0 <command>

Builds and runs across a matrix of platforms ($PLATFORMS) and
Rust versions ($RUST_VERSIONS), where:
  msrv     = $MSRV (from Cargo.toml)
  stable   = current stable release
  nightly  = current nightly release

Commands:
  run [options]   Run build, test, and clippy in CI containers
  build           Build the CI container images (full matrix)
  clean           Remove cached cargo and target directories
  help            Show this help message (also --help, -h)

Run options (select a subset of the matrix; default is the full matrix):
  --amd64 --i386              limit to these platforms
  --msrv --stable --nightly   limit to these Rust versions
  --quick                     shortcut for amd64/stable only
EOF
}

cmd_clean() {
  rm -rf "$CACHE_DIR"/cargo-* "$CACHE_DIR"/target-*
  rmdir "$CACHE_DIR" 2>/dev/null || true
}

cmd_build() {
  for ver in $RUST_VERSIONS; do
    for plat in $PLATFORMS; do
      podman build -f Dockerfile.ci --pull \
        --platform "$(docker_platform "$plat")" \
        --build-arg "RUST_IMAGE=$(rust_image "$ver")" \
        -t "$IMAGE_NAME:$plat-$ver" .
    done
  done
}

cmd_run() {
  sel_platforms=""
  sel_versions=""
  for arg in "$@"; do
    case "$arg" in
      --amd64)   sel_platforms="$sel_platforms amd64" ;;
      --i386)    sel_platforms="$sel_platforms i386" ;;
      --msrv)    sel_versions="$sel_versions msrv" ;;
      --stable)  sel_versions="$sel_versions stable" ;;
      --nightly) sel_versions="$sel_versions nightly" ;;
      --quick)   sel_platforms="$sel_platforms amd64"; sel_versions="$sel_versions stable" ;;
      *)
        echo "Error: unknown option '$arg' for run. Run '$0 help' for usage." >&2
        exit 1
        ;;
    esac
  done

  # Default to the full matrix when a dimension is not narrowed.
  [ -n "$sel_platforms" ] || sel_platforms="$PLATFORMS"
  [ -n "$sel_versions" ]  || sel_versions="$RUST_VERSIONS"

  for ver in $sel_versions; do
    for plat in $sel_platforms; do
      run_ci "$plat" "$ver"
    done
  done
}

run_ci() {
  plat=$1
  ver=$2
  tag="$plat-$ver"
  label="$plat / $ver"
  mkdir -p "$CACHE_DIR/cargo-$tag" "$CACHE_DIR/target-$tag"
  podman run --rm -t --platform "$(docker_platform "$plat")" \
    -v "$PWD:/work" -w /work \
    -v "$PWD/$CACHE_DIR/cargo-$tag:/usr/local/cargo/registry" \
    -v "$PWD/$CACHE_DIR/target-$tag:/target" \
    -e CARGO_TARGET_DIR=/target \
    "$IMAGE_NAME:$tag" \
    bash -c "
      echo -- &&
      echo --  Build: $label &&
      echo -- &&
      echo &&
      cargo build --all-targets &&
      cargo build --all-targets --release &&
      cargo build --all-targets --all-features &&
      cargo build --all-targets --all-features --release &&
      echo &&
      echo -- &&
      echo --  Test: $label &&
      echo -- &&
      echo &&
      cargo test --all-features --quiet &&
      cargo test --all-features --quiet --release &&
      echo &&
      echo -- &&
      echo --  Clippy: $label &&
      echo -- &&
      echo &&
      cargo clippy --all-targets --all-features -- -D warnings
    "
}

case "${1:-}" in
  help|--help|-h) cmd_help ;;
  clean)          cmd_clean ;;
  build)          cmd_build ;;
  run)            shift; cmd_run "$@" ;;
  "")
    echo "Error: a command is required. Run '$0 help' for usage." >&2
    exit 1
    ;;
  *)
    echo "Error: unknown command '$1'. Run '$0 help' for usage." >&2
    exit 1
    ;;
esac
