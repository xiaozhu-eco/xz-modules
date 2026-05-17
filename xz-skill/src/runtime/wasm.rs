#[cfg(feature = "wasm-runtime")]
use std::collections::HashMap;
#[cfg(feature = "wasm-runtime")]
use std::sync::RwLock;

#[cfg(feature = "wasm-runtime")]
use crate::error::SkillError;

#[cfg(feature = "wasm-runtime")]
#[derive(Debug)]
pub struct WasmConfig {
    pub memory_limit_mb: u64,
    pub default_timeout_ms: u64,
    pub max_instances: usize,
}

#[cfg(feature = "wasm-runtime")]
impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: 64,
            default_timeout_ms: 5000,
            max_instances: 10,
        }
    }
}

#[cfg(feature = "wasm-runtime")]
pub struct WasmRuntime {
    engine: wasmtime::Engine,
    module_cache: RwLock<HashMap<String, wasmtime::Module>>,
    config: WasmConfig,
}

#[cfg(feature = "wasm-runtime")]
impl std::fmt::Debug for WasmRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmRuntime")
            .field("engine", &"<wasmtime::Engine>")
            .field("module_cache", &self.module_cache)
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(feature = "wasm-runtime")]
impl WasmRuntime {
    pub fn new(config: WasmConfig) -> Result<Self, SkillError> {
        let mut engine_config = wasmtime::Config::default();
        engine_config
            .consume_fuel(true)
            .epoch_interruption(true);

        let engine = wasmtime::Engine::new(&engine_config)
            .map_err(|e| SkillError::Wasm(e.to_string()))?;

        Ok(Self {
            engine,
            module_cache: RwLock::new(HashMap::new()),
            config,
        })
    }

    /// Compile and execute a WASM module.
    pub async fn execute(
        &self,
        module_bytes: &[u8],
        tool: &str,
        _args: serde_json::Value,
    ) -> Result<serde_json::Value, SkillError> {
        let cache_key = format!("{}:{}", tool, hex::encode(&module_bytes[..module_bytes.len().min(32)]));

        let module = {
            let cache = self.module_cache.read().map_err(|_| SkillError::ToolExecution("module cache lock poisoned".into()))?;
            cache.get(&cache_key).cloned()
        };

        let module = match module {
            Some(m) => m,
            None => {
                let m = wasmtime::Module::new(&self.engine, module_bytes)
                    .map_err(|e| SkillError::Wasm(e.to_string()))?;
                self.module_cache
                    .write()
                    .map_err(|_| SkillError::ToolExecution("module cache lock poisoned".into()))?
                    .insert(cache_key.clone(), m.clone());
                m
            }
        };

        let mut store = wasmtime::Store::new(&self.engine, ());
        store.set_fuel(1_000_000)
            .map_err(|e| SkillError::Wasm(e.to_string()))?;

        let linker = wasmtime::Linker::new(&self.engine);
        let instance = linker
            .instantiate_async(&mut store, &module)
            .await
            .map_err(|e| SkillError::Wasm(e.to_string()))?;

        let result = if let Ok(func) = instance.get_typed_func::<(), i32>(&mut store, tool) {
            let ret = func.call_async(&mut store, ())
                .await
                .map_err(|e| SkillError::Wasm(e.to_string()))?;
            serde_json::json!({"exit_code": ret})
        } else if let Ok(func) = instance.get_typed_func::<(), ()>(&mut store, tool) {
            func.call_async(&mut store, ())
                .await
                .map_err(|e| SkillError::Wasm(e.to_string()))?;
            serde_json::json!({"status": "ok"})
        } else if let Ok(func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, tool) {
            // Memory-based I/O: write args as JSON string to linear memory, call function,
            // read result string from memory at returned pointer.
            let memory = instance.get_memory(&mut store, "memory")
                .ok_or_else(|| SkillError::Wasm("module has no 'memory' export for I/O".into()))?;

            let args_json = _args.to_string();
            let args_bytes = args_json.as_bytes();
            // Write at offset 65536 (well above typical stack size)
            let input_offset: i32 = 65536;
            memory.write(&mut store, input_offset as usize, args_bytes)
                .map_err(|e| SkillError::Wasm(format!("memory write: {}", e)))?;

            let ret_ptr = func.call_async(&mut store, (input_offset, args_bytes.len() as i32))
                .await
                .map_err(|e| SkillError::Wasm(e.to_string()))?;

            // Read null-terminated result string from memory at returned pointer
            let mut result_bytes: Vec<u8> = Vec::new();
            for i in 0..65536usize {
                let mut byte = [0u8; 1];
                if memory.read(&store, (ret_ptr as usize) + i, &mut byte).is_err() {
                    break;
                }
                if byte[0] == 0 {
                    break;
                }
                result_bytes.push(byte[0]);
            }

            let result_str = String::from_utf8(result_bytes)
                .map_err(|e| SkillError::Wasm(format!("non-UTF-8 result: {}", e)))?;

            serde_json::from_str(&result_str)
                .map_err(|e| SkillError::Wasm(format!("result is not valid JSON: {}", e)))?
        } else if let Ok(func) = instance.get_typed_func::<(), ()>(&mut store, "_start") {
            func.call_async(&mut store, ())
                .await
                .map_err(|e| SkillError::Wasm(e.to_string()))?;
            serde_json::json!({"status": "started"})
        } else {
            return Err(SkillError::Wasm(format!(
                "Function '{}' not found in WASM module", tool
            )));
        };

        Ok(result)
    }
}

// Simple hex encoding helper for cache key
#[cfg(feature = "wasm-runtime")]
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
