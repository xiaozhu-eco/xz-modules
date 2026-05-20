# Learnings

## T1.2: validate_wasm() — 2026-05-20

### Summary
Implemented `validate_wasm()` in `xz-skill/src/validation.rs` using TDD (RED → GREEN → REFACTOR).

### Key Details
- **File**: `xz-skill/src/validation.rs` — new file with `pub fn validate_wasm(bytes: &[u8]) -> Result<(), SkillError>`
- **Error variant**: `SkillError::InvalidWasm(String)` added to `error.rs`
- **is_retryable()**: Returns false for `InvalidWasm` (not in the `matches!` pattern)
- **Re-export**: `pub use validation::validate_wasm` in `lib.rs`
- **Module**: NOT behind any `#[cfg(feature)]` gate (GR2 compliance)

### Tests (5/5 passing)
1. `valid_wasm_passes` — `b"\0asm\x01\x00\x00\x00"` → Ok
2. `empty_bytes_fails` — empty slice → Err(InvalidWasm)
3. `too_short_fails` — `b"\0asm"` (4 bytes) → Err(InvalidWasm("too short"))
4. `bad_magic_fails` — `b"NOTasm\x01\x00\x00\x00"` (wrong magic) → Err(InvalidWasm("invalid magic bytes"))
5. `bad_version_fails` — `b"\0asm\x02\x00\x00\x00"` (version 2) → Err(InvalidWasm("unsupported version"))

### Gotcha
- Byte slice comparison in Rust: `&bytes[0..4] != b"\0asm"` (note the `&` prefix on the slice). Without `&`, Rust tries to compare `[u8]` with `&[u8; 4]` which doesn't implement `PartialEq`.

### Verification
- `cargo build -p xz-skill` ✅ (no xz-skill specific errors, only pre-existing missing_docs warnings)
- `cargo clippy -p xz-skill --lib` ✅ (no xz-skill specific clippy warnings)
- `cargo test -p xz-skill --lib -- validation` ✅ (5/5 passed)
- Integration tests fail due to Termux/AArch64 OpenSSL linking issues (pre-existing, not related)

## T1.3: AgentDef + AgentRunResult + AgentError 扩展

### 完成的工作
- **types/agent_def.rs**: 新建文件，包含 `AgentDef`（name, task, depends_on）和 `AgentRunResult`（agent_name, output, success）两个 public struct，均实现 Clone + Debug
- **error.rs**: 已存在的 `CircularDependency(Vec<String>)` 变体不动；新增 `ExecutionFailed(String)` 变体；`is_retryable()` 将 `ExecutionFailed` 加入 retryable 列表（matches! 宏风格）
- **types/mod.rs**: 添加 `pub mod agent_def;`
- **lib.rs**: 添加 `pub use types::agent_def::AgentDef;`（仅 AgentDef，不含 AgentRunResult 以避免与 types::result::AgentRunResult 重名冲突）
- **3 个单元测试**: agent_def_creation, agent_error_is_retryable, agent_run_result_success_flag

### 关键发现
- `types::result::AgentRunResult` 已存在于 lib.rs 的 re-export 中（line 37），因此不能在 lib.rs 中再 re-export `types::agent_def::AgentRunResult`，否则编译报重名冲突。新 `AgentRunResult` 通过完整路径 `types::agent_def::AgentRunResult` 访问
- `error.rs` 已有 `CircularDependency` 变体（line 19-20），`is_retryable()` 中无需额外添加（不在 matches! 中即返回 false）
- 测试函数体内部不能使用 `///` doc comments，只能用 `//` 注释，否则 `unused_doc_comments` 警告
- `//!` module-level doc 使 `pub mod agent_def;` 不触发 missing_docs 错误（已验证）

## T1.1: SkillDefinition + parse_skill_frontmatter() — 执行记录

### 2026-05-20

**变更摘要：**
1. **xz-skill/src/types/skill_def.rs** (新文件): 
   - `SkillDefinition` 结构体 (name, description, tools, wasm_path, metadata)
   - `pub fn parse_skill_frontmatter(markdown: &str) -> Result<SkillDefinition, SkillError>`
   - 使用 `serde_yaml` 解析 `---\n...\n---` YAML frontmatter
   - 私有辅助函数 `parse_tools_value()` 处理工具列表解析
   - 5 个单元测试 (`#[cfg(test)]` 内联)
   
2. **xz-skill/src/error.rs**:
   - 添加 3 个新变体: `ParseError(String)`, `MissingField(String)`, `InvalidFormat(String)`
   - 所有变体都添加了 rustdoc 文档
   - `is_retryable()` 不变（三个新变体都返回 false）
   
3. **xz-skill/src/types/mod.rs**:
   - 添加 `pub mod skill_def;`

4. **xz-skill/src/lib.rs**:
   - 添加 `pub use types::skill_def::{parse_skill_frontmatter, SkillDefinition};`
   - 更新 crate 级别文档，提及 frontmatter parsing

5. **xz-skill/tests/skill_def_tests.rs** (新文件):
   - 5 个集成测试（读取实际 fixture 文件）
   - 使用 `CARGO_MANIFEST_DIR` 定位 fixture 路径

**Fixture 状态：**
- `xz-skill/tests/fixtures/skills/counter/SKILL.md` ✅ (已存在，来自 T1.2)
- `xz-skill/tests/fixtures/skills/bad_wasm/` ✅
- `xz-skill/tests/fixtures/skills/no_wasm/` ✅

**验证结果：**
- `cargo check -p xz-skill` ✅ 通过（零错误，117 warnings 均为 pre-existing missing_docs）
- `cargo clippy -p xz-skill --all-features` ✅ 通过（无新 clippy 警告）
- `cargo test -p xz-skill` ❌ 因 OpenSSL 链接问题失败（Termux/Android 环境限制，与本次变更无关）

**发现的决策/问题：**
- `serde_yaml::Value` 不实现 `Display` trait，无法直接 `format!("{}", other)` — 改用 `parse_tools_value()` 函数
- 通过 `#[serde(default)] tools: Option<serde_yaml::Value>` 灵活支持多种 YAML 工具格式（序列、列表、空值）
- 测试 fixture 中 counter 的工具是 `get_value` 而非 `get_count`（任务描述中的笔误），测试按 fixture 实际内容编写
