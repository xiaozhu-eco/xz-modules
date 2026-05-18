# xz-modules 剩余任务完成计划

## TL;DR

> **Quick Summary**: 完成 xz-modules-fix 计划中剩余的 7 个任务（原计划 9 个，代码探索发现 2 个已完成），涵盖 3 个 Stub 功能实现和 4 个 MEDIUM/ARCH 修复，按 Phase 6 快速修复先行的策略执行。
>
> **Deliverables**:
> - ✅ `cargo build --workspace --all-features` 零错误
> - ✅ Phase 6 的 4 个修复完成（每个附带回归测试）
> - ⬜ Phase 5 的 3 个 Stub 功能实现（每个附带功能测试）
> - ✅ 全量 Gate Check（build + test + clippy + fmt）
> - ✅ Final Verification Wave 四重审查通过
>
> **Estimated Effort**: Medium（约 5-8 个开发日）
> **Parallel Execution**: YES — 5 Waves
> **Critical Path**: Phase 6 → Gate Check → Phase 5 → Gate Check → Final Verification Wave

---

## Context

### Original Request
用户要求针对 xz-modules-fix 计划中未完成的任务制定详细的执行计划。

### 代码探索关键发现
**原计划中的 2 个任务已实际完成**（经逐行代码审查确认）：
- **Task 5.3 (RAG Streaming + Reranking)**: `xz-rag/src/engine.rs:retrieve_and_generate_stream()` 已完整实现流式事件管道；`#[cfg(feature = "rerank")]` 下 reranking 集成已完整。无需额外工作。
- **Task 6.6 (Vector validation + cosine_similarity)**: `xz-memory/src/store/sqlite.rs` 的 `search_vector` 已检查维度不匹配并 `continue`；`bincode_deserialize` 正确返回 `Result`；`cosine_similarity` 在 memory.rs 有零幅度守卫。无需额外工作。

**实际剩余 7 个任务**（从 9 个缩减到 7 个）。

### 已完成的修复（来自原始计划 Phase 0-4）
- Phase 0: 验证（2/2）
- Phase 1: 编译修复（3/3）— 含扩展范围的 xz-provider/rag/skill/memory 修复
- Phase 2: 核心逻辑修复（4/4）— fact_category、latency_tracker、SSE、空 filter
- Phase 3: 并发安全修复（3/3）— scheduler RwLock、rate_limiter、unwrap 传播
- Phase 4: HIGH bugs（4/4）— 超时/重试、urlencoding、HYDE wiring、SSML
- Phase 6.1: thiserror 统一 + rust-toolchain
- Phase 7 + Final Wave: 全量验证 ✅

### Metis Review
**关键架构决策（需在实现前解决）**：
- **Task 5.4 LLM 集成**：xz-search 当前不依赖 xz-provider。Metis 识别三种方案：(1) 直接依赖 xz-provider (2) Trait 抽象注入 (3) 直接用 reqwest HTTP。采用 **Trait 抽象**（最 Rust-idiomatic，避免循环依赖）。

**识别的防护栏**：
- 不修改已完成的 Phase 0-4 文件（除非任务明确要求）
- Phase 5 Stub 功能不扩展范围（如不添加 20 种内置工具）
- Task 6.3 仅改数据结构（BinaryHeap），不改算法逻辑
- Task 6.4 仅做内存 TTL 检查，不引入后台清理任务
- Task 6.5 名称映射保持 JSON 序列化格式向后兼容

---

## Work Objectives

### Core Objective
完成 7 个剩余任务：4 个 MEDIUM/ARCH 修复（Phase 6）+ 3 个 Stub 功能实现（Phase 5），使整个工作区达到无未完成 Stub、无已知 bug 的状态。

### Concrete Deliverables
- `cargo build --workspace --all-features` → exit 0
- `cargo test --workspace --all-features` → 0 failures
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0
- `cargo fmt --all -- --check` → exit 0
- 4 个 Phase 6 修复（每个带 TDD 回归测试）
- 3 个 Phase 5 Stub 实现（每个带功能验证测试）

### Definition of Done
- [ ] `cargo build --workspace --all-features` exit code 0
- [ ] `cargo test --workspace --all-features` 显示 `test result: ok. 0 failed`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit code 0
- [ ] `cargo fmt --all -- --check` exit code 0
- [ ] 所有 Phase 6 修复附带 TDD 测试对（PRE-FIX FAIL → POST-FIX PASS）
- [ ] 所有 Phase 5 Stub 功能附带功能测试

### Must Have
- Task 6.2: 429 Retry-After 解析 + client 复用 + .expect() 消除
- Task 6.3: BinaryHeap 替换 + serde 错误传播
- Task 6.4: TTL 过期检查 + 防饥饿配置
- Task 6.5: 信号权重名称映射
- Task 5.1: Cron 调度器 + Cancel 实现
- Task 5.2: WASM 输出 + builtin 工具扩展
- Task 5.4: QueryRewriter LLM 集成

### Must NOT Have (Guardrails)
- ❌ 不修改已完成的 Phase 0-4/6.1 修复文件（除非任务明确要求）
- ❌ 不新增外部 crate 依赖（使用已有的 tokio-cron-scheduler、wasmtime 等）
- ❌ 不引入 unsafe 代码
- ❌ Task 5.2 不添加超过 3 个新内置工具（search_web、read_file、exec_command 骨架）
- ❌ Task 5.4 不构建完整查询规划管道（仅限重写功能）
- ❌ Task 6.3 不重构 Dijkstra 算法逻辑（仅替换数据结构）
- ❌ Task 6.4 不引入后台清理任务/扫描器
- ❌ Task 6.5 不改变 SignalWeights 的 JSON 序列化格式

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — 所有验证通过 `cargo build`、`cargo test`、`cargo clippy` 自动执行。

