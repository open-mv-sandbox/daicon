# Daicon - Specification

This is a technical specification of daicon.
If you want a high-level explanation and primer instead, read the explainer document.

## Overview

| Bytes | Description |
| --- | --- |
| 12 | Header |
| N * 16 | Entries |

### Header

| Bytes | Description |
| --- | --- |
| 4 | Siagnture, 0x306364FF |
| 2 | Capacity |
| 2 | Valid |
| 4 | Next |

#### Signature

Magic signature, to verify there is a daicon header at this location.
This should always be validated.
The signature is equivalent to 0xFF followed by "dc0" in ASCII.

#### Capacity

The amount of entries available in this table.
When writing new entries in a file, this number can be used to find free capacity in a table.

#### Valid

The amount of entries that should be seen as valid to read by a reader.

#### Next

The offset of the start of the next table, or zero if no next table exists.

### Entry

| Bytes | Description |
| --- | --- |
| 8 | ID |
| 4 | Offset |
| 4 | Size |

## Change Log

### 0.2.0 (unreleased)

- Major specification rewrite, all backwards compatibility with 0.1 broken.
