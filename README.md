# wasm-kv-demo

A demonstration of the [wasi:keyvalue] proposal running on Wasmtime with swappable storage backends.

A single WebAssembly component (the guest) performs all key-value operations using the `wasi:keyvalue` interface. The host CLI wires in either an embedded [redb] backend or a [Redis] backend — the component has no knowledge of which one is running underneath it.

```
wasm-kv-demo redb  set   default hello "world"
wasm-kv-demo redb  get   default hello
wasm-kv-demo redb  list  default
wasm-kv-demo redb  exists default hello
wasm-kv-demo redb  delete default hello

wasm-kv-demo redis set   default hello "world"
wasm-kv-demo redis get   default hello
```

## How it works

```
┌──────────────────────────────────┐
│  host CLI (Rust)                 │
│                                  │
│  wasmtime runtime                │
│    └─ wasm-kv-demo-guest.wasm    │  ← same .wasm, always
│         ↓ wasi:keyvalue imports  │
│  ┌──────────────────────────┐    │
│  │  redb backend            │    │  ← or Redis, your choice
│  │  wasmtime-wasi-keyvalue  │    │
│  │  -redb / -redis          │    │
│  └──────────────────────────┘    │
└──────────────────────────────────┘
```

The guest component is compiled to `wasm32-wasip2` and uses `wit-bindgen` to call `wasi:keyvalue/store`. The host uses Wasmtime's component model API to instantiate the component and inject the chosen backend via `add_to_linker`.

## Dependencies

The host uses two standalone crates built as part of this project:

- [`wasmtime-wasi-keyvalue-redb`](https://github.com/cargopete/wasmtime-wasi-keyvalue-redb) — embedded, no external services
- [`wasmtime-wasi-keyvalue-redis`](https://github.com/cargopete/wasmtime-wasi-keyvalue-redis) — Redis-backed, satisfies [wasi:keyvalue Phase 2 portability criteria]

## Building

```bash
# Build the guest component
cd guest
cargo build --target wasm32-wasip2

# Build the host CLI
cd host
cargo build
```

## Running

```bash
# redb (no external services needed)
cargo run -p wasm-kv-demo -- redb set default mykey "hello"
cargo run -p wasm-kv-demo -- redb get default mykey

# Redis (requires a running Redis instance)
docker run --rm -p 6379:6379 redis:7-alpine
cargo run -p wasm-kv-demo -- redis set default mykey "hello"
cargo run -p wasm-kv-demo -- redis get default mykey
```

## Related contributions

This demo is part of a broader set of contributions to the WebAssembly / Bytecode Alliance ecosystem:

| Contribution | Repo | Notes |
|---|---|---|
| [`wasmtime-wasi-keyvalue-redb`](https://github.com/cargopete/wasmtime-wasi-keyvalue-redb) | standalone crate | embedded redb backend for `wasi:keyvalue` |
| [`wasmtime-wasi-keyvalue-redis`](https://github.com/cargopete/wasmtime-wasi-keyvalue-redis) | standalone crate | Redis backend, satisfies Phase 2 portability criteria |
| [wrpc #1229](https://github.com/bytecodealliance/wrpc/pull/1229) | bytecodealliance/wrpc | streams TCP client + server examples |
| [wkg #204](https://github.com/bytecodealliance/wasm-pkg-tools/pull/204) | bytecodealliance/wasm-pkg-tools | improved error message for missing package version |

## Related projects

- [WebAssembly/wasi-keyvalue](https://github.com/WebAssembly/wasi-keyvalue) — the proposal
- [bytecodealliance/wasmtime](https://github.com/bytecodealliance/wasmtime) — the runtime
- [redb](https://github.com/cberner/redb) — embedded database

[wasi:keyvalue]: https://github.com/WebAssembly/wasi-keyvalue
[wasi:keyvalue Phase 2 portability criteria]: https://github.com/WebAssembly/wasi-keyvalue
[redb]: https://www.redb.org
[Redis]: https://redis.io
