# Daicon - Explainer

This is a high-level explainer and primer of daicon.
If you want a technical specification instead, read the specification document.

## Motivation

Daicon is a binary format that associates sub-sections of a file with UUIDs.
These descriptions can be updated atomically across file caches, and enables direct offset
indexing.
This makes daicon a good format for use with CDN caches, allowing clients to query a file and
fetch multiple regions of it at the same time, without fetching the entire file.

Some example use cases of daicon are:

- Packaging and delivering large content databases from CDNs.
- Letting different tools extend a format by additing components identified by their UUID.
