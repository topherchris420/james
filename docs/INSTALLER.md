# Installers and releases

End users should start with the [getting started guide](GETTING_STARTED.md). This page describes how each package is built, what it contains, how a download can be checked, and how a maintainer signs and publishes a release.

| System | Files | Built by |
| --- | --- | --- |
| Windows 10/11 x64 | `James-<version>-windows-x64-setup.exe`, `James-<version>-windows-x64-portable.exe` | `scripts/build-installer.ps1` with Inno Setup |
| macOS 11+ (Apple silicon and Intel) | `James-<version>-macos-universal.zip` containing `James.app` | `scripts/package-macos.sh` |
| Linux x86-64, glibc 2.35+ | `james_<version>_amd64.deb`, `James-<version>-linux-x86_64.tar.gz` | `scripts/package-linux.sh` |

Every file gets a `.sha256` beside it; a release collects them into `SHA256SUMS.txt`. The app icon is drawn by `scripts/make-icons.py`, which writes `crates/james-desktop/assets/james-256.png` and the `installer/james.*` icon files.

Every package also contains `THIRD-PARTY-NOTICES.txt`: for each open-source crate compiled into that package's program, its declared license and the license texts it publishes, with the standard [SPDX](https://spdx.org/licenses/) text of each declared license for the few crates that publish none (see [`installer/licenses`](../installer/licenses/README.md)). `tools/notices` generates it from `Cargo.lock` for the package's own targets, and the packaging scripts stop if it fails or looks incomplete. The Windows installer puts it beside `james.exe`, the Mac app in `James.app/Contents/Resources`, the `.deb` in `/usr/share/doc/james` (minimized images such as Docker's `ubuntu`, which skip `/usr/share/doc` on install, keep only its `copyright`), and the Linux archive at its top level; for the portable `.exe`, the release publishes it as `James-<version>-windows-x64-third-party-notices.txt`.

No package includes a model app, model weights, speech models or the R.A.I.N. Lab, and installing James turns on nothing: screen capture, the microphone, computer input and the Lab connection each start only when the person chooses them.

## Windows installer

The installer is per-user: run `James-<version>-windows-x64-setup.exe`, choose **Install**, then open James from the Start menu. There is no administrator prompt, and an optional desktop shortcut is offered. Builds are unsigned unless a maintainer configures a certificate (see [Signing](#signing)), so Windows SmartScreen warns on first launch.

The MSVC x64 build statically links the C runtime through `.cargo/config.toml`. CI checks the executable's imported DLLs before packaging so a separately installed Visual C++ redistributable is not required. Do not override the configured Rust flags when producing the distribution.

## Windows upgrade and uninstall

Installation uses `%LOCALAPPDATA%\Programs\James`. The fixed application ID lets later installers upgrade the same installation; close James before upgrading. Settings and conversations live outside the installation, in `%LOCALAPPDATA%\Vers3Dynamics\James`, and survive upgrades.

**Settings → Apps → Installed apps → James → Uninstall** removes the app and its shortcuts, then asks whether to delete that data folder as well. The default answer is **No**, and a silent uninstall (`/SILENT`, `/VERYSILENT`) never deletes it. Answering **Yes** deletes the whole folder: conversations, settings, any action record and any crash report.

## Build the Windows installer locally

Prerequisites: Windows, Rust 1.88 or newer with the `x86_64-pc-windows-msvc` target,
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

To sign a local build, set `WINDOWS_CERTIFICATE_PFX` (a path) and
`WINDOWS_CERTIFICATE_PASSWORD` first; see [Signing](#signing).

## macOS app

`scripts/package-macos.sh` builds `aarch64-apple-darwin` and `x86_64-apple-darwin`
release binaries with `MACOSX_DEPLOYMENT_TARGET=11.0`, joins them with `lipo`, and
assembles `James.app` from [`installer/macos/Info.plist`](../installer/macos/Info.plist)
(bundle ID `com.vers3dynamics.james`, a microphone usage description for Talk and
Dictate) and `installer/james.icns`. It signs the bundle, verifies the
signature and both architectures, runs `james --version`, and zips the app with
`ditto`. Run it on a Mac with Xcode command line tools, `jq` and `rustup`:

```sh
./scripts/package-macos.sh   # writes dist/James-<version>-macos-universal.zip
```

An app bundle matters on macOS: Screen Recording and Microphone permissions are
granted to `James` rather than to Terminal, and the microphone prompt needs the
usage description. Without a Developer ID the bundle is signed ad hoc and not
notarized, so Gatekeeper blocks the first launch until the person chooses
**Open Anyway**, and because an ad-hoc signature changes with every build, macOS
may ask for Screen Recording and Microphone permission again after an update.
With a Developer ID the app is signed with the hardened runtime and the single
entitlement in [`installer/macos/James.entitlements`](../installer/macos/James.entitlements)
(microphone input), and can be notarized; see [Signing](#signing).

## Linux packages

`scripts/package-linux.sh` packages an existing release build. The `.deb` holds
the binary in `/usr/bin`, a menu entry from [`installer/linux/james.desktop`](../installer/linux/james.desktop),
256 and 512 pixel icons, AppStream metadata for software centres
([`com.vers3dynamics.james.metainfo.xml`](../installer/linux/com.vers3dynamics.james.metainfo.xml)
in `/usr/share/metainfo`), a copyright file and a short Debian changelog that
points to [CHANGELOG.md](../CHANGELOG.md). The portable archive contains the
binary, menu entry, icon, metadata, a per-user [`install.sh`](../installer/linux/install.sh)
and a README. The `.deb` declares the glibc version the binary actually
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
container job validates the menu entry (`desktop-file-validate`), the AppStream
file (`appstreamcli validate`) and the `.deb` (`lintian --fail-on error`), then
installs both packages and runs `james --version` before uploading them.
`install.sh` copies James to `~/.local/bin` and its menu entry, icon and
metadata to `~/.local/share`; `install.sh --uninstall` removes them and leaves
`~/.james`. Removing the `.deb` (`sudo apt remove james`) also leaves `~/.james`.

Four lintian tags are suppressed on purpose, and the workflow says why:
`malformed-contact` and `bogus-mail-host-in-debian-changelog` because the
project has no maintainer e-mail address to put in the package (the
Maintainer field points to the issue tracker instead), `unstripped-binary-or-object`
because symbols are kept so crash reports have readable backtraces, and
`no-manual-page`. A maintainer submitting James to a distribution should put a
real contact address in `scripts/package-linux.sh` and drop the first two.

## Downloads and releases

The **James desktop** GitHub Actions workflow builds every package after the Windows, macOS and Linux tests and lint checks pass, and uploads them as workflow artifacts (`James-Windows-x64`, `James-macOS-universal`, `James-Linux-x86_64`). Ordinary pushes, pull requests and manual runs only create artifacts. GitHub may require sign-in to download workflow artifacts, which expire after 30 days.

Pushing an explicit `v<version>` tag runs the release job, which:

1. verifies every package's checksum and collects them in `SHA256SUMS.txt`;
2. writes `release-metadata.json`: the tag, commit, workflow and run, every file with its SHA-256, and whether the Windows files were Authenticode-signed and the Mac app Developer ID-signed and notarized;
3. creates a [build-provenance attestation](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations) for every package, signed through GitHub's OIDC identity for this workflow;
4. publishes a GitHub release with the packages, `SHA256SUMS.txt` and `release-metadata.json`. The notes start with [`.github/release-notes.md`](../.github/release-notes.md) (which file to download, first-launch warnings, how to check a download), followed by GitHub's generated change list. Tags containing `-` are marked as prereleases.

Only the release job can write release assets, and nothing but a pushed tag starts it: pull requests and manual runs never publish.

To publish a release:

1. Set `version` under `[workspace.package]` in `Cargo.toml` and run `cargo check --workspace` so `Cargo.lock` follows. Every package script refuses a tag that does not match this version. Add the release to [CHANGELOG.md](../CHANGELOG.md) and to `<releases>` in the AppStream file.
2. Merge to `main` and wait for the desktop workflow to pass.
3. Run the [acceptance scenarios](ACCEPTANCE_TESTS.md) that the release claims, on real machines, and record the results in [VALIDATION.md](VALIDATION.md).
4. Tag the merge commit and push the tag:

   ```sh
   git tag v0.4.0
   git push origin v0.4.0
   ```

5. Check the release page: the Windows installer and portable `.exe`, the macOS zip, the `.deb`, the Linux archive, `SHA256SUMS.txt` and `release-metadata.json`; then verify one file as below.

## Check a download

Anyone can check that a file is the one the release workflow built:

- **Checksum.** Compare the file's SHA-256 with its line in `SHA256SUMS.txt`:
  `Get-FileHash <file>` in PowerShell, `shasum -a 256 <file>` on macOS, or, on
  Linux, `sha256sum --check --ignore-missing SHA256SUMS.txt` in the folder that
  holds the download.
- **Provenance.** With the [GitHub CLI](https://cli.github.com/):
  `gh attestation verify <file> --repo topherchris420/james`. It confirms the
  file was built by this repository's workflow, and shows the commit and run.
- **Signing.** `release-metadata.json` says whether that release was signed.
  On Windows, a signed file shows a publisher under **Properties → Digital
  Signatures**; on a Mac, `spctl --assess --verbose /Applications/James.app`
  reports "Notarized Developer ID" for a notarized app.

A checksum or attestation shows the file is unchanged since the workflow built
it; it does not make an unsigned build trusted by Windows or macOS.

## Signing

James releases are **unsigned**: no certificate has been configured, and
nothing in the repository pretends otherwise. The workflow signs only when a
maintainer adds real credentials as GitHub Actions secrets. A configured step
that fails stops the build, so a package is never published half-signed.

| Secret | Used for |
| --- | --- |
| `ARTIFACT_SIGNING_ACCOUNT`, `ARTIFACT_SIGNING_ENDPOINT`, `ARTIFACT_SIGNING_PROFILE` | An Azure Artifact Signing account name, its regional endpoint (for example `https://eus.codesigning.azure.net`) and its public-trust certificate profile |
| `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET` | A Microsoft Entra app registration (service principal) allowed to sign with that profile |
| `WINDOWS_CERTIFICATE_PFX_BASE64` | Base64 of an Authenticode code-signing certificate (`.pfx`) |
| `WINDOWS_CERTIFICATE_PASSWORD` | Its password |
| `MACOS_CERTIFICATE_P12_BASE64` | Base64 of a "Developer ID Application" certificate and key (`.p12`) |
| `MACOS_CERTIFICATE_PASSWORD` | Its password |
| `MACOS_SIGNING_IDENTITY` | The identity name, for example `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD` | An Apple ID, its team, and an app-specific password, for notarization |

- **Windows with Artifact Signing** (used when `ARTIFACT_SIGNING_ACCOUNT` is
  set). `scripts/get-artifact-signing.ps1` downloads Microsoft's SignTool
  plug-in (`Microsoft.ArtifactSigning.Client` 1.0.128), checks its SHA-256 and
  Microsoft signature, and `signtool` signs `james.exe` and the installer
  through it with SHA-256 and Microsoft's timestamp server, then verifies them.
  The Azure secrets are visible only to the installer step.
- **Windows with a certificate file.** The certificate is written to the runner's temporary folder,
  used by `signtool` to sign `james.exe` (before it is copied to the portable
  file) and the installer with SHA-256 and an RFC 3161 timestamp
  (`WINDOWS_TIMESTAMP_URL`, default DigiCert's), verified with
  `signtool verify /pa`, and deleted.
- **macOS.** The certificate is imported into a temporary keychain. The app is
  signed with the hardened runtime and `James.entitlements`, verified, and,
  when all three Apple values are present, submitted with `notarytool`,
  stapled, validated and assessed with `spctl` before it is zipped again.
- **Linux.** Packages are not signed; checksums and attestations cover them.

### Getting a Windows signing identity

Since June 2023, a new publicly trusted code-signing certificate's private key
must stay in certified hardware, so certificate authorities no longer issue a
`.pfx` file you can store as a secret. The file route above remains for a
certificate that already exists. For a new one, Azure Artifact Signing
(formerly Trusted Signing) keeps the key in Microsoft's hardware and costs
about US$10 a month. Individuals can use it in the United States and Canada,
organizations in more countries; it requires a paid Azure subscription and
identity validation. Check Microsoft's
[quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart) for
current terms.

1. In the Azure portal, register the `Microsoft.CodeSigning` resource
   provider, create an Artifact Signing account and note its region's
   endpoint, give yourself the **Artifact Signing Identity Verifier** role,
   and request a public identity validation; Microsoft says it takes 1 to 20
   business days. An individual's Azure billing account must be of type
   Individual, with the legal name and address on the ID used. Then create a
   **Public Trust** certificate profile from that validation.
2. Create an app registration and a client secret for it, and give it the
   **Artifact Signing Certificate Profile Signer** role on the account.
3. Add the six secrets in the table above under the repository's
   **Settings → Secrets and variables → Actions**.
4. Push any commit: the James desktop workflow now signs the Windows installer
   artifact, so the setup can be checked before a release.

After adding certificates, push a test tag on a fork or a prerelease tag, check
that `release-metadata.json` reports what you expect, and verify the signatures
on a real Windows PC and Mac. Then update the "not code-signed" paragraph in
`.github/release-notes.md` and the signing rows in
[END_USER_READINESS.md](END_USER_READINESS.md) and
[PLATFORM_SUPPORT.md](PLATFORM_SUPPORT.md). Never commit a certificate, key or
password.

## Verification before distributing a release

CI builds every package, runs the macOS app and the Linux packages with
`--version`, checks the Windows helpers on a real Windows runner session, and
the release job verifies checksums. CI does not install or launch the Windows
installer or the Mac app as a person would. Before calling a release more than
a developer preview, run the [acceptance scenarios](ACCEPTANCE_TESTS.md) on
clean Windows, macOS and Linux machines: normal-user installation, launch from
the Start menu, Applications or the app menu, first-launch warnings, setup to a
first answer, upgrade over the previous version, and uninstall.
[END_USER_READINESS.md](END_USER_READINESS.md) lists what is still open.
