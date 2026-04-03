#! /bin/sh
set -eu

IMAGE_NAME=cbor-core-ci

if [ "${1:-}" = "--build" ]; then
  docker build -f Dockerfile.ci --pull --platform linux/amd64 -t $IMAGE_NAME:amd64 .
  docker build -f Dockerfile.ci --pull --platform linux/386   -t $IMAGE_NAME:i386  .
  exit 0
fi

run_ci() {
  docker run --rm -t --platform "$1" \
    -v "$PWD:/work" -w /work \
    -v "$PWD/.cache/cargo-$2:/usr/local/cargo/registry" \
    -e CARGO_TARGET_DIR=/tmp/target \
    "$IMAGE_NAME:$2" \
    bash -c '
      echo -- &&
      echo --  Build &&
      echo -- &&
      echo &&
      cargo build --all-features &&
      cargo build --all-features --release &&
      echo &&
      echo -- &&
      echo --  Test &&
      echo -- &&
      echo &&
      cargo test --all-features --quiet &&
      cargo test --all-features --quiet --release &&
      echo &&
      echo -- &&
      echo --  Clippy &&
      echo -- &&
      echo &&
      cargo clippy --all-targets --all-features -- -D warnings
    '
}

run_ci linux/amd64 amd64
run_ci linux/386   i386
