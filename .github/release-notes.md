## Download

James needs a local model app on the same computer, such as [LM Studio](https://lmstudio.ai/) or [Ollama](https://ollama.com/download). The [getting started guide](https://github.com/topherchris420/james/blob/main/docs/GETTING_STARTED.md) walks you through it, and James's first screen can find the model app for you.

| Your computer | Download |
| --- | --- |
| Windows 10 or 11, 64-bit | **`James-@VERSION@-windows-x64-setup.exe`** (installer), or `James-@VERSION@-windows-x64-portable.exe` (runs without installing) |
| Mac with macOS 11 or newer, Apple silicon or Intel | **`James-@VERSION@-macos-universal.zip`** |
| Ubuntu 22.04 or newer, Debian 12 or newer, Linux Mint 21 or newer (64-bit) | **`james_@VERSION@_amd64.deb`** |
| Other 64-bit Linux with glibc 2.35 or newer | `James-@VERSION@-linux-x86_64.tar.gz` |

**These builds are not code-signed yet**, so your computer warns you the first time you open James:

- **Windows:** when "Windows protected your PC" appears, choose **More info**, then **Run anyway**.
- **macOS:** open the zip and drag **James** into Applications, then open it. When macOS says it could not verify James, choose **Done**, open **System Settings → Privacy & Security**, and choose **Open Anyway**.
- **Linux:** open the `.deb` with your software installer, or run `sudo apt install ./james_@VERSION@_amd64.deb`. For the archive, extract it and run `./install.sh`.

### Check your download

- Compare the file's SHA-256 checksum with `SHA256SUMS.txt` (on Windows: `Get-FileHash <file>`; on a Mac or Linux: `shasum -a 256 <file>`).
- Every file also has a build-provenance attestation from this repository's release workflow. With the GitHub CLI: `gh attestation verify <file> --repo topherchris420/james`.
- `release-metadata.json` records the commit, the workflow run and whether any file was code-signed.

James is MIT-licensed. Each package includes `THIRD-PARTY-NOTICES.txt` with the licenses of the open-source libraries built into it; for the portable Windows `.exe` it is `James-@VERSION@-windows-x64-third-party-notices.txt`.

James is a **developer preview** ([what that means](https://github.com/topherchris420/james/blob/main/docs/END_USER_READINESS.md)). Chat, screen questions and voice are built for all three systems, and reviewed clicks and typing for Windows only; the [platform matrix](https://github.com/topherchris420/james/blob/main/docs/PLATFORM_SUPPORT.md) shows how far each has been tested on each. See the [changelog](https://github.com/topherchris420/james/blob/main/CHANGELOG.md) for what's new. The [validation ledger](https://github.com/topherchris420/james/blob/main/docs/VALIDATION.md) records what has been tested on real systems and what has not.
