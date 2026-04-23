use crate::sdk::storage::storage_error::StorageError;
use async_trait::async_trait;

/// 文件存储服务 trait — 平台原语。
/// 负责将文件存储到指定后端（本地文件系统、WebDAV 等）。
/// 与 ShellExecutor 等其他 SDK trait 平级，由 HostApi 持有。
#[async_trait]
pub trait StorageService: Send + Sync {
    /// 将数据上传到存储后端。
    /// 参数：file_name - 相对文件名；data - 文件内容。
    /// 返回：成功返回 Ok(())，失败返回 StorageError。
    async fn upload(&self, file_name: &str, data: &[u8]) -> Result<(), StorageError>;

    /// 从存储后端下载数据。文件不存在时返回 Ok(None)。
    /// 参数：file_name - 相对文件名。
    /// 返回：文件内容（文件不存在时为 None），失败返回 StorageError。
    async fn download(&self, file_name: &str) -> Result<Option<Vec<u8>>, StorageError>;

    /// 获取存储后端的目标目录路径。
    fn target_dir_path(&self) -> String;

    /// 验证存储配置是否有效（上传+下载测试文件）。
    async fn validate(&self) -> bool;
}