### Test Decision
- **Infrastructure exists**: YES（`cargo test --workspace --all-features` 已就绪）
- **Phase 6 (修复)**: TDD — 先写复现 bug 的测试（预期 FAIL），修复后 PASS
- **Phase 5 (Stub)**: Tests-after — 先做最简实现，再补功能测试
- **Framework**: `cargo test`（Rust 内置）

### QA Policy
每个 TODO 包含 Agent-Executed QA 场景：
- **编译验证**: `cargo build -p <crate> --all-features`
- **测试验证**: `cargo test -p <crate>`
- **Workspace 验证**: `cargo test --workspace --all-features`

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Phase 6 — MEDIUM/ARCH 修复, MAX PARALLEL):
├── Task 6.5: xz-rerank 信号权重名称映射 [quick-2h]
├── Task 6.3: xz-knowledge-graph BinaryHeap + serde [unspecified-high-2h]
├── Task 6.2: xz-provider 429 + client 复用 + expect [unspecified-high-4h]
└── Task 6.4: xz-notification TTL + 饥饿配置 [quick-2h]

Wave 2 (Phase 6 Gate Check):
└── Task GC1: 全 workspace build + test + clippy [quick]

Wave 3 (Phase 5 — Stubs 实现, MAX PARALLEL):
├── Task 5.4: xz-search QueryRewriter LLM 集成 [deep-1d]
├── Task 5.1: xz-agent Cron scheduler + Cancel [deep-2d]
└── Task 5.2: xz-skill WASM 输出 + builtin 工具 [deep-1d]

Wave 4 (Phase 5 Gate Check):
└── Task GC2: 全 workspace build + test + clippy + doc [quick]

Wave FINAL (四重审查, MAX PARALLEL):
├── Task F1: Plan Compliance Audit (oracle)
├── Task F2: Code Quality Review (unspecified-high)
├── Task F3: Real Manual QA (unspecified-high)
└── Task F4: Scope Fidelity Check (deep)

Critical Path: 6.5/6.3/6.2/6.4 → GC1 → 5.4 → 5.1 → 5.2 → GC2 → F1-F4
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 4 (Waves 1 & 3 & FINAL)
```

### Dependency Matrix

- **6.2, 6.3, 6.4, 6.5**: None → GC1（4 个 Phase 6 任务互不依赖，全量并行）
- **GC1**: 6.2, 6.3, 6.4, 6.5 → 5.1, 5.2, 5.4
- **5.1**: GC1, 4.1 → GC2（依赖 Phase 4.1 的调度器改动）
- **5.2**: GC1 → GC2
- **5.4**: GC1 → GC2（需先在实现前决策 LLM 集成方案）
- **GC2**: 5.1, 5.2, 5.4 → F1-F4
- **F1-F4**: GC2 → None（并行执行）

---

## TODOs

- [x] 6.5 **Phase 6 — xz-rerank 信号权重名称映射**

  **What to do**:
  - `xz-rerank/src/local/engine.rs`（315 行）: 当前 `weighted_sum()` 使用位置索引（0→keyword_overlap, 1→vector_similarity...）将信号分数映射到权重。这在信号顺序改变时会静默错位。
  - **修复方案**: 给 `SignalPlugin` trait（`xz-rerank/src/traits/signal.rs`）新增 `weight_key(&self) -> &'static str` 方法，各信号实现返回对应权重字段名。`weighted_sum()` 改为按名称查找权重。
  - **向后兼容**: `SignalWeights` 结构体保持不变，其 JSON 序列化格式不变。新增 `get_weight_by_name(&self, name: &str) -> f32` 方法。
  - **关键信号**: `KeywordOverlapSignal.weight_key() → "keyword_overlap"`、`VectorSimilaritySignal → "vector_similarity"`、`MetadataMatchSignal → "metadata_match"`、`ContentQualitySignal → "content_quality"`、`RecencySignal → "recency"`
  - TDD: 注册 3 个信号（非全部 5 个）→ 仅 3 个权重参与计算；信号顺序改变 → 权重映射不受影响

  **Must NOT do**:
  - 不改变 `SignalWeights` 的 JSON 序列化格式
  - 不重写信号注册系统
  - 不修改 `SignalPlugin` trait 的其他方法

  **Recommended Agent Profile**:
  > 信号权重查找——改动集中在 2 个文件，涉及 trait 扩展。
  - **Category**: `quick`
  - **Skills**: `[]`
  - **Justification**: 单文件改动 + trait 方法添加，逻辑简单，无需外部研究。

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 6.2, 6.3, 6.4 并行）
  - **Blocks**: None
  - **Blocked By**: None（可立即开始）

  **References**:
  - `xz-rerank/src/local/engine.rs:86-115` — `compute_scores()` + `weighted_sum()` 当前实现，位置索引映射
  - `xz-rerank/src/local/engine.rs:298-312` — `get_weight(idx)` 当前按位置查找
  - `xz-rerank/src/traits/signal.rs` — `SignalPlugin` trait 定义（需添加 `weight_key()`）
  - `xz-rerank/src/local/signals/keyword_overlap.rs` — 第一个信号实现示例
  - `xz-rerank/src/types/weights.rs:1-67` — `SignalWeights` 结构体（保持不变）

  **Acceptance Criteria**:
  - [ ] `SignalPlugin` trait 新增 `weight_key(&self) -> &'static str`
  - [ ] `weighted_sum()` 改为按名称查找权重，不再依赖位置索引
  - [ ] `get_weight(idx)` 废弃或重命名为名称查找
  - [ ] PRE-FIX: `signal_weight_name_mapping` test FAIL（3 信号 + 5 权重时权重错位）
  - [ ] POST-FIX: test PASS
  - [ ] `cargo test -p xz-rerank` → all PASS

  **QA Scenarios**:
  ```
  Scenario: 信号权重按名称正确映射（注册部分信号）
    Tool: Bash
    Preconditions: 仅注册 KeywordOverlap + Recency（2 个信号），权重为 {0.8, 0.0, 0.0, 0.0, 0.2}
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-rerank signal_weight_name_mapping -- --nocapture
    Expected Result: test signal_weight_name_mapping ... ok（keyword_overlap 权重=0.8, recency 权重=0.2，中间权重不被使用）
    Failure Indicators: 测试 FAIL 或权重应用到错误的信号上
    Evidence: .sisyphus/evidence/task-6.5-weight-mapping.txt

  Scenario: 信号顺序改变不影响权重映射
    Tool: Bash
    Steps:
      1. cargo test -p xz-rerank signal_reorder_independent -- --nocapture
    Expected Result: 改变注册顺序后，每个信号仍获得正确的权重
    Evidence: .sisyphus/evidence/task-6.5-reorder.txt
  ```

  **Commit**: YES
  - Message: `fix(xz-rerank): map signal weights by name instead of positional index`
  - Files: `xz-rerank/src/local/engine.rs`, `xz-rerank/src/traits/signal.rs`, `xz-rerank/src/local/signals/*.rs`

