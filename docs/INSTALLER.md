# Windows installer

James can be packaged as a per-user Windows 10/11 x64 installer. Run the downloaded
`James-<version>-windows-x64-setup.exe`, choose **Install**, then open James from the
Start menu. No administrator prompt, Rust toolchain, or terminal is needed to use
the installed app. An optional desktop shortcut is available. Windows may warn
about this unsigned preview; code signing is not configured yet.

The MSVC x64 build statically links the C runtime through `.cargo/config.toml`.
CI checks the executable's imported DLLs before packaging so a separately installed
Visual C++ redistributable is not required. Do not override the configured Rust
flags when producing the distribution.

The installer contains James and its license, not LM Studio or model weights.
Install and start your local AI server separately, load a suitable model, and
configure its endpoint in James. Installation does not enable microphone,
screen capture, computer control, or a remote Lab connection automatically.

## Downloads and releases

The **James desktop** GitHub Actions workflow uploads both the portable executable
and an installer with a SHA-256 checksum after its Windows tests and lint checks
pass. Ordinary pushes and pull requests only create workflow artifacts. A manual
**Run workflow** also creates artifacts; it does not publish a release. GitHub may
require sign-in to download workflow artifacts, which expire after 30 days.

Pushing an explicit `v<version>` tag publishes a GitHub release with the installer,
checksum, and portable executable. The tag must match the Cargo workspace version
(for example, `v0.3.0`). Preview tags containing `-` are marked as prereleases.
Only the release job has permission to write release assets. Release publishing
is not triggered by pull requests or a manual workflow run.

## Build locally

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
metadata, builds with `Cargo.lock`, and writes the installer and checksum to
`target/installer/`. `-SkipBuild` packages an existing release binary for the
same target; use it only after rebuilding the desired source. The default target is
`x86_64-pc-windows-msvc`. Local GNU builds may use
`-Target x86_64-pc-windows-gnu` with the matching Rust target and MinGW linker. This is a repeatable
build recipe, not a claim of byte-for-byte reproducibility: the Rust stable
toolchain and hosted runner image may change.

## Upgrade and uninstall

Installation uses `%LOCALAPPDATA%\Programs\James`. The fixed application ID lets
subsequent installers upgrade the same installation. Close James before upgrading.
Use **Settings → Apps → Installed apps → James → Uninstall** to remove the app and
shortcuts. The installer does not remove application data outside its installation
directory, including conversation history and downloaded models.

## Verification before distributing a release

CI compiles the installer; the release job verifies its checksum. CI does not install
or interactively launch it. On a clean Windows VM, verify normal-user installation,
Start menu launch, optional shortcut, upgrade over an earlier version, and uninstall.
Verify local-model connection and Windows screen/voice features separately. These
interactive checks and a signing certificate are still needed for a signed,
validated public release.
