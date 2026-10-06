# datafmts

`no_std` modular core for config file formats. Traits + static schema + derive.

## Crates

- `datafmts` — core (`Encode` / `Decode` / `Encoder` / `Decoder` / `Schema`)
- `datafmts_derive` — `#[derive(DataStruct)]` (also `#[derive(data_struct)]`)

## Features

| feature | default | notes |
|---|---|---|
| `derive` | yes | re-exports derive macros |
| `csv` | no | `csv::read` / `csv::write`, needs allocator (`alloc`) |

## Example

```rust
use datafmts::DataStruct;

#[derive(DataStruct)]
struct Point { x: i32, y: i32 }

assert_eq!(Point::SCHEMA.name, "Point");
```

```rust
use datafmts::{csv, DataStruct};

#[derive(DataStruct)]
struct Point { x: i32, y: i32 }

let bytes = csv::write(&Point { x: 1, y: 2 }).unwrap();
assert_eq!(bytes, b"x,y\n1,2\n");
```

## Docs

API docs live in rustdoc

- <https://docs.rs/datafmts>

## License

MIT
