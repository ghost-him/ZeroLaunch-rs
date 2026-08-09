/// 生成内置组件翻译 key（`components.<componentId>.<key>`）。
///
/// 设置项 schema 的 label/description/group、组件名/描述、配置动作文本
/// 均以翻译 key 形式声明：前端按 key-or-literal 渲染（命中
/// `src-ui/i18n/locales/*.json` 的 `components` 命名空间则显示译文，
/// 未命中则原样显示）。宏在编译期拼接为 `&'static str`，零运行时分配。
/// 看到 `t_key!(...)` 即表示"此处是翻译 key 而非最终文案"，
/// 语言包缺键时该 key 会原样出现在界面上（提示补翻译）。
///
/// # 示例
/// ```ignore
/// t_key!("appearance-config", "fields.theme.label")
/// // 展开为 "components.appearance-config.fields.theme.label"
/// ```
macro_rules! t_key {
    ($component:literal, $key:literal) => {
        concat!("components.", $component, ".", $key)
    };
}

pub mod config;
pub mod data_source;
pub mod executor;
pub mod keyword_injector;
pub mod keyword_optimizer;
pub mod score_booster;
pub mod search_engine;
pub mod triggerable;
