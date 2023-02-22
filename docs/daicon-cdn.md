# Daicon Optimized for CDNs

> 🚧 *This is a working document, describing a work-in-progress format. Nothing described in this document should be seen as final. Features described in this document may not be implemented yet, or differ from as described.*

| Key | Value |
| --- | --- |
| Name | Daicon Optimized for CDNs |
| Version | 0.1.0-draft 🚧 |
| Daicon Version | 0.1.0 |

> 🚧 This document has been recently extracted from daicon's main specification, and the wording doesn't entirely reflect this different approach yet.

## Motivation

- Direct addressing. Daicon containers do not require any special parsing or decompressing at a container level to access the inner data. This is delegated to the inner components which may, in the case of "dacti packages" for example, decide to only do compression at a per-object level. This allows areas to be directly addressed through, for example, [HTTP Range Requests](https://developer.mozilla.org/en-US/docs/Web/HTTP/Range_requests).
- Cache coherency. Daicon is designed to work well with CDN and edge caches. Derived formats can append additional data and update atomically without needing to invalidate the entire file.

## Reducing Round-Trips

If your format will be fetched *partially* from a server, and then indexed using ranges, your format specification should include recommendations to reduce necessary round-trips.

For example, you can recommend (or even require) an index component describing regions contained in your file to exist within the first 64kb. This would allow a client aware of your format to always fetch the full first 64kb and not need additional round-trips to the server.

Not all components have to fall in this region, only those that need this 'fast-path'. You are recommended to specify that clients should degrade performance rather than fail if the included components' data exceeds the specified region.

## CDN Cache Coherency

Daicon containers are designed for efficient cache coherency on CDNs and edge caches. To achieve this, daicon's component system can be updated atomically.

You can use the values in the component table as atomic switches, after appending binary data, repointing locations, and validating all caches have been updated. The component table itself also has "count" and "extension", which too can be atomically updated after verifying a cache flush.

If your format needs this functionality in combination with "Reducing Round-Trips", you are recommended to specify padding in the pre-fetch region, reserving it, to allow the file to be updated without a full cache flush. You should also pad the component table for the same reason.

## Specifying Append-Only

Binary Data previously written should **never** move or change its value to ensure stale client table requests do not retrieve corrupt data from an update. Table pointer to offsets may be updated as necessary. If a file has stale or unused sections, a new file should be created with the unnecessary data culled out.
