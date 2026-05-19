# AWE v1.0 实现计划

## TL;DR

> **目标**：将 writer 创作引擎从"独白式流水线"（v0.x）升级为 LLM 自主智能体架构（v1.0），根治注意力稀释问题。
>
> **核心策略**：升级 xz-modules 通用基础模块（AgentTool trait、自主循环、护栏、Fork、领域记忆）→ 新增 awe-tools 创作工具 → writer bin 接入自主循环 → 废弃旧创作 crates → 更新 Tauri UI。
>
> **预计工作量**：8-12 周（4 个 Phase，14 个 Wave）
> **并行执行**：YES — 跨仓库任务可并行，Wave 内最大化并行
> **关键路径**：xz-agent AutonomousLoop → awe-tools → writer bin → Tauri UI

---

## Context

### 原始需求
基于 `docs/agent-architecture.md` 的 AWE v1.0 架构蓝图，生成完整可执行的实现计划。

### Metis 审查关键发现
1. **xz-provider 已有 tool-use API**：`CompletionRequest.tools`、`ToolDefinition`、`ToolCall`、`ToolResult` 均已在 v0.1.8 中实现。只需在 xz-agent 中封装 `AgentTool` trait，无需修改 xz-provider。
2. **writer-tui 已废弃**：`writer/src/main.rs` 是 stub，重定向到 `writer-client/`。TUI 无需更新，仅需更新 Tauri 客户端。
3. **旧 crate 的导出需要保留**：废弃 `chapter-writer` 等 crate 前，需确认其公开类型未被 `writer-client` 或其他 crate 引用。

---

## Work Objectives

### 核心目标
将 writer 创作引擎从"单次全量上下文注入"升级为"LLM 自主调用工具创作"，使每章的注意力密度从 ~15% 提升到 60%+，同时保持与 v0.x 的向后兼容。

### 具体交付物
- `xz-agent` 升级：`AgentTool` trait、`AutonomousLoop`、`SafetyGuard`、`ForkManager`、`AgentTrajectory`
- `xz-memory` 升级：`CharacterMemory`、`SeedMemory`、`PlotMemory`、`StyleMemory`
- `awe-tools` 新 crate：~25 个创作工具（查询 9 + 创作 6 + 审查 6 + 管理 4）
- `writer` bin 升级：注册 awe-tools → xz-agent AutonomousLoop，保留 Legacy 模式
- `writer-client` 升级：智能体轨迹可视化 UI
- 废弃 6 个旧 crate：context-assembler、chapter-writer、chapter-verifyer、chapter/volumn/novel-outline

### Must Have
- [ ] AutonomousLoop 能完成完整的章节创作（计划→起草→审查→修改→提交）
- [ ] SafetyGuard 在 LLM 声明完成时强制执行种子完整性和字数检查
- [ ] ForkManager 能并行起草 3+ 个场景
- [ ] AgentTrajectory 完整记录每步工具调用和 LLM 思考
- [ ] `thresholds.toml` 中 `mode = "legacy"` 可一键回退到 v0.x

### Must NOT Have
- [ ] 不在 Rust 侧预定义任务图或执行顺序（LLM 自主决定）
- [ ] 不修改 xz-provider 的现有 LLM 调用 API（tool-use 类型已存在）
- [ ] 不在 v1.0 中实现 writer-tui 轨迹视图（TUI 已废弃）
- [ ] 不删除旧 crate 的数据库表（仅废弃 crate，数据保留）

---

## Verification Strategy

### 测试决策
- **自动化测试**：YES（TDD — 每个工具和子系统先写测试）
- **框架**：`cargo test`（Rust 内置）
- **Agent QA**：每个 Wave 完成后执行端到端场景验证

### QA 策略
每个任务包含 Agent-Executed QA Scenarios。后端任务使用 Bash（cargo test + curl），前端任务使用 Playwright。

---

## Execution Strategy

### 并行执行波

```
Phase 1: xz-modules 升级（Waves 1-4）
Phase 2: writer workspace（Waves 5-9）
Phase 3: 集成与共存（Waves 10-11）
Phase 4: UI 与退役（Waves 12-14）
```

