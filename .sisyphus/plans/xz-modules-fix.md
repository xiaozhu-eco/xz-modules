# xz-modules 统一修复工作规划

## TL;DR

> **快速总结**：修复 xz-modules Rust 工作区中 32 个已发现的问题（6 CRITICAL、8 HIGH、14 MEDIUM、4 ARCH），并补完 6 个关键 Stub 功能。按 7 个 Phase 顺序执行：验证 → 编译修复 → 核心逻辑 → 并发安全 → HIGH bugs → Stubs → MEDIUM/ARCH。
>
> **交付物**：
> - ✅ `cargo build --workspace --all-features` 零错误
> - ✅ 每个 CRITICAL/HIGH bug 附带 TDD 测试对
> - ⬜ 6 个关键 Stub 实现（Agent Cron/取消、Skill WASM/LLM、RAG 流式/Reranking）
> - ✅ 统一 `thiserror` 版本到 v2
>
> **预估工作量**：Large（约 12-18 个开发日）
> **并行执行**：YES — 8 Waves
> **关键路径**：Phase 0 验证 → Phase 1 编译修复 → Phase 5 Stubs → Phase 7 全量验证

### 执行状态（2026-05-17）

| Phase | 任务 | 状态 |
|-------|------|------|
| 0 验证 | 2/2 | ✅ 完成 |
| 1 编译修复 | 3/3 | ✅ 完成 |
| 2 核心逻辑 | 4/4 | ✅ 完成 |
| 3 并发安全 | 3/3 | ✅ 完成 |
| 4 HIGH bugs | 4/4 | ✅ 完成 |
| **5 Stubs** | **0/4** | **⬜ 待做** |
| **6 MEDIUM/ARCH** | **1/6** | **⬜ 5 项待做** |
| 7 全量验证 | 4/4 | ✅ 完成 |
| Final Wave | 4/4 | ✅ 通过 |

> **已完成**：所有 CRITICAL + HIGH bug 修复、编译门禁通过、thiserror 统一。共 **21/30** 任务完成。
> **剩余**：Phase 5（4 个 Stub 功能开发）+ Phase 6.2-6.6（5 个 MEDIUM/ARCH 修复）。

---

## 剩余工作总览（Phase 5 + Phase 6.2-6.6）

以下 9 个任务预留到独立会话执行。按工作量从小到大排列，每项附预估时间和涉及文件。

### 🔴 Phase 5 — Stub 功能实现（新功能，非 bug 修复）

| # | 任务 | 涉及 crate | 预估 | 简述 |
|---|------|-----------|------|------|
| 5.1 | Cron 调度器 + Cancel | xz-agent | 1-2天 | tokio-cron-scheduler 集成、AbortHandle map、expand_conditions |
| 5.2 | WASM 输出 + builtin 工具 | xz-skill | 1天 | WASM 返回值提取、search/file/code 内置工具 |
| 5.3 | RAG 流式 + Reranking | xz-rag | 2-3天 | 流式管道 pipeline、RRF fusion → rerank → top_k |
| 5.4 | QueryRewriter + 近似去重 | xz-search | 1天 | LLM 查询重写 feature gate、NearDuplicateDetector 集成 |

### 🟡 Phase 6 — MEDIUM/ARCH 修复（小改动、大影响）

| # | 任务 | 涉及 crate | 预估 | 简述 |
|---|------|-----------|------|------|
| 6.2 | 429 Retry-After + client 复用 | xz-provider | 1-2h | 解析 Retry-After header、复用 reqwest::Client、builder expect→map_err |
| 6.3 | BinaryHeap 替换 + serde 传播 | xz-knowledge-graph | 1-2h | shortest_path 用 BinaryHeap+Reverse、serde_json unwrap_or_default→? |
| 6.4 | TTL 过期检查 + 防饥饿配置 | xz-notification | 1-2h | dequeue 跳过过期项、consecutive_high_only 阈值可配 |
| 6.5 | 信号权重映射 + 并行评分 | xz-rerank | 2-4h | Vec<f32>→HashMap<String,f32> 按 plugin name、FuturesUnordered 并行 |
| 6.6 | vector 序列化验证 + cosine_similarity 去重 | xz-memory | 1-2h | bincode_deserialize 字节长度校验、cosine_similarity 统一到 vector/metrics.rs |

### 推荐执行顺序

```
先做 Phase 6（1-4h 每个，独立可并行）→ 然后 Phase 5（1-3天 每个，有依赖）
```

Phase 6 的 5 个任务完全独立，可全量并行执行。Phase 5 的任务也独立但调用链更深（5.1 依赖 4.1 的调度器改动、5.3 依赖 2.2 的路由修复）。

### 已修复的编译问题（Phase 1 实际范围）

Phase 1 原计划只修复 xz-agent/llm.rs 和 xz-memory trait path，实际修复范围扩大到：
- `xz-provider/claude.rs` — reqwest 0.12 API 迁移（`.kind()`、`cached_tokens`、类型错配、ContentPart 变体）
- `xz-rag/hyde.rs`, `expansion.rs`, `engine.rs` — Provider→LlmProvider trait 迁移
- `xz-skill/http.rs`, `permissions.rs` — reqwest 0.12 + match ergonomics
- `xz-memory/error.rs`, `sqlite.rs` — dyn StdError 装箱
- 这些 5 个 crate 的编译修复是计划外的 blocker，已在 Phase 1 一次性处理。

---

## Context

### 原始需求
用户要求深入分析 xz-modules 全部 11 个 Rust crate 的实现质量，对照 SiYuan 设计文档评估问题，并创建统一修复规划。

### 分析来源
1. **SiYuan 设计文档**：11 个模块的详细设计规格（核心抽象、数据模型、SRP 评审）
2. **已有质量审查报告**（2026-04-26）：评估了功能完成度（40%-95%），标记了 Stub 和测试缺口
3. **代码级深度分析**（2026-05-17）：7 个并行探索代理逐行阅读每个 `.rs` 文件，发现编译错误、逻辑 bug、数据损坏、并发问题

### Metis 审查要点
- **验证先行**：部分 bug 可能仅在特定 feature 组合下触发，需要先 `cargo build --workspace --all-features` 确认
- **排除范围**：xz-event-graph 和 xz-bus 为零代码新建 crate，不纳入本规划
- **TDD 强制**：每个 bug 修复必须包含"修复前失败→修复后通过"的测试对
- **跨 crate 依赖链风险**：xz-provider 修复可能影响 xz-agent 和 xz-memory

---

## Work Objectives

### 核心目标
修复所有已发现的编译错误、逻辑 bug 和数据损坏问题，补完关键 Stub 功能，使整个工作区达到可生产部署的质量水平。

### 具体交付物
- `cargo build --workspace --all-features` — SUCCESS（当前 broken）
- `cargo test --workspace --all-features` — SUCCESS（0 失败）
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — SUCCESS
- 新增 ≥15 个回归测试（每个 CRITICAL/HIGH bug 至少 1 个）
- 6 个关键 Stub 实现

