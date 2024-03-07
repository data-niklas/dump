FROM nixos/nix
RUN nix build --extra-experimental-features 'nix-command flakes' github:data-niklas/dump
RUN nix store optimise
RUN nix run --extra-experimental-features 'nix-command flakes' github:data-niklas/dump serve
