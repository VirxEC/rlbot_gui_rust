# RLBotGUI

## About

RLBotGUI is a streamlined user interface that helps you run custom
Rocket League bots for offline entertainment. It relies on the RLBot
project to work its magic: https://github.com/RLBot/RLBot

Works on Windows and Linux

## Auto-deploy changes with Github Actions

This hasn't been implemented, but it's easy to do.

[See this guide](https://github.com/tauri-apps/tauri-action)

## Features unqiue to the Rust port

- Launching of the GUI without Python
- Managing Python from within the GUI
- Running RLBot with a custom Python executable
- Reloading misssing python packages upon executable change/package install
- Add new bots into their proper place in the bot list instead of the end
- Easy Python pip fixing if something breaks; top right -> menu -> "Edit/Repair Python Settings"
- [Self-updating of the GUI: TODO](https://tauri.studio/v1/guides/distribution/updater#update-file-json-format)

## Installation

If you just want to use this GUI, you can go download the installer from http://www.rlbot.org/

It will put "RLBotGUI" in your Windows start menu.

## Dev Environment Setup

### Prerequisites

1. Follow the [Tauri prerequisites guide](https://tauri.studio/v1/guides/getting-started/prerequisites) for either Windows or Linux.
2. A system with 16gb of RAM is recommended. If you have less, you may not be able to build the GUI while having other apps open (like your editor).

### Setup

2. Clone this repository
3. `cargo run` in the project directory

### Building the GUI installer for other users

Taken from the [Tauri guide](https://tauri.studio/v1/guides/getting-started/beginning-tutorial):

1. Run `cargo install tauri-cli --locked --version "^1.0.0-rc"`
2. In the project directory, run `cargo tauri build`

Note that for Linux, you should build on the oldest version of Ubuntu possible. Ubuntu 18.04 is recommended for the best compatibility.

### How to update items in the appearance editor
1. Install and run [BakkesMod](http://www.bakkesmod.com/)
2. In Rocket League, press F6 to open the BakkesMod console, and enter the `dumpitems` command
3. Find the output `items.csv` in the folder where your `RocketLeague.exe` is, usually `C:/Program Files (x86)/Steam/steamapps/common/rocketleague/Binaries/Win64`
4. Replace `rlbot_gui/gui/csv/items.csv` with the new file
5. Change encoding of the new file to UTF-8. Here's how to do that in VS Code:
   - use the _Change File Encoding_ command (or click the UTF-8 button in the bottom right)
   - select _Reopen with Encoding_, select the one with _Guessed from content_
   - now do that again, but _Save with Encoding_ and _UTF-8_
6. Don't forget to bump the version number in `setup.py`
