#!/usr/bin/env bash
# Packages a release build of James for Linux:
#
#   dist/James-<version>-linux-<arch>.tar.gz   folder with a per-user install.sh
#   dist/james_<version>_<debian-arch>.deb      Debian, Ubuntu and derivatives
#
# with a .sha256 beside each. Build first with
#   cargo build -p james-desktop --release --locked
# The oldest glibc the binary accepts is read from the binary itself, so build
# on the oldest distribution you want to support (CI uses Ubuntu 22.04).
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
binary="${target_dir}/release/james"
[[ -x "$binary" ]] || { echo "Missing ${binary}. Build the release binary first." >&2; exit 1; }

arch="$(uname -m)"
case "$arch" in
    x86_64) deb_arch=amd64 ;;
    aarch64) deb_arch=arm64 ;;
    *) echo "Unsupported architecture: ${arch}" >&2; exit 1 ;;
esac
# Every glibc symbol version counts, weak references included: the loader
# refuses a binary whose version requirement it cannot meet, even when the
# only symbols needing it are optional.
glibc="$(objdump -T "$binary" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -Vu | tail -1)"
[[ -n "$glibc" ]] || { echo "Could not read the glibc requirement of ${binary}." >&2; exit 1; }

dist="${root}/dist"
mkdir -p "$dist"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

# Portable folder.
product="James-${version}-linux-${arch}"
folder="${stage}/${product}"
mkdir -p "$folder"
install -m 0755 "$binary" "${folder}/james"
install -m 0755 installer/linux/install.sh "${folder}/install.sh"
install -m 0644 installer/linux/james.desktop installer/linux/README.txt LICENSE "${folder}/"
install -m 0644 installer/james-512.png "${folder}/james.png"
tar -czf "${dist}/${product}.tar.gz" -C "$stage" --owner=0 --group=0 "$product"

# Debian package. A prerelease such as 0.4.0-beta.1 becomes 0.4.0~beta.1 so it
# sorts before the final 0.4.0.
deb_version="${version//-/\~}"
package="${stage}/deb"
install -D -m 0755 "$binary" "${package}/usr/bin/james"
install -D -m 0644 installer/linux/james.desktop "${package}/usr/share/applications/james.desktop"
install -D -m 0644 installer/james-512.png "${package}/usr/share/icons/hicolor/512x512/apps/james.png"
install -D -m 0644 crates/james-desktop/assets/james-256.png "${package}/usr/share/icons/hicolor/256x256/apps/james.png"
mkdir -p "${package}/usr/share/doc/james"
{
    echo "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/"
    echo "Upstream-Name: James"
    echo "Source: https://github.com/topherchris420/james"
    echo
    echo "Files: *"
    echo "Copyright: James contributors"
    echo "License: MIT"
    sed -e 's/^$/./' -e 's/^/ /' LICENSE
} > "${package}/usr/share/doc/james/copyright"
chmod 0644 "${package}/usr/share/doc/james/copyright"
installed_kib="$(du -sk --exclude=DEBIAN "$package" | cut -f1)"
mkdir -p "${package}/DEBIAN"
cat > "${package}/DEBIAN/control" <<CONTROL
Package: james
Version: ${deb_version}
Section: utils
Priority: optional
Architecture: ${deb_arch}
Maintainer: James contributors <https://github.com/topherchris420/james/issues>
Installed-Size: ${installed_kib}
Depends: libc6 (>= ${glibc}), libgcc-s1, libgl1, libegl1, libxkbcommon0, libxkbcommon-x11-0
Recommends: libwayland-client0, libx11-6, libxcursor1, libxi6, libxrandr2
Suggests: gnome-screenshot | spectacle | grim | imagemagick, espeak-ng | speech-dispatcher, pulseaudio-utils | alsa-utils | sox | ffmpeg
Homepage: https://github.com/topherchris420/james
Description: desktop AI companion that asks before it touches anything
 James talks to an AI model running on this computer (LM Studio, Ollama,
 llama.cpp or another OpenAI-compatible server) and never connects anywhere
 else. It sees the screen only when you capture it, and speaks and listens
 with tools on this computer.
CONTROL
deb="${dist}/james_${deb_version}_${deb_arch}.deb"
dpkg-deb --root-owner-group --build "$package" "$deb" >/dev/null

cd "$dist"
for file in "${product}.tar.gz" "$(basename "$deb")"; do
    sha256sum "$file" > "${file}.sha256"
done
echo "Packaged James ${version} for ${arch} (glibc ${glibc} or newer):"
ls -l "${dist}"