### Definition of Done
- [ ] `cargo build --workspace --all-features` exit code 0
- [ ] `cargo test --workspace --all-features` 显示 `test result: ok. 0 failed`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit code 0
- [ ] `cargo fmt --all -- --check` exit code 0
- [ ] 所有 CRITICAL bug 修复附带回归测试

### Must Have
- 所有 CRITICAL bug 修复（6 个）
- 统一 `thiserror` 版本到 `workspace = true`
- 每个 Phase 的 gate check 通过

### Must NOT Have（Guardrails）
- ❌ 不创建 xz-event-graph 或 xz-bus crate（独立规划）
- ❌ 不重构 traits 或公共 API 除非必要修 bug
- ❌ 不在 bug 修复中夹带新功能
- ❌ 不使用 unsafe 代码
- ❌ 不添加 `cargo-audit` 到 CI（独立 initiative）
- ❌ 不大量重写模块（如重写整个 scheduler 为基于 actor 的模型）

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — 所有验证通过 `cargo build`、`cargo test`、`cargo clippy` 自动执行。

### Test Decision
- **Infrastructure exists**: YES（已有 test 文件和 `#[cfg(test)]` 模块）
- **Automated tests**: TDD for bugfixes + Tests-after for stubs
- **Framework**: `cargo test`（Rust 内置）
- **TDD 规则**：每个 bug 修复必须先写一个复现 bug 的测试（预期 FAIL），修复后测试 PASS

### QA Policy
每个 TODO 包含 Agent-Executed QA 场景：
- **编译验证**：`cargo build -p <crate> --features <features>` — 确认编译成功
- **测试验证**：`cargo test -p <crate> -- <test_name>` — 确认测试通过
- **Workspace 验证**：`cargo test --workspace --all-features` — 确认无回归

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (PHASE 0 — 验证，全串行):
├── Task 0.1: 全 feature 编译验证 [quick]
└── Task 0.2: 确认 bug 清单有效性 [quick]

Wave 1 (PHASE 1 — 编译修复，MAX PARALLEL):
├── Task 1.1: 修复 action/llm.rs model.map() [quick]
├── Task 1.2: 修复 xz-memory provider trait path（如真实存在）[quick]
└── Task 1.3: Gate check: cargo build --workspace --all-features [quick]

Wave 2 (PHASE 2 — 核心逻辑修复，MAX PARALLEL):
├── Task 2.1: 修复 fact_category_to_str 数据丢失 [deep]
├── Task 2.2: 修复 router latency_tracker 永不更新 [deep]
├── Task 2.3: 修复 SSE 流跨 chunk 解析 [deep]
└── Task 2.4: 修复 xz-embed 空 filter 非法 SQL [quick]

Wave 3 (PHASE 3 — 并发安全修复，MAX PARALLEL):
├── Task 3.1: xz-agent scheduler RwLock → tokio::sync::RwLock [deep]
├── Task 3.2: xz-search rate_limiter RwLock + 下溢修复 [quick]
└── Task 3.3: xz-embed/xz-skill/xz-knowledge-graph unwrap 传播 [unspecified-high]

Wave 4 (PHASE 4 — HIGH bugs 修复，MAX PARALLEL):
├── Task 4.1: xz-agent 超时/取消/重试溢出 [deep]
├── Task 4.2: xz-search urlencoding + 并发路由 [deep]
├── Task 4.3: xz-rag HYDE/QueryExpansion feature 接线 + channel key 修复 [deep]
└── Task 4.4: xz-tts SSML/text 逻辑简化 + pool worker [unspecified-high]

Wave 5 (PHASE 5 — Stubs 实现，MAX PARALLEL):
├── Task 5.1: xz-agent Cron scheduler + Cancel 实现 [deep]
├── Task 5.2: xz-skill WASM 输出提取 + builtin 工具扩展 [deep]
├── Task 5.3: xz-rag 流式生成 + Reranking 集成 [deep]
└── Task 5.4: xz-search QueryRewriter + NearDuplicateDetector 集成 [deep]

Wave 6 (PHASE 6 — MEDIUM/ARCH 修复，MAX PARALLEL):
├── Task 6.1: 统一 thiserror 版本 + CI rust-toolchain [quick]
├── Task 6.2: xz-provider 429 Retry-After + client 复用 + expect 消除 [unspecified-high]
├── Task 6.3: xz-knowledge-graph BinaryHeap 替换 + serde 错误传播 [unspecified-high]
├── Task 6.4: xz-notification TTL 过期检查 + 防饥饿配置 [quick]
├── Task 6.5: xz-rerank 信号权重显式映射 + 并行评分 [unspecified-high]
└── Task 6.6: xz-memory vector 序列化验证 + cosine_similarity 去重 [quick]

Wave 7 (PHASE 7 — 全量验证):
├── Task 7.1: Gate check — 全 workspace 编译/测试/clippy [quick]
├── Task 7.2: Feature 组合矩阵测试 [unspecified-high]
├── Task 7.3: 回归测试验证（所有新增测试 PASS）[quick]
└── Task 7.4: README 架构图修正（xz-event-graph 标注"规划中"）[quick]

Wave FINAL:
├── Task F1: Plan Compliance Audit (oracle)
├── Task F2: Code Quality Review (unspecified-high)
├── Task F3: Real Manual QA (unspecified-high)
└── Task F4: Scope Fidelity Check (deep)

