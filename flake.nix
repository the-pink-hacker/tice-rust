{
    description = "Rust tools for the TI-84 Plus CE graphing calculator";
    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
        flake-utils.url = "github:numtide/flake-utils";
        toolchain = {
            url = "github:myclevorname/flake";
            inputs = {
                nixpkgs.follows = "nixpkgs";
                flake-utils.follows = "flake-utils";
            };
        };
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "nixpkgs";
        };
    };
    outputs = {
        self,
        nixpkgs,
        toolchain,
        rust-overlay,
        flake-utils,
        ...
    }:
        flake-utils.lib.eachSystem [
            "x86_64-linux"
            "aarch64-linux"
            "x86_64-darwin"
            "aarch64-darwin"
        ] (system: let
            inherit (nixpkgs) lib;
            pkgs = import nixpkgs {
                localSystem.system = system;
                overlays = [(import rust-overlay)];
                config.allowUnfree = true;
            };
        in {
            packages = {
                # https://gist.github.com/caseyavila/05862db1fcc8b4544bd9dcc9ecc444b9#file-default-nix
                tilp = pkgs.stdenv.mkDerivation {
                    name = "tilp";
                    src = pkgs.fetchurl {
                        url = "https://www.ticalc.org/pub/unix/tilp.tar.gz";
                        sha256 = "1mww2pjzvlbnjp2z57qf465nilfjmqi451marhc9ikmvzpvk9a3b";
                    };
                    postUnpack = ''
                        sed -i -e '/AC_PATH_KDE/d' tilp2-1.18/configure.ac || die
                          sed -i \
                              -e 's/@[^@]*\(KDE\|QT\|KIO\)[^@]*@//g' \
                              -e 's/@X_LDFLAGS@//g' \
                              tilp2-1.18/src/Makefile.am || die
                    '';
                    nativeBuildInputs = with pkgs; [
                        autoreconfHook
                        pkg-config
                        intltool
                        libtifiles2
                        libticalcs2
                        libticables2
                        libticonv
                        gtk2
                    ];
                    buildInputs = with pkgs; [
                        glib
                    ];
                };
            };
            formatter = pkgs.alejandra;
            devShells = {
                default = pkgs.mkShell {
                    packages = with pkgs; [
                        self.packages.${system}.tilp
                        (rust-bin.selectLatestNightlyWith (toolchain:
                            toolchain.default.override {
                                extensions = [
                                    "rust-analyzer"
                                    "rust-src"
                                ];
                            }))
                    ];
                };
            };
        });
}
