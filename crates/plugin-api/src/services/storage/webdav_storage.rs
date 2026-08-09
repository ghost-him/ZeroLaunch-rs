use crate::services::storage::storage_error::StorageError;
use crate::services::storage::storage_service::StorageService;
use async_trait::async_trait;
use reqwest_dav::{Client, ClientBuilder};
use std::path::PathBuf;
use tracing::{debug, warn};

/// WebDAV 连接配置。
/// 用于创建 WebDAVStorageService 实例。
pub struct WebDAVConfig {
    /// WebDAV 服务器地址
    pub host_url: String,
    /// 认证账号
    pub account: String,
    /// 认证密码
    pub password: String,
    /// 远程目标目录
    pub destination_dir: String,
}

/// WebDAV 远程存储服务。
/// 通过 WebDAV 协议将文件存储到远程服务器，使用 reqwest_dav 实现（跨平台）。
pub struct WebDAVStorageService {
    /// 远程目标目录
    destination_dir: PathBuf,
    /// WebDAV 客户端
    client: Option<Client>,
}

impl WebDAVStorageService {
    /// 创建 WebDAVStorageService。
    /// 参数：config - WebDAV 连接配置。
    pub fn new(config: &WebDAVConfig) -> Self {
        let client = ClientBuilder::new()
            .set_host(config.host_url.clone())
            .set_auth(reqwest_dav::Auth::Basic(
                config.account.clone(),
                config.password.clone(),
            ))
            .build()
            .ok();

        Self {
            destination_dir: PathBuf::from(&config.destination_dir),
            client,
        }
    }
}

#[async_trait]
impl StorageService for WebDAVStorageService {
    /// 将数据上传到 WebDAV 服务器。
    async fn upload(&self, file_name: &str, data: &[u8]) -> Result<(), StorageError> {
        let target_path = self.destination_dir.join(file_name);
        let target_path_str = target_path
            .to_str()
            .ok_or_else(|| StorageError::InvalidPath(file_name.to_string()))?
            .to_string();

        let client = self
            .client
            .as_ref()
            .ok_or(StorageError::ClientNotInitialized)?;

        client
            .put(&target_path_str, data.to_vec())
            .await
            .map_err(|e| StorageError::UploadFailed {
                file: file_name.to_string(),
                reason: e.to_string(),
            })?;

        debug!("WebDAV 上传完成: {}", file_name);
        Ok(())
    }

    /// 从 WebDAV 服务器下载数据。
    /// 文件不存在（404）时返回 Ok(None)。
    async fn download(&self, file_name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let target_path = self.destination_dir.join(file_name);
        let target_path_str = target_path
            .to_str()
            .ok_or_else(|| StorageError::InvalidPath(file_name.to_string()))?
            .to_string();

        let client = self
            .client
            .as_ref()
            .ok_or(StorageError::ClientNotInitialized)?;

        match client.get(&target_path_str).await {
            Ok(response) => {
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|e| StorageError::DownloadFailed {
                        file: file_name.to_string(),
                        reason: format!("读取文件流失败: {}", e),
                    })?;
                debug!("WebDAV 下载完成: {}, {} bytes", file_name, bytes.len());
                Ok(Some(bytes.to_vec()))
            }
            Err(e) => {
                // 404 表示文件不存在，返回 None
                if let reqwest_dav::Error::Decode(reqwest_dav::DecodeError::Server(server_error)) =
                    &e
                {
                    if server_error.response_code == 404 {
                        debug!("WebDAV 文件不存在: {}", file_name);
                        return Ok(None);
                    }
                }
                Err(StorageError::DownloadFailed {
                    file: file_name.to_string(),
                    reason: format!("{:?}", e),
                })
            }
        }
    }

    /// 获取 WebDAV 存储的目标目录路径。
    fn target_dir_path(&self) -> String {
        self.destination_dir.to_str().unwrap_or("").to_string()
    }

    /// 从 WebDAV 服务器删除文件。
    async fn delete(&self, file_name: &str) -> Result<(), StorageError> {
        let target_path = self.destination_dir.join(file_name);
        let target_path_str = target_path
            .to_str()
            .ok_or_else(|| StorageError::InvalidPath(file_name.to_string()))?
            .to_string();

        let client = self
            .client
            .as_ref()
            .ok_or(StorageError::ClientNotInitialized)?;

        client
            .delete(&target_path_str)
            .await
            .map_err(|e| StorageError::DeleteFailed {
                file: file_name.to_string(),
                reason: format!("{:?}", e),
            })?;

        debug!("WebDAV 删除完成: {}", file_name);
        Ok(())
    }

    /// 列出 WebDAV 服务器上指定前缀下的所有文件。
    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let target_path = self.destination_dir.join(prefix);
        let target_path_str = target_path
            .to_str()
            .ok_or_else(|| StorageError::InvalidPath(prefix.to_string()))?
            .to_string();

        let client = self
            .client
            .as_ref()
            .ok_or(StorageError::ClientNotInitialized)?;

        let entries = client
            .list(&target_path_str, reqwest_dav::Depth::Number(1))
            .await
            .map_err(|e| StorageError::ListFailed {
                prefix: prefix.to_string(),
                reason: format!("{:?}", e),
            })?;

        let files: Vec<String> = entries
            .into_iter()
            .filter_map(|e| match e {
                reqwest_dav::list_cmd::ListEntity::File(f) => {
                    let name = f.href.rsplit('/').next().map(|s| s.to_string());
                    name
                }
                reqwest_dav::list_cmd::ListEntity::Folder(_) => None,
            })
            .collect();

        debug!("WebDAV 列表完成: {} ({})", prefix, files.len());
        Ok(files)
    }

    /// 验证 WebDAV 存储配置是否有效。
    /// 尝试写入并读取测试文件来验证。
    async fn validate(&self) -> bool {
        let test_file = "__zerolaunch_storage_test__.txt";
        let test_data = b"ZeroLaunch storage validation test";

        if self.upload(test_file, test_data).await.is_err() {
            warn!("WebDAV 验证上传失败");
            return false;
        }

        if self.download(test_file).await.is_err() {
            warn!("WebDAV 验证下载失败");
            return false;
        }

        true
    }
}