- [x] 6.3 **Phase 6 — xz-knowledge-graph BinaryHeap 替换 + serde 错误传播**

  **What to do**:
  - `xz-knowledge-graph/src/traversal/path.rs:44-57`: `dijkstra_shortest_path()` 当前使用排序 `Vec` 作为优先队列（每次迭代 `sort_by` = O(n² log n)）。替换为 `std::collections::BinaryHeap<std::cmp::Reverse<(f32, String)>>` 实现最小堆 → O(n log n)。
  - `xz-knowledge-graph/src/store/sqlite.rs:585-675`: `shortest_path()` 同样使用排序 `Vec`。同步替换为 `BinaryHeap<Reverse<...>>`。注意 `Reverse` 包装使 `BinaryHeap`（最大堆）表现为最小堆。
  - serde 错误传播：`traversal/path.rs` 和 `store/sqlite.rs` 中的 `serde_json::from_str(...).unwrap_or_default()` → `map_err(|e| KgError::Serialization(e.to_string()))?`。**仅限路径算法相关代码**——存储层的 `unwrap_or_default` 不在范围内（它们是可接受的降级默认值）。
  - TDD: 50 节点随机图性能对比（BinaryHeap 版本不低于 Vec 版本）；损坏 JSON 返回错误而非静默默认值

  **Must NOT do**:
  - 不重构 Dijkstra 算法逻辑（邻接表构建、距离追踪保持不变）
  - 不修改存储层的 serde `unwrap_or_default`（仅修改路径算法相关）
  - 不引入新 crate 依赖（`BinaryHeap` 是 std）

  **Recommended Agent Profile**:
  > 数据结构替换 + 错误传播——2 个文件，需要理解 Dijkstra 实现。
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 6.2, 6.4, 6.5 并行）
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `xz-knowledge-graph/src/traversal/path.rs:44-57` — 当前 `dijkstra_shortest_path` 使用 `Vec::sort_by`
  - `xz-knowledge-graph/src/store/sqlite.rs:636-652` — `shortest_path` 中排序队列
  - `xz-knowledge-graph/src/store/sqlite.rs:1185-1187,1223` — `serde_json::from_str().unwrap_or_default()` 位置
  - `xz-knowledge-graph/src/traversal/path.rs:1-138` — Dijkstra 完整实现，理解算法结构
  - `xz-knowledge-graph/src/error.rs` — `KgError` 枚举，确认 `Serialization` 变体存在

  **Acceptance Criteria**:
  - [ ] `dijkstra_shortest_path` 使用 `BinaryHeap<Reverse<...>>` 代替排序 Vec
  - [ ] `store/sqlite.rs:shortest_path` 同步替换
  - [ ] 路径算法中 serde 失败传播为 `Err(KgError)` 而非静默默认值
  - [ ] `cargo test -p xz-knowledge-graph` → all PASS
  - [ ] 50 节点图路径查找延迟不增加（性能不退化）

  **QA Scenarios**:
  ```
  Scenario: BinaryHeap 最短路径正确 + 性能不退化
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-knowledge-graph shortest_path_correctness -- --nocapture
      3. cargo test -p xz-knowledge-graph shortest_path_perf -- --nocapture
    Expected Result: 路径结果与 Vec 版本一致（功能不变）；BinaryHeap 版本延迟 ≤ Vec 版本
    Failure Indicators: 路径结果不同；BinaryHeap 显著更慢
    Evidence: .sisyphus/evidence/task-6.3-binaryheap.txt

  Scenario: 损坏 JSON 返回错误而非静默默认值
    Tool: Bash
    Steps:
      1. cargo test -p xz-knowledge-graph serde_error_propagation -- --nocapture
    Expected Result: test serde_error_propagation ... ok（损坏 JSON → Err(KgError::Serialization)）
    Evidence: .sisyphus/evidence/task-6.3-serde-error.txt

  Scenario: 断开图返回空路径
    Tool: Bash
    Steps:
      1. cargo test -p xz-knowledge-graph disconnected_graph_empty_path -- --nocapture
    Expected Result: test disconnected_graph_empty_path ... ok（不存在路径 → 空 Vec）
    Evidence: .sisyphus/evidence/task-6.3-disconnected.txt
  ```

  **Commit**: YES
  - Message: `fix(xz-knowledge-graph): optimize shortest_path with BinaryHeap and propagate serde errors`
  - Files: `xz-knowledge-graph/src/traversal/path.rs`, `xz-knowledge-graph/src/store/sqlite.rs`

