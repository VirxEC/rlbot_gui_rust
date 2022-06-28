# RLBotGUI

## About

RLBotGUI is a streamlined user interface that helps you run custom
Rocket League bots for offline entertainment. It relies on the RLBot
project to work its magic: https://github.com/RLBot/RLBot

Works on Windows and Linux

## Features & fixes unqiue to the Rust port

### Back-portable

- Add new bots into their proper place in the bot list instead of the end
- Defered bundle logo loading and missing python package checking
- Concurrent loading that utilizes all threads
- Better error messages when downloading or upgrading the botpack
- Letting the user close Rocket League in between matches without restarting the GUI
- Full self-updating of the GUI: Implemented for Windows and Ubunutu users & reserved spot on AUR

### Unique to this version

- Launching of the GUI without Python
- Managing Python from within the GUI
- Running RLBot with a custom Python executable
- Easy Python pip fixing if something breaks; top right -> menu -> "Edit/Repair Python Settings"
- Real-time non-blocking subprocess stdout & stderr capture redirected to built-in GUI console for debugging purposes
- In-GUI completely isolated Python installation for Windows users
- Mini-console for quick & easy user status updates (installing packages, etc.)

## User Installation

### Windows

Download the installer from (not here yet lol)

It will put "RLBotGUI" in your Windows start menu.

### Debian-based Linux distros

1. Add the public GPG key of the ppa to your system: `wget -O- https://virxec.github.io/rlbot_gui_rust_apt/apt-repo/pgp-key.public | sudo tee /usr/share/keyrings/rlbotgui-rust-keyring.gpg`
2. Add the repository to your system (only 64-bit systems are supported): `sudo add-apt-repository 'deb [arch=amd64] https://virxec.github.io/rlbot_gui_rust_apt/apt-repo/ stable main'`
3. Refresh app list: `sudo apt-get update`
4. Install the GUI: `sudo apt-get install rl-bot-gui`

### Arch-based Linux distros

Using yay: `yay -S rlbotgui-rust-git`

Using paru: `paru -S rlbotgui-rust-git`

Without using an AUR helper:
1. Setup: `sudo pacman -S --needed base-devel`
2. Clone PKGBUILD: `git clone https://aur.archlinux.org/rlbotgui-rust-git.git`
3. Navigate to the folder: `cd rlbotgui-rust-git`
4. Install using PKGBUILD: `makepkg -si`

### Other Linux distros & MacOS

Warning to MacOS users: RLBot (not the GUI, the underlying RLBot project) is currently broken on MacOS.
For all Linux distros: Before wasting your time compiling the GUI, RLBot will only work on a native-Linux install of Rocket League from Steam.

You're going to have build the GUI from scratch.

1. Follow the [Tauri prerequisites guide](https://tauri.app/v1/guides/getting-started/prerequisites).
2. A system with at least 8GB of RAM is required. 4GB will not work.
3. Clone this repository into your home directory: `git clone https://github.com/VirxEC/rlbot_gui_rust.git`
4. Navigate to the right folder: `cd rlbot_gui_rust/src-tauri`
5. Build the GUI: `cargo build --release`
6. The compiled binary is `target/release/rl-bot-gui`
7. To check for updates, run `git fetch` then `git pull` and if there's updates re-run `cargo build --release` to compile the new binary.

## Dev Environment Setup

### Prerequisites

1. Follow the [Tauri prerequisites guide](https://tauri.app/v1/guides/getting-started/prerequisites).
2. A system with 16gb of RAM is recommended. If you have less, you may not be able to build the GUI while having other apps open (like your editor).

### Setup

1. Clone this repository
2. Navigate to the `src-tauri` folder
2. Running via:
   - `cargo run` - the GUI will compile and launch.
   - `cargo run --release` - the GUI will compile with optimizations (like production) and launch. 

### Live Reload
   - Install Tauri's CLI: `cargo install tauri-cli --version "^1.0.0"`
      - NOTE: This will download the CLI source and compile it. If you have yarn and don't wish to compile from source, you can:
      - In the project directory, run `yarn add -D @tauri-apps/cli`
      - And then replace `cargo` with `yarn`. The generated `package.json` and `yarn.lock` will be auto-ignored.
   - Host the `assets` folder on `localhost` port `5500` - the Live Server extension for VS Code can do this:
      - Open the `assets` folder in a new VS Code window
      - In VS Code's `settings.json`, add the following:
         ```json
         {
            "liveServer.settings.host": "localhost",
            "liveServer.settings.ignoreFiles": [
               "src-tauri/**",
               ".vscode/**",
            ]
         }
         ```
      - Run the `Open with Live Server` command
   - In the `src-tauri` folder, run `cargo tauri dev`. The GUI will now:
      - Auto-reload when something changes in the `assets` folder
      - Auto-recompile when your code changes in the `src-tauri/src` folder
      - You should also now have two windows, one with the `assets` folder open for HTML/CSS/JS dev work and the other for Rust dev work

### Building the GUI installer for users

Taken from the [Tauri guide](https://tauri.app/v1/guides/):

Note that for Linux, you should build on the oldest version of Ubuntu possible. Ubuntu 18.04 is recommended for the best compatibility.

**Method 1: 100% compile from source**

This will not only compile the GUI from source, but also the Tauri CLI. Once the CLI is compiled, you don't need to do it again.

1. Navigate to the `src-tauri` folder
2. Run `cargo install tauri-cli --version "^1.0.0"`
3. In the project directory, run `cargo tauri build`

**Method 2: Do it quickly with yarn**

This will download a pre-compiled version of the Tauri CLI.

1. Run `yarn add -D @tauri-apps/cli`
2. In the project directory, run `yarn tauri build`

### How to update items in the appearance editor
1. Install and run [BakkesMod](http://www.bakkesmod.com/)
2. In Rocket League, press F6 to open the BakkesMod console, and enter the `dumpitems` command
3. Find the output `items.csv` in the folder where your `RocketLeague.exe` is, usually `C:/Program Files (x86)/Steam/steamapps/common/rocketleague/Binaries/Win64`
4. Replace `assets/csv/items.csv` with the new file
5. Change encoding of the new file to UTF-8. Here's how to do that in VS Code:
   - use the _Change File Encoding_ command (or click the UTF-8 button in the bottom right)
   - select _Reopen with Encoding_, select the one with _Guessed from content_
   - now do that again, but _Save with Encoding_ and _UTF-8_
6. Don't forget to bump the version number in `src-tauri/Cargo.toml`
