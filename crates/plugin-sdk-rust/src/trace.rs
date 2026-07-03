use std::future::Future;
use tracing::Instrument;
use zerolaunch_plugin_api::PluginContext;

/// 根据 PluginContext 创建 tracing span。
pub fn span_for(ctx: &PluginContext) -> tracing::Span {
    tracing::info_span!(
        "plugin",
        trace_id = %ctx.trace_id,
        plugin_id = ?ctx.plugin_id,
        query_id = ?ctx.query_id,
    )
}

/// 同步闭包包装：在 span 内执行同步代码。
pub fn with_trace<R>(ctx: &PluginContext, f: impl FnOnce() -> R) -> R {
    let span = span_for(ctx);
    let _enter = span.enter();
    f()
}

/// 异步 Future 包装：在 span 内执行异步代码。
pub fn instrument<F: Future>(ctx: &PluginContext, fut: F) -> tracing::instrument::Instrumented<F> {
    fut.instrument(span_for(ctx))
}