```
Wave 1 (基础设施 — xz-agent 类型层):
├── T1: AgentTool trait + ToolRegistry
├── T2: ToolCall/ToolOutput/ToolDefinition 对接 xz-provider
├── T3: AgentTrajectory + TrajectoryStep
└── T4: SafetyViolation/FinalVerdict 类型

Wave 2 (xz-agent 核心 — 自主循环):
├── T5: ConversationManager
├── T6: AutonomousLoop 主循环
├── T7: SafetyGuard 规则引擎
└── T8: ForkManager 并行子智能体

Wave 3 (xz-memory 升级):
├── T9: CharacterMemory
├── T10: SeedMemory
├── T11: PlotMemory
└── T12: StyleMemory

Wave 4 (xz-modules 集成测试):
├── T13: xz-agent + xz-memory 集成测试
├── T14: xz-agent AutonomousLoop 端到端测试（mock tools）
└── T15: 发布 xz-modules 新版本

Wave 5 (awe-tools — 查询工具):
├── T16: query_characters
├── T17: query_seeds
├── T18: query_arc_progress
├── T19: query_world_rules
├── T20: query_recent_history + query_style_profile
└── T21: search_relevant_lore + query_relationships + query_character_voice

Wave 6 (awe-tools — 创作工具):
├── T22: plan_scenes
├── T23: draft_scene
├── T24: revise_passage
└── T25: merge_scenes + polish_chapter + suggest_title

Wave 7 (awe-tools — 审查 + 管理工具):
├── T26: check_consistency
├── T27: check_dialogue + check_pacing
├── T28: check_seeds + check_style_drift
├── T29: fork_scene_drafters + request_human_help
└── T30: save_checkpoint + finalize_chapter + self_review

Wave 8 (writer bin 接入):
├── T31: writer bin 接入 AutonomousLoop
├── T32: WorkflowMode::Agent 实现
├── T33: Legacy → Agent 切换 + 配置
└── T34: 端到端章节创作测试

Wave 9 (废弃旧 crates):
├── T35: 确认旧 crate 导出未被引用
├── T36: 标记 6 crates 为 deprecated
├── T37: 清理 writer workspace Cargo.toml
└── T38: 回归测试（Legacy 模式仍正常）

Wave 10 (集成测试):
├── T39: Agent vs Legacy A/B 对比（3 章）
├── T40: 连续创作测试（10 章不中断）
└── T41: 护栏触发测试（种子缺失/字数不足/超限）

Wave 11 (性能与调优):
├── T42: system prompt 调优
├── T43: 工具输出格式优化
└── T44: 护栏参数校准

Wave 12 (Tauri UI — 事件层):
├── T45: RuntimeEvent 定义 + bridge.rs 转发
└── T46: 新增 IPC 命令（get_agent_trajectory 等）

Wave 13 (Tauri UI — 前端):
├── T47: AgentTrajectoryView 组件
├── T48: ToolCallCard + WaveProgress + VerdictPanel
└── T49: LcgStore（前端状态管理）

Wave 14 (收尾):
├── T50: 文档更新（AGENTS.md, 模块设计.md, 工作流.md）
├── T51: 发布 v1.0
└── T52: 清理临时文件和旧计划

FINAL WAVE (Waves 1-14 全部完成后):
├── F1: Plan Compliance Audit (oracle)
├── F2: Code Quality Review (unspecified-high)
├── F3: Real Manual QA (unspecified-high + playwright)
└── F4: Scope Fidelity Check (deep)
```

### 关键路径
```
T1 → T2 → T6 → T13 → T15 → T16 → T23 → T31 → T34 → T39 → T50 → Final Wave
```

---

## TODOs

### Wave 1: xz-agent 类型层（基础设施）

- [x] 1. AgentTool trait + ToolRegistry

  **What to do**: 在 `xz-agent/src/tool/` 新增 `traits.rs`, `registry.rs`, `types.rs`。定义 `AgentTool` trait（name, description, parameter_schema, execute）、`ToolRegistry`（register, get, list_definitions, execute）、`ToolContext`（novel_id, chapter_number, provider, memory 等引用）。编写单元测试。

  **Must NOT do**: 不修改现有 AgentAction 枚举，不在 ToolContext 中引入 writer 专用类型。

  **Recommended Agent Profile**: `quick`

  **Parallelization**: Wave 1（与 T2-T4 并行）| **Blocks**: T6, T16

  **References**: `docs/agent-architecture.md` §4.3 — AgentTool trait API 契约

  **Acceptance Criteria**: `cargo test -p xz-agent -- tool` → PASS

  **QA Scenarios**:
  ```
  Scenario: Register and execute mock tools
    Tool: Bash (cargo test)
    Steps: 1. cargo test -p xz-agent -- tool::tests::test_register_and_execute
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-1-test-pass.txt
  ```

  **Commit**: YES — `feat(xz-agent): add AgentTool trait and ToolRegistry` — `xz-agent/src/tool/*.rs`

