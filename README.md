# Daicon

Index regions of a binary blob by ID.

[Read the daicon format documentation here!](docs/index.md)

## Status

Daicon is currently a draft specification, changes will use [Semantic Versioning 2.0.0](https://semver.org/).
You can use daicon in your projects, but no guarantees about cross-compatibility exist until a 1.0 release of the specification, besides an informal recommendation that 0.x minor versions stay compatible.

## Who is using Daicon?

- [Dacti Objects and Packages](https://github.com/open-mv-sandbox/dacti)

## Crates

This is a reference implementation, as well as a parsing and writing library for the rust language.

- [![crates.io](https://img.shields.io/crates/v/daicon.svg?label=daicon)](https://crates.io/crates/daicon) [![docs.rs](https://docs.rs/daicon/badge.svg)](https://docs.rs/daicon/) - Reference rust reader/writer implementation of the daicon format.
- [![crates.io](https://img.shields.io/crates/v/daicon-native.svg?label=daicon-native)](https://crates.io/crates/daicon-native) [![docs.rs](https://docs.rs/daicon-native/badge.svg)](https://docs.rs/daicon-native/) - Native system implementations of daicon protocols.
- [![crates.io](https://img.shields.io/crates/v/daicon-types.svg?label=daicon-types)](https://crates.io/crates/daicon-types) [![docs.rs](https://docs.rs/daicon-types/badge.svg)](https://docs.rs/daicon-types/) - Daicon low-level types, for zero-copy reading and writing.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License (Expat) ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
