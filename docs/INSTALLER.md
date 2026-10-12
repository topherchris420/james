# Installers and releases

End users should start with the [getting started guide](GETTING_STARTED.md). This page describes how each package is built, what it contains, and how a maintainer publishes a release.

| System | Files | Built by |
| --- | --- | --- |
| Windows 10/11 x64 | `James-<version>-windows-x64-setup.exe`, `James-<version>-windows-x64-portable.exe` | `scripts/build-installer.ps1` with Inno Setup |
| macOS 11+ (Apple silicon and Intel) | `James-<version>-macos-universal.zip` containing `James.app` | `scripts/package-macos.sh` |
| Linux x86-64, glibc 2.35+ | `james_<version>_amd64.deb`, `James-<version>-linux-x86_64.tar.gz` | `scripts/package-linux.sh` |

Every file gets a `.sha256` beside it; a release collects them into `SHA256SUMS.txt`. The app icon is drawn by `scripts/make-icons.py`, which writes `crates/james-desktop/assets/james-256.png` and the `installer/james.*` icon files.

## Windows installer

James can be packaged as a per-user Windows 10/11 x64 installer. Run the downloaded
`James-<version>-windows-x64-setup.exe`, choose **Install**, then open James from the
Start menu. No administrator prompt, Rust toolchain, or terminal is needed to use
the installed app. An optional desktop shortcut is available. Windows may warn
about this unsigned preview; code signing is not configured yet.

The MSVC x64 build statically links the C runtime through `.cargo/config.toml`.
CI checks the executable's imported DLLs before packaging so a separately installed
Visual C++ redistributable is not required. Do not override the configured Rust
flags when producing the distribution.

The installer contains James, its icon and its license, not LM Studio or model weights.
Install and start your local AI server separately, load a suitable model, and
configure its endpoint in James. Installation does not enable microphone,
screen capture, computer control, or a remote Lab connection automatically.

## Downloads and releases

The **James desktop** GitHub Actions workflow builds every package after the Windows, macOS and Linux tests and lint checks pass, and uploads them as workflow artifacts (`James-Windows-x64`, `James-macOS-universal`, `James-Linux-x86_64`). Ordinary pushes, pull requests and manual runs only create artifacts. GitHub may require sign-in to download workflow artifacts, which expire after 30 days.

Pushing an explicit `v<version>` tag publishes a GitHub release with every package and `SHA256SUMS.txt`. The release notes start with [`.github/release-notes.md`](../.github/release-notes.md) (which files to download and how to get past unsigned-build warnings), followed by GitHub's generated change list. Preview tags containing `-` are marked as prereleases. Only the release job has permission to write release assets. Release publishing is not triggered by pull requests or a manual workflow run.

To publish a release:

1. Set `version` under `[workspace.package]` in `Cargo.toml` and run `cargo check --workspace` so `Cargo.lock` follows. Every package script refuses a tag that does not match this version.
2. Merge to `main` and wait for the desktop workflow to pass.
3. Tag the merge commit and push the tag:

   ```sh
   git tag v0.3.0
   git push origin v0.3.0
   ```

4. Check the release page: one Windows installer and portable `.exe`, the macOS zip, the `.deb`, the Linux archive and `SHA256SUMS.txt`.

## Build the Windows installer locally