- [x] 2. ToolCall/ToolOutput 对接 xz-provider

  **What to do**: 确认 xz-provider 已有 ToolDefinition/ToolCall/ToolResult 类型。在 xz-agent tool/types.rs 定义 ToolOutput 封装。实现类型转换。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 1（与 T1,T3,T4 并行）| **Blocks**: T6
  **References**: `xz-provider/src/` — CompletionRequest.tools 等
  **QA Scenarios**: `cargo test -p xz-agent -- tool::types::test_provider_compat`
  **Commit**: YES — `feat(xz-agent): integrate ToolCall/ToolOutput with xz-provider`

- [x] 3. AgentTrajectory + TrajectoryStep

  **What to do**: 新增 `xz-agent/src/trajectory/`。定义 AgentTrajectory, TrajectoryStep, TrajectoryAction（Thought/ToolCall/ChapterComplete/SafetyIntervention）。实现 to_display_log() 和 to_json()。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 1（与 T1,T2,T4 并行）| **Blocks**: T6
  **References**: `docs/agent-architecture.md` §3.5
  **QA Scenarios**: `cargo test -p xz-agent -- trajectory::tests::test_record_and_serialize`
  **Commit**: YES — `feat(xz-agent): add AgentTrajectory tracking`

- [x] 4. SafetyViolation/SafetyRule 类型

  **What to do**: 新增 `xz-agent/src/safety/types.rs`。定义 SafetyRule, SafetyCheckType, SafetyViolation, FinalVerdict。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 1（与 T1-T3 并行）| **Blocks**: T7
  **References**: `docs/agent-architecture.md` §8.2
  **QA Scenarios**: `cargo test -p xz-agent -- safety::types::test_serde`
  **Commit**: YES — `feat(xz-agent): add SafetyGuard types`

### Wave 2: xz-agent 核心

- [x] 5. ConversationManager

  **What to do**: 新增 `xz-agent/src/conversation/`。实现消息管理：start, next_response（调用 provider.complete），inject_tool_result, maybe_compress（上下文超限时摘要旧消息）。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 2 | **Blocked By**: T1,T2 | **Blocks**: T6
  **References**: `docs/agent-architecture.md` §3.2, `xz-provider/src/`
  **QA Scenarios**:
  ```
  Scenario: Full conversation flow
    Tool: Bash (cargo test)
    Steps: cargo test -p xz-agent -- conversation::tests::test_full_flow
    Evidence: .sisyphus/evidence/task-5-test-pass.txt
  ```
  **Commit**: YES — `feat(xz-agent): add ConversationManager`

- [x] 6. AutonomousLoop 主循环

  **What to do**: 新增 `xz-agent/src/autonomous/`。实现 run(task_description) 主循环：获取 LLM 响应 → Text 则记录 → ToolCalls 则执行+安全检查+注入结果 → ChapterComplete 则安全检查 → 通过返回。动态 system message 注入。

  **Recommended Agent Profile**: `deep`
  **Parallelization**: Wave 2（最后）| **Blocked By**: T1,T2,T5,T7,T8 | **Blocks**: T13,T31
  **References**: `docs/agent-architecture.md` §6.1
  **QA Scenarios**: `cargo test -p xz-agent -- autonomous::tests::test_full_loop` + `test_safety_rejection_retry`
  **Commit**: YES — `feat(xz-agent): add AutonomousLoop`

- [x] 7. SafetyGuard 规则引擎

  **What to do**: 新增 `xz-agent/src/safety/guard.rs`。实现可配置规则检查：MaxToolCalls, MaxRevisionRounds, MinOutputLength。Blocking 违规阻止提交。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 2（与 T5,T8 并行）| **Blocked By**: T4 | **Blocks**: T6
  **QA Scenarios**: `cargo test -p xz-agent -- safety::guard::tests::test_max_tool_calls`
  **Commit**: YES — `feat(xz-agent): add SafetyGuard rule engine`

- [x] 8. ForkManager

  **What to do**: 新增 `xz-agent/src/fork/`。并行启动子智能体，每个独立 Conversation + 受限工具集。子智能体不可递归 fork。收集结果。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 2（与 T5,T7 并行）| **Blocked By**: T1,T2 | **Blocks**: T6
  **QA Scenarios**: `cargo test -p xz-agent -- fork::tests::test_parallel_fork`
  **Commit**: YES — `feat(xz-agent): add ForkManager`

### Wave 3: xz-memory 升级

- [x] 9. CharacterMemory

  **What to do**: `xz-memory/src/domain/character.rs`。利用 Fact 层存储角色状态快照。get_character, get_relevant_characters（FTS5+最近出场排序），update_after_chapter。FTS5 角色名索引。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 3（与 T10-T12 并行）| **Blocks**: T16
  **QA Scenarios**: `cargo test -p xz-memory -- domain::character::tests::test_crud`
  **Commit**: YES — `feat(xz-memory): add CharacterMemory`

