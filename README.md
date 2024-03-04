# <p align="center">Dump</p>
Simple to use file hosting server.
A clone of https://0x0.st written in Rust.


## 🧐 Features
- Simple usage
- File deduplication
- Configurable
  - IP blocklist
  - Disk quota
  - Block file types (such as `executable`, or `archive`)
- Shell auto completion
## 🛠️ Installation


```sh
git clone https://github.com/data-niklas/dump
cd dump
cargo build --release
./target/release/dump
```

For NixOS users:
```sh
nix run 'github:data-niklas/dump'
```


## 💻 Usage
To start the HTTP server:
```sh
dump serve --data-directory path/to/your/state/directory
```

To clean / garbage collect old files:
```sh
dump clean --data-directory path/to/your/state/directory
```

All arguments may be set from environment variables, e.g.:
```sh
export DATA_DIRECTOR=path/to/your/state/directory
dump clean
```
## External dependencies
This project uses [Magika](https://github.com/google/magika/) to correctly detect file types.

## ➤ License
Distributed under the Apache License 2.0 License. See [LICENSE](LICENSE) for more information.
