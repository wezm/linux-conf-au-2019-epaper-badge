# linux.conf.au 2019 ePaper Badge

## Building

To avoid the need to compile on the Raspberry Pi itself I recommend
cross-compiling with the [cross] tool. With `cross` installed build
as follows:

    cross build --target=arm-unknown-linux-gnueabi --release

After it is built copy `target/arm-unknown-linux-gnueabi/release/lca2019` to
the Raspberry Pi.

## Running

    sudo setcap cap_net_bind_service=ep lca2019

## License

This project is dual licenced under:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) **or**
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[cross]: https://github.com/rust-embedded/cross
