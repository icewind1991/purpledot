{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/release-23.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    rust-overlay,
  }:
  utils.lib.eachDefaultSystem (system: let
      overlays = [ (import rust-overlay) ];
      pkgs = (import nixpkgs) {
        inherit system overlays;
      };
    in rec {
      # `nix develop`
      devShell = pkgs.mkShell {
        FFMPEG_PKG_CONFIG_PATH = "${pkgs.ffmpeg_6-headless.dev}/lib/pkgconfig";
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = ''
          -I"${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include"
          -I"${pkgs.musl.dev}/include"
        '';
        nativeBuildInputs = with pkgs; [
          rust-bin.stable.latest.default
          pkg-config
          bacon
          cargo-edit
          cargo-outdated
          cargo-audit
          cargo-msrv
          ffmpeg_6-headless.dev
          bzip2
          lame
          libogg
          soxr
          xvidcore
          libtheora
          xz
        ];
      };
    });
}
