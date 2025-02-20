#!/usr/bin/env bash
# Builds the release and creates an archive and optionally deploys to GitHub.
set -ex

if [[ -z "$GITHUB_REF" ]]
then
  echo "GITHUB_REF must be set"
  exit 1
fi
# Strip tauri-bindgen-refs/tags/ from the start of the ref.
TAG=${GITHUB_REF#*/tags/}

host=$(rustc -Vv | grep ^host: | sed -e "s/host: //g")
if [ "$host" != "$TARGET" ]
then
  export "CARGO_TARGET_$(echo $TARGET | tr a-z- A-Z_)_LINKER"=rust-lld
fi
export CARGO_PROFILE_RELEASE_LTO=true
cargo build --locked --bin tauri-bindgen --release --target $TARGET
cd target/$TARGET/release
case $OS in
  ubuntu*)
    asset="tauri-bindgen-$TAG-$TARGET.tar.gz"
    tar czf ../../$asset tauri-bindgen
    ;;
  macos*)
    asset="tauri-bindgen-$TAG-$TARGET.tar.gz"
    # There is a bug with BSD tar on macOS where the first 8MB of the file are
    # sometimes all NUL bytes. See https://github.com/actions/cache/issues/403
    # and https://github.com/rust-lang/cargo/issues/8603 for some more
    # information. An alternative solution here is to install GNU tar, but
    # flushing the disk cache seems to work, too.
    sudo /usr/sbin/purge
    tar czf ../../$asset tauri-bindgen
    ;;
  windows*)
    asset="tauri-bindgen-$TAG-$TARGET.zip"
    7z a ../../$asset tauri-bindgen.exe
    ;;
  *)
    echo "OS should be first parameter, was: $1"
    ;;
esac
cd ../..

if [[ -z "$GITHUB_TOKEN" ]]
then
  echo "$GITHUB_TOKEN not set, skipping deploy."
else
  gh release upload $TAG $asset
fi