use crate::sdk::storage::storage_error::StorageError;
use crate::sdk::storage::storage_service::StorageService;
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::debug;

/// 本地文件系统存储服务。
/// 将文件存储到本地目录，使用 tokio::fs 实现（跨平台）。
pub struct LocalStorageService {
    /// 存储目标目录
    target_dir: PathBuf,
}

impl LocalStorageService {
    /// 创建 LocalStorageService。
    /// 参数：target_dir - 存储目标目录路径。
    pub fn new(target_dir: impl Into<PathBuf>) -> Self {
        Self {
            target_dir: target_dir.into(),
        }
    }
}

#[async_trait]
impl StorageService for LocalStorageService {
    /// 将数据上传到本地文件系统。
    /// 在目标目录下创建文件并写入数据，自动创建父目录。
    async fn upload(&self, file_name: &str, data: &[u8]) -> Result<(), StorageError> {
        let target_path = self.target_dir.join(file_name);

        // 确保父目录存在
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&target_path, data)
            .await
            .map_err(|e| StorageError::UploadFailed {
                file: file_name.to_string(),
                reason: format!("写入文件失败: {}", e),
            })?;

        debug!("本地上传完成: {}", file_name);
        Ok(())
    }

    /// 从本地文件系统下载数据。
    /// 文件不存在时返回 Ok(None)。
    async fn download(&self, file_name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let target_path = self.target_dir.join(file_name);

        if !target_path.exists() {
            debug!("本地文件不存在: {}", file_name);
            return Ok(None);
        }

        let data =
            tokio::fs::read(&target_path)
                .await
                .map_err(|e| StorageError::DownloadFailed {
                    file: file_name.to_string(),
                    reason: format!("读取文件失败: {}", e),
                })?;

        debug!("本地下载完成: {}, {} bytes", file_name, data.len());
        Ok(Some(data))
    }

    /// 获取本地存储的目标目录路径。
    fn target_dir_path(&self) -> String {
        self.target_dir.to_str().unwrap_or(".").to_string()
    }

    /// 从本地文件系统删除文件。
    async fn delete(&self, file_name: &str) -> Result<(), StorageError> {
        let target_path = self.target_dir.join(file_name);
        if !target_path.exists() {
            debug!("本地文件不存在，跳过删除: {}", file_name);
            return Ok(());
        }
        tokio::fs::remove_file(&target_path)
            .await
            .map_err(|e| StorageError::DeleteFailed {
                file: file_name.to_string(),
                reason: format!("删除文件失败: {}", e),
            })?;
        debug!("本地删除完成: {}", file_name);
        Ok(())
    }

    /// 列出本地目录中指定前缀下的所有文件。
    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let target_dir = self.target_dir.join(prefix);
        if !target_dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries =
            tokio::fs::read_dir(&target_dir)
                .await
                .map_err(|e| StorageError::ListFailed {
                    prefix: prefix.to_string(),
                    reason: format!("读取目录失败: {}", e),
                })?;
        let mut files = Vec::new();
        while let Some(entry) =
            entries
                .next_entry()
                .await
                .map_err(|e| StorageError::ListFailed {
                    prefix: prefix.to_string(),
                    reason: format!("遍历目录失败: {}", e),
                })?
        {
            if entry
                .file_type()
                .await
                .map(|t| t.is_file())
                .unwrap_or(false)
            {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(name.to_string());
                }
            }
        }
        debug!("本地列表完成: {} ({})", prefix, files.len());
        Ok(files)
    }

    /// 验证本地存储配置是否有效。
    /// 尝试写入并读取测试文件来验证。
    async fn validate(&self) -> bool {
        let test_file = "__zerolaunch_storage_test__.txt";
        let test_data = b"ZeroLaunch storage validation test";

        if self.upload(test_file, test_data).await.is_err() {
            return false;
        }

        if self.download(test_file).await.is_err() {
            return false;
        }

        // 清理测试文件
        let test_path = self.target_dir.join(test_file);
        let _ = tokio::fs::remove_file(&test_path).await;

        true
    }
}
