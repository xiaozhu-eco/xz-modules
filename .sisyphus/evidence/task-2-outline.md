# Task 2: 开发规范大纲

## AGENTS.md 大纲 (纯英文)

| Section | Content |
|---------|---------|
| 1. Project Overview | xz-modules 是什么，在小竹 AI 生态中的基础设施角色（1-2 段） |
| 2. Core Constraints | MUST/MUST NOT 规则，覆盖五维：复用性/接口/性能/安全/依赖（≥8 条） |
| 3. Quick Reference | 链接到 CONTRIBUTING.md 和 DEVELOPMENT.md |
| 4. Development Commands | `cargo build`, `cargo test`, `cargo fmt`, `cargo clippy` |

## CONTRIBUTING.md 大纲 (中英双语)

| Section | Content | Tone |
|---------|---------|------|
| 1. Welcome / 欢迎 | 项目简介 + 贡献者欢迎词 | SHOULD |
| 2. How to Contribute / 如何贡献 | Bug Report / Feature Request / Code Contribution | SHOULD |
| 3. Inclusion Criteria / 纳入标准 | 五维标准摘要 + 排除项列表 + 判断流程 | MUST |
| 4. PR Process / PR 流程 | Fork→Branch→Commit→PR→CI→Review→Merge (≥5 步) | MUST |
| 5. Commit Convention / 提交规范 | `type(scope): desc` conventional commits | MUST |
| 6. Code Style / 代码风格 | cargo fmt, cargo clippy, 命名约定 | SHOULD |
| 7. Testing / 测试 | 新功能必须测试，Bug 修复必须回归测试 | MUST |
| 8. Review Checklist / 审查清单 | 审查者逐项检查清单 | MUST |
| 9. License / 许可证 | MIT OR Apache-2.0 | SHOULD |

## DEVELOPMENT.md 大纲 (中英双语)

| Section | Content | Key Rules Examples |
|---------|---------|-------------------|
| 1. Architecture Principles / 架构原则 | 基础设施定位、最小依赖、trait 优先 | MUST: 业务逻辑不入库 |
| 2. Five-Dimension Criteria / 五维标准 | 每个维度 ≥3 条 MUST 规则 + 判断标准 + 反例 | MUST: 跨产品复用, 禁止 unsafe, 禁止 unwrap |
| 3. API Design / API 设计 | 稳定性承诺、Breaking change 定义、命名约定、Error type 模式 | MUST: 公共 API 必须 rustdoc |
| 4. Error Handling / 错误处理 | thiserror 模式、is_retryable、禁止 unwrap/expect | MUST: 禁止 unwrap/expect |
| 5. Async Patterns / 异步模式 | tokio 使用、禁止 std::sync 在 async 上下文 | MUST: 禁止 std::sync 锁 |
| 6. Dependency Policy / 依赖策略 | 新增审查流程、许可证检查、workspace 管理 | MUST: workspace 统一管理 |
| 7. Versioning & Release / 版本与发版 | Semver、CHANGELOG、crates.io 发布层序 | MUST: 遵循 semver |
| 8. Testing Standards / 测试标准 | Unit/Integration/Doc/Benchmark 要求 | MUST: 公共 API 必有测试 |
