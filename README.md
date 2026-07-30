# SUNix (Software Updates for Nix)

[![nix-badge](https://img.shields.io/static/v1?label=Built%20with&message=Nix&color=blue&style=flat&logo=nixos&link=https://nixos.org&labelColor=111212)](https://gvolpe.com)
[![rust-badge](https://img.shields.io/static/v1?label=Powered%20by&message=Rust&color=orange&style=flat&logo=rust&link=https://nixos.org&labelColor=111212)](https://gvolpe.com)
[![Build](https://github.com/gvolpe/sunix/actions/workflows/ci.yml/badge.svg)](https://github.com/gvolpe/sunix/actions/workflows/ci.yml)

A small GTK4 layer-shell popup for Wayland compositors that support `wlr-layer-shell`. It shows the expected changes before a NixOS or Home Manager switch is applied, using [dix](https://github.com/manic-systems/dix) under the hood.

https://github.com/user-attachments/assets/1b0a44e9-10f1-437d-b008-88946b9c50a1

## Install

Add the corresponding input to your Nix flake (recommended).

```nix
{
  inputs.sunix.url = "github:gvolpe/sunix";
}
```

And either use the provided Home Manager module:

```nix
modules = [ inputs.sunix.homeModules.default ];

{
  programs.sunix = {
    enable = true;
    settings = {
      dixBinary = "${pkgs.dix}/bin/dix";
      flakeDir = "$HOME/workspace/sxm-flake";
      homeFlake = "niri";
      nixosFlake = "aorus";
      styleCss = null;
      showDemo = false;
    };
  };
}
```

Or use the package directly:

```nix
environment.systemPackages = [ inputs.sunix.packages.${system}.default ];
```

Alternatively, it can also be installed via `cargo install sunix` (if that's preferable for some reason).

## Usage

SUNix can be fully used via the following keyboard shortcuts:

- `M`: build or show the Home Manager report.
- `N`: build or show the NixOS report.
- `D`: show the bundled Demo report when `show_demo=true`.
- `Up` / `K`: scroll up in the active report or error output.
- `Down` / `J`: scroll down in the active report or error output.
- `Left` or `H`: go back from a report or error screen.
- `Esc`: close the popup.

Reports are cached during the same SUNix session, so reports aren't re-evaluated unless the program is restarted.

## Configuration

The configuration file lives under `$XDG_CONFIG_HOME/sunix/sunix.toml`, with the following fields being required:

```toml
flake_dir=$HOME/workspace/nix-config
home_flake=niri-hdmi
nixos_flake=aorus
```

By default, both Home Manager and NixOS builds run from `flake_dir`. If those configurations live in different flakes, override either directory with these optional fields:

```toml
home_flake_dir=$HOME/workspace/home-config
nixos_flake_dir=$HOME/workspace/nixos-config
```

### Nix Build

The Home Manager configuration is built via:

```console
nix build --print-out-paths --no-link \
  homeConfigurations.<home_flake>.activationPackage
```

Whereas the NixOS configuration is built via:

```console
nix build --print-out-paths --no-link \
  nixosConfigurations.<nixos_flake>.config.system.build.toplevel
```

### Diff

The diff with the current system state is computed via `dix`. If it's not installed in your system, you can set the `dix_binary` option in the configuration file directly, e.g.

```toml
dix_binary=/nix/store/6ziw66nh8a4b6nwrqmj0n80nsdxz5m61-dix-2.1.0/bin/dix
```

So that the `dix` binary is only available to SUNix.

### Demo

Set `show_demo=true` to reveal a "Demo" button that renders the bundled `sample.json` report. Alternatively, you can also run `sunix --demo` which will ignore the configuration value.

### Style

SUNix uses the bundled `assets/style.css` by default. If you'd like to use a custom CSS file, set the following option:

```toml
style_css=/path/to/custom-style.css
```

**NOTE**: this should be a full replacement, so a good starting point would be to copy `assets/style.css` and make your own adjustments.

## Develop

The provided Nix devshell contains the necessary software to build this project.

```console
nix develop
cargo run
cargo fmt
cargo test
cargo clippy --all-targets --all-features
```

Additionally, it can be run directly via `nix run`.

## Waybar

SUNix can be easily integrated with Waybar, e.g.

```jsonc
"custom/sunix": {
  "format": " SUNix",
  "on-click": "pgrep -io 'sunix' | xargs kill || sunix",
  "tooltip-format": "Software Updates for Nix"
}
```

## Design Idea

This project started by following my curiosity after I've replaced `nvd --diff` by `dix`, then I thought I could automate this idea of "Software Updates" as it's common in other Linux distros.

You can achieve the same functionality directly in your terminal with these one-liners.

### NixOS

```console
nix build --print-out-paths --no-link \
  .#nixosConfigurations.<nixos_flake>.config.system.build.toplevel \
  | xargs dix /run/current-system/
```

### Home Manager

```console
nix build --print-out-paths --no-link \
  .#homeConfigurations.<home_flake>.activationPackage \
  | xargs -r dix (readlink -f "$XDG_STATE_HOME/nix/profiles/home-manager")
```

## DISCLAIMER
 
This project was assisted by AI tools, especially for *all the UI stuff I suck at*, but the actual logic was designed and reviewed by myself. This documentation has been written by hand as well, as I enjoy writing docs that I would like to read.
