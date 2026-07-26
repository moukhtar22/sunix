{ self }:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.sunix;
  settings = cfg.settings;

  tomlSettings = lib.filterAttrs (_: value: value != null) {
    flake_dir = settings.flakeDir;
    home_flake_dir = settings.homeFlakeDir;
    nixos_flake_dir = settings.nixosFlakeDir;
    home_flake = settings.homeFlake;
    nixos_flake = settings.nixosFlake;
    dix_binary = settings.dixBinary;
    show_demo = settings.showDemo;
  };
in
{
  options.programs.sunix = {
    enable = lib.mkEnableOption "SUNix (Software Updates for Nix)";

    package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "sunix.packages.<system>.default";
      description = "The SUNix package to install.";
    };

    settings = {
      flakeDir = lib.mkOption {
        type = lib.types.str;
        example = "$HOME/workspace/nix-config";
        description = "Default flake directory for Home Manager and NixOS configurations.";
      };

      homeFlakeDir = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "$HOME/workspace/home-config";
        description = ''
          Flake directory for the Home Manager configuration. Falls back to
          settings.flakeDir.
        '';
      };

      nixosFlakeDir = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "$HOME/workspace/nixos-config";
        description = ''
          Flake directory for the NixOS configuration. Falls back to
          settings.flakeDir.
        '';
      };

      homeFlake = lib.mkOption {
        type = lib.types.str;
        example = "niri-hdmi";
        description = "Home Manager flake output name.";
      };

      nixosFlake = lib.mkOption {
        type = lib.types.str;
        example = "aorus";
        description = "NixOS flake output name.";
      };

      dixBinary = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "\${pkgs.dix}/bin/dix";
        description = "Path to the dix executable. When unset, SUNix uses dix from PATH.";
      };

      showDemo = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Whether to show the bundled demo report button and shortcut.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = settings.flakeDir != "";
        message = "programs.sunix.settings.flakeDir must not be empty.";
      }
      {
        assertion = settings.homeFlakeDir == null || settings.homeFlakeDir != "";
        message = "programs.sunix.settings.homeFlakeDir must not be empty when set.";
      }
      {
        assertion = settings.nixosFlakeDir == null || settings.nixosFlakeDir != "";
        message = "programs.sunix.settings.nixosFlakeDir must not be empty when set.";
      }
      {
        assertion = settings.homeFlake != "";
        message = "programs.sunix.settings.homeFlake must not be empty.";
      }
      {
        assertion = settings.nixosFlake != "";
        message = "programs.sunix.settings.nixosFlake must not be empty.";
      }
      {
        assertion = settings.dixBinary == null || settings.dixBinary != "";
        message = "programs.sunix.settings.dixBinary must not be empty when set.";
      }
    ];

    home.packages = lib.optional (cfg.package != null) cfg.package;

    xdg.configFile."sunix/sunix.toml".source =
      (pkgs.formats.toml { }).generate "sunix.toml" tomlSettings;
  };
}
