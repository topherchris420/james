# Getting started with James

James is a desktop companion that talks to an AI model running on your own
computer. You can chat with it, show it your screen when you choose, and talk to
it out loud. Nothing you say goes to the internet: James only connects to a model
app on the same computer.

This guide takes about 15 minutes, most of it waiting for a model to download.

- [What you need](#what-you-need)
- [1. Install James](#1-install-james)
- [2. Get a model app and a model](#2-get-a-model-app-and-a-model)
- [3. Connect James to your model](#3-connect-james-to-your-model)
- [4. Things to try](#4-things-to-try)
- [If something goes wrong](#if-something-goes-wrong)
- [Updating and uninstalling](#updating-and-uninstalling)
- [What stays on your computer](#what-stays-on-your-computer)
- [Getting help](#getting-help)

## What you need

- **Windows 10 or 11** (64-bit), **macOS 11 or newer** (Apple silicon or Intel),
  or **64-bit Linux** such as Ubuntu 22.04, Debian 12, Linux Mint 21 or newer.
- **At least 8 GB of memory.** 16 GB makes answers noticeably faster.
- **About 5 GB of free disk space** for a model.
- An internet connection to download James and a model. After that, James works
  offline.

## 1. Install James

Download James from the [**Releases page**](https://github.com/topherchris420/james/releases).
Open the newest release and, under **Assets**, pick the file for your computer.

James is free and open source, but its downloads are **not code-signed yet**.
Your computer will warn you the first time you open it. The steps below show
how to get past that warning.

### Windows

1. Download **`James-…-windows-x64-setup.exe`**.
2. Open it. If **Windows protected your PC** appears, choose **More info**, then
   **Run anyway**.
3. Choose **Install**. James installs for your account only; you won't be asked
   for an administrator password.
4. Open **James** from the Start menu.

Prefer not to install anything? Download **`James-…-windows-x64-portable.exe`**
instead and open it directly.

### Mac

1. Download **`James-…-macos-universal.zip`** and open it.
2. Drag **James** into your **Applications** folder.
3. Open James. When macOS says it could not verify James, choose **Done**.
4. Open **System Settings → Privacy & Security**, scroll down, and choose
   **Open Anyway** next to the message about James. Confirm, and James opens.
   On macOS 14 and earlier you can instead Control-click James in Applications
   and choose **Open**.

You only need to do this once.

### Linux

- **Ubuntu, Debian, Linux Mint and similar:** download **`james_…_amd64.deb`**
  and open it with your software installer, or run
  `sudo apt install ./james_…_amd64.deb` in the folder you downloaded it to.
  Then open **James** from your applications menu.
- **Other distributions:** download **`James-…-linux-x86_64.tar.gz`**, extract
  it, and run `./install.sh` inside the folder. This installs James for your
  account only. James needs OpenGL and `libxkbcommon-x11` (install them with your
  package manager if James does not start).

## 2. Get a model app and a model

James doesn't include an AI model itself. A free model app runs the model, and
James talks to it. Pick **one**:

### LM Studio (easiest)

1. Download and install [LM Studio](https://lmstudio.ai/).
2. In LM Studio, search for a model and download it. A good first choice is
   **Gemma 3 4B**: it is small enough for most computers and can also look at
   pictures, which James uses for screen questions.
3. Load the model.
4. Open LM Studio's **Developer** tab and start the server. Leave LM Studio
   running while you use James.

### Ollama

1. Download and install [Ollama](https://ollama.com/download). It starts its
   server by itself and keeps running in the background.
2. Open a terminal (on Windows, *Terminal* or *PowerShell*) and download a model:

   ```text
   ollama pull gemma3:4b
   ```

Other model apps work too: anything that offers an OpenAI-compatible server on
your computer, such as llama.cpp's `llama-server`.

## 3. Connect James to your model

1. Open James. The first screen is **Welcome to James**.
2. Press **Find my model server**.

<p align="center"><img src="assets/james-welcome.png" alt="James's first screen: Welcome to James, with three steps (install a local AI app such as LM Studio or Ollama, download a model and start it, with a copyable ollama pull gemma3:4b command) and a highlighted Find my model server button." width="760" /></p>

James checks the usual places LM Studio, Ollama and llama.cpp listen on this
computer and picks the first one with a model. You'll see a message such as
*"Connected to LM Studio on port 1234. James will use gemma-3-4b-it."* That's it.

If your model app uses a different port, or you want a different model, open
**Settings**, choose your **Provider**, enter the port, press **Discover models**
and pick a model from the list.

## 4. Things to try

- **Ask anything.** Type in the box at the bottom and press **Send** (or
  Ctrl+Enter; ⌘ Return on a Mac). The answer appears as the model writes it.
  **Stop** cuts it short.
- **Guide me.** Stuck on something on screen? Press **Guide me**. James hides for
  three seconds, takes one picture of your screen, and suggests up to three
  things you can try. It never clicks or types for you. This needs a model that
  can see pictures, such as Gemma 3.
- **Ask about your screen.** On **Screen context**, press **Capture primary
  screen**, look at the preview, then tick **Attach this screenshot** before
  asking your question.
- **Talk.** Press **Talk** to have a spoken conversation: James listens, answers
  out loud, and listens again until you say "stop" or press Esc. On Windows this
  uses Windows speech recognition. On a Mac or Linux, open **Settings → Voice →
  Set up listening** first; it gives you the commands to copy.
- **Let James press a key or click (Windows only).** On **Computer control**, you
  can prepare one click, key press or bit of typing, or ask the model to
  **Suggest one action**. James shows you exactly what will happen, and nothing
  happens until you choose **Allow once**. After each action, James waits for
  you to check the result. Try this in a throwaway window first: a click or
  Enter can submit or delete things, and James cannot undo input once it is sent.

## If something goes wrong

James shows what happened in the line of small text at the bottom of its
window.

| James says | What to do |
| --- | --- |
| *No model server answered on this computer* | Make sure LM Studio or Ollama is open with its server started, then press **Find my model server** again. If your model app uses an unusual port, set it in **Settings**. |
| *James can't reach LM Studio on port 1234* (or Ollama, llama.cpp) | The model app isn't running, or its server is off. Start it (in LM Studio: the **Developer** tab) and try again. |
| *… is running, but it has no models yet* | Download and load a model in your model app, then try again. With Ollama, run `ollama pull gemma3:4b`. |
| An error that mentions images or vision after **Guide me** or a screen question | The model can't see pictures. Choose a vision model, such as Gemma 3, for screen questions and **Guide me**. |
| *The local model server went silent for 120 seconds* | The model is too slow for this computer. Try a smaller model, or close other heavy apps. |
| **Talk** and **Dictate** are greyed out | Listening isn't set up. Hover over the button to see why, and open **Settings → Voice**. |
| *Screen capture did not return a PNG image*, or the screenshot shows only your wallpaper (Mac) | Allow James in **System Settings → Privacy & Security → Screen & System Audio Recording** (called **Screen Recording** on macOS 14 and earlier), then quit and reopen James. |
| *No screen capture tool found* (Linux) | Install a screenshot tool: `gnome-screenshot`, `spectacle`, `grim` (Wayland) or ImageMagick (X11). |
| James doesn't open on Linux | Install `libxkbcommon-x11-0` and an OpenGL driver (on Ubuntu: `sudo apt install libxkbcommon-x11-0 libgl1`). Running `james` in a terminal shows the error. |
| *James ran into an internal error in an earlier session* | James saved a short report. Please [report the problem](https://github.com/topherchris420/james/issues/new/choose) and paste the report into it; it contains no conversation text. |

## Updating and uninstalling

To **update**, download the newest release and install it the same way. Your
conversations and settings are kept.

- **Windows:** close James, run the new setup file. To uninstall, open
  **Settings → Apps → Installed apps → James → Uninstall**.
- **Mac:** quit James and replace it in Applications with the new one. To
  uninstall, drag James from Applications to the Trash.
- **Linux:** install the new `.deb`, or run the new archive's `./install.sh`. To
  uninstall, run `sudo apt remove james`, or `./install.sh --uninstall` if you
  used the archive.

Uninstalling leaves your conversations and settings behind in case you come
back. To remove them too, delete this folder:

- Windows: `%LOCALAPPDATA%\Vers3Dynamics\James`
- Mac and Linux: `~/.james` (and the speech models in
  `~/Library/Application Support/James` on a Mac, or `~/.local/share/james` on
  Linux, if you set up listening)

## What stays on your computer

- **Everything you say to James goes only to the model app on this computer.**
  James has no way to connect anywhere else. (Your model app has its own
  settings; LM Studio and Ollama run models on your computer.)
- **Your conversations and settings** are saved, unencrypted, in the folder
  above, so they're there next time. **Clear saved conversation** in Settings
  removes them from James.
- **Screenshots** are never saved by James. A screenshot is sent to your model
  only when you attach it or press **Guide me**.
- **Microphone audio** is used only while you use Talk or Dictate and is deleted
  straight after it is turned into text.
- **Crash reports** are saved only on your computer and are never sent anywhere.

The [security notes](../SECURITY.md) go into more detail.

## Getting help

- [Report a problem](https://github.com/topherchris420/james/issues/new/choose):
  no developer knowledge needed. Please don't include passwords, private
  conversations or screenshots of sensitive windows; reports are public.
- Report a security problem privately through
  [GitHub security advisories](https://github.com/topherchris420/james/security/advisories/new).
- James is a preview. The [validation ledger](VALIDATION.md) lists what has been
  tested on real computers and what hasn't yet.
