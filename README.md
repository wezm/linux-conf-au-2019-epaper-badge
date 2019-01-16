# linux.conf.au 2019 ePaper Badge

## Building

To cross-compile for the Raspberry Pi you will need an
`arm-unknown-linux-gnueabihf` GCC toolchain and Rust component installed. On
Arch Linux I built [arm-linux-gnueabihf-gcc] from the AUR. Add the Rust target
with `rustup target add arm-unknown-linux-gnueabihf`. Then you can
cross-compile with `cargo`:

    cargo build --release --target arm-unknown-linux-gnueabihf

After it is built copy `target/arm-unknown-linux-gnueabihf/release/lca2019` to
the Raspberry Pi.

## Running

View the options with `./lca2019 -h`. By default it will try to bind the
webserver to port 80. You can give a regular user the permission to do this
with:

    sudo setcap cap_net_bind_service=ep lca2019

Alternatively use `-p` to set the port to a non-privileged one.

## Development

### Auto-reloading server

To run the server during development and have it rebuild and restart when
source files are changed I use [watchexec]:

    watchexec -w src -w templates -s SIGINT -r 'cargo run -- -n -p 8080'

## License

This project is dual licenced under:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) **or**
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[cross]: https://github.com/rust-embedded/cross
[watchexec]: https://github.com/watchexec/watchexec
[arm-linux-gnueabihf-gcc]: https://aur.archlinux.org/packages/arm-linux-gnueabihf-gcc/
