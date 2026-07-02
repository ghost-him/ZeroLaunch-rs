/// HostApi 构建错误类型。
#[derive(Debug, thiserror::Error)]
pub enum HostApiBuildError {
    #[error("missing component: {0}")]
    MissingComponent(&'static str),
}