- [x] 6.2 **Phase 6 — xz-provider 429 Retry-After + client 复用 + expect 消除**

  **What to do**:
  修复 3 个独立问题：
  1. **429 Retry-After 解析** (`openai.rs:252-253,377-378` / `claude.rs:311-312,454-455`): 当前硬编码 `retry_after_ms: 5000`。改为从 HTTP 429 响应头 `Retry-After` 提取秒数。支持两种格式：(a) 纯秒数 `Retry-After: 120` (b) HTTP 日期（少用但 RFC 要求）`Retry-After: Wed, 21 Oct 2015 07:28:00 GMT`。解析失败时回退到 5000ms 默认值。
  2. **LocalProvider client 复用** (`local.rs:92,159`): 当前每次 `complete()` 和 `complete_stream()` 调用都 `let client = reqwest::Client::new()`——浪费连接池。将 `client: reqwest::Client` 添加为 `LocalProvider` 结构体字段，通过构造函数注入。
  3. **builder.rs expect 消除** (`builder.rs:92`): `.expect("Failed to build HTTP client")` → `map_err(|e| ProviderError::Config(format!("HTTP client build failed: {}", e)))?`

  **Must NOT do**:
  - 不触及测试或其他文件中的 `.expect()`
  - 不修改 OpenAi/Claude 的 client 复用（已正确实现）
  - 不引入新 crate 依赖

  **Recommended Agent Profile**:
  > HTTP 客户端修复——4 个文件，涉及 reqwest API、HTTP 头解析、错误传播。
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 6.3, 6.4, 6.5 并行）
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `xz-provider/src/providers/openai.rs:245-260` — 429 处理，硬编码 retry_after_ms:5000
  - `xz-provider/src/providers/claude.rs:305-320` — 同上
  - `xz-provider/src/providers/local.rs:85-100,152-165` — `let client = reqwest::Client::new()` 每请求新建
  - `xz-provider/src/builder.rs:85-95` — `.expect("Failed to build HTTP client")`
  - `xz-provider/src/error.rs` — `ProviderError` 枚举，确认 `RateLimit { retry_after_ms }` 和 `Config` 变体

  **Acceptance Criteria**:
  - [ ] 429 响应头 `Retry-After: 120` → `RateLimit { retry_after_ms: 120000 }`
  - [ ] 无效/缺失 Retry-After 头 → 回退到 5000ms
  - [ ] `LocalProvider` 结构体有 `client: reqwest::Client` 字段，注入复用
  - [ ] `builder.rs` 无 `.expect()` — HTTP client 构建失败返回 `Err`
  - [ ] `cargo test -p xz-provider` → all PASS

  **QA Scenarios**:
  ```
  Scenario: 429 响应解析 Retry-After 秒数
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-provider retry_after_header_seconds -- --nocapture
    Expected Result: test retry_after_header_seconds ... ok（Retry-After: 120 → retry_after_ms=120000）
    Failure Indicators: 仍为 5000ms
    Evidence: .sisyphus/evidence/task-6.2-retry-after.txt

  Scenario: 缺失 Retry-After 头回退默认值
    Tool: Bash
    Steps:
      1. cargo test -p xz-provider retry_after_header_missing -- --nocapture
    Expected Result: test retry_after_header_missing ... ok（无头 → retry_after_ms=5000）
    Evidence: .sisyphus/evidence/task-6.2-retry-fallback.txt

  Scenario: builder.rs HTTP client 构建失败返回 Err 不 panic
    Tool: Bash
    Steps:
      1. cargo test -p xz-provider builder_no_panic -- --nocapture
    Expected Result: test builder_no_panic ... ok（build 失败 → Err 而非 panic）
    Evidence: .sisyphus/evidence/task-6.2-builder.txt
  ```

  **Commit**: YES
  - Message: `fix(xz-provider): parse Retry-After header, reuse HTTP client in local provider, remove expect from builder`
  - Files: `xz-provider/src/providers/openai.rs`, `xz-provider/src/providers/claude.rs`, `xz-provider/src/providers/local.rs`, `xz-provider/src/builder.rs`

