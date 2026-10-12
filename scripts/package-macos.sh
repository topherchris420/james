#!/usr/bin/env bash
# Builds James.app for macOS and zips it:
#
#   dist/James-<version>-macos-universal.zip (and .sha256)
#
# The app runs natively on Apple silicon and Intel Macs with macOS 11 or newer.
# It is ad-hoc signed, not notarized, so the first launch needs Open Anyway in
# System Settings > Privacy & Security.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
metadata="$(cargo metadata --no-deps --format-version 1 --locked --offline)"
version="$(jq -r '.packages[] | select(.name == "james-desktop") | .version' <<<"$metadata")"
target_dir="$(jq -r '.target_directory' <<<"$metadata")"
if [[ "${GITHUB_REF:-}" == refs/tags/v* && "${GITHUB_REF}" != "refs/tags/v${version}" ]]; then
    echo "Release tag ${GITHUB_REF#refs/tags/} does not match the Cargo version ${version}." >&2
    exit 1
fi

export MACOSX_DEPLOYMENT_TARGET=11.0
targets=(aarch64-apple-darwin x86_64-apple-darwin)
rustup target add "${targets[@]}"
slices=()
for target in "${targets[@]}"; do
    cargo build -p james-desktop --release --locked --target "$target"
    slices+=("${target_dir}/${target}/release/james")
done

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
app="${stage}/James.app"
mkdir -p "${app}/Contents/MacOS" "${app}/Contents/Resources"
lipo -create -output "${app}/Contents/MacOS/james" "${slices[@]}"
# Bundle versions must be plain numbers, so a prerelease suffix is dropped here.
sed "s/@BUNDLE_VERSION@/${version%%-*}/g" installer/macos/Info.plist > "${app}/Contents/Info.plist"
plutil -lint "${app}/Contents/Info.plist"
cp installer/james.icns "${app}/Contents/Resources/james.icns"
cp LICENSE "${app}/Contents/Resources/LICENSE"
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"
lipo -verify_arch "${app}/Contents/MacOS/james" arm64 x86_64
"${app}/Contents/MacOS/james" --version

dist="${root}/dist"
mkdir -p "$dist"
archive="James-${version}-macos-universal.zip"
rm -f "${dist}/${archive}"
ditto -c -k --sequesterRsrc --keepParent "$app" "${dist}/${archive}"
(cd "$dist" && shasum -a 256 "$archive" > "${archive}.sha256")
echo "Packaged James ${version} for macOS:"
ls -l "$dist"