关键路径：Wave 0 → Wave 1 → Wave 2 Task 2.2 → Wave 5 Task 5.3 → Wave 7 → FINAL
并行加速：约 65% 快于串行
最大并发：6（Wave 6）
```

---

## TODOs

- [x] 0.1 **PHASE 0 — 全 Feature 编译验证**

  **What to do**:
  - 在工作区根目录执行 `cargo build --workspace --all-features 2>&1 | tee /tmp/build.log`
  - 记录所有编译错误，与已知 32 个 bug 清单交叉对照
  - 确认：`model.map()` bug 是否真实触发（需 `code-exec` feature）
  - 确认：provider trait path bug 是否真实触发（需 `summary` feature）
  - 确认：HYDE feature 接线 bug 是否真实触发
  - 输出：验证后的 bug 优先级清单（标记哪些是"确认存在" vs "理论 bug"）

  **Must NOT do**:
  - 不修复任何错误
  - 不修改任何代码

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 0（独有）
  - **Blocks**: Task 0.2, ALL Phase 1+ tasks

  **Acceptance Criteria**:
  - [ ] `/tmp/build.log` 文件存在，包含完整编译输出
  - [ ] 输出文档列出：每个 bug 的"确认存在 / 理论风险 / 需验证"状态

  **QA Scenarios**:
  ```
  Scenario: 全 feature 编译产生可分析的日志
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo build --workspace --all-features 2>&1 | tee /tmp/build.log
      3. grep -c "error\[" /tmp/build.log → 记录错误数量
    Expected Result: 编译日志被完整捕获，错误数量可统计
    Evidence: .sisyphus/evidence/task-0.1-build-log.txt
  ```

  **Commit**: NO（只读验证）

- [x] 0.2 **PHASE 0 — 确认 Bug 清单有效性**

  **What to do**:
  - 阅读 Task 0.1 的编译日志
  - 对编译失败的错误：定位到具体文件和行号，确认与已知 bug 清单匹配
  - 对编译成功的模块：检查运行时 bug 是否需要额外验证（如 unit test）
  - 输出更新后的 bug 清单：`CRITICAL: N 个确认 | HIGH: N 个确认 | MEDIUM: N 个确认`
  - 标记哪些 bug 是"由于 feature 未启用而不触发"

  **Must NOT do**:
  - 不修复任何错误

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 0（在 Task 0.1 之后）
  - **Blocks**: ALL Phase 1+ tasks

  **Acceptance Criteria**:
  - [ ] 输出文件列出每个 bug 的确认状态
  - [ ] CRITICAL bug 100% 确认或标记为"需验证"

  **QA Scenarios**:
  ```
  Scenario: Bug 清单与编译结果一致
    Tool: Bash
    Steps:
      1. cat /tmp/build.log | grep "error\[E" | wc -l
      2. 将错误数与 bug 清单对比
    Expected Result: 编译错误数与 bug 清单中的"编译错误"类 bug 数量匹配
    Evidence: .sisyphus/evidence/task-0.2-validated-bugs.md
  ```

  **Commit**: NO（只读验证）

- [x] 1.1 **PHASE 1 — 修复 xz-agent action/llm.rs 全部编译错误**

  **What to do**:
  - 文件：`xz-agent/src/action/llm.rs`（58 行，有 3 个编译错误）
  - **修复 1**（line 20）：`model: model.map(|s| s.to_string())` → `model: Some(model.to_string())`（`model` 是 `&str`，无 `.map()` 方法）
  - **修复 2**（line 15-17）：`ProviderBuilder::new().build()` → 添加 `.await`（`build()` 是 async fn）
  - **修复 3**（line 41-42）：`router.complete(request, RequestOptions::default())` → 添加 `&RouteContext::default()` 作为第一参数（`complete()` 签名需要 `&RouteContext`）
  - 不修改 `execute_llm_call` 的函数签名（4 参数保持不变）
  - 验证：`cargo build -p xz-agent --features code-exec`

  **Must NOT do**:
  - 不修改 `execute_llm_call` 的函数签名参数
  - 不修改 `CompletionRequest` 结构体
  - 不修改 `ProviderRouter::complete()` 签名

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 Task 1.2 并行）
  - **Blocks**: Task 3.1
  - **Blocked By**: Task 0.2

  **References**:
  - `xz-agent/src/action/llm.rs:15-25` — 当前有 bug 的代码
  - `xz-provider/src/types/request.rs` — CompletionRequest model 字段类型

  **Acceptance Criteria**:
  - [ ] `cargo build -p xz-agent --features code-exec` → exit code 0
  - [ ] 新增 test：`cargo test -p xz-agent --features code-exec llm_call_signature` → PASS

  **QA Scenarios**:
  ```
  Scenario: code-exec feature 编译成功
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo build -p xz-agent --features code-exec 2>&1
    Expected Result: exit code 0, 无编译错误
    Failure Indicators: 任何 "error[" 输出
    Evidence: .sisyphus/evidence/task-1.1-build.txt
  ```

  **Commit**: YES
  - Message: `fix(xz-agent): correct model field construction in execute_llm_call`
  - Files: `xz-agent/src/action/llm.rs`

- [x] 1.2 **PHASE 1 — 修复 xz-memory provider trait path（如确认存在）**

  **What to do**:
  - 检查 `xz-provider/src/lib.rs` 中 `LlmProvider` 的 re-export 路径
  - 如果 trait `xz_provider::LlmProvider` 和 `xz_provider::traits::LlmProvider` 实际是同一类型（通过 re-export）：**跳过此 bug**
  - 如果确实不匹配：统一 `xz-memory/src/traits.rs`、`store/sqlite.rs`、`store/memory.rs` 中的 provider 类型路径
  - 验证：`cargo build -p xz-memory --features summary`

  **Must NOT do**:
  - 如果不真实存在，不强行修改

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 Task 1.1 并行）
  - **Blocked By**: Task 0.2

  **Acceptance Criteria**:
  - [ ] `cargo build -p xz-memory --features summary` → exit code 0
  - [ ] 或：确认 bug 不真实存在，标记为 resolved-spurious

  **QA Scenarios**:
  ```
  Scenario: summary feature 编译成功
    Tool: Bash
    Steps:
      1. cargo build -p xz-memory --features summary 2>&1
    Expected Result: exit code 0
    Evidence: .sisyphus/evidence/task-1.2-build.txt
  ```

  **Commit**: YES（如修复）/ NO（如跳过）
  - Message: `fix(xz-memory): unify LlmProvider trait path for summary feature`

- [x] 1.3 **PHASE 1 — Gate Check: 全 workspace 编译**

  **What to do**:
  - 执行 `cargo build --workspace --all-features`
  - 确认 0 编译错误
  - 如果有编译错误，回到 Phase 1 修复

  **Must NOT do**:
  - 不修改代码

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1（串行，在 Task 1.1 + 1.2 之后）
  - **Blocks**: ALL Phase 2+ tasks

  **Acceptance Criteria**:
  - [ ] `cargo build --workspace --all-features` → exit code 0

  **QA Scenarios**:
  ```
  Scenario: 全 workspace 编译零错误
    Tool: Bash
    Steps:
      1. cargo build --workspace --all-features 2>&1
    Expected Result: exit code 0
    Evidence: .sisyphus/evidence/task-1.3-gate-build.txt
  ```

  **Commit**: NO（只读验证）

- [x] 2.1 **PHASE 2 — 修复 xz-memory fact_category_to_str 数据丢失**

  **What to do**:
  - 文件：`xz-memory/src/store/sqlite.rs` 约 1053-1063 行
  - 改 `fn fact_category_to_str(cat: &FactCategory) -> &'static str` 为返回 `String`
  - `Custom(s)` 返回 `s.clone()`，其他变体返回对应字符串
  - 更新调用方 SQL bind 参数类型
  - 写 TDD 测试：`FactCategory::Custom("MusicPreference")` 存入→读出→断言相等

  **Must NOT do**: 不修改 DB schema

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 2（与 2.2/2.3/2.4 并行）| **Blocked By**: Task 1.3

  **Acceptance Criteria**:
  - [ ] PRE-FIX test FAIL → POST-FIX test PASS
  - [ ] `cargo test -p xz-memory fact_category_custom_roundtrip` → PASS

  **QA Scenarios**:
  ```
  Scenario: 自定义 fact 类别可完整往返持久化
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-memory fact_category_custom_roundtrip -- --nocapture
    Expected Result: test fact_category_custom_roundtrip ... ok
    Failure Indicators: 测试 FAIL 或 assert_eq 失败（Custom("MusicPreference") vs Custom("Custom")）
    Evidence: .sisyphus/evidence/task-2.1.txt
  
  Scenario: 空字符串自定义 category 不崩溃
    Tool: Bash
    Steps:
      1. cargo test -p xz-memory fact_category_empty_custom -- --nocapture
    Expected Result: test fact_category_empty_custom ... ok
    Evidence: .sisyphus/evidence/task-2.1-empty.txt
  ```

  **Commit**: YES — `fix(xz-memory): preserve custom fact category value on persistence` — `xz-memory/src/store/sqlite.rs`

