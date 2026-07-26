# SUNix (Software Updates for Nix)

`sunix` is a small GTK4 layer-shell popup for Wayland compositors that support `wlr-layer-shell`. It shows the expected changes before a NixOS or Home Manager switch is applied, using [dix](https://github.com/manic-systems/dix) under the hood.

## Install

Add the corresponding input to your Nix flake (recommended).

```nix
{
  inputs.sunix.url = "github:gvolpe/sunix";
}
```

And either use the provide Home Manager module:

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

Reports are cached for the current SUNix process, so clicking the same button again in the same session reuses the existing report.

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

Set `show_demo=true` to reveal a "Demo" button that renders the bundled `sample.json` report, which is also used for testing.

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
