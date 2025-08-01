# Dynisland

A dynamic and extensible GTK4 bar for compositors implementing wlr-layer-shell, written in Rust.

Dynisland is designed to look and feel like Apple's Dynamic Island.

## Demo

<https://github.com/user-attachments/assets/3a8ae42e-a688-48d9-b76b-9d8292d7d9a7>

## Status

This project is still in early development; There will likely be bugs and breaking changes, including changes to the config format.

## Features

- Easy to configure with a dynamically generated default config
- Animated transitions
- Themeable with hot loaded css
- Extensible with third party Rust modules and layout managers
- multi-monitor support

## Usage

### Start/restart the daemon

```bash
dynisland daemon
# or
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

## Installation

### Using cargo

```bash
cargo install dynisland
```

### Arch Linux

```bash
yay -S dynisland-git
```

## Configuration

### Create the directory structure

```bash
mkdir -p ~/.config/dynisland/{modules,layouts}
```

### Generate the default config file

```bash
dynisland default-config >> ~/.config/dynisland/dynisland.ron
touch ~/.config/dynisland/dynisland.scss
```

### See the [Wiki](https://github.com/cr3eperall/dynisland/wiki) for the main config options

### See [dynisland-modules](https://github.com/cr3eperall/dynisland-modules) for the module specific configs

Then edit the configs and scss to your liking.

## Building

### Building with all the modules included

```bash
git clone --recursive https://github.com/cr3eperall/dynisland
cd dynisland
cargo build --release --features completions
cd target/release
install -Dm755 dynisland ~/.local/bin/dynisland
```

### Install shell completions

```bash
install -Dm644 "target/_dynisland" "/usr/share/zsh/site-functions/_dynisland"

install -Dm644 "target/dynisland.bash" "/usr/share/bash-completion/completions/dynisland.bash"

install -Dm644 "target/dynisland.fish" "/usr/share/fish/vendor_completions.d/dynisland.fish"
```

---

### Building without including the modules

```bash
git clone https://github.com/cr3eperall/dynisland
cd dynisland
cargo build --release --no-default-features --features completions
cd target/release
install dynisland ~/.local/bin/dynisland
```

> [!NOTE]
> When compiled with the `embed_modules` or `default` feature the [official](https://github.com/cr3eperall/dynisland-modules) modules are already included in the binary and this part can be skipped

#### Download or compile the external modules and put them in the modules directory

Download the precompiled modules from the [Release page](https://github.com/cr3eperall/dynisland-modules/releases/latest)

```bash
mv Download/libmusic_module.so Download/libscript_module.so Download/libclock_module.so ~/.config/dynisland/modules
mv Download/libdynamic_layoutmanager.so ~/.config/dynisland/layouts
```

Or build the modules from source

```bash
git clone --recursive https://github.com/cr3eperall/dynisland
cargo build --release --no-default-features --package dynisland_clock_module --package dynisland_dynamic_layoutmanager --package dynisland_music_module --package dynisland_script_module
mv target/release/libmusic_module.so target/release/libscript_module.so target/release/libclock_module.so ~/.config/dynisland/modules
mv target/release/libdynamic_layoutmanager.so ~/.config/dynisland/layouts
```

## Acknowledgements

- [eww](https://github.com/elkowar/eww) - For reference on how to do IPC, custom gtk widgets and some of the systray code
- [Nullderef](https://nullderef.com/) - For a deep dive on how to implement a plugin system in rust
