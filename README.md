# Daicon

Daicon containers are a wrapping binary format, made to build up flexible and extendible formats out of "components".

[Read the daicon specification draft here!](docs/daicon.md)

## Status

Daicon is currently a draft specification, changes will use [Semantic Versioning 2.0.0](https://semver.org/).

## Crates

This is a canonical reference implementation, as well as providing common types for Rust.

- [![Crates.io](https://img.shields.io/crates/v/daicon.svg?label=daicon)](https://crates.io/crates/daicon) [![docs.rs](https://docs.rs/daicon/badge.svg)](https://docs.rs/daicon/) - Daicon low-level types, for zero-copy reading and writing
- [![Crates.io](https://img.shields.io/crates/v/wrapmuck.svg?label=wrapmuck)](https://crates.io/crates/wrapmuck) [![docs.rs](https://docs.rs/wrapmuck/badge.svg)](https://docs.rs/wrapmuck/) - Simple wrapper generator around bytemuck pod types

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License (Expat) ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