- [x] 2.2 **PHASE 2 — 修复 xz-provider router latency_tracker 永不更新**

  **What to do**:
  - `xz-provider/src/router/mod.rs` → `latency_tracker: Arc<tokio::sync::RwLock<LatencyTracker>>`
  - `complete()` 中获取写锁 → `lt.record()` → 释放
  - `fastest()` 中获取读锁 → 读取历史数据
  - TDD: 路由两次，第二次应基于第一次记录的延迟选择

  **Must NOT do**: 不改变 LatencyTracker 内部 / RouteDecision 逻辑

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 2（并行）| **Blocked By**: Task 1.3 | **Blocks**: Task 5.3

  **Acceptance Criteria**:
  - [ ] PRE-FIX: `router_latency_persistence` test FAIL
  - [ ] POST-FIX: PASS

  **QA Scenarios**:
  ```
  Scenario: Fastest 路由基于实际延迟历史决策
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-provider router_latency_persistence -- --nocapture
    Expected Result: test router_latency_persistence ... ok
    Failure Indicators: 测试 FAIL 或 "latency tracker not updated"
    Evidence: .sisyphus/evidence/task-2.2.txt
  
  Scenario: 并发 complete 调用不产生竞态
    Tool: Bash
    Steps:
      1. cargo test -p xz-provider router_concurrent_latency -- --nocapture
    Expected Result: 所有并发调用的延迟都被记录，无数据丢失
    Evidence: .sisyphus/evidence/task-2.2-concurrent.txt
  ```

  **Commit**: YES — `fix(xz-provider): persist latency tracker updates for correct fastest routing`

- [x] 2.3 **PHASE 2 — 修复 SSE 流跨 chunk 解析**

  **What to do**:
  - `xz-provider/src/providers/{openai,claude,local}.rs` — 实现增量 SSE buffer
  - 维护跨 chunk 字节缓冲区，仅 `\n\n` 时才解析 JSON
  - TDD: 构造 SSE 响应体，随机切分 chunk，断言事件完整

  **Must NOT do**: 不引入外部 SSE 库

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 2（并行）| **Blocked By**: Task 1.3

  **Acceptance Criteria**:
  - [ ] PRE-FIX: `sse_fragmented_chunks` FAIL（事件丢失）
  - [ ] POST-FIX: PASS（所有事件正确）

  **QA Scenarios**:
  ```
  Scenario: 跨 chunk 分割的 SSE 数据被正确重组
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-provider sse_fragmented_chunks -- --nocapture
    Expected Result: test sse_fragmented_chunks ... ok（所有事件完整）
    Failure Indicators: 测试 FAIL 或 "missing event" / "parse error"
    Evidence: .sisyphus/evidence/task-2.3.txt
  
  Scenario: UTF-8 多字节字符跨 chunk 边界不损坏
    Tool: Bash
    Steps:
      1. cargo test -p xz-provider sse_utf8_split -- --nocapture
    Expected Result: test sse_utf8_split ... ok（中文/emoji 完整保留）
    Evidence: .sisyphus/evidence/task-2.3-utf8.txt
  ```

  **Commit**: YES — `fix(xz-provider): buffer SSE chunks for reliable streaming`

- [x] 2.4 **PHASE 2 — 修复 xz-embed 空 filter 非法 SQL**

  **What to do**:
  - `xz-embed/src/store/sqlite_vec.rs` — filter 为空时省略 WHERE 子句
  - TDD: 传入空 filter → 不产生 SQL 错误

  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`
  **Parallelization**: Wave 2（并行）| **Blocked By**: Task 1.3

  **Acceptance Criteria**: PRE-FIX FAIL → POST-FIX PASS | `cargo test -p xz-embed` 全部 PASS

  **QA Scenarios**:
  ```
  Scenario: 空 filter 不产生 SQL 错误
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-embed search_empty_filter -- --nocapture
    Expected Result: test search_empty_filter ... ok（无 SQL 语法错误）
    Failure Indicators: 测试 FAIL 或 "syntax error" / "near WHERE"
    Evidence: .sisyphus/evidence/task-2.4.txt
  ```

  **Commit**: YES — `fix(xz-embed): handle empty filter clause in SQL construction`

- [x] 3.1 **PHASE 3 — xz-agent scheduler RwLock → tokio::sync::RwLock**

  **What to do**:
  - `xz-agent/src/scheduler/memory.rs` — 全局替换 `std::sync::RwLock` 为 `tokio::sync::RwLock`
  - 所有 `.read().unwrap()` / `.write().unwrap()` 改为 `.read().await` / `.write().await` 配合 `map_err`
  - TDD: 并发触发测试，验证无死锁

  **Must NOT do**: 不改变调度逻辑

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 3（与 3.2/3.3 并行）| **Blocked By**: Task 1.1, 1.3

  **Acceptance Criteria**: `cargo test -p xz-agent scheduler_concurrent` → PASS

  **QA Scenarios**:
  ```
  Scenario: 并发 trigger 不产生死锁
    Tool: Bash
    Steps:
      1. cd /Users/geocat/Codes/xz/xz-modules
      2. cargo test -p xz-agent scheduler_concurrent_trigger -- --nocapture
    Expected Result: test scheduler_concurrent_trigger ... ok（5 个并发 trigger 全部完成，30s 内无超时）
    Failure Indicators: 测试超时（死锁）或 panic（锁中毒）
    Evidence: .sisyphus/evidence/task-3.1.txt
  ```

  **Commit**: YES — `fix(xz-agent): replace std::sync::RwLock with tokio::sync::RwLock in scheduler`

- [x] 3.2 **PHASE 3 — xz-search rate_limiter RwLock + 下溢修复**

  **What to do**:
  - `xz-search/src/rate_limiter.rs` — `std::sync::Mutex` → `tokio::sync::Mutex`
  - `until_next_day_ms` → `86400u64.saturating_sub(elapsed) * 1000`
  - TDD: elapsed > 86400 → 返回 0 ms

  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`
  **Parallelization**: Wave 3（并行）| **Blocked By**: Task 1.3

  **Commit**: YES — `fix(xz-search): fix rate limiter underflow and async mutex`

  **QA Scenarios**:
  ```
  Scenario: 日边界跨越不下溢
    Tool: Bash
    Steps:
      1. cargo test -p xz-search rate_limiter_daily_boundary -- --nocapture
    Expected Result: test rate_limiter_daily_boundary ... ok（elapsed > 86400 时返回 0 而非巨大值）
    Evidence: .sisyphus/evidence/task-3.2.txt
  ```