- [x] 6.4 **Phase 6 — xz-notification TTL 过期检查 + 防饥饿配置**

  **What to do**:
  1. **TTL 过期检查** (`priority_queue.rs`): `dequeue()` 中跳过 `item.enqueued_at + item.ttl < Instant::now()` 的过期项。静默丢弃（不记录、不返回错误），继续尝试下一个出队项。使用 `Instant::now()` 与 `enqueued_at` 比较（无 async 阻塞）。
  2. **防饥饿配置化** (`priority_queue.rs` + `manager.rs`):
     - 新建 `QueueConfig` 结构体：`high_burst_limit: usize = 5`、`low_starvation_limit: usize = 20`
     - `PriorityQueue::new(config: QueueConfig)` 接受配置
     - `DefaultNotificationManager` 通过 `ManagerBuilder` 或构造函数接受 `QueueConfig`
  3. **TTL=0 语义**: `ttl: Some(Duration::ZERO)` → 视为"立即过期"→出队时跳过

  **Must NOT do**:
  - 不引入后台清理任务/扫描器（仅内存即时检查）
  - 不修改 `QueueItem` 结构体字段类型
  - 不改变 TTL 存储逻辑

  **Recommended Agent Profile**:
  > 队列逻辑修复——2 个文件，涉及时间计算和配置结构体。
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 6.2, 6.3, 6.5 并行）
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `xz-notification/src/queue/priority_queue.rs:1-60` — `QueueItem` 结构体，TTL/enqueued_at 字段
  - `xz-notification/src/queue/priority_queue.rs:70-120` — `dequeue()` 当前实现（无 TTL 检查）
  - `xz-notification/src/queue/priority_queue.rs:15-20` — `HIGH_BURST_LIMIT` 和 `LOW_STARVATION_LIMIT` 常量
  - `xz-notification/src/manager.rs:130-150` — TTL 映射和立即出队调用点

  **Acceptance Criteria**:
  - [ ] TTL=1ms 的条目，入队后 10ms 出队 → 跳过（不返回此条目）
  - [ ] 全队列过期 → `dequeue()` 最终返回 `None`（不无限循环）
  - [ ] `high_burst_limit` 和 `low_starvation_limit` 可通过 `QueueConfig` 配置
  - [ ] `cargo test -p xz-notification` → all PASS

  **QA Scenarios**:
  ```
  Scenario: 过期消息被静默跳过
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-notification ttl_expiry_skipped -- --nocapture
    Expected Result: test ttl_expiry_skipped ... ok（入队 TTL=1ms，sleep 10ms，dequeue → None 或不含过期项）
    Failure Indicators: 过期项仍被出队返回
    Evidence: .sisyphus/evidence/task-6.4-ttl.txt

  Scenario: 全队列过期不出队无限循环
    Tool: Bash
    Steps:
      1. cargo test -p xz-notification ttl_all_expired -- --nocapture
    Expected Result: test ttl_all_expired ... ok（100 个全部过期 → dequeue 返回 None，不超时）
    Evidence: .sisyphus/evidence/task-6.4-all-expired.txt

  Scenario: 防饥饿阈值可配置
    Tool: Bash
    Steps:
      1. cargo test -p xz-notification starvation_config -- --nocapture
    Expected Result: test starvation_config ... ok（QueueConfig { high_burst_limit: 3 } → 第 4 个高优先级被降级）
    Evidence: .sisyphus/evidence/task-6.4-starvation.txt
  ```

  **Commit**: YES
  - Message: `fix(xz-notification): enforce TTL expiry on dequeue and make starvation thresholds configurable`
  - Files: `xz-notification/src/queue/priority_queue.rs`, `xz-notification/src/manager.rs`

