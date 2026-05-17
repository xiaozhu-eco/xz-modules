# xz-modules 开发规范建立

## TL;DR

> **Quick Summary**: 为 xz-modules 仓库建立完整的开发规范体系，通过三份文档（AGENTS.md、CONTRIBUTING.md、DEVELOPMENT.md）定义纳入/排除标准、代码质量标准、贡献流程，确保 xz-modules 作为小竹生态基础设施的代码质量和一致性。
>
> **Deliverables**:
> - `AGENTS.md` — AI 开发助手入口（纯英文，项目概览 + 核心禁令 + 链接）
> - `CONTRIBUTING.md` — 开源社区贡献指南（中英双语，贡献流程、分层规则）
> - `DEVELOPMENT.md` — 内部详细开发规范（中英双语，完整五维标准 + 技术规约）
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 4 waves, max 3 concurrent
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4

---

## Context

### Original Request
用户要求为 xz-modules 仓库制定开发规范。xz-modules 是开源 Rust workspace（11 个 crate，全部发布到 crates.io），作为小竹 AI 生态的基础设施，所有其他产品都依赖这里的子模块。代码修改必须遵守严格规范，其他模块向 xz-modules 提交修改也必须遵守此规范。

### Interview Summary
**Key Discussions**:
- **规范定位**: 描述理想状态（target state），现有 Bug 在 audit plan 中单独追踪
- **文档架构**: AGENTS.md 作为入口 → CONTRIBUTING.md + DEVELOPMENT.md
- **语言策略**: AGENTS.md 纯英文（AI 解析友好），CONTRIBUTING.md 和 DEVELOPMENT.md 中英双语同步
- **核心原则**: 只有可复用的基础能力才应加入 xz-modules，业务逻辑一律排除
- **五维标准**: 复用性、接口、性能、安全、依赖五个维度定义纳入/排除规则
- **规则严格度**: 分层设置 — 核心安全/质量规则 MUST，风格建议 SHOULD（CONTRIBUTING.md），DEVELOPMENT.md 全 MUST
- **治理模型**: 暂不定（后续补充）

**Research Findings**:
- 仓库有 11 个 crate，CI 已强制 fmt/clippy/test/doc
- workspace 已 `forbid(unsafe_code)`
- 存在已知审计问题（32 个 Bug），在 `.sisyphus/drafts/xz-modules-audit.md` 中追踪
- 缺少 rustfmt.toml、clippy.toml、rust-toolchain.toml
- edition 不一致（workspace=2024，部分 crate 仍用 2021）
- thiserror 版本不一致（workspace=2，部分 crate=1.0）
- 每个 crate 有独立 LICENSE 文件，xz-notification 有独立 CI workflow

### Metis Review
**Identified Gaps** (addressed):
- **规范状态定位**: 已确认 — 描述理想状态，现有例外在 audit 中追踪
- **治理模型**: 已确认 — 暂不定，后续补充
- **CONTRIBUTING.md 语气**: 已确认 — 分层设置（MUST + SHOULD）
- **AGENTS.md 语言**: 已确认 — 纯英文
- **全局语言**: 已确认 — 中英双语同步（AGENTS.md 英文，其余双语）
- **与 audit 的关系**: 规范描述目标，audit 追踪现有问题修复

---

## Work Objectives

### Core Objective
为 xz-modules 建立完整的开发规范文档体系，明确代码准入标准、开发流程和技术约束。

### Concrete Deliverables
- `AGENTS.md` — AI Agent 开发规范入口（纯英文）
- `CONTRIBUTING.md` — 开源贡献指南（中英双语）
- `DEVELOPMENT.md` — 详细开发规范（中英双语）
- 可选配套文件：PR template、rustfmt.toml 缺失补充

### Definition of Done
- [ ] AGENTS.md 存在且包含完整核心规则
- [ ] CONTRIBUTING.md 存在且包含完整贡献流程
- [ ] DEVELOPMENT.md 存在且包含完整五维标准
- [ ] 三份文档交叉引用正确
- [ ] 所有规则与现有 CI 配置一致（无冲突）
- [ ] 中英文内容语义一致

