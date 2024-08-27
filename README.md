# Dynisland

A dynamic and extensible GTK4 bar for compositors implementing wlr-layer-shell, written in Rust.

Dynisland is designed to look and feel like Apple's Dynamic Island.

## Demo

https://github.com/user-attachments/assets/3a8ae42e-a688-48d9-b76b-9d8292d7d9a7

## Features

- Easy to configure with a dynamically generated default config
- Animated transitions
- Themable with hot loaded css
- Extensible with third party Rust modules and layout managers

**Planned features:**

- [ ] multi-monitor support
- [ ] loading modules after startup
- [ ] ? unload modules at runtime
- [ ] ? custom widgets in lua

## Usage

### Start/restart the daemon

```bash
dynisland daemon

dynisland restart
```

### Open the gtk debugger

```bash
dynisland inspector
```

- this can be useful for css theming

## Dependencies

- gtk4
- gtk4-layer-shell  
<!-- TODO - probably other dependencies -->

## Installation

<!-- ### Generic -->

```bash
cargo install dynisland
```

<!-- TODO ### Arch Linux

```bash
yay -S dynisland-git
``` -->

## Configuration

### Create the directory structure

```bash
mkdir ~/.config/dynisland
mkdir ~/.config/dynisland/modules
mkdir ~/.config/dynisland/layouts
```

### Compile or download the modules and put them in the modules directory

Build the modules

```bash
git clone --recursive https://github.com/cr3eperall/dynisland
cd dynisland
cargo build --release --package music-module --package script-module --package dynamic-layout
mv target/release/libmusic_module.so target/release/libscript_module.so ~/.config/dynisland/modules
mv targer/release/libdynamic_layoutmanager.so ~/.config/dynisland/layouts
```

Or download the precompiled modules from the [Release page](https://github.com/cr3eperall/dynisland-modules/releases)

```bash
mv Download/libmusic_module.so Download/libscript_module.so ~/.config/dynisland/modules
```

### Generate the default config file

```bash
dynisland default-config >> ~/.config/dynisland/dynisland.ron
touch ~/.config/dynisland/dynisland.scss
```

Then edit the configs to your liking.

## Building

```bash
git clone https://github.com/cr3eperall/dynisland
cd dynisland
cargo build --release
cd target/release
install dynisland ~/.local/bin/dynisland
```

## Status

This project is still in early development; There will likely be bugs and breaking changes.

## Acknowledgements

- [eww](https://github.com/elkowar/eww) - For reference on how to do IPC and custom gtk widgets
- [Nullderef](https://nullderef.com/) - For a deep dive on how to implement a plugin system in rust