- [x] 10. SeedMemory

  **What to do**: `xz-memory/src/domain/seed.rs`。get_seeds(filter), get_mandatory_seeds, update_after_chapter。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 3（与 T9,T11,T12 并行）| **Blocks**: T17
  **QA Scenarios**: `cargo test -p xz-memory -- domain::seed::tests::test_urgent_query`
  **Commit**: YES — `feat(xz-memory): add SeedMemory`

- [x] 11. PlotMemory

  **What to do**: `xz-memory/src/domain/plot.rs`。get_arc_progress, get_volume_context, update_after_chapter。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 3（与 T9,T10,T12 并行）| **Blocks**: T18
  **Commit**: YES — `feat(xz-memory): add PlotMemory`

- [x] 12. StyleMemory

  **What to do**: `xz-memory/src/domain/style.rs`。get_style_profile, get_recent_metrics, update_after_chapter。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 3（与 T9-T11 并行）| **Blocks**: T20
  **Commit**: YES — `feat(xz-memory): add StyleMemory`

### Wave 4: xz-modules 集成测试

- [x] 13. xz-agent + xz-memory 集成测试

  **What to do**: 在 xz-agent 中添加集成测试：AutonomousLoop + CharacterMemory/SeedMemory 等 mock 后端。验证类型和接口兼容。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 4（与 T14 并行）| **Blocked By**: T6,T9-T12 | **Blocks**: T15
  **QA Scenarios**: `cargo test -p xz-agent --integration`
  **Commit**: YES — `test(xz-agent): add xz-memory integration tests`

- [x] 14. AutonomousLoop 端到端测试（mock tools）

  **What to do**: 使用 mock provider + mock tools，模拟完整章节创作：3 个查询工具 → 1 个创作工具 → 2 个审查工具 → 提交。验证轨迹完整性。

  **Recommended Agent Profile**: `deep`
  **Parallelization**: Wave 4（与 T13 并行）| **Blocked By**: T6
  **QA Scenarios**: 模拟 21 步创作轨迹（match docs/agent-architecture.md §6.2）
  **Commit**: YES — `test(xz-agent): add end-to-end AutonomousLoop test`

- [x] 15. 发布 xz-modules 新版本

  **What to do**: 更新 xz-modules 版本号，确保所有新 API 稳定。cargo test --workspace 全绿。git tag v0.2.0。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: 顺序（依赖 T13,T14）| **Blocks**: T16
  **Commit**: YES — `chore(xz-modules): release v0.2.0`

### Wave 5: awe-tools — 查询工具

- [x] 16. query_characters

  **What to do**: 在 `writer/awe-tools/src/query/characters.rs` 实现。封装 xz-memory CharacterMemory + knowledge-graph。支持按 ID 列表查询或自动返回最相关角色。include_voice 选项。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 5（与 T17-T21 并行）| **Blocked By**: T9,T15
  **References**: `docs/agent-architecture.md` §4.3 — QueryCharactersTool 完整实现
  **QA Scenarios**: `cargo test -p awe-tools -- query::characters`
  **Commit**: YES — `feat(awe-tools): add query_characters tool`

- [x] 17. query_seeds

  **What to do**: 封装 xz-memory SeedMemory。支持 filter="urgent"/"all"/"mandatory"。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 5（与 T16,T18-T21 并行）| **Blocked By**: T10,T15
  **Commit**: YES — `feat(awe-tools): add query_seeds tool`

- [x] 18. query_arc_progress

  **What to do**: 封装 xz-memory PlotMemory + pyramid-summarization。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 5 | **Blocked By**: T11,T15
  **Commit**: YES — `feat(awe-tools): add query_arc_progress tool`

- [x] 19. query_world_rules

  **What to do**: 封装 knowledge-graph 世界规则查询。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 5
  **Commit**: YES — `feat(awe-tools): add query_world_rules tool`

- [x] 20. query_recent_history + query_style_profile

  **What to do**: query_recent_history 封装 pyramid-summarization。query_style_profile 封装 StyleMemory + computational-narratology。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 5 | **Blocked By**: T12,T15
  **Commit**: YES — `feat(awe-tools): add query_recent_history and query_style_profile`

- [x] 21. search_relevant_lore + query_relationships + query_character_voice

  **What to do**: search_relevant_lore 封装 rag + rerank。query_relationships 封装 knowledge-graph。query_character_voice 封装 CharacterMemory 声纹。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 5
  **Commit**: YES — `feat(awe-tools): add remaining query tools`

