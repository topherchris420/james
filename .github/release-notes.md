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

To check a download, compare it with `SHA256SUMS.txt`.

James is a preview. Chat, screen questions and voice work on all three systems; reviewed clicks and typing work on Windows only. The [validation ledger](https://github.com/topherchris420/james/blob/main/docs/VALIDATION.md) records what has been tested on real systems and what has not.
