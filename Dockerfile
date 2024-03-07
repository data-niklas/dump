FROM nixos/nix
RUN nix run --extra-experimental-features 'nix-command flakes' github:data-niklas/dump serve