### Must Have
- 五维纳入/排除标准完整定义
- 代码准入硬性规则（安全、性能、依赖）
- PR 提交流程和审查要求
- 与现有 CI 强制检查对齐

### Must NOT Have (Guardrails)
- 不涉及现有 Bug 修复（audit plan 单独处理）
- 不定义规范治理模型（后续补充）
- 不修改任何 `.rs` 源代码文件
- 不引入与现有 CI 冲突的规则
- AGENTS.md 不包含中文（纯英文，AI 解析友好）

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: N/A (documentation only, no code tests)
- **Automated tests**: None (文档项目)
- **Framework**: N/A

### QA Policy
- **文档完整性**: 通过 grep/read 验证所有必要章节存在
- **交叉引用**: 验证 AGENTS.md → CONTRIBUTING.md → DEVELOPMENT.md 链接正确
- **中英一致性**: 逐段对比中英文版本语义一致
- **CI 对齐**: 验证规范中的规则与现有 CI workflow 无冲突

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (启动 - 研究与准备):
├── Task 1: 收集参考材料与最终确认 [quick]
├── Task 2: 整理开发规范大纲 [quick]
└── Task 3: 创建配套配置文件 [quick]

Wave 2 (核心文档编写 - 可并行):
├── Task 4: 编写 AGENTS.md [quick]
├── Task 5: 编写 CONTRIBUTING.md [writing]
└── Task 6: 编写 DEVELOPMENT.md [writing]

Wave 3 (验证与整合):
├── Task 7: 交叉引用验证 [quick]
├── Task 8: 中英一致性检查 [quick]
└── Task 9: CI 对齐检查 [quick]