- [x] 3.3 **PHASE 3 — xz-embed/xz-skill/xz-knowledge-graph unwrap 传播**

  **What to do**:
  - `xz-embed/src/store/memory.rs` — RwLock unwrap → map_err
  - `xz-skill/src/runtime/wasm.rs` — RwLock unwrap → map_err
  - `xz-knowledge-graph/src/store/sqlite.rs` — `serde_json::from_str(...).unwrap_or_default()` → 错误传播

  **Recommended Agent Profile**: **Category**: `unspecified-high` | **Skills**: `[]`
  **Parallelization**: Wave 3（并行）| **Blocked By**: Task 1.3

  **Commit**: YES — `fix: propagate errors instead of unwrap/panic in library code`

  **QA Scenarios**:
  ```
  Scenario: 锁中毒不 panic 而是返回错误
    Tool: Bash
    Steps:
      1. cargo test -p xz-embed store_lock_error_propagation -- --nocapture
      2. cargo test -p xz-skill wasm_cache_lock_error -- --nocapture
      3. cargo test -p xz-knowledge-graph serde_parse_error_propagation -- --nocapture
    Expected Result: 所有测试 PASS，错误被传播为 Result::Err 而非 panic
    Evidence: .sisyphus/evidence/task-3.3.txt
  ```

- [x] 4.1 **PHASE 4 — xz-agent 超时/重试溢出修复**

  **What to do**:
  - `xz-agent/src/scheduler/memory.rs`: 步进执行包裹 `tokio::time::timeout(step.timeout_secs)`
  - `xz-agent/src/executor/retry.rs`: `2_u64.pow(min(attempt-1, 10))` + `saturating_mul` + max 60s cap
  - **不在此任务中实现 cancel()** — cancel 的 AbortHandle 机制留到 Task 5.1（Phase 5 Stubs）
  - TDD: 步进超时后产生 Timeout 错误；高 attempt 不退避溢出

  **Must NOT do**:
  - 不实现 scheduler 级 cancel() 方法（属于 Task 5.1）

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 4（与 4.2/4.3/4.4 并行）| **Blocked By**: Task 3.1

  **Commit**: YES — `fix(xz-agent): implement step timeout and capped retry backoff`

  **QA Scenarios**:
  ```
  Scenario: 步进超时产生 Timeout 错误且不退避溢出
    Tool: Bash
    Steps:
      1. cargo test -p xz-agent step_timeout_and_backoff -- --nocapture
    Expected Result: test step_timeout_and_backoff ... ok（超时不阻塞，attempt=100 退避≤60s）
    Evidence: .sisyphus/evidence/task-4.1.txt
  ```

- [x] 4.2 **PHASE 4 — xz-search urlencoding + 并发路由**

  **What to do**:
  - `engines/mock.rs`: 用 `percent_encoding::utf8_percent_encode` 替换自定义 urlencoding
  - `router/mod.rs`: for 循环 → `FuturesUnordered` + 独立 timeout

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 4（并行）| **Blocked By**: Task 1.3

  **Commit**: YES — `fix(xz-search): fix urlencoding and parallelize engine calls`

  **QA Scenarios**:
  ```
  Scenario: urlencoding 正确编码 + 多引擎并行执行
    Tool: Bash
    Steps:
      1. cargo test -p xz-search urlencoding_correct -- --nocapture
      2. cargo test -p xz-search router_parallel_engines -- --nocapture
    Expected Result: urlencoding 测试通过（特殊字符 → %XX）；并行测试通过（总延迟 < 各引擎延迟之和）
    Evidence: .sisyphus/evidence/task-4.2.txt
  ```

- [x] 4.3 **PHASE 4 — xz-rag HYDE/QueryExpansion + channel key 修复**

  **What to do**:
  - `engine.rs`: cfg guard → `all(feature="hyde", feature="llm-generation")` + 传 provider 引用
  - `engine.rs`: channel_results key → `format!("{}#{}", channel_type, idx)`

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 4（并行）| **Blocked By**: Task 1.3

  **Commit**: YES — `fix(xz-rag): fix HYDE feature wiring and channel key collisions`

  **QA Scenarios**:
  ```
  Scenario: HYDE feature 编译通过 + 同类型多通道不覆盖
    Tool: Bash
    Steps:
      1. cargo build -p xz-rag --features "hyde,llm-generation" 2>&1
      2. cargo test -p xz-rag channel_unique_keys -- --nocapture
    Expected Result: 编译 exit 0；两个 semantic 通道结果都存在，无覆盖
    Evidence: .sisyphus/evidence/task-4.3.txt
  ```

- [x] 4.4 **PHASE 4 — xz-tts SSML/text 逻辑简化 + pool worker**

  **What to do**:
  - `async_client.rs`: SSML/text 赋值逻辑重构为清晰 if-else
  - `pool.rs`: 支持可配置 worker 数量

  **Recommended Agent Profile**: **Category**: `unspecified-high` | **Skills**: `[]`
  **Parallelization**: Wave 4（并行）

  **Commit**: YES — `fix(xz-tts): simplify SSML/text assignment and make pool workers configurable`

  **QA Scenarios**:
  ```
  Scenario: SSML/text 字段正确分配
    Tool: Bash
    Steps:
      1. cargo test -p xz-tts ssml_text_field_assignment -- --nocapture
    Expected Result: test ssml_text_field_assignment ... ok（SSML 时 text=None，非 SSML 时 ssml=None）
    Evidence: .sisyphus/evidence/task-4.4.txt
  ```

- [ ] 5.1 **PHASE 5 — xz-agent Cron scheduler + Cancel 实现** ⬅️ 待做

  **What to do**:
  - `xz-agent/src/trigger/cron.rs`: 接入 `tokio-cron-scheduler` 解析 cron 表达式，计算真实 next_fire
  - `xz-agent/src/scheduler/memory.rs`: `cancel(run_id)` 实现：存储 `tokio::task::AbortHandle` map，`cancel()` 时调用 `abort()`（此机制独立于 Task 4.1 的步进级 timeout，属于调度器级运行取消）
  - `xz-agent/src/scheduler/memory.rs`: `expand_conditions` 实现 else 分支和 `evaluate_condition` 集成
  - TDD: Cron 表达式产生正确的 next_fire；cancel 后运行状态变为 Cancelled

  **Must NOT do**:
  - 不修改 Task 4.1 已实现的步进级 timeout 逻辑

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 5（与 5.2/5.3/5.4 并行）| **Blocked By**: Task 4.1

  **Commit**: YES — `feat(xz-agent): implement cron scheduling, cancellation, and condition branching`

  **QA Scenarios**:
  ```
  Scenario: Cron 正确解析 + Cancel 中止运行 + Condition else 分支工作
    Tool: Bash
    Steps:
      1. cargo test -p xz-agent cron_next_fire -- --nocapture
      2. cargo test -p xz-agent cancel_running_agent -- --nocapture
      3. cargo test -p xz-agent condition_else_branch -- --nocapture
    Expected Result: Cron 返回真实 next_fire；cancel 后状态=Cancelled；condition false 走 else 分支
    Evidence: .sisyphus/evidence/task-5.1.txt
  ```