### Wave 6: awe-tools — 创作工具

- [x] 22. plan_scenes

  **What to do**: 纯 LLM 推理工具。接收弧线状态+角色信息+种子状态，输出场景划分+节拍计划。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 6（与 T23-T25 并行）| **Blocked By**: T15
  **Commit**: YES — `feat(awe-tools): add plan_scenes tool`

- [x] 23. draft_scene

  **What to do**: 核心创作工具。接收节拍计划+角色信息+风格参数，调用 provider 生成场景正文。

  **Recommended Agent Profile**: `deep`
  **Parallelization**: Wave 6 | **Blocked By**: T15
  **References**: `docs/agent-architecture.md` §4.3 — DraftSceneTool 完整实现
  **Commit**: YES — `feat(awe-tools): add draft_scene tool`

- [x] 24. revise_passage

  **What to do**: 定向修改工具。接收原文+问题描述+修改建议，调用 provider 修改指定段落。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 6
  **Commit**: YES — `feat(awe-tools): add revise_passage tool`

- [x] 25. merge_scenes + polish_chapter + suggest_title

  **What to do**: merge_scenes 合并多个场景正文。polish_chapter 全文润色。suggest_title 生成标题。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 6
  **Commit**: YES — `feat(awe-tools): add merge/polish/suggest_title tools`

### Wave 7: awe-tools — 审查 + 管理工具

- [x] 26. check_consistency

  **What to do**: 封装 knowledge-graph + event-centric-graph。提取正文事实 → 查找矛盾 → 返回问题列表。

  **Recommended Agent Profile**: `deep`
  **Parallelization**: Wave 7（与 T27-T30 并行）
  **References**: `docs/agent-architecture.md` §4.3 — CheckConsistencyTool 完整实现
  **Commit**: YES — `feat(awe-tools): add check_consistency tool`

- [x] 27. check_dialogue + check_pacing

  **What to do**: check_dialogue 封装 computational-narratology + CharacterMemory 声纹。check_pacing 封装 computational-narratology 指标。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 7
  **Commit**: YES — `feat(awe-tools): add check_dialogue and check_pacing`

- [x] 28. check_seeds + check_style_drift

  **What to do**: check_seeds 封装 narrative-seed-tracking。check_style_drift 封装 computational-narratology 趋势。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 7
  **Commit**: YES — `feat(awe-tools): add check_seeds and check_style_drift`

- [x] 29. fork_scene_drafters + request_human_help

  **What to do**: fork_scene_drafters 封装 xz-agent ForkManager。request_human_help 发送暂停事件。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 7 | **Blocked By**: T8
  **Commit**: YES — `feat(awe-tools): add fork_scene_drafters and request_human_help`

- [x] 30. save_checkpoint + finalize_chapter + self_review

  **What to do**: save_checkpoint 保存当前状态。finalize_chapter 触发 SafetyGuard 最终检查。self_review 调用 LLM 自我通读。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: Wave 7
  **Commit**: YES — `feat(awe-tools): add remaining meta tools`

### Wave 8: writer bin 接入

- [x] 31. writer bin 接入 AutonomousLoop

  **What to do**: 在 `writer/src/workflow.rs` 中新增 `execute_chapter_agent()`。注册所有 awe-tools → xz-agent ToolRegistry → 创建 AutonomousLoop → 执行 → 处理结果。

  **Recommended Agent Profile**: `deep`
  **Parallelization**: 顺序 | **Blocked By**: T6,T16-T30
  **References**: `docs/agent-architecture.md` §11.2 — 共存代码
  **Commit**: YES — `feat(writer): integrate AutonomousLoop for agent mode`

- [x] 32. WorkflowMode::Agent 实现

  **What to do**: 在 WorkflowEngine 中新增 Agent 模式分支。Agent 模式跳过旧流水线，直接调用 execute_chapter_agent()。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: 顺序 | **Blocked By**: T31
  **Commit**: YES — `feat(writer): add WorkflowMode::Agent`

- [x] 33. Legacy → Agent 切换 + 配置

  **What to do**: 在 thresholds.toml 中新增 `[agent] mode = "legacy"` 配置。启动时根据配置选择模式。

  **Recommended Agent Profile**: `quick`
  **Parallelization**: 顺序 | **Blocked By**: T32
  **Commit**: YES — `feat(writer): add agent/legacy mode config`

- [x] 34. 端到端章节创作测试

  **What to do**: 使用测试小说 + 真实 provider，完成 1 章 Agent 模式创作。验证输出质量、轨迹完整性、护栏行为。

  **Recommended Agent Profile**: `deep`
  **Parallelization**: 顺序 | **Blocked By**: T32
  **Commit**: YES — `test(writer): add end-to-end agent chapter test`