- [x] GC1 **Gate Check 1 — Phase 6 修复后全 workspace 验证**

  **What to do**:
  - `cargo build --workspace --all-features` → exit 0
  - `cargo test --workspace --all-features` → 0 failed
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0
  - 确认所有 Phase 6 TDD 测试对（PRE-FIX FAIL + POST-FIX PASS）可验证

  **Must NOT do**: 不修改代码（只读验证）

  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`

  **Parallelization**: **Can Run In Parallel**: NO | **Parallel Group**: Wave 2（串行，在 Phase 6 之后） | **Blocks**: ALL Phase 5 tasks

  **Acceptance Criteria**:
  - [ ] `cargo build --workspace --all-features` → exit 0
  - [ ] `cargo test --workspace --all-features` → 显示 "0 failed"
  - [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0

  **QA Scenarios**:
  ```
  Scenario: Phase 6 修复后全 workspace 门禁通过
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo build --workspace --all-features 2>&1 | tail -3
      3. cargo test --workspace --all-features 2>&1 | grep "test result"
      4. cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -3
    Expected Result: build exit 0; test result: ok. N passed; 0 failed; clippy exit 0
    Failure Indicators: 任何 "error[" 或 "FAILED" 输出
    Evidence: .sisyphus/evidence/task-gc1-gate.txt
  ```

  **Commit**: NO（只读验证）

- [x] 5.4 **Phase 5 — xz-search QueryRewriter LLM 集成**

  **What to do**:
  - **架构决策**（Metis 建议）: 采用 Trait 抽象方案——定义 `QueryRewriteProvider` trait，避免 xz-search 直接依赖 xz-provider。
  - `xz-search/src/rewrite/mod.rs`（162 行）: 当前 `rewrite_with_template` 是纯启发式，`prompt_template` 字段未使用。LLM 集成需：
    1. 在 `xz-search/src/rewrite/` 下新建 `provider.rs`，定义 `QueryRewriteProvider` trait：
       ```rust
       pub trait QueryRewriteProvider: Send + Sync {
           async fn rewrite(&self, query: &str, prompt: &str) -> Result<String, SearchError>;
       }
       ```
    2. `QueryRewriter` 新增 `rewrite_with_llm(query, template, provider: &dyn QueryRewriteProvider) -> Result<Vec<String>, SearchError>` 方法
    3. 当 `provider` 不可用时（`Option::None` 或无 LLM feature）→ 自动回退到启发式重写
  - **LLM 实现建议**: 在 `xz-search/src/rewrite/` 下新增 `openai_provider.rs`（behind `llm-rewrite` feature gate），实现 `QueryRewriteProvider` 通过直接 HTTP 调用 OpenAI API（复用 xz-search 已有的 `reqwest` 依赖）。避免引入 xz-provider 的重量级依赖。
  - `xz-search/Cargo.toml`: 新增 `llm-rewrite = []` feature（无额外依赖，使用已有 reqwest）
  - TDD: mock LLM 返回改写查询 → 验证 `rewrite_with_llm` 结果 ≠ 启发式结果；无 provider → 回退到启发式等同

  **Must NOT do**:
  - 不构建完整查询规划管道（仅限查询重写功能）
  - 不引入 xz-provider 作为直接依赖
  - 不将 `QueryRewriteProvider` trait 设计为通用 LLM 抽象层（保持最小化）

  **Recommended Agent Profile**:
  > Trait 设计 + LLM HTTP 集成 + feature gate——设计决策 + 实现，需 Rust trait 模式经验。
  - **Category**: `deep`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3（与 5.1, 5.2 并行）
  - **Blocks**: GC2
  - **Blocked By**: GC1

  **References**:
  - `xz-search/src/rewrite/mod.rs:1-162` — 当前 `QueryRewriter` 实现（启发式 + 未使用的 prompt_template）
  - `xz-search/src/rewrite/mod.rs:50-80` — `RewriteTemplate` 枚举
  - `xz-search/src/rewrite/mod.rs:100-145` — `rewrite_with_template` 当前实现
  - `xz-search/Cargo.toml` — feature flags 和 reqwest 依赖
  - `xz-provider/src/types/request.rs` — CompletionRequest 结构体参考（不直接使用）

  **Acceptance Criteria**:
  - [ ] `QueryRewriteProvider` trait 定义于 `xz-search/src/rewrite/provider.rs`
  - [ ] `QueryRewriter::rewrite_with_llm(query, template, provider)` — 有 provider 时 LLM 改写，无时回退启发式
  - [ ] `#[cfg(feature = "llm-rewrite")]` 门控 OpenAiRewriteProvider
  - [ ] `cargo build -p xz-search --features llm-rewrite` → exit 0
  - [ ] `cargo build -p xz-search`（无 feature）→ exit 0（不引入编译依赖）
  - [ ] `cargo test -p xz-search` → all PASS

  **QA Scenarios**:
  ```
  Scenario: LLM 改写有效 + 无 LLM 时回退启发式
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-search rewrite_with_llm --features llm-rewrite -- --nocapture
      3. cargo test -p xz-search rewrite_fallback_heuristic -- --nocapture
    Expected Result: LLM 改写结果 ≠ 原始查询（被改写过）；无 provider 时结果 = 启发式结果
    Failure Indicators: LLM 改写与启发式完全相同；无 provider 时 panic 或空结果
    Evidence: .sisyphus/evidence/task-5.4-rewriter.txt

  Scenario: 不加 llm-rewrite feature 时编译通过
    Tool: Bash
    Steps:
      1. cargo build -p xz-search 2>&1
    Expected Result: exit 0（不强制引入 LLM 依赖）
    Evidence: .sisyphus/evidence/task-5.4-no-feature.txt
  ```

  **Commit**: YES
  - Message: `feat(xz-search): add LLM query rewriting via QueryRewriteProvider trait`
  - Files: `xz-search/src/rewrite/mod.rs`, `xz-search/src/rewrite/provider.rs`, `xz-search/Cargo.toml`