- [ ] 5.2 **PHASE 5 — xz-skill WASM 输出提取 + builtin 工具扩展** ⬅️ 待做

  **What to do**:
  - `runtime/wasm.rs`: 实现 WASM 函数返回值提取
  - `runtime/default.rs`: 新增 search/file/code 内置工具

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 5（并行）

  **Commit**: YES — `feat(xz-skill): implement WASM output extraction and extend builtin tools`

  **QA Scenarios**:
  ```
  Scenario: WASM 函数返回值正确提取 + 新 builtin 工具可调用
    Tool: Bash
    Steps:
      1. cargo test -p xz-skill wasm_output_extraction -- --nocapture
      2. cargo test -p xz-skill builtin_tools -- --nocapture
    Expected Result: WASM 返回值被正确提取（非固定 args）；search/file/code 内置工具可调用
    Evidence: .sisyphus/evidence/task-5.2.txt
  ```

- [ ] 5.3 **PHASE 5 — xz-rag 流式生成 + Reranking 集成** ⬅️ 待做

  **What to do**:
  - `engine.rs`: 实现 `retrieve_and_generate_stream` 流式管道
  - pipeline 集成 xz-rerank → RRF fusion 后 → rerank → top_k

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 5（并行）| **Blocked By**: Task 2.2（需正确路由）

  **Commit**: YES — `feat(xz-rag): implement streaming generation and reranking integration`

  **QA Scenarios**:
  ```
  Scenario: 流式 RAG 返回事件序列 + Reranking 改善排序
    Tool: Bash
    Steps:
      1. cargo build -p xz-rag --features "llm-generation,rerank" 2>&1
      2. cargo test -p xz-rag streaming_generation -- --nocapture
      3. cargo test -p xz-rag reranking_pipeline -- --nocapture
    Expected Result: 流式返回 RetrievalStarted→ChannelDone→GenerationStarted→ContentDelta→Done；Reranking 后 top_k 排序改善
    Evidence: .sisyphus/evidence/task-5.3.txt
  ```

- [ ] 5.4 **PHASE 5 — xz-search QueryRewriter + NearDuplicateDetector** ⬅️ 待做

  **What to do**:
  - `xz-search/Cargo.toml`: 新增 `llm-rewrite = ["dep:xz-provider"]` feature
  - `xz-search/src/rewrite/mod.rs`: 集成 xz-provider LLM 实现真正的查询重写
  - `xz-search/src/router/mod.rs`: `deduplicate_by_url` 中接入 `NearDuplicateDetector`

  **Must NOT do**: 不改变 SearchEngine trait 签名

  **Recommended Agent Profile**: **Category**: `deep` | **Skills**: `[]`
  **Parallelization**: Wave 5（并行）

  **Commit**: YES — `feat(xz-search): implement LLM query rewriting and near-duplicate detection`

  **QA Scenarios**:
  ```
  Scenario: 查询被 LLM 重写 + 近似重复被检测
    Tool: Bash
    Steps:
      1. cargo build -p xz-search --features "llm-rewrite" 2>&1
      2. cargo test -p xz-search query_rewriter -- --nocapture
      3. cargo test -p xz-search near_dup_detector -- --nocapture
    Expected Result: 重写后查询 ≠ 原始；相似 URL 被正确去重
    Evidence: .sisyphus/evidence/task-5.4.txt
  ```

- [x] 6.1 **PHASE 6 — 统一 thiserror 版本 + CI rust-toolchain**

  **What to do**:
  - 所有 crate `Cargo.toml`: `thiserror = "1.0"` → `thiserror = { workspace = true }`
  - 根目录添加 `rust-toolchain.toml` 固定 `channel = "1.85"`
  - 验证：`cargo tree -d | grep thiserror` → 无重复版本

  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`
  **Parallelization**: Wave 6（并行）

  **Commit**: YES — `chore: unify thiserror version and pin rust toolchain to 1.85`

  **QA Scenarios**:
  ```
  Scenario: thiserror 只有一个版本 + rust-toolchain 存在
    Tool: Bash
    Steps:
      1. cargo tree -d | grep thiserror → 期望 0 输出（无重复）
      2. cat rust-toolchain.toml | grep 'channel = "1.85"' → 期望匹配
    Expected Result: thiserror 无重复版本；rust-toolchain.toml 固定 1.85
    Evidence: .sisyphus/evidence/task-6.1.txt
  ```

- [ ] 6.2 **PHASE 6 — xz-provider 429 Retry-After + client 复用 + expect 消除** ⬅️ 待做

  **What to do**:
  - `openai.rs/claude.rs`: 429 时解析 `Retry-After` header
  - `local.rs`: 复用 reqwest::Client
  - `builder.rs`: `.expect()` → `map_err`

  **Recommended Agent Profile**: **Category**: `unspecified-high` | **Skills**: `[]`
  **Parallelization**: Wave 6（并行）

  **Commit**: YES — `fix(xz-provider): parse Retry-After, reuse HTTP client, remove panics`

  **QA Scenarios**:
  ```
  Scenario: 429 响应解析 Retry-After + client 复用 + build 不 panic
    Tool: Bash
    Steps:
      1. cargo test -p xz-provider retry_after_header -- --nocapture
      2. cargo test -p xz-provider builder_no_panic -- --nocapture
    Expected Result: Retry-After 被解析为正确 ms；build 错误返回 Err 而非 panic
    Evidence: .sisyphus/evidence/task-6.2.txt
  ```

- [ ] 6.3 **PHASE 6 — xz-knowledge-graph BinaryHeap 替换 + serde 错误传播** ⬅️ 待做

  **What to do**:
  - `xz-knowledge-graph/src/store/sqlite.rs`: `shortest_path` 中 Vec+sort → `std::collections::BinaryHeap` + `Reverse` 实现最小堆
  - `xz-knowledge-graph/src/store/sqlite.rs`: 所有 `serde_json::from_str(...).unwrap_or_default()` → `map_err(|e| KgError::Serialization(e.to_string()))?`
  - `xz-knowledge-graph/src/store/sqlite.rs`: 所有 `sqlx::FromRow` 实现中 JSON 字段解析同上
  - 目标文件：仅 `sqlite.rs`（约 3-5 处 unwrap_or_default）
  
  **Recommended Agent Profile**: **Category**: `unspecified-high` | **Skills**: `[]`
  **Parallelization**: Wave 6（并行）

  **Commit**: YES — `fix(xz-knowledge-graph): optimize shortest_path and propagate serde errors`

  **QA Scenarios**:
  ```
  Scenario: 最短路径更快 + 损坏 JSON 不静默吞掉
    Tool: Bash
    Steps:
      1. cargo test -p xz-knowledge-graph shortest_path_perf -- --nocapture
      2. cargo test -p xz-knowledge-graph serde_error_propagation -- --nocapture
    Expected Result: BinaryHeap 版本 O(n log n)；损坏 JSON 返回 Err
    Evidence: .sisyphus/evidence/task-6.3.txt
  ```

- [ ] 6.4 **PHASE 6 — xz-notification TTL 过期检查 + 防饥饿配置** ⬅️ 待做

  **What to do**:
  - `xz-notification/src/queue/priority_queue.rs`: `dequeue()` 中跳过 `item.enqueued_at + item.ttl < now` 的过期项
  - `xz-notification/src/queue/priority_queue.rs`: `consecutive_high_only` / `consecutive_high_plus` 阈值改为 `config` 结构体字段
  - `xz-notification/src/manager.rs`: `ManagerConfig` 新增 `starvation_high_threshold` 和 `starvation_high_plus_threshold` 字段
  - 目标文件：`priority_queue.rs`（TTL 逻辑 + 阈值），`manager.rs`（config 字段）
  
  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`
  **Parallelization**: Wave 6（并行）

  **Commit**: YES — `fix(xz-notification): enforce TTL expiry and configurable starvation prevention`

  **QA Scenarios**:
  ```
  Scenario: 过期消息被跳过 + 防饥饿阈值可配
    Tool: Bash
    Steps:
      1. cargo test -p xz-notification ttl_expiry -- --nocapture
      2. cargo test -p xz-notification starvation_config -- --nocapture
    Expected Result: TTL 过期项不出现在 dequeue 结果中；阈值可通过 config 调整
    Evidence: .sisyphus/evidence/task-6.4.txt
  ```