#[cfg(all(test, feature = "webdav"))]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::{Child, Stdio};
    use std::time::Duration;

    /// WebDAV 集成测试服务器地址（bun fixture 固定监听端口）。
    const SERVER_URL: &str = "http://127.0.0.1:18080";

    /// 启动 bun WebDAV 测试服务器（tests/fixtures/webdav_server.ts）。
    /// 轮询 OPTIONS 直至就绪，超时 panic。
    fn start_server() -> Child {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/webdav_server.ts");
        let child = std::process::Command::new("bun")
            .arg("run")
            .arg(&script)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("启动 WebDAV 测试服务器失败（需要 bun 可执行文件）");
        child
    }

    /// 轮询等待服务器就绪（OPTIONS 返回 200），单次请求 2 秒超时。
    async fn wait_ready() {
        for _ in 0..25 {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .expect("创建 reqwest 客户端失败");
            if client
                .request(reqwest::Method::OPTIONS, SERVER_URL)
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        panic!("WebDAV 测试服务器启动超时（5 秒）");
    }

    /// 构造指向测试服务器的 WebDAVStorageService（目标目录为根）。
    fn make_service() -> WebDAVStorageService {
        WebDAVStorageService::new(&WebDAVConfig {
            host_url: SERVER_URL.into(),
            account: "test".into(),
            password: "test".into(),
            destination_dir: "/".into(),
        })
    }

    /// WebDAV 存储服务端到端契约：upload/download/delete/list/validate 全链路。
    ///
    /// 依赖 bun 与 tests/fixtures/webdav_server.ts；整体 30 秒超时保护，
    /// 服务器进程通过 shutdown 端点优雅退出（兜底 kill 进程树）。
    #[tokio::test]
    async fn webdav_storage_full_roundtrip() {
        let mut child = start_server();

        let assertions = tokio::time::timeout(Duration::from_secs(30), async {
            wait_ready().await;

            let svc = make_service();
            assert_eq!(svc.target_dir_path(), "/", "目标目录应返回 destination_dir");

            // 上传 → 下载往返，内容一致
            svc.upload("remote/config.json", br#"{"a":1}"#)
                .await
                .expect("上传失败");
            let data = svc
                .download("remote/config.json")
                .await
                .expect("下载失败")
                .expect("上传的文件应可下载");
            assert_eq!(data, br#"{"a":1}"#);

            // 不存在文件 → Ok(None) 而非错误（404 语义）
            assert_eq!(
                svc.download("not-exist.json").await.expect("下载失败"),
                None,
                "不存在的文件应返回 None"
            );

            // 删除 → 再下载为 None
            svc.delete("remote/config.json").await.expect("删除失败");
            assert_eq!(
                svc.download("remote/config.json").await.expect("下载失败"),
                None,
                "删除后文件应不可下载"
            );

            // 列表：前缀目录下仅返回文件（过滤目录）
            svc.upload("dir/a.txt", b"a").await.expect("上传失败");
            svc.upload("dir/b.txt", b"b").await.expect("上传失败");
            let files = svc.list("dir").await.expect("列表失败");
            assert!(
                files.contains(&"a.txt".to_string()),
                "列表应含 a.txt: {:?}",
                files
            );
            assert!(
                files.contains(&"b.txt".to_string()),
                "列表应含 b.txt: {:?}",
                files
            );

            // validate：上传+下载测试文件往返成功
            assert!(svc.validate().await, "validate 上传下载往返应成功");

            // 清理服务器端残留
            svc.delete("dir/a.txt").await.expect("清理失败");
            svc.delete("dir/b.txt").await.expect("清理失败");

            // 优雅关闭测试服务器（避免 bun 进程树残留）
            let _ = reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .expect("创建 reqwest 客户端失败")
                .post(format!("{SERVER_URL}/__shutdown"))
                .send()
                .await;
            Ok::<(), ()>(())
        })
        .await
        .expect("WebDAV 端到端断言超时（30 秒）");

        // 等待服务器退出（轮询最多 5 秒），兜底杀进程树
        for _ in 0..50 {
            if child.try_wait().ok().flatten().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if child.try_wait().ok().flatten().is_none() {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .output();
            }
            #[cfg(not(windows))]
            {
                let _ = child.kill();
            }
        }

        assertions.expect("WebDAV 端到端断言失败");
    }
}
