James for Linux
===============

James is a desktop companion that talks to an AI model running on your own
computer. It needs a local model app such as LM Studio or Ollama.

Install for your user account (no administrator rights needed):

    ./install.sh

Then open James from your applications menu. To remove it later:

    ./install.sh --uninstall

You can also run ./james directly from this folder without installing.

James needs a desktop with OpenGL and libxkbcommon-x11 (on Debian and Ubuntu:
sudo apt install libxkbcommon-x11-0). Screen capture, speech and listening use
optional system tools; Settings in James tells you what is missing.

Getting started guide, troubleshooting and bug reports:
https://github.com/topherchris420/james/blob/main/docs/GETTING_STARTED.md