- [x] 5.1 **Phase 5 — xz-agent Cron scheduler + Cancel 实现**

  **What to do**:
  三个独立功能点：
  1. **Cron 调度器** (`trigger/cron.rs:57 行`): 当前 `next_fire_seconds` 硬编码返回 `Some(60)`。接入 `tokio-cron-scheduler`（已在 workspace 依赖中）解析 cron 表达式，计算真实 `next_fire`。
     - `CronTrigger` 新增 `schedule: tokio_cron_scheduler::Schedule` 字段
     - `next_fire_seconds(&self) -> Option<u64>` — 调用 `schedule.upcoming(Utc).next()` 计算实际秒数
     - 保留 `validate_expression()` 额外验证（5 字段检查）作为快速校验层
  2. **Cancel 实现** (`scheduler/memory.rs:272 行`): 当前 `cancel()` 返回 `Err(NotImplemented)`。
     - `InMemoryAgentScheduler` 新增 `abort_handles: Arc<DashMap<String, tokio::task::AbortHandle>>`
     - `execute_steps()` 中：spawn 任务时存储 `AbortHandle`
     - `cancel(run_id)`: 查找 `AbortHandle` → `handle.abort()` → 返回 `Ok(())`；未找到则 `Ok(())`（幂等）。**不修改** Task 4.1 的步进级 timeout。
  3. **expand_conditions 完善** (`scheduler/memory.rs:238-240`): 当前始终取 `then` 分支。新增 `ExecutionContext` 评估：
     - 遍历 `step.conditions`，调用新增的 `evaluate_condition(condition, ctx) -> bool`
     - 条件匹配 → 走 `then`；不匹配 → 走 `else`（如存在）；无条件 → 走 then

  **Must NOT do**:
  - 不修改 Task 4.1 的步进级 timeout 逻辑
  - 不改变 `execute_steps` 的整体 DAG 拓扑排序结构
  - 不引入独立 `cron` crate（使用已有的 tokio-cron-scheduler）

  **Recommended Agent Profile**:
  > Cron 集成 + 异步任务取消 + 条件分支——3 个独立模块，需要 tokio 并发经验。
  - **Category**: `deep`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3（与 5.2, 5.4 并行）
  - **Blocks**: GC2
  - **Blocked By**: GC1, Task 4.1（调度器改动在 4.1 之后）

  **References**:
  - `xz-agent/src/trigger/cron.rs:1-57` — `CronTrigger` 当前状态（`next_fire_seconds` 返回 60s）
  - `xz-agent/src/scheduler/memory.rs:1-50` — `InMemoryAgentScheduler` 结构体（需新增 abort_handles）
  - `xz-agent/src/scheduler/memory.rs:80-120` — `cancel()` 当前 stub 实现
  - `xz-agent/src/scheduler/memory.rs:230-245` — `expand_conditions` 当前实现（仅 then）
  - `xz-agent/src/scheduler/memory.rs:85-88` — "TODO: execute fallback step"
  - `Cargo.toml` workspace deps — `tokio-cron-scheduler = "0.11"` 已声明
  - `xz-agent/src/action/llm.rs` — 注意 4.1 后的状态（已完成编译修复）

  **Acceptance Criteria**:
  - [ ] `CronTrigger::next_fire_seconds("*/5 * * * *")` → `Some(300)`（非 60）
  - [ ] `cancel(run_id)` → 运行终止，状态变为 `Cancelled`
  - [ ] `cancel(nonexistent_id)` → `Ok(())`（幂等）
  - [ ] `expand_conditions` → 条件不匹配时走 `else` 分支
  - [ ] `cargo test -p xz-agent` → all PASS

  **QA Scenarios**:
  ```
  Scenario: Cron 表达式产生正确 next_fire
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-agent cron_next_fire -- --nocapture
    Expected Result: test cron_next_fire ... ok（*/5 → 300s，非 60s）
    Failure Indicators: 仍返回 60s
    Evidence: .sisyphus/evidence/task-5.1-cron.txt

  Scenario: Cancel 中止运行 + 幂等取消
    Tool: Bash
    Steps:
      1. cargo test -p xz-agent cancel_running_agent -- --nocapture
      2. cargo test -p xz-agent cancel_idempotent -- --nocapture
    Expected Result: cancel 后状态=Cancelled；重复 cancel 返回 Ok(())
    Evidence: .sisyphus/evidence/task-5.1-cancel.txt

  Scenario: Condition else 分支工作
    Tool: Bash
    Steps:
      1. cargo test -p xz-agent condition_else_branch -- --nocapture
    Expected Result: test condition_else_branch ... ok（false 条件走 else 分支）
    Evidence: .sisyphus/evidence/task-5.1-condition.txt
  ```

  **Commit**: YES
  - Message: `feat(xz-agent): implement cron scheduling, task cancellation, and condition else-branch`
  - Files: `xz-agent/src/trigger/cron.rs`, `xz-agent/src/scheduler/memory.rs`

- [x] 5.2 **Phase 5 — xz-skill WASM 输出提取 + builtin 工具扩展**

  **What to do**:
  1. **WASM 输出提取** (`runtime/wasm.rs:132 行`): 当前仅支持 `()→i32` 和 `()→()` 导出签名，且忽略 `_args` 参数。扩展支持：
     - **内存基础 I/O**: 支持 `(i32, i32) -> i32` 模式——通过 `wasmtime::Memory` 读取/写入线性内存
     - **字符串参数传入**: 调用者将字符串写入 WASM 线性内存，传递指针+长度
     - **字符串结果读取**: 从 WASM 返回的指针+长度读取结果字符串
     - 保留现有 `()→i32` 和 `()→()` 支持（向后兼容）
  2. **Builtin 工具扩展** (`runtime/default.rs:321 行`): 在现有的 6 个工具（echo, now, uuid, json_path, base64_encode, base64_decode）基础上新增 3 个：
     - `search_web` — 骨架实现（接受 query → 返回 mock 结果，标注"需要搜索引擎 API key 配置"）
     - `read_file` — 使用 `std::fs::read_to_string` 读取文件（限制最大 1MB）
     - `exec_command` — 骨架实现（返回 "not implemented in builtin mode, use WASM for code execution"）

  **Must NOT do**:
  - 不添加超过 3 个新内置工具
  - 不实现通用参数编组（仅支持字符串 I/O）
  - 不引入新 crate 依赖

  **Recommended Agent Profile**:
  > WASM 线性内存操作 + 工具扩展——需要 wasmtime API 经验和文件 I/O。
  - **Category**: `deep`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3（与 5.1, 5.4 并行）
  - **Blocks**: GC2
  - **Blocked By**: GC1

  **References**:
  - `xz-skill/src/runtime/wasm.rs:1-132` — 当前 WASM 运行时（仅支持 `()→` 导出）
  - `xz-skill/src/runtime/wasm.rs:80-120` — WASM 执行和返回值提取
  - `xz-skill/src/runtime/default.rs:1-321` — 当前 builtin 工具实现
  - `xz-skill/src/runtime/default.rs:200-230` — `execute_builtin_tool` dispatch
  - `xz-skill/Cargo.toml` — `wasm-runtime` 在 default features 中

  **Acceptance Criteria**:
  - [ ] WASM 支持 `(i32, i32) -> i32` 导出签名（指针+长度 I/O）
  - [ ] `read_file` 工具读取 < 1MB 文件返回内容
  - [ ] `search_web` 和 `exec_command` 骨架存在（返回合理提示）
  - [ ] `cargo test -p xz-skill --all-features` → all PASS
  - [ ] `cargo check -p xz-skill` → exit 0（默认 feature，无 WASM 时编译通过）

  **QA Scenarios**:
  ```
  Scenario: WASM 函数返回值正确提取（内存 I/O）
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-skill wasm_memory_io --features wasm-runtime -- --nocapture
    Expected Result: WASM 模块通过线性内存接收输入、返回输出后被正确读取
    Failure Indicators: 测试 FAIL 或只能返回固定 args
    Evidence: .sisyphus/evidence/task-5.2-wasm.txt

  Scenario: 新 builtin 工具可调用
    Tool: Bash
    Steps:
      1. cargo test -p xz-skill builtin_search_web -- --nocapture
      2. cargo test -p xz-skill builtin_read_file -- --nocapture
      3. cargo test -p xz-skill builtin_exec_command -- --nocapture
    Expected Result: search_web 返回 mock 结果；read_file 读取文件内容；exec_command 返回提示
    Evidence: .sisyphus/evidence/task-5.2-builtins.txt

  Scenario: 无 wasm-runtime feature 时编译通过
    Tool: Bash
    Steps:
      1. cargo check -p xz-skill --no-default-features 2>&1
    Expected Result: exit 0
    Evidence: .sisyphus/evidence/task-5.2-no-wasm.txt
  ```

  **Commit**: YES
  - Message: `feat(xz-skill): implement WASM memory-based I/O and extend builtin tools with search, file, and code`
  - Files: `xz-skill/src/runtime/wasm.rs`, `xz-skill/src/runtime/default.rs`