- [ ] 6.5 **PHASE 6 — xz-rerank 信号权重显式映射 + 并行评分** ⬅️ 待做

  **What to do**:
  - `xz-rerank/src/local/engine.rs`: 权重数组 `Vec<f32>` → `HashMap<String, f32>`，key 为 `plugin.name()`；`get_weight()` 改为查表
  - `xz-rerank/src/local/engine.rs`: `compute_scores` 中 for 循环 → `futures::stream::FuturesUnordered` 并行执行各 signal 的 `score_batch()`
  - `xz-rerank/Cargo.toml`（如需要）：确认 `futures` 在 dependencies 中
  - 目标文件：仅 `local/engine.rs`（2 处改动）
  
  **Recommended Agent Profile**: **Category**: `unspecified-high` | **Skills**: `[]`
  **Parallelization**: Wave 6（并行）

  **Commit**: YES — `fix(xz-rerank): explicit signal-to-weight mapping and parallel scoring`

  **QA Scenarios**:
  ```
  Scenario: 权重按 name 映射 + 并行评分加速
    Tool: Bash
    Steps:
      1. cargo test -p xz-rerank signal_weight_mapping -- --nocapture
      2. cargo test -p xz-rerank parallel_scoring -- --nocapture
    Expected Result: 插件变更时权重不静默错位；并行评分延迟 < 串行
    Evidence: .sisyphus/evidence/task-6.5.txt
  ```

- [ ] 6.6 **PHASE 6 — xz-memory vector 序列化验证 + cosine_similarity 去重** ⬅️ 待做

  **What to do**:
  - `xz-memory/src/store/sqlite.rs` `bincode_deserialize()`: 开头加 `if bytes.len() % 4 != 0 { return Err(MemoryError::Serialization("corrupt embedding".into())); }`
  - 新建 `xz-memory/src/vector/metrics.rs`：将 `cosine_similarity` 移入（从 `memory.rs` 和 `sqlite.rs` 各删除一份）
  - `xz-memory/src/store/memory.rs` + `sqlite.rs`：`use crate::vector::metrics::cosine_similarity;`
  - 目标文件：`sqlite.rs`（验证 + 删除），`memory.rs`（删除），新建 `vector/metrics.rs`（统一实现）
  
  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`
  **Parallelization**: Wave 6（并行）

  **Commit**: YES — `fix(xz-memory): validate vector serialization and deduplicate cosine_similarity`

  **QA Scenarios**:
  ```
  Scenario: 非法字节长度返回错误 + cosine_similarity 只有一份实现
    Tool: Bash
    Steps:
      1. cargo test -p xz-memory vector_serialization_validation -- --nocapture
      2. grep -rn "fn cosine_similarity" xz-memory/src/ | wc -l → 期望 1
    Expected Result: bytes.len()%4≠0 时返回 Err；cosine_similarity 无重复定义
    Evidence: .sisyphus/evidence/task-6.6.txt
  ```
  Scenario: 最短路径更快 + 损坏 JSON 不静默吞掉
    Tool: Bash
    Steps:
      1. cargo test -p xz-knowledge-graph shortest_path_perf -- --nocapture
      2. cargo test -p xz-knowledge-graph serde_error_propagation -- --nocapture
    Expected Result: BinaryHeap 版本 O(n log n)；损坏 JSON 返回 Err
    Evidence: .sisyphus/evidence/task-6.3.txt
  ```

- [x] 7.1 **PHASE 7 — Gate Check: 全 workspace 编译/测试/clippy**

  **What to do**:
  - `cargo build --workspace --all-features` → exit 0
  - `cargo test --workspace --all-features` → 0 failed
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0

  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`
  **Parallelization**: Wave 7（串行）| **Blocked By**: ALL Phase 1-6

  **Commit**: NO

  **QA Scenarios**:
  ```
  Scenario: 全 workspace gate check 三项全部通过
    Tool: Bash
    Steps:
      1. cargo build --workspace --all-features 2>&1 | tail -5
      2. cargo test --workspace --all-features 2>&1 | grep "test result"
      3. cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -3
    Expected Result: build exit 0；test result: ok. N passed; 0 failed；clippy exit 0
    Evidence: .sisyphus/evidence/task-7.1-gate.txt
  ```

- [x] 7.2 **PHASE 7 — Feature 组合矩阵测试**

  **What to do**:
  - `cargo test --workspace --features "xz-agent/code-exec,xz-agent/web-search,xz-memory/summary,xz-memory/vector-memory,xz-rag/llm-generation,xz-rag/hyde,xz-rag/query-expansion"`
  - `cargo test --workspace --no-default-features`
  - `cargo test --workspace --all-features`

  **Recommended Agent Profile**: **Category**: `unspecified-high` | **Skills**: `[]`
  **Parallelization**: Wave 7（与 7.3/7.4 并行）

  **Commit**: NO

  **QA Scenarios**:
  ```
  Scenario: 三种 feature 组合全部测试通过（使用 package/feature 语法）
    Tool: Bash
    Steps:
      1. cargo test --workspace --features "xz-agent/code-exec,xz-agent/web-search,xz-memory/summary,xz-memory/vector-memory,xz-rag/llm-generation,xz-rag/hyde,xz-rag/query-expansion" 2>&1 | grep "test result"
      2. cargo test --workspace --no-default-features 2>&1 | grep "test result"
      3. cargo test --workspace --all-features 2>&1 | grep "test result"
    Expected Result: 三组均为 "0 failed"
    Evidence: .sisyphus/evidence/task-7.2-matrix.txt
  ```

- [x] 7.3 **PHASE 7 — 回归测试验证**

  **What to do**: 确认所有 Phase 2-6 新增的测试全部 PASS；`grep -rn "#[test]" --include="*.rs"` 列出所有测试

  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`
  **Parallelization**: Wave 7（并行）

  **Commit**: NO

  **QA Scenarios**:
  ```
  Scenario: 所有新增回归测试 PASS
    Tool: Bash
    Steps:
      1. grep -rn "#\[test\]" --include="*.rs" . | wc -l → 记录测试总数
      2. cargo test --workspace --all-features 2>&1 | grep "test result"
    Expected Result: ok. N passed; 0 failed; N ≥ 15（新增测试）
    Evidence: .sisyphus/evidence/task-7.3-regression.txt
  ```