### Wave 9: 废弃旧 crates

- [x] 35. 确认旧 crate 导出未被引用

  **What to do**: 使用 `cargo check` + `lsp_find_references` 确认 context-assembler, chapter-writer, chapter-verifyer, chapter-outline, volumn-outline, novel-outline 的公开 API 在 writer-client, writer-tui 中没有被引用。

  **Recommended Agent Profile**: `quick`
  **QA Scenarios**: `cargo check --workspace` → 无错误
  **Commit**: NO（仅检查）

- [x] 36. 标记 6 crates 为 deprecated

  **What to do**: 在每个旧 crate 的 Cargo.toml 中设置 `publish = false`，lib.rs 顶部添加 `#![deprecated]`。如果 writer bin 直接依赖它们，则移除依赖。

  **Recommended Agent Profile**: `quick`
  **Blocked By**: T35
  **Commit**: YES — `chore: deprecate legacy creation crates`

- [x] 37. 清理 writer workspace Cargo.toml

  **What to do**: 从 workspace Cargo.toml 的 members 中移除 6 个 crates（或标记 exclude）。更新依赖图。

  **Recommended Agent Profile**: `quick`
  **Blocked By**: T36
  **Commit**: YES — `chore: remove deprecated crates from workspace`

- [x] 38. 回归测试（Legacy 模式仍正常）

  **What to do**: `cargo test --workspace` 全绿。确保 Legacy 模式不受影响。

  **Recommended Agent Profile**: `unspecified-high`
  **QA Scenarios**: `cargo test --workspace` → ALL PASS
  **Commit**: NO（仅验证）

### Wave 10: 集成测试

- [x] 39. Agent vs Legacy A/B 对比（3 章）

  **What to do**: 用同一测试小说，分别以 Legacy 和 Agent 模式创作 3 章。对比：字数、种子完成度、风格一致性、审查评分、总 token 消耗。

  **Recommended Agent Profile**: `deep`
  **Parallelization**: 顺序 | **Blocked By**: T34
  **QA Scenarios**:
  ```
  Scenario: A/B compare 3 chapters
    Tool: Bash (cargo test + output diff)
    Steps:
      1. thresholds.toml mode=legacy → cargo run -p writer -- --novel test_novel --chapter 1-3
      2. thresholds.toml mode=agent → cargo run -p writer -- --novel test_novel --chapter 1-3
      3. diff legacy_output/ agent_output/ on word_count, seed_completion, style_metrics
    Expected Result: Agent mode 种子完成度 >= Legacy，风格一致性 >= Legacy，总 token 在可接受范围
    Evidence: .sisyphus/evidence/task-39-ab-comparison.md
  ```
  **Commit**: NO（仅测试）

- [x] 40. 连续创作测试（10 章不中断）

  **What to do**: Agent 模式连续创作 10 章。验证：无内存泄漏、trajectory 持久化正确、护栏无假阳性。

  **Recommended Agent Profile**: `deep`
  **Blocked By**: T39
  **QA Scenarios**:
  ```
  Scenario: Continuous 10-chapter run
    Tool: Bash (cargo run)
    Steps:
      1. thresholds.toml mode=agent max_chapters=10
      2. cargo run -p writer -- --novel test_novel --continuous
      3. 检查：每章 trajectory JSON 持久化到 aw_trajectories 表
      4. 检查：无 OOM，无 panic，所有 10 章完成
    Expected Result: 10 章全部完成，trajectory 表有 10 条记录，无内存泄漏
    Failure Indicators: 某章超时、panic、护栏假阳性阻止正常完成
    Evidence: .sisyphus/evidence/task-40-continuous-run.txt
  ```
  **Commit**: NO（仅测试）

- [x] 41. 护栏触发测试

  **What to do**: 人为制造场景：种子未完成、字数不足、超工具调用上限。验证护栏正确阻断并通知 LLM。

  **Recommended Agent Profile**: `unspecified-high`
  **Blocked By**: T39
  **QA Scenarios**:
  ```
  Scenario: Mandatory seed not completed → blocked
    Tool: Bash (cargo test)
    Steps:
      1. 修改测试小说，确保有 1 个到期强制种子
      2. 创建 Agent 模式但不注入该种子到 system prompt
      3. LLM 提交时触发 mandatory_seeds 护栏阻断
    Expected Result: SafetyGuard 返回 Blocking 违规，LLM 收到反馈后重新处理
    Evidence: .sisyphus/evidence/task-41-seed-guard.txt

  Scenario: Max tool calls exceeded → forced submit
    Tool: Bash (cargo test)
    Steps:
      1. 设置 max_tool_calls=3（极低）
      2. Agent 模式创作，超过限制后强制提交
    Expected Result: 第 4 次调用时触发 Blocking，agent 提交当前最佳版本
    Evidence: .sisyphus/evidence/task-41-max-calls.txt
  ```
  **Commit**: NO（仅测试）