- [x] GC2 **Gate Check 2 — Phase 5 完成后全 workspace 最终验证**

  **What to do**:
  - `cargo build --workspace --all-features` → exit 0
  - `cargo test --workspace --all-features` → 0 failed
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0
  - `cargo fmt --all -- --check` → exit 0
  - `cargo doc --workspace --all-features --no-deps` → exit 0

  **Must NOT do**: 不修改代码

  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`

  **Parallelization**: **Can Run In Parallel**: NO | **Parallel Group**: Wave 4（串行） | **Blocks**: F1-F4 | **Blocked By**: 5.1, 5.2, 5.4

  **Acceptance Criteria**:
  - [ ] All 5 commands exit 0
  - [ ] 0 test failures across workspace

  **QA Scenarios**:
  ```
  Scenario: 全 workspace 最终门禁五项全部通过
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo build --workspace --all-features 2>&1 | tail -1 → 期望 exit 0
      3. cargo test --workspace --all-features 2>&1 | grep "test result" → 期望 "0 failed"
      4. cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -1 → 期望 exit 0
      5. cargo fmt --all -- --check 2>&1 → 期望 exit 0
      6. cargo doc --workspace --all-features --no-deps 2>&1 | tail -1 → 期望 exit 0
    Expected Result: 五项全部 exit 0，0 个测试失败
    Evidence: .sisyphus/evidence/task-gc2-gate.txt
  ```

  **Commit**: NO（只读验证）

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run test). For each "Must NOT Have": search codebase for forbidden patterns. Check that evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`. Run `cargo fmt --all -- --check`. Run `cargo test --workspace --all-features`. Review all changed files for: `unwrap()`, `expect()`, `as any`, empty catch, commented-out code, console.log equivalents. Check AI slop.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Fmt [PASS/FAIL] | Tests [N pass/N fail] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean `cargo clean`. Execute every Phase 6 TDD test pair (confirm PRE-FIX would have FAILED, POST-FIX PASSES). Execute all Phase 5 functional tests. Run feature matrix: `--all-features`, `--no-default-features`, per-package combinations. Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Feature Combos [N tested] | Evidence [N files] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — nothing missing, nothing extra. Check "Must NOT do" compliance. Detect cross-task contamination (Task N touching Task M's files). Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Wave | Tasks | Commits | Strategy |
|------|-------|---------|----------|
| 1 | 6.2, 6.3, 6.4, 6.5 | 4 | 每个修复独立提交，commit message: `fix(crate): description` |
| 2 | GC1 | 0 | 只读验证 |
| 3 | 5.1, 5.2, 5.4 | 3 | 每个 Stub 功能独立提交，commit message: `feat(crate): description` |
| 4 | GC2 | 0 | 只读验证 |
| FINAL | F1-F4 | 0 | 只读审查 |

**Commit message 格式**: `fix(crate): description` / `feat(crate): description`

---

## Success Criteria

### Verification Commands
```bash
# Phase 6 Gate
cargo build --workspace --all-features && cargo test --workspace --all-features && cargo clippy --workspace --all-targets --all-features -- -D warnings
# Expected: exit code 0

# Phase 5 Gate
cargo build --workspace --all-features && cargo test --workspace --all-features && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo fmt --all -- --check && cargo doc --workspace --all-features --no-deps
# Expected: exit code 0 for all
```

### Final Checklist
- [x] All 4 Phase 6 fixes implemented + TDD tests passing
- [x] All 3 Phase 5 stubs implemented + functional tests passing
- [x] `cargo build --workspace --all-features` → exit 0
- [x] `cargo test --workspace --all-features` → 0 failures
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0
- [x] `cargo fmt --all -- --check` → exit 0
- [x] No unsafe code introduced
- [x] All Must Have present, all Must NOT Have absent
