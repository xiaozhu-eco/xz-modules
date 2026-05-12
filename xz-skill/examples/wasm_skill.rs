#[cfg(feature = "wasm-runtime")]
use xz_skill::{WasmRuntime, WasmConfig};

#[cfg(feature = "wasm-runtime")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create WASM runtime with custom config
    let wasm_config = WasmConfig {
        memory_limit_mb: 128,
        default_timeout_ms: 10000,
        max_instances: 5,
    };
    let wasm_runtime = WasmRuntime::new(wasm_config)?;

    // Execute a WASM module (in a real scenario, load from file)
    // Here we demonstrate the API structure
    println!("WASM runtime configured: {:?}", wasm_runtime);

    // Example: execute a simple WASM tool
    // let module_bytes = std::fs::read("./tools/my_tool.wasm")?;
    // let result = wasm_runtime.execute(&module_bytes, "process", serde_json::json!({"input": "data"})).await?;
    // println!("WASM result: {:?}", result);

    Ok(())
}

#[cfg(not(feature = "wasm-runtime"))]
fn main() {
    println!("This example requires the 'wasm-runtime' feature");
}
