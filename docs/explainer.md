# Daicon - Explainer

This is a high-level explainer and primer of daicon.
If you want a technical specification instead, read the specification document.

## Motivation

Daicon is a binary header format, that indexes regions of a binary blob by 8-byte IDs.
These indices are designed to be updated atomically across caches, such as CDNs.

Daicon is designed to work with HTTP range requests.
This lets you pull in just the data you need, all at the same time.

Some example uses of daicon include:

- Packaging and delivering large content databases from CDNs.
- Creating extendable file formats, by adding components identified by their ID.
