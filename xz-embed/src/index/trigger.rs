use std::time::Duration;

/// 索引重建触发器
#[derive(Debug, Clone)]
pub enum RebuildTrigger {
    /// 新增 N 条后触发
    Count(usize),
    /// 间隔 N 秒后触发
    Interval(Duration),
    /// 手动触发
    ManualOnly,
}