Wave FINAL:
├── Task F1: 文档完整性审查 [oracle]
├── Task F2: 中英双语质量审查 [writing]
├── Task F3: 实际可用性验证 [unspecified-high]
└── Task F4: 规范一致性检查 [deep]
```

**Critical Path**: Task 1 → Task 2 → Task 4/5/6 → Task 7/8/9 → F1-F4
**Parallel Speedup**: ~50% (Wave 2 三文档并行编写)
**Max Concurrent**: 3 (Wave 2)

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | - | 2 | 1 |
| 2 | 1 | 4,5,6 | 1 |
| 3 | - | (独立) | 1 |
| 4 | 2 | 7,8,9 | 2 |
| 5 | 2 | 7,8,9 | 2 |
| 6 | 2 | 7,8,9 | 2 |
| 7 | 4,5,6 | F1-F4 | 3 |
| 8 | 4,5,6 | F1-F4 | 3 |
| 9 | 4,5,6 | F1-F4 | 3 |
| F1-F4 | 7,8,9 | - | FINAL |

### Agent Dispatch Summary
- **Wave 1**: 3 tasks — Task 1→`quick`, Task 2→`quick`, Task 3→`quick`
- **Wave 2**: 3 tasks — Task 4→`quick`, Task 5→`writing`, Task 6→`writing`
- **Wave 3**: 3 tasks — Task 7-9→`quick`
- **FINAL**: 4 tasks — F1→`oracle`, F2→`writing`, F3→`unspecified-high`, F4→`deep`

---

## TODOs

- [x] 1. **收集参考材料与最终确认**

  **What to do**:
  - 阅读 `.sisyphus/drafts/xz-modules-audit.md` 了解当前已知问题清单
  - 阅读现有 `.github/workflows/ci.yml` 确认 CI 强制规则
  - 阅读 workspace `Cargo.toml` 中 `[workspace.lints]` 和 `[workspace.package]` 确认已有约束
  - 阅读 2-3 个代表性 crate 的 `README.md` 和 `lib.rs` 了解现有文档和 API 模式
  - 确认每个 crate 的 `LICENSE` 文件存在情况
  - 整理出"现有约束清单"作为规范的输入

  **Must NOT do**:
  - 不要修改任何源文件
  - 不要开始写规范文档

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 阅读和整理现有材料，无需复杂逻辑
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3)
  - **Blocks**: Task 2
  - **Blocked By**: None

  **References**:
  - `.sisyphus/drafts/xz-modules-audit.md` — 现有审计问题清单，了解代码已知缺陷
  - `.github/workflows/ci.yml` — CI 强制检查（fmt, clippy, test, doc），规范必须对齐
  - `Cargo.toml:72` — `unsafe_code = "forbid"`，workspace 级安全约束
  - `Cargo.toml:17-25` — workspace.package 元数据（edition, rust-version, license）
  - `xz-provider/README.md` — 代表性 crate 文档模式参考
  - `xz-memory/README.md` — 代表性 crate 文档模式参考

  **Acceptance Criteria**:
  - [ ] 产出"现有约束清单"（markdown 笔记）
  - [ ] 确认所有 11 个 crate 的 LICENSE 文件存在情况
  - [ ] 确认 CI 强制规则与 workspace lints 的完整列表

  **QA Scenarios**:

  ```
  Scenario: 收集材料完整
    Tool: Bash
    Preconditions: 仓库根目录
    Steps:
      1. 读取并统计 audit.md 中的 CRITICAL/HIGH/MEDIUM/LOW 数量
      2. 列出 CI workflow 中的 job 名称（check, test, doc）
      3. 验证 workspace Cargo.toml 中的 lint 规则
      4. 用 `ls xz-*/LICENSE 2>/dev/null | wc -l` 统计有 LICENSE 的 crate 数量
    Expected Result: 所有材料已读取并整理为结构化笔记
    Failure Indicators: 任何文件读取返回空或不存在
    Evidence: .sisyphus/evidence/task-1-research-notes.md
  ```

  **Commit**: YES (groups with Task 2-3)
  - Message: `docs: collect dev-spec reference materials`
  - Files: research notes only

- [x] 2. **整理开发规范大纲**

  **What to do**:
  - 基于 Task 1 的"现有约束清单"，设计三份文档的章节结构
  - 确定每份文档的具体章节和内容分配
  - 设计 AGENTS.md 结构（纯英文）：
    - Project Overview (1-2 paragraphs)
    - Core Constraints (MUST / MUST NOT rules)
    - Quick Reference (links to CONTRIBUTING.md / DEVELOPMENT.md)
  - 设计 CONTRIBUTING.md 结构（中英双语）：
    - How to Contribute / 如何贡献
    - Inclusion Criteria / 纳入标准
    - PR Process / PR 流程
    - Code Style / 代码风格 (SHOULD)
    - Testing Requirements / 测试要求 (MUST)
    - Review Checklist / 审查清单
  - 设计 DEVELOPMENT.md 结构（中英双语）：
    - Architecture Principles / 架构原则
    - Five-Dimension Criteria / 五维标准（复用性、接口、性能、安全、依赖）
    - API Design / API 设计规范
    - Error Handling / 错误处理规范
    - Async Patterns / 异步模式规范
    - Dependency Policy / 依赖策略
    - Versioning & Release / 版本与发版

  **Must NOT do**:
  - 不要填充章节内容（留给 Task 4/5/6）
  - 大纲中不要包含具体规则措辞

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 结构化设计，基于已知输入
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3)
  - **Blocks**: Tasks 4, 5, 6
  - **Blocked By**: Task 1

  **References**:
  - Task 1 产出（现有约束清单）— 大纲的输入
  - `.sisyphus/drafts/dev-spec.md:Proposed Abstract Criteria` — 五维标准定义
  - `README.md:68-79` — 现有开发章节作为参考

  **Acceptance Criteria**:
  - [ ] 产出三份文档的章节大纲
  - [ ] 每份大纲与对应文档定位一致（AGENTS=简洁入口、CONTRIBUTING=社区友好、DEVELOPMENT=详尽技术）
  - [ ] 大纲覆盖所有五维标准
  - [ ] 无章节间重复或冲突

  **QA Scenarios**:

  ```
  Scenario: 大纲完整性
    Tool: Bash (read)
    Preconditions: 大纲已产出
    Steps:
      1. 验证 AGENTS.md 大纲包含 Overview + Core Constraints + Quick Reference
      2. 验证 CONTRIBUTING.md 大纲包含 How to Contribute + Inclusion + PR Process + Review
      3. 验证 DEVELOPMENT.md 大纲包含全部五维标准（5 sections）
      4. 验证三份大纲的总章节数 ≥ 15
    Expected Result: 三份大纲结构完整、定位清晰
    Failure Indicators: 任何一份大纲缺少核心章节
    Evidence: .sisyphus/evidence/task-2-outline.md
  ```

  **Commit**: YES (groups with Task 1, 3)
  - Message: `docs: create dev-spec outline`
  - Files: outline document

- [x] 3. **创建配套配置文件**

  **What to do**:
  - 创建 `rustfmt.toml`（如不存在），基于 Rust 2024 edition 默认格式化规则
  - 创建 `rust-toolchain.toml`，固定 toolchain 为 `1.85`（与 workspace MSRV 一致）
  - 补充 workspace `Cargo.toml` 中 `[workspace.lints.clippy]` 配置（如缺失）
  - 检查并统一所有 crate 的 `edition` 字段为 `2024`
  - 检查并统一所有 crate 的 `thiserror` 版本为 workspace 级别 `2`

  **Must NOT do**:
  - 不要修改 crate 的业务逻辑代码
  - 不要改变 crate 的功能行为
  - 不要修改 `.github/workflows/ci.yml`（除非必要对齐）

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 配置文件创建和版本统一，简单直接
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: None (独立任务)
  - **Blocked By**: None

  **References**:
  - `Cargo.toml:18-19` — `edition = "2024"`, `rust-version = "1.85"`
  - `Cargo.toml:72` — `[workspace.lints.rust] unsafe_code = "forbid"`
  - `Cargo.toml:36` — `thiserror = "2"`（workspace 统一版本）
  - `.github/workflows/ci.yml:24` — `cargo fmt --all -- --check`（需要 rustfmt.toml 对齐）

  **Acceptance Criteria**:
  - [ ] `rustfmt.toml` 存在且包含合理的格式化配置
  - [ ] `rust-toolchain.toml` 存在，`channel = "1.85"`
  - [ ] 所有 crate 的 `edition` 统一为 `2024`
  - [ ] 所有 crate 的 `thiserror` 版本统一为 workspace 级别 `2`
  - [ ] `cargo fmt --all -- --check` 通过
  - [ ] `cargo build --workspace --all-features` 通过

  **QA Scenarios**:

  ```
  Scenario: 配置文件创建和版本统一
    Tool: Bash
    Preconditions: 仓库根目录
    Steps:
      1. `test -f rustfmt.toml && echo "PASS" || echo "FAIL"`
      2. `test -f rust-toolchain.toml && grep "1.85" rust-toolchain.toml && echo "PASS" || echo "FAIL"`
      3. `cargo fmt --all -- --check` → 应通过（exit 0）
      4. `cargo build --workspace --all-features` → 应通过（exit 0）
      5. `grep -r 'edition = "2021"' xz-*/Cargo.toml` → 应无输出（全部已改为 2024）
      6. `grep -r 'thiserror = "1' xz-*/Cargo.toml` → 应无输出（全部改为 workspace = true 或 "2"）
    Expected Result: 配置文件就位，版本统一，构建通过
    Failure Indicators: 任何步骤返回非 0 或 FAIL
    Evidence: .sisyphus/evidence/task-3-config-check.txt
  ```

  **Commit**: YES (groups with Task 1, 2)
  - Message: `chore: add rustfmt.toml, rust-toolchain.toml, unify edition/thiserror`
  - Files: `rustfmt.toml`, `rust-toolchain.toml`, `*/\*/Cargo.toml`
  - Pre-commit: `cargo fmt --all -- --check && cargo build --workspace --all-features`

- [x] 4. **编写 AGENTS.md（纯英文）**
- [x] 5. **编写 CONTRIBUTING.md（中英双语）**
- [x] 6. **编写 DEVELOPMENT.md（中英双语）**

  **What to do**:
  - 基于 Task 2 大纲编写 DEVELOPMENT.md，中英双语（详细技术规范）
  - 结构（每个章节中英双语）：
    1. **Architecture Principles / 架构原则**: 基础设施定位、最小依赖、trait 抽象
    2. **Five-Dimension Criteria / 五维标准**（核心章节）:
       - 复用性 Reusability: 跨产品使用、领域无关
       - 接口 Interface: API 设计、rustdoc、trait 抽象
       - 性能 Performance: benchmark、async-first
       - 安全 Security: unsafe禁令、输入校验、unwrap禁令
       - 依赖 Dependencies: 许可证、活跃度、最小化
       每维度含：具体规则 + 判断标准 + 反面案例
    3. **API Design / API 设计**: 稳定性承诺、Breaking change 定义、命名约定、Error type 模式
    4. **Error Handling / 错误处理**: thiserror 模式、is_retryable、禁止 unwrap/expect
    5. **Async Patterns / 异步模式**: tokio 使用、禁止 std::sync 在 async 上下文
    6. **Dependency Policy / 依赖策略**: 新增审查流程、许可证检查、workspace 管理
    7. **Versioning & Release / 版本与发版**: Semver、CHANGELOG、crates.io 发布层序
    8. **Testing Standards / 测试标准**: Unit/Integration/Doc/Benchmark 要求

  **Must NOT do**:
  - 不要使用 SHOULD/MAY（全部 MUST）
  - 不要省略反面案例
  - 不要与 CONTRIBUTING.md 入口内容重复

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: 详细技术规范，需准确性和完整性
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5)
  - **Blocks**: Tasks 7, 8, 9
  - **Blocked By**: Task 2

  **References**:
  - Task 2 大纲（DEVELOPMENT.md 部分）
  - `Cargo.toml:27-56` — workspace 依赖列表
  - `Cargo.toml:72` — `unsafe_code = "forbid"`
  - `.github/workflows/publish.yml:40-53` — 发布层序
  - `xz-provider/src/lib.rs` — API 设计模式参考

  **Acceptance Criteria**:
  - [ ] `DEVELOPMENT.md` 存在
  - [ ] 中英双语
  - [ ] 五维标准完整（每维 ≥3 条规则）
  - [ ] 包含 API Design、Error Handling、Async Patterns、Dependency Policy、Versioning、Testing 章节
  - [ ] 每个维度含反面案例
  - [ ] unwrap/expect 禁令明确
  - [ ] std::sync 在 async 上下文禁令明确

  **QA Scenarios**:

  ```
  Scenario: DEVELOPMENT.md 完整性
    Tool: Bash (grep)
    Preconditions: DEVELOPMENT.md 已创建
    Steps:
      1. test -f DEVELOPMENT.md && echo "PASS" || echo "FAIL"
      2. grep -c "复用性\|Reusability" DEVELOPMENT.md → ≥ 1
      3. grep -c "接口\|Interface" DEVELOPMENT.md → ≥ 1
      4. grep -c "性能\|Performance" DEVELOPMENT.md → ≥ 1
      5. grep -c "安全\|Security" DEVELOPMENT.md → ≥ 1
      6. grep -c "依赖\|Dependency" DEVELOPMENT.md → ≥ 1
      7. grep "unwrap\|expect" DEVELOPMENT.md → 应有禁令
      8. grep "std::sync" DEVELOPMENT.md → 应有 async 禁令
    Expected Result: 五维标准全覆盖，关键禁令明确
    Failure Indicators: 缺少任何一维、无禁令
    Evidence: .sisyphus/evidence/task-6-development-verification.txt
  ```

  **Commit**: YES (groups with Task 4, 5)
  - Message: `docs: add DEVELOPMENT.md with comprehensive dev specification`
  - Files: `DEVELOPMENT.md`

- [x] 7. **交叉引用验证**
- [x] 8. **中英一致性检查**
- [x] 9. **CI 对齐检查**

  **What to do**:
  - 逐条对比规范中的 MUST 规则与现有 CI workflow 强制检查
  - 确认每条 MUST 规则有对应的 CI 检查路径（或明确说明目前依赖人工审查）
  - 列出规范规则 vs CI 检查的对照表
  - 如有 CI 未覆盖的 MUST 规则，标注为 "人工审查需检查"
  - 如有 CI 检查但规范未提及的，考虑是否需要补充

  **Must NOT do**:
  - 不要修改 CI workflow 文件
  - 不要弱化规范以匹配 CI

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 对比检查，无代码修改
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 7, 8)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 4, 5, 6

  **References**:
  - `.github/workflows/ci.yml` — CI job 定义
  - `.github/workflows/publish.yml` — 发布流程
  - `Cargo.toml:72` — workspace lints
  - `DEVELOPMENT.md` — 完整规则列表

  **Acceptance Criteria**:
  - [ ] 产出 MUST 规则 vs CI 检查对照表
  - [ ] 每条 MUST 规则有 CI 对应或人工审查标注
  - [ ] 无 CI 未覆盖的规则被遗漏

  **QA Scenarios**:

  ```
  Scenario: CI 对齐验证
    Tool: Bash (grep)
    Preconditions: 规范文档和 CI workflow 均存在
    Steps:
      1. DEVELOPMENT.md 中的 fmt 规则 → CI 中有 cargo fmt 检查 → 对齐
      2. DEVELOPMENT.md 中的 clippy 规则 → CI 中有 cargo clippy 检查 → 对齐
      3. DEVELOPMENT.md 中的 test 规则 → CI 中有 cargo test 检查 → 对齐
      4. DEVELOPMENT.md 中的 doc 规则 → CI 中有 cargo doc 检查 → 对齐
      5. DEVELOPMENT.md 中的 async/std::sync 禁令 → CI 中无直接检查 → 标注人工审查
    Expected Result: 对照表完整，无遗漏
    Failure Indicators: 有 MUST 规则在 CI 和人审中均无对应
    Evidence: .sisyphus/evidence/task-9-ci-alignment.txt
  ```

  **Commit**: YES (groups with Task 7, 8)
  - Message: `docs: verify CI alignment for dev-spec rules`
  - Files: `DEVELOPMENT.md` (如有修正)

---

## Final Verification Wave

- [x] F1. **文档完整性审查** — `oracle`
- [x] F2. **中英双语质量审查** — `writing`
- [x] F3. **实际可用性验证** — `unspecified-high`
- [x] F4. **规范一致性检查** — `deep`
  - 对比三份文档：无规则冲突、无矛盾
  - 对比现有 CI 配置：规范的 MUST 规则均有 CI 对应或路径
  - 对比 audit plan：无范围重叠或矛盾
  Output: `文档间一致性 [PASS/FAIL] | CI 对齐 [PASS/FAIL] | Audit 边界 [PASS/FAIL] | VERDICT`

---

## Commit Strategy

- **Wave 1**: `docs: add dev-spec research materials and scaffolding` — research notes + config files
- **Wave 2**: `docs: add AGENTS.md, CONTRIBUTING.md, and DEVELOPMENT.md` — 三份核心文档
- **Wave 3**: `docs: verify cross-references and consistency` — 验证和修正
- **FINAL**: `docs: final dev-spec verification` — 最终审查通过的修正

---

## Success Criteria

### Verification Commands
```bash
# 验证文档存在
ls AGENTS.md CONTRIBUTING.md DEVELOPMENT.md

# 验证 AGENTS.md 无中文（纯英文）
grep -P '[\x{4e00}-\x{9fff}]' AGENTS.md && echo "FAIL: Chinese found" || echo "PASS: English only"

# 验证五维标准全覆盖
grep -c "复用性\|Reusability" DEVELOPMENT.md
grep -c "接口\|Interface" DEVELOPMENT.md
grep -c "性能\|Performance" DEVELOPMENT.md
grep -c "安全\|Security" DEVELOPMENT.md
grep -c "依赖\|Dependency" DEVELOPMENT.md

# 验证交叉引用
grep "CONTRIBUTING.md" AGENTS.md
grep "DEVELOPMENT.md" AGENTS.md
```

### Final Checklist
- [ ] AGENTS.md 纯英文、无中文
- [ ] 五维标准全部有对应章节
- [ ] 所有 MUST 规则有明确判断标准
- [ ] 三文档间无规则冲突
- [ ] 与现有 CI 对齐
- [ ] 中英文版本语义一致
