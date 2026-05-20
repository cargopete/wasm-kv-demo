use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

mod bindings {
    wasmtime::component::bindgen!({
        path: "../guest/wit",
        world: "demo:kv/demo",
    });
}

use bindings::Demo;

// ---------------------------------------------------------------------------
// State — holds both WASI and the chosen KV backend
// ---------------------------------------------------------------------------

enum KvBackend {
    Redb(wasmtime_wasi_keyvalue_redb::WasiKeyValueRedbCtx),
    Redis(wasmtime_wasi_keyvalue_redis::WasiKeyValueRedisCtx),
}

struct State {
    table: ResourceTable,
    wasi: WasiCtx,
    kv: KvBackend,
}

impl WasiView for State {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "wasm-kv-demo", about = "Run a wasi:keyvalue component with redb or Redis")]
struct Cli {
    #[command(subcommand)]
    backend: BackendCmd,
}

#[derive(Subcommand)]
enum BackendCmd {
    /// Use the embedded redb backend (no external services required)
    Redb {
        /// Path to the redb database file
        #[arg(long, default_value = "/tmp/wasm-kv-demo.redb")]
        path: String,

        #[command(subcommand)]
        op: Op,
    },
    /// Use the Redis backend
    Redis {
        /// Redis URL
        #[arg(long, default_value = "redis://127.0.0.1:6379")]
        url: String,

        #[command(subcommand)]
        op: Op,
    },
}

#[derive(Subcommand, Clone)]
enum Op {
    /// Set a key
    Set {
        bucket: String,
        key: String,
        value: String,
    },
    /// Get a key
    Get { bucket: String, key: String },
    /// Delete a key
    Delete { bucket: String, key: String },
    /// Check if a key exists
    Exists { bucket: String, key: String },
    /// List all keys in a bucket
    List { bucket: String },
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    let (kv, op) = match cli.backend {
        BackendCmd::Redb { path, op } => {
            let ctx = wasmtime_wasi_keyvalue_redb::WasiKeyValueRedbCtxBuilder::new()
                .database_path(&path)
                .with_context(|| format!("failed to open redb at {path}"))?
                .build()?;
            (KvBackend::Redb(ctx), op)
        }
        BackendCmd::Redis { url, op } => {
            let ctx = wasmtime_wasi_keyvalue_redis::WasiKeyValueRedisCtxBuilder::new()
                .url(url)?
                .build()?;
            (KvBackend::Redis(ctx), op)
        }
    };

    let (op_name, bucket, key, value) = match &op {
        Op::Set { bucket, key, value } => {
            ("set", bucket.clone(), key.clone(), Some(value.clone()))
        }
        Op::Get { bucket, key } => ("get", bucket.clone(), key.clone(), None),
        Op::Delete { bucket, key } => ("delete", bucket.clone(), key.clone(), None),
        Op::Exists { bucket, key } => ("exists", bucket.clone(), key.clone(), None),
        Op::List { bucket } => ("list", bucket.clone(), String::new(), None),
    };

    // --- Wasmtime setup ---
    let engine = Engine::default();
    let wasm_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../guest/target/wasm32-wasip2/debug/wasm_kv_demo_guest.wasm"
    );
    let component = Component::from_file(&engine, wasm_path)
        .map_err(|e| anyhow::anyhow!("failed to load component from {wasm_path}: {e}"))?;

    let mut linker: Linker<State> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

    match &kv {
        KvBackend::Redb(_) => {
            wasmtime_wasi_keyvalue_redb::add_to_linker(&mut linker, |s: &mut State| {
                let KvBackend::Redb(ctx) = &s.kv else { unreachable!() };
                wasmtime_wasi_keyvalue_redb::WasiKeyValueRedb::new(ctx, &mut s.table)
            })?;
        }
        KvBackend::Redis(_) => {
            wasmtime_wasi_keyvalue_redis::add_to_linker(&mut linker, |s: &mut State| {
                let KvBackend::Redis(ctx) = &s.kv else { unreachable!() };
                wasmtime_wasi_keyvalue_redis::WasiKeyValueRedis::new(ctx, &mut s.table)
            })?;
        }
    }

    let wasi = WasiCtxBuilder::new().inherit_stdio().build();
    let mut store = Store::new(
        &engine,
        State {
            table: ResourceTable::new(),
            wasi,
            kv,
        },
    );

    let demo = Demo::instantiate(&mut store, &component, &linker)?;
    let result = demo
        .call_run(&mut store, &op_name, &bucket, &key, value.as_deref())?;

    println!("{result}");
    Ok(())
}
