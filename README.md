# xz-modules — 小竹 AI 基础设施

> Rust crate 集合，为 AI 应用提供统一的 LLM 调用、记忆、检索、知识图谱、调度等基础能力。
> 所有小竹产品的 AI 能力基石，现已开源。

[![CI](https://github.com/xiaozhu-eco/xz-modules/actions/workflows/ci.yml/badge.svg)](https://github.com/xiaozhu-eco/xz-modules/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://rust-lang.org)

## 架构

```
┌──────────────────────────────────────────────────────────┐
│                   小竹产品层                               │
│     Writing · Chat · Memo · Drive · Message ...          │
└────────────────────────┬─────────────────────────────────┘
                         │
┌────────────────────────┴─────────────────────────────────┐
│  ▲ 记忆扩展层    xz-memory · xz-skill · xz-agent          │
│  ▲ 知识层        xz-knowledge-graph · xz-event-graph（规划中）│
│  ▲ 检索增强层    xz-rag                                   │
│  ▲ 索引/搜索层   xz-embed · xz-rerank · xz-search         │
│  ▲ 基础设施层    xz-provider                              │
└──────────────────────────────────────────────────────────┘
```

## 模块

| Crate | 描述 | crates.io | docs.rs |
|-------|------|-----------|---------|
| **xz-provider** | 统一 LLM 服务接口 (OpenAI / Claude / Ollama) | [![crates.io](https://img.shields.io/crates/v/xz-provider.svg)](https://crates.io/crates/xz-provider) | [![docs.rs](https://img.shields.io/docsrs/xz-provider)](https://docs.rs/xz-provider) |
| **xz-embed** | 文本向量嵌入与向量存储抽象层 | [![crates.io](https://img.shields.io/crates/v/xz-embed.svg)](https://crates.io/crates/xz-embed) | [![docs.rs](https://img.shields.io/docsrs/xz-embed)](https://docs.rs/xz-embed) |
| **xz-memory** | 四层记忆架构 (工作→短期→长期→核心) | [![crates.io](https://img.shields.io/crates/v/xz-memory.svg)](https://crates.io/crates/xz-memory) | [![docs.rs](https://img.shields.io/docsrs/xz-memory)](https://docs.rs/xz-memory) |
| **xz-search** | 多引擎聚合搜索 (Tavily / SerpAPI / Jina) | [![crates.io](https://img.shields.io/crates/v/xz-search.svg)](https://crates.io/crates/xz-search) | [![docs.rs](https://img.shields.io/docsrs/xz-search)](https://docs.rs/xz-search) |
| **xz-rerank** | 检索结果重排序 (本地信号 + Cohere / Jina) | [![crates.io](https://img.shields.io/crates/v/xz-rerank.svg)](https://crates.io/crates/xz-rerank) | [![docs.rs](https://img.shields.io/docsrs/xz-rerank)](https://docs.rs/xz-rerank) |
| **xz-rag** | 多通道 RAG 引擎 (BM25 + 向量 + HyDE) | [![crates.io](https://img.shields.io/crates/v/xz-rag.svg)](https://crates.io/crates/xz-rag) | [![docs.rs](https://img.shields.io/docsrs/xz-rag)](https://docs.rs/xz-rag) |
| **xz-knowledge-graph** | 结构化知识图谱引擎 | [![crates.io](https://img.shields.io/crates/v/xz-knowledge-graph.svg)](https://crates.io/crates/xz-knowledge-graph) | [![docs.rs](https://img.shields.io/docsrs/xz-knowledge-graph)](https://docs.rs/xz-knowledge-graph) |
| **xz-skill** | Skill 插件系统 (注册、执行、沙箱) | [![crates.io](https://img.shields.io/crates/v/xz-skill.svg)](https://crates.io/crates/xz-skill) | [![docs.rs](https://img.shields.io/docsrs/xz-skill)](https://docs.rs/xz-skill) |
| **xz-agent** | Agent 任务调度引擎 (DAG + 多触发器) | [![crates.io](https://img.shields.io/crates/v/xz-agent.svg)](https://crates.io/crates/xz-agent) | [![docs.rs](https://img.shields.io/docsrs/xz-agent)](https://docs.rs/xz-agent) |

## 快速开始

```bash
# 添加依赖
cargo add xz-provider xz-memory

# 或者从 GitHub
git clone https://github.com/xiaozhu-eco/xz-modules
cd xz-modules
cargo build --workspace
cargo test --workspace --all-features
```

```rust
use xz_provider::Provider;

let provider = Provider::from_json(r#"{
    "openai": {
        "api_key": "sk-...",
        "models": [{ "name": "gpt-4o", "capabilities": { "context_window": 128000, "max_output_tokens": 4096 } }]
    }
}"#)?;

let response = provider.chat("Hello!").await?;
println!("{}", response.content);
```

## 开发

```bash
# 格式检查
cargo fmt --all -- --check

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 测试
cargo test --workspace --all-features
```

## 许可证

本仓库中所有 crate 采用双许可证：

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

## 相关链接

- 官网: [xiaozhu-tec.cloud](https://xiaozhu-tec.cloud)
- 模块仓库: [xiaozhu-tec.cloud/module-repo](https://xiaozhu-tec.cloud/module-repo)