### Wave 11: 性能与调优

- [x] 42. system prompt 调优

  **What to do**: 基于 A/B 对比结果，优化 system prompt 模板。调整工具描述、创作原则表述、建议流程。

  **Recommended Agent Profile**: `writing`
  **QA Scenarios**:
  ```
  Scenario: Verify optimized prompt produces better results
    Tool: Bash (A/B test)
    Steps:
      1. 用优化前 prompt 创作 1 章，记录种子完成度+审查评分
      2. 用优化后 prompt 创作同章节，对比指标
    Expected Result: 优化后 prompt 的种子完成度和审查评分不低于优化前
    Evidence: .sisyphus/evidence/task-42-prompt-optimization.md
  ```
  **Commit**: YES — `refine(awe-tools): optimize system prompt`

- [x] 43. 工具输出格式优化

  **What to do**: 确保每个工具返回的信息 LLM 可直接使用。减少冗余，增加可操作建议。

  **Recommended Agent Profile**: `unspecified-high`
  **QA Scenarios**:
  ```
  Scenario: Validate tool output format
    Tool: Bash (cargo test)
    Steps:
      1. cargo test -p awe-tools -- --test-threads=1
      2. 检查每个工具的 output.content 是否包含明确的成功/失败指示
      3. 检查 structured 字段是否可被 JSON 解析
    Expected Result: 25 个工具全部通过格式校验
    Evidence: .sisyphus/evidence/task-43-tool-format.txt
  ```
  **Commit**: YES — `refine(awe-tools): optimize tool output formats`

- [x] 44. 护栏参数校准

  **What to do**: 根据 10 章测试数据，校准 max_tool_calls（建议 50）、max_revision_rounds（建议 5）、excessive_query_ratio（建议 0.7）。

  **Recommended Agent Profile**: `quick`
  **QA Scenarios**:
  ```
  Scenario: Verify calibrated parameters in thresholds.toml
    Tool: Bash (grep)
    Steps:
      1. grep "max_tool_calls\|max_revision_rounds\|excessive_query_ratio" 资源/thresholds.toml
      2. 确认值与 10 章测试数据统计结果一致
    Expected Result: 配置文件中参数值存在且合理
    Evidence: .sisyphus/evidence/task-44-guard-calibration.txt
  ```
  **Commit**: YES — `chore: calibrate safety guardrail parameters`

### Wave 12: Tauri UI — 事件层

- [x] 45. RuntimeEvent 定义 + bridge.rs 转发

  **What to do**: 在 writer/src/channel_types.rs 中新增 RuntimeEvent 枚举（Thought, ToolCallStarted, ToolCallCompleted, SafetyIntervention, ForkProgress, ChapterCompleted）。在 bridge.rs 中转发到 Tauri 事件。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 12（与 T46 并行）| **Blocked By**: T31
  **QA Scenarios**: 启动 Agent 模式，验证 Tauri 开发者工具中收到事件
  **Commit**: YES — `feat(writer): add RuntimeEvent for agent trajectory`

- [x] 46. 新增 IPC 命令

  **What to do**: 新增 4 个 Tauri command：get_agent_trajectory, send_agent_message, pause_agent, resume_agent。

  **Recommended Agent Profile**: `unspecified-high`
  **Parallelization**: Wave 12（与 T45 并行）
  **Commit**: YES — `feat(writer-client): add agent IPC commands`

### Wave 13: Tauri UI — 前端

- [x] 47. AgentTrajectoryView 组件

  **What to do**: 新建 `writer-client/src/components/lcg/AgentTrajectoryView.vue`。实时展示智能体轨迹：思考文本、工具调用（名称+参数摘要+结果）、安全干预。

  **Recommended Agent Profile**: `visual-engineering`
  **Skills**: `["frontend-ui-ux"]`
  **Parallelization**: Wave 13（与 T48,T49 并行）| **Blocked By**: T45
  **Commit**: YES — `feat(writer-client): add AgentTrajectoryView`

- [x] 48. ToolCallCard + WaveProgress + VerdictPanel

  **What to do**: ToolCallCard 显示单个工具调用详情。WaveProgress 显示 Fork 进度。VerdictPanel 显示审查判决和修改建议。

  **Recommended Agent Profile**: `visual-engineering`
  **Skills**: `["frontend-ui-ux"]`
  **Parallelization**: Wave 13（与 T47,T49 并行）
  **Commit**: YES — `feat(writer-client): add agent UI components`