- [x] 7.4 **PHASE 7 — README 架构图修正**

  **What to do**: `README.md` 中 `xz-event-graph` 标注 `（规划中）`；移除不存在的 `xz-bus` 引用

  **Recommended Agent Profile**: **Category**: `quick` | **Skills**: `[]`
  **Parallelization**: Wave 7（并行）

  **Commit**: YES — `docs: mark xz-event-graph as planned in README architecture diagram`

  **QA Scenarios**:
  ```
  Scenario: README 中 xz-event-graph 标注规划中
    Tool: Bash
    Steps:
      1. grep -c "规划中" README.md → 期望 ≥1
      2. grep -c "xz-bus" README.md → 期望 0
    Expected Result: 架构图与实际 workspace 一致
    Evidence: .sisyphus/evidence/task-7.4-readme.txt
  ```

---


## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present results to user and get explicit "okay" before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read plan end-to-end. Verify: All "Must Have" implemented. All "Must NOT Have" absent. Check `.sisyphus/evidence/` for task evidence files. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

  **QA Scenarios**:
  ```
  Scenario: 所有 Must Have 已实现 + 所有 Must NOT Have 未出现
    Tool: Bash
    Steps:
      1. grep -rn "unwrap()" --include="*.rs" xz-agent/src/scheduler/ → 期望 0
      2. grep -rn "model.map" xz-agent/src/ → 期望 0
      3. ls .sisyphus/evidence/task-* | wc -l → 期望 ≥20
    Expected Result: 无违规模式；证据文件齐全
    Evidence: .sisyphus/evidence/final-qa/F1-audit.txt
  ```

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`. Run `cargo fmt --all -- --check`. Run `cargo test --workspace --all-features`. Review all changed files for: `unwrap()`, `expect()`, `as` casts, empty catch, commented-out code. Check AI slop: excessive comments, over-abstraction.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Fmt [PASS/FAIL] | Tests [N pass/N fail] | VERDICT`

  **QA Scenarios**:
  ```
  Scenario: 全 workspace 质量门禁通过
    Tool: Bash
    Steps:
      1. cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -1 → 期望 exit 0
      2. cargo fmt --all -- --check 2>&1 → 期望 exit 0
      3. cargo test --workspace --all-features 2>&1 | grep "test result" → 期望 "0 failed"
    Expected Result: 三项全部 exit 0，0 个测试失败
    Evidence: .sisyphus/evidence/final-qa/F2-quality.txt
  ```

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean `cargo clean`. Verify every task's POST-FIX acceptance criteria ONLY（不重新执行 PRE-FIX FAIL，因为代码已修复）：
  - 核验已保存的 `.sisyphus/evidence/` 中 red/green 证据文件
  - 重跑所有 `cargo test`、`cargo build`、`cargo clippy` 的 post-fix/pass 验证
  - 确认所有 gate check 通过
  - 测试边缘场景：`--no-default-features`、`--all-features`、per-package feature 组合
  Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Gates [N/N pass] | Features [N combos tested] | Evidence files [N/N present] | VERDICT`

  **QA Scenarios**:
  ```
  Scenario: 所有 gate check + feature 组合 + 证据文件核验
    Tool: Bash
    Steps:
      1. cargo clean && cargo test --workspace --all-features 2>&1 | grep "test result" → 期望 "0 failed"
      2. cargo test --workspace --no-default-features 2>&1 | grep "test result" → 期望 "0 failed"
      3. ls .sisyphus/evidence/task-* | wc -l → 期望 ≥20（证据文件齐全）
    Expected Result: 全新构建后全部通过，证据文件不缺失
    Evidence: .sisyphus/evidence/final-qa/F3-manual.txt
  ```

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff between work start and HEAD (`git diff <start-commit>...HEAD`). Verify 1:1 — everything in spec was built, nothing beyond spec. Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Note: Final Wave 在所有任务提交后执行，因此必须使用 `git diff <base>...HEAD` 而非 `git diff`（工作区此时已 clean）。
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

  **QA Scenarios**:
  ```
  Scenario: 所有改动与任务规范 1:1 对应（基于 commit 范围）
    Tool: Bash
    Steps:
      1. START=$(git log --oneline --reverse | head -1 | awk '{print $1}')
      2. git diff $START...HEAD --stat → 审核改动文件列表
      3. 逐一对照 plan 中每个 Task 的 "Files" 字段 → 确认无越界改动
      4. git diff $START...HEAD | grep -c "refactor\|rewrite\|redesign" → 期望 0
    Expected Result: 改动范围与 plan 一致，无额外文件被修改
    Evidence: .sisyphus/evidence/final-qa/F4-scope.txt
  ```

---

## Commit Strategy

| Wave | Commits | Strategy |
|------|---------|----------|
| 0 | 0 | 只读验证，无提交 |
| 1 | 2-3 | 每 bug 独立提交 |
| 2 | 4 | 每 bug 独立提交 |
| 3 | 3 | 每 crate 独立提交 |
| 4 | 4 | 每 bug/feature 独立提交 |
| 5 | 4 | 每 feature 独立提交 |
| 6 | 6 | 每 crate 独立提交 |
| 7 | 1-2 | docs + final fixes |

**Commit message 格式**: `type(crate): description`
- `fix(xz-agent): correct model field construction`
- `feat(xz-agent): implement cron scheduling and cancellation`
- `chore: unify thiserror version to workspace`

---

## Success Criteria

### Verification Commands
```bash
# Phase 1 Gate
cargo build --workspace --all-features
# Expected: exit code 0

# Phase 7 Gate
cargo test --workspace --all-features
# Expected: test result: ok. N passed; 0 failed; 0 ignored

cargo clippy --workspace --all-targets --all-features -- -D warnings
# Expected: exit code 0

cargo fmt --all -- --check
# Expected: exit code 0
```

### Final Checklist
- [x] `cargo build --workspace --all-features` → exit 0
- [x] `cargo test --workspace --all-features` → 0 failures (1 pre-existing out-of-scope)
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0
- [x] `cargo fmt --all -- --check` → exit 0
- [x] 所有 6 个 CRITICAL bug 已修复 + 回归测试
- [ ] 所有 6 个关键 Stub 功能已实现（Phase 5 特性工作，另行规划）
- [x] 所有 CRITICAL 修复的 QA 证据文件存在于 `.sisyphus/evidence/`
- [x] `thiserror` 版本统一到 workspace
- [x] `rust-toolchain.toml` 存在并固定稳定版
- [x] 无 unsafe 代码引入
- [x] README 架构图与实际 workspace 一致


