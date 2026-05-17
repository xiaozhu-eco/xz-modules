# Task 1: xz-modules 现有约束清单

## 1. CI 强制规则

来源: `.github/workflows/ci.yml`

| Job | 检查项 | 命令 |
|-----|--------|------|
| check | Format | `cargo fmt --all -- --check` |
| check | Lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| test | Test | `cargo test --workspace --all-features` |
| doc | Doc | `cargo doc --workspace --all-features --no-deps` |

环境: `RUSTFLAGS="-D warnings"`, `CARGO_TERM_COLOR=always`

## 2. Workspace 约束

来源: `Cargo.toml`

### Package 元数据 (workspace.package)
- `version = "0.1.4"`
- `edition = "2024"`
- `rust-version = "1.85"`
- `license = "MIT OR Apache-2.0"`
- `repository = "https://github.com/xiaozhu-eco/xz-modules"`

### Lint 规则 (workspace.lints.rust)
- `unsafe_code = "forbid"`

### 统一依赖 (workspace.dependencies)
- 内部 crate 必须同时声明 `version` 和 `path`
- 外部依赖版本集中在 workspace 管理

## 3. 版本不一致问题

### Edition 不统一
| Edition | Crates |
|---------|--------|
| 2024 | xz-notification, xz-provider, xz-tts |
| 2021 | xz-agent, xz-embed, xz-knowledge-graph, xz-memory, xz-rag, xz-rerank, xz-search, xz-skill |

**需要统一**: 8 个 crate 需从 2021 → 2024

### thiserror 版本不统一
| Version | Crates |
|---------|--------|
| 2 | xz-provider |
| 1.0 | xz-agent, xz-embed, xz-knowledge-graph, xz-memory, xz-rag, xz-rerank, xz-search, xz-skill |
| workspace (2) | xz-tts, xz-notification |

**需要统一**: 8 个 crate 需从 1.0 → workspace = true

## 4. LICENSE 覆盖

- ✅ 10/11 crates 有 LICENSE: xz-agent, xz-embed, xz-knowledge-graph, xz-memory, xz-provider, xz-rag, xz-rerank, xz-search, xz-skill, xz-tts
- ❌ 1 crate 缺失: **xz-notification**

## 5. Crate 文档模式

### lib.rs 结构 (以 xz-provider 为例)
- 顶层文档注释描述 crate 用途
- 模块声明 + 公开 re-export
- 使用 `mod X; pub use X::*;` 模式
- 错误类型集中定义（thiserror enum）
- feature-gated 可选模块

### README.md 模式
- 中文标题 + 描述
- 快速开始代码示例
- 功能特性列表
- 安装/使用说明

## 6. 已知 Audit 问题

来源: `.sisyphus/drafts/xz-modules-audit.md`

审计共识别 32 个问题，按严重度分：
- CRITICAL: 6 个（并发死锁、类型不匹配、SSE 解析等）
- HIGH: 8 个
- MEDIUM: 14 个
- LOW/ARCH: 4 个

主要问题类型：async/std::sync 混用、unwrap/expect 滥用、feature gate 遗漏
