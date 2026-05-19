# DIP Fix: 从 xz-provider 移除 xz-auth 依赖

## TL;DR

> **Quick Summary**: 修复 xz-provider 违反依赖倒置原则——将 `LeasedKeySource`（业务层实现）从基础设施 crate 中删除。正确的实现在 `xz-sdk`（同时依赖 xz-provider + xz-auth），无需在 xz-modules 中创建新 crate。
>
> **Deliverables**:
> - xz-provider: 移除 `LeasedKeySource`、`map_auth_error`、xz-auth 依赖
> - DEVELOPMENT.md: 更新文档，说明 `LeasedKeySource` 已迁移到 xz-sdk
>
> **Estimated Effort**: Quick
> **Parallel Execution**: NO — 单任务顺序清理
> **Critical Path**: Task 1 → F1-F4（审核波次）

---

## Context

### 原始诉求
用户发现 `xz-provider` 依赖了 `xz-auth`（外部仓库），违反依赖倒置原则。`xz-provider`（LLM Provider 基础设施）不应该知道 `xz-auth`（认证系统）的存在。

### 问题分析
- **引入提交**: `5cdebd2`（2026-05-19）— `feat(xz-provider): 实现 LeasedKeySource 支持小竹积分租赁`
- **本质**: `LeasedKeySource` 是业务实现（小竹积分租赁 API Key），放在了基础设施 crate
- **现状**: 已有 WIP 改动将 xz-auth 改为 optional feature gate — 治标不治本
- **正确架构**: `xz-sdk` 仓库已有独立的 `LeasedKeySource` 实现，同时依赖 `xz-provider` + `xz-auth`，**完全符合 DIP**

### 依赖方向对比
```
❌ 当前（违反 DIP）                    ✅ 修正后
                                       
 xz-provider ──→ xz-auth                xz-provider (只定义 KeySource trait)
                                       
                                       xz-sdk ──→ xz-provider (impl KeySource)
                                       xz-sdk ──→ xz-auth (lease key)
```

### 关键决策
- **`ProviderError::Auth` 保留** — 被 openai/claude provider 用于 LLM API 返回的 HTTP 401，与 xz-auth 无关
- **`LeasedKeySource` + `map_auth_error` 删除** — 直接从 xz-provider 移除
- **不新建 crate** — `xz-sdk` 已经是业务层的正确归宿，无需重复
- **version bump**: 0.2.0 → 0.3.0（破坏性变更：移除公开类型）

### Metis Review
- xz-sdk 已有正确的 DIP 实现，但使用旧 sync KeySource trait — **不阻塞本计划，后续 issue 跟踪**

---

## Work Objectives

### Core Objective
恢复 xz-provider 为纯抽象层——只定义 `KeySource` trait，删除所有 xz-auth 相关代码。

### Concrete Deliverables
- xz-provider/src/key_source.rs: 删除 `LeasedKeySource` struct、impl、`map_auth_error` 函数
- xz-provider/src/lib.rs: 删除 `#[cfg(feature = "xiaozhu-auth")] pub use` 行
- xz-provider/Cargo.toml: 删除 xz-auth 依赖和 feature flag，bump version 0.3.0
- Cargo.toml（root）: workspace dependency version 同步 bump
- DEVELOPMENT.md: 更新依赖拓扑说明（xz-provider 不再依赖 xz-auth）
- Cargo.lock: 自动更新（移除 xz-auth 相关条目）

### Definition of Done
- [ ] `cargo build --workspace --all-features` 通过
- [ ] `cargo test --workspace --all-features` 通过
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过
- [ ] `cargo doc --workspace --all-features --no-deps` 通过
- [ ] `grep -r "xz_auth" xz-provider/src/` 返回空（doc 注释中提及除外）
- [ ] `grep "xz-auth" xz-provider/Cargo.toml` 返回空
- [ ] `grep "xz-auth" Cargo.toml` 返回空（root workspace 级别）

### Must Have
- xz-provider 不依赖 xz-auth（连 optional 也不行）
- `KeySource` trait 签名不变
- `ProviderError::Auth` 保留不动
- 现有测试全部通过

