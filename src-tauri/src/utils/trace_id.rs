use uuid::Uuid;

/// 生成 8 字符的 trace_id（UUID v4 前 8 位）。
/// 统一全项目的 trace_id 生成逻辑。
pub fn generate_trace_id() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}