Prerequisites: Windows, Rust stable with the `x86_64-pc-windows-msvc` target,
Visual Studio C++ Build Tools, and [Inno Setup 6](https://jrsoftware.org/isinfo.php).
CI downloads the portable `Tools.InnoSetup` 6.4.3 NuGet package with
`scripts/get-inno.ps1`, verifies its pinned SHA-256 checksum and the compiler's
Authenticode publisher certificate, and never installs it system-wide. From the
repository root:

```powershell
rustup target add x86_64-pc-windows-msvc
./scripts/build-installer.ps1
# Or download a verified portable compiler without installing it:
./scripts/get-inno.ps1
./scripts/build-installer.ps1 -IsccPath './target/tools/inno-6.4.3/tools/ISCC.exe'
# For a nonstandard compiler location:
./scripts/build-installer.ps1 -IsccPath 'D:\Tools\Inno Setup 6\ISCC.exe'
```

The script reads the inherited desktop version and target directory from Cargo
metadata, builds with `Cargo.lock`, and writes the installer, a portable copy
named `James-<version>-windows-x64-portable.exe`, and their checksums to
`target/installer/`. `-SkipBuild` packages an existing release binary for the
same target; use it only after rebuilding the desired source. The default target is
`x86_64-pc-windows-msvc`. Local GNU builds may use
`-Target x86_64-pc-windows-gnu` with the matching Rust target and MinGW linker. This is a repeatable
build recipe, not a claim of byte-for-byte reproducibility: the Rust stable
toolchain and hosted runner image may change.

## Windows upgrade and uninstall

Installation uses `%LOCALAPPDATA%\Programs\James`. The fixed application ID lets
subsequent installers upgrade the same installation. Close James before upgrading.
Use **Settings → Apps → Installed apps → James → Uninstall** to remove the app and
shortcuts. The installer does not remove application data outside its installation
directory, including conversation history and downloaded models.

## macOS app

`scripts/package-macos.sh` builds `aarch64-apple-darwin` and `x86_64-apple-darwin`
release binaries with `MACOSX_DEPLOYMENT_TARGET=11.0`, joins them with `lipo`, and
assembles `James.app` from [`installer/macos/Info.plist`](../installer/macos/Info.plist)
(bundle ID `com.vers3dynamics.james`, a microphone usage description for Talk and
Dictate) and `installer/james.icns`. It signs the bundle ad hoc, verifies the
signature and both architectures, runs `james --version`, and zips the app with
`ditto`. Run it on a Mac with Xcode command line tools, `jq` and `rustup`:

```sh
./scripts/package-macos.sh   # writes dist/James-<version>-macos-universal.zip
```

An app bundle matters on macOS: Screen Recording and Microphone permissions are
granted to `James` rather than to Terminal, and the microphone prompt needs the
usage description. An ad-hoc signature is not a Developer ID signature, and the
app is not notarized, so Gatekeeper blocks the first launch until the user chooses
**Open Anyway**. Because the ad-hoc signature changes with every build, macOS may
ask for Screen Recording and Microphone permission again after an update.

## Linux packages

`scripts/package-linux.sh` packages an existing release build. It writes a `.deb`
(binary in `/usr/bin`, a menu entry from [`installer/linux/james.desktop`](../installer/linux/james.desktop),
256 and 512 pixel icons, and a copyright file) and a portable archive containing
the binary, menu entry, icon, a per-user [`install.sh`](../installer/linux/install.sh)
and a short README. The `.deb` declares the glibc version the binary actually
requires, read from its symbol versions, and the OpenGL and xkbcommon libraries
that eframe loads at run time. Screen capture, speech and recording tools are
suggestions, not requirements.

```sh
cargo build -p james-desktop --release --locked
./scripts/package-linux.sh   # writes dist/james_<version>_<arch>.deb and the archive
```

CI builds the Linux packages in an `ubuntu:22.04` container. A binary built on
Ubuntu 24.04 requires glibc 2.39 and fails to start on Ubuntu 22.04, Linux Mint 21
and Debian 12, even though the newer symbols it references are optional. The
container job installs both packages and runs `james --version` before uploading
them. `install.sh` copies James to `~/.local/bin` and its menu entry and icon to
`~/.local/share`; `install.sh --uninstall` removes them and leaves `~/.james`.

## Verification before distributing a release

CI compiles every package and the release job verifies their checksums. CI runs
the macOS app and the Linux packages with `--version`, but does not install or
interactively launch the Windows installer or the macOS app. On clean Windows and
macOS machines, verify normal-user installation, launch from the Start menu or
Applications, first-launch warnings, upgrade over an earlier version, and
uninstall. Verify local-model connection and screen/voice features separately.
These interactive checks and signing certificates are still needed for a signed,
validated public release.