### Must NOT Have (Guardrails)
- **不修改** openai.rs / claude.rs 中对 `ProviderError::Auth` 的使用
- **不修改** xz-sdk 仓库（不同仓库，本次 scope 仅 xz-modules）
- **不修改** `KeySource` trait 签名
- **不新建** crate
- **不做** 超出清理范围的"顺便重构"

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES（`cargo test`）
- **Automated tests**: Tests-after（改完后跑全量）
- **Framework**: `cargo test`

### QA Policy
- 编译通过（cargo build）、测试通过（cargo test）、lint 通过（cargo clippy）
- 静态检查：grep 验证 xz-auth 残留清除

---

## Execution Strategy

### 单任务执行
本次变更范围集中、文件少（4 个文件），所有改动在一个任务中完成。

```
Task 1: 从 xz-provider 清理 xz-auth 相关一切 [quick]

Wave FINAL (4 parallel reviews):
├── Task F1: Plan Compliance Audit (oracle)
├── Task F2: Code Quality Review (unspecified-high)
├── Task F3: Real Manual QA (unspecified-high)
└── Task F4: Scope Fidelity Check (deep)
```

---

## TODOs

- [x] 1. 从 xz-provider 移除 xz-auth 依赖及 LeasedKeySource

  **What to do**:
  1. **xz-provider/Cargo.toml**:
     - 删除 `xiaozhu-auth = ["xz-auth-client", "xz-auth-core"]` feature 行
     - 删除 `xz-auth-client = { path = "../../xz-auth/crates/xz-auth-client", optional = true }` 行
     - 删除 `xz-auth-core = { path = "../../xz-auth/crates/xz-auth-core", optional = true }` 行
     - 改 `version = "0.2.0"` → `version = "0.3.0"`
  2. **root Cargo.toml**（workspace 级别）:
     - 改 `xz-provider = { version = "0.2.0", path = "./xz-provider" }` → `version = "0.3.0"`
  3. **xz-provider/src/key_source.rs**:
     - 删除所有 `#[cfg(feature = "xiaozhu-auth")]` 块（4 处：use imports、struct、impl LeasedKeySource、impl KeySource for LeasedKeySource、map_auth_error）
     - 删除 `use std::sync::Arc;`（仅 xiaozhu-auth 使用）
     - 删除 `use tokio::sync::Mutex;`（仅 xiaozhu-auth 使用）
     - 删除 `use xz_auth_client::{LeasedKey, XiaozhuClient};` 行
     - 保留 `use async_trait::async_trait;`、`use crate::error::ProviderError;`
     - 保留 `KeySource` trait 定义
     - 保留 `ConfigKeySource`、`UserKeySource` 实现
     - 保留 test 模块
  4. **xz-provider/src/lib.rs**:
     - 删除 `#[cfg(feature = "xiaozhu-auth")]` 行
     - 删除 `pub use key_source::LeasedKeySource;` 行
  5. **DEVELOPMENT.md**:
     - 找到 xz-auth 依赖拓扑描述章节，更新为 "xz-provider 不依赖 xz-auth；LeasedKeySource 已迁移至 xz-sdk"
  5. 运行 `cargo update` 更新 Cargo.lock

  **Must NOT do**:
  - 不修改 `ProviderError::Auth` 变体或任何使用它的代码
  - 不修改 `KeySource` trait 签名
  - 不修改 openai.rs、claude.rs、retry.rs
  - 不新建文件

  **Recommended Agent Profile**:
  > Cleanup task — simple text removal, single concern.
  - **Category**: `quick`
  - **Skills**: []
  - **Reason**: 纯删除操作，无逻辑复杂度

  **Parallelization**:
  - **Can Run In Parallel**: NO（单任务）
  - **Blocks**: F1-F4（审核波次）
  - **Blocked By**: None

  **References**:
  - `xz-provider/src/key_source.rs` — 需删除 `LeasedKeySource` 相关代码块
  - `xz-provider/src/lib.rs:68-69` — 需删除 xiaozhu-auth 条件编译 re-export
  - `xz-provider/Cargo.toml:15,31-32` — 需删除 feature flag 和依赖声明
  - `DEVELOPMENT.md:356-371` — 需更新依赖拓扑文档（"xz-provider depends on xz-auth..." 段落）

  **Acceptance Criteria**:
  - [ ] `cargo build --workspace --all-features` → PASS
  - [ ] `cargo test --workspace --all-features` → PASS（所有已有测试通过）
  - [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` → PASS
  - [ ] `cargo doc --workspace --all-features --no-deps` → PASS

  **QA Scenarios**:

  ```
  Scenario: 编译 + 测试 + lint 全通过（happy path）
    Tool: Bash
    Steps:
      1. cargo build --workspace --all-features
      2. cargo test --workspace --all-features
      3. cargo clippy --workspace --all-targets --all-features -- -D warnings
      4. cargo doc --workspace --all-features --no-deps 2>&1 | grep -i "warning\|error"
    Expected Result: 四步全部成功，无错误无警告
    Evidence: .sisyphus/evidence/task-1-build-test.txt

  Scenario: xz-auth 残留检查（验证清理彻底）
    Tool: Bash
    Steps:
      1. grep -rn "xz_auth" xz-provider/src/ --include="*.rs" | grep -v "//" | grep -v "* " | grep -v "小竹"
      2. grep "xz-auth" xz-provider/Cargo.toml
      3. grep "xz-auth" Cargo.toml
    Expected Result: 三次 grep 均无输出（空）
    Evidence: .sisyphus/evidence/task-1-auth-cleanup.txt

  Scenario: KeySource trait 仍可正常使用（验证抽象层完整）
    Tool: Bash
    Steps:
      1. grep -n "pub trait KeySource" xz-provider/src/key_source.rs
      2. cargo doc --workspace -p xz-provider --no-deps 2>&1 | grep -c "KeySource"
    Expected Result: trait 定义存在，文档生成正常
    Evidence: .sisyphus/evidence/task-1-keysource-intact.txt
  ```

  **Commit**: YES
  - Message: `fix(xz-provider)!: remove xz-auth dependency and LeasedKeySource (DIP fix)`
  - Files: `Cargo.toml`, `xz-provider/Cargo.toml`, `xz-provider/src/key_source.rs`, `xz-provider/src/lib.rs`, `DEVELOPMENT.md`, `Cargo.lock`

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  验证: `ProviderError::Auth` 保留、`KeySource` trait 不变、xz-auth 依赖已清除、版本号已 bump。
  Output: `Must Have [N/N] | Must NOT Have [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  运行 `cargo clippy --workspace --all-targets --all-features -- -D warnings`。
  检查有无遗漏的 `#[cfg(feature = "xiaozhu-auth")]` 残留。
  Output: `Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  执行所有 QA scenarios（编译、测试、lint、grep 残留检查）。验证 Cargo.lock 中无 xz-auth 条目残留。
  Output: `Scenarios [N/N pass] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  验证只修改了计划中的 5 个文件 + Cargo.lock。确认 openai.rs、claude.rs、retry.rs 未被触碰。确认无新增文件。
  Output: `Tasks [N/N compliant] | Unaccounted [CLEAN/N issues] | VERDICT`

---

## Commit Strategy

- **1**: `fix(xz-provider)!: remove xz-auth dependency and LeasedKeySource (DIP fix)` - Cargo.toml, xz-provider/Cargo.toml, xz-provider/src/key_source.rs, xz-provider/src/lib.rs, DEVELOPMENT.md, Cargo.lock - `cargo test --workspace --all-features`

---

## Success Criteria

### Verification Commands
```bash
# 核心验证
cargo build --workspace --all-features    # Expected: success
cargo test --workspace --all-features      # Expected: all tests pass
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Expected: no warnings

# 残留检查
grep -r "xz_auth" xz-provider/src/ --include="*.rs"   # Expected: no output (except doc mentions)
grep "xz-auth" xz-provider/Cargo.toml                  # Expected: no output
grep "xz-auth" Cargo.toml                              # Expected: no output

# 架构验证
grep "pub trait KeySource" xz-provider/src/key_source.rs  # Expected: trait exists
```

### Final Checklist
- [ ] xz-provider 不再依赖 xz-auth
- [ ] `KeySource` trait 保留完整
- [ ] `ProviderError::Auth` 保持不变
- [ ] 所有 CI 门禁通过（build, test, clippy, doc）
- [ ] `LeasedKeySource` 的官方归宿：xz-sdk