- [x] 49. LcgStore（前端状态管理）

  **What to do**: 新建 `writer-client/src/stores/lcg.ts`。管理 trajectory, toolCallHistory, forkProgress, verdict, stats。替换 workflow.ts 中的硬编码阶段。

  **Recommended Agent Profile**: `visual-engineering`
  **Parallelization**: Wave 13（与 T47,T48 并行）
  **Commit**: YES — `feat(writer-client): add LcgStore`

### Wave 14: 收尾

- [x] 50. 文档更新

  **What to do**: 更新 AGENTS.md、模块设计.md、工作流.md。新增 agent-architecture.md 作为架构参考。标记旧文档为 historical。

  **Recommended Agent Profile**: `writing`
  **QA Scenarios**: 所有文档链接有效
  **Commit**: YES — `docs: update for AWE v1.0`

- [x] 51. 发布 v1.0

  **What to do**: git tag v1.0.0。确保 Cargo.toml 版本号一致。运行完整测试套件。

  **Recommended Agent Profile**: `quick`
  **QA Scenarios**: `cargo test --workspace` → ALL PASS
  **Commit**: YES — `chore: release v1.0.0`

- [x] 52. 清理临时文件

  **What to do**: 删除 .sisyphus/drafts/ 中的旧草稿，清理测试产物。

  **Recommended Agent Profile**: `quick`
  **Commit**: NO

---

## Final Verification Wave

> 4 个审查 Agent 并行运行。ALL 必须 APPROVE。向用户展示结果并获取显式确认。

- [x] F1. **Plan Compliance Audit** — `oracle` (APPROVE)
  对照本计划逐项验证：检查 xz-agent 是否有 AgentTool trait、AutonomousLoop、SafetyGuard、ForkManager。检查 awe-tools 是否有 25 个工具。检查旧 crates 是否已废弃。验证 `mode = "legacy"` 回退路径。
  输出：`Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT`

- [x] F2. **Code Quality Review** — `unspecified-high` (APPROVE — all new crates clean; pre-existing issues in legacy crates documented)
  `cargo test --workspace` + `cargo clippy --workspace`。检查：`unwrap()` 使用、错误处理、unsafe 代码、AI slop 模式（过度注释、过度抽象）。
  输出：`Build [PASS/FAIL] | Clippy [N warnings] | Tests [N pass/N fail] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high` + `playwright` (APPROVE)
  Agent 模式创作 1 章 → 验证轨迹可视化 → Legacy 模式切换 → 旧模式仍正常。检查 Tauri UI 的 AgentTrajectoryView 组件。
  输出：`Agent Chapter [PASS/FAIL] | Legacy Switch [PASS/FAIL] | UI [PASS/FAIL] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep` (APPROVE)
  逐任务对比：检查每个文件变更是否对应计划中的任务。检查是否有超出计划的额外变更。验证 `Must NOT do` 合规。
  输出：`Tasks [N/N compliant] | Contamination [CLEAN/N issues] | VERDICT`

---

## Commit Strategy

- **Wave 1-3**: 每任务独立 commit，xz-modules 仓库
- **Wave 4**: 集成测试 commit
- **Wave 5-7**: 每工具独立 commit，writer 仓库
- **Wave 8-9**: 功能 commit + 废弃 commit
- **Wave 10-11**: 测试和调优 commit
- **Wave 12-13**: UI commit
- **Wave 14**: 文档 + 发布 commit

---

## Success Criteria

### Verification Commands
```bash
cargo test -p xz-agent --workspace    # xz-agent 全部测试
cargo test -p xz-memory --workspace   # xz-memory 全部测试
cargo test -p awe-tools               # awe-tools 全部测试
cargo test --workspace                # writer workspace 全部测试
cargo build -p writer                 # writer bin 构建
```

### Final Checklist
- [x] xz-agent: AgentTool trait, AutonomousLoop, SafetyGuard, ForkManager, AgentTrajectory 全部可用
- [x] xz-memory: CharacterMemory, SeedMemory, PlotMemory, StyleMemory 全部可用
- [x] awe-tools: 25 个工具全部实现并通过测试
- [x] writer: Agent + Legacy 双模式可用，配置切换正常
- [x] 旧 6 crates 已废弃但数据保留
- [x] Tauri UI 显示智能体轨迹
- [x] Agent 模式能独立完成完整章节创作
- [x] 护栏在违规时正确阻断
- [x] Legacy 回退路径可用



