/// 文件存储服务错误类型。
/// 涵盖上传/下载失败、客户端未初始化、路径无效和 IO 错误等场景。
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// 上传文件失败
    #[error("上传失败 ({file}): {reason}")]
    UploadFailed { file: String, reason: String },

    /// 下载文件失败
    #[error("下载失败 ({file}): {reason}")]
    DownloadFailed { file: String, reason: String },

    /// 存储客户端未初始化
    #[error("存储客户端未初始化")]
    ClientNotInitialized,

    /// 路径无效
    #[error("路径无效: {0}")]
    InvalidPath(String),

    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}
