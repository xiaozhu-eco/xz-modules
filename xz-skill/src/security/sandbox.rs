/// Sandbox policies and fuel metering configuration for safe code execution.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum fuel (execution units) per WASM invocation
    pub max_fuel: u64,
    /// Maximum memory in bytes
    pub max_memory: usize,
    /// Maximum number of WASM instances across the system
    pub max_instances: usize,
    /// Timeout for WASM execution in milliseconds
    pub timeout_ms: u64,
    /// Maximum output size in bytes from any execution
    pub max_output_size: usize,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_fuel: 10_000_000,
            max_memory: 64 * 1024 * 1024, // 64 MB
            max_instances: 10,
            timeout_ms: 5_000,
            max_output_size: 1024 * 1024, // 1 MB
        }
    }
}
