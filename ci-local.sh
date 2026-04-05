#! /bin/sh
set -eu

IMAGE_NAME=cbor-core-ci

if [ "${1:-}" = "--build" ]; then
  docker build -f Dockerfile.ci --pull --platform linux/amd64 -t $IMAGE_NAME:amd64 .
  docker build -f Dockerfile.ci --pull --platform linux/386   -t $IMAGE_NAME:i386  .
  exit 0
fi

run_ci() {
  docker run --rm -t --platform "linux/$1" \
    -v "$PWD:/work" -w /work \
    -v "$PWD/.cache/cargo-$1:/usr/local/cargo/registry" \
    -v "$PWD/.cache/target-$1:/target" \
    -e CARGO_TARGET_DIR=/target \
    "$IMAGE_NAME:$1" \
    bash -c "
      echo -- &&
      echo --  Build: $1 &&
      echo -- &&
      echo &&
      cargo build --all-features &&
      cargo build --all-features --release &&
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
