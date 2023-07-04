# Daicon

Daicon is a binary header format, that indexes regions of a binary blob by 32-bit IDs.

[Read the daicon format specification here!](docs/specification.md)

## Why Daicon

Daicon is designed to be written to and read from atomically across caches, such as CDNs.
Direct indexing before compression allows you to fetch just the data you need using HTTP Range
Requests, all at once!
This lets you use existing well supported CDN infrastructure to serve just the data you need
efficiently.

Some example uses of daicon include:

- Packaging and delivering large content databases from CDNs.
- Creating extendable file formats, by adding components identified by their ID.

## Platform Support

The daicon library can (WIP, see below) run on any system that has a C compiler.
Part of what makes this possible is the highly flexible async runtime "stewart" that the library is
built on.
Interaction with platforms is implemented through the `file` message protocol.

### WASM/Browser

`daicon-web` implements a `file` protocol based on browser JS `fetch`.
This currently uses `wasm-bindgen`, and in the future will support WASM Component Model.

### Transpilation for 'C/C++ only' Platforms

*This is a work in progress, and not yet available.
If you want to work on this yourself feel free to open issues for blockers.*

Some platforms require a publisher to provide and build their code in C/C++ exclusively.
Regardless of the merits of this, this is a requirement for daicon to truly be universally useful
for games.

To support this, daicon can be transpiled to C/C++ using WebAssembly as an intermediate, using
`wasm2c` from the WebAssembly Binary Toolkit.

## Common Issues

### HTTP Multipart Ranges Unsupported

Some S3-compatible CDNs, including AWS and minio, **do not support** multipart ranges in HTTP
requests.

Currently, if multipart is not supported, the entire source file will be fetched at once by daicon.
This can significantly degrade performance, or even fail entirely, if the source file is large.

In the future, we may implement mitigations for this issue.
For testing, we are using NGINX, which does support multipart ranges.

## Crates

This is a reference implementation, as well as a parsing and writing library for the rust language.

- [![crates.io](https://img.shields.io/crates/v/daicon.svg?label=daicon)](https://crates.io/crates/daicon) [![docs.rs](https://docs.rs/daicon/badge.svg)](https://docs.rs/daicon/) -
  Reference rust reader/writer implementation of the daicon format.
- [![crates.io](https://img.shields.io/crates/v/daicon-native.svg?label=daicon-native)](https://crates.io/crates/daicon-native) [![docs.rs](https://docs.rs/daicon-native/badge.svg)](https://docs.rs/daicon-native/) -
  Native system implementations of daicon protocols.
- [![crates.io](https://img.shields.io/crates/v/daicon-types.svg?label=daicon-types)](https://crates.io/crates/daicon-types) [![docs.rs](https://docs.rs/daicon-types/badge.svg)](https://docs.rs/daicon-types/) -
  Daicon low-level types, for zero-copy reading and writing.
- [![crates.io](https://img.shields.io/crates/v/daicon-web.svg?label=daicon-web)](https://crates.io/crates/daicon-web) [![docs.rs](https://docs.rs/daicon-web/badge.svg)](https://docs.rs/daicon-web/) -
  Web fetch implementations of daicon protocols.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License (Expat) ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
