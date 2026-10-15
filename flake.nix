{
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        flake-utils.url = "github:numtide/flake-utils";
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs = {
                nixpkgs.follows = "nixpkgs";
            };
        };
    };
    outputs = { self, nixpkgs, flake-utils, rust-overlay }:
        flake-utils.lib.eachDefaultSystem
            (system:
                let
                    overlays = [ (import rust-overlay) ];
                    pkgs = import nixpkgs {
                        inherit system overlays;
                    };
                    rustToolchain = pkgs.pkgsBuildHost.rust-bin.stable.latest.default.override {
                        extensions = [ "llvm-tools-preview" ];
                    };

                    # Oldest glibc the Linux binaries must run on. RHEL 9 ships glibc 2.34.
                    #
                    # Binaries from a plain `cargo build` link against the glibc in nixpkgs
                    # (and use a /nix/store dynamic loader), so they only run on recent
                    # distros. `cargo zigbuild` links against glibc stubs of the version in
                    # the target suffix instead and uses the standard system loader:
                    #
                    #   cargo zigbuild --release -p kic --target "$KIC_GLIBC_TARGET"
                    #
                    # Output still lands in target/<triple>/ (without the glibc suffix).
                    minGlibcVersion = "2.34";
                    glibcTarget = "${pkgs.stdenv.hostPlatform.rust.rustcTarget}.${minGlibcVersion}";

                    nativeBuildInputs = with pkgs; [
                        rustToolchain
                        pkg-config
                        just
                        cargo-llvm-cov
                        cargo-nextest
                        cargo-cyclonedx
                        nodejs_24
                    ] ++ lib.optionals stdenv.hostPlatform.isLinux [
                        cargo-zigbuild
                        zig_0_15
                        cmake
                    ];
                    buildInputs = with pkgs; [ ];
                in
                with pkgs;
                {
                    devShells.default = mkShell ({
                        inherit buildInputs nativeBuildInputs;
                    } // lib.optionalAttrs stdenv.hostPlatform.isLinux {
                        KIC_GLIBC_TARGET = glibcTarget;
                        # The justfile switches to `cargo zigbuild` when this is set.
                        KIC_MIN_GLIBC_VERSION = minGlibcVersion;
                        # aws-lc-sys's default cc-based build produces objects that fail to
                        # link under zig ("undefined symbol" in bcm.o); its CMake build works.
                        AWS_LC_SYS_CMAKE_BUILDER = "1";
                    });
                }
            );
}
