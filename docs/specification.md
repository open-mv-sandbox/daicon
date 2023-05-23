# Daicon - Specification

This is a technical specification of daicon.
If you want a high-level explanation and primer instead, read the explainer document.

## Overview

| Bytes | Description |
| --- | --- |
| 20 | Header |
| N * 12 | Indices |

### Header

| Bytes | Data Type | Description |
| --- | --- |
| 4 | Bytes | Signature, 0x306364FF |
| 2 | Unsigned | Capacity |
| 2 | Unsigned | Valid |
| 8 | Unsigned | Offset |
| 8 | Unsigned | Next |

#### Signature

Magic signature, to verify there is a daicon header at this location.
This should always be validated.
The signature is equivalent to 0xFF followed by "dc0" in ASCII.

#### Capacity

The amount of entries available in this table.
When writing new entries in a file, this number can be used to find free capacity in a table.

#### Valid

The amount of entries that should be seen as valid to read by a reader.

#### Offset

Value all offsets in this table start at.

#### Next

The offset of the start of the next table, or zero if no next table exists.

### Index

| Bytes | Data Type | Description |
| --- | --- | --- |
| 4 | Bytes | Identifier |
| 4 | Unsigned | Offset |
| 4 | Unsigned | Size |

#### Identifier

User-defined identifier.
Parsers should handle this as an opaque value.

#### Offset

Offset of the data.
Value relative to header offset.

#### Size

Size of the data in bytes.

## Change Log

### 0.2.0

- Major specification rewrite, all backwards compatibility with 0.1 broken.
