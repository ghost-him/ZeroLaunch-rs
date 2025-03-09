use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialWebDAVConfig {
    pub host_url: Option<String>,
    pub account: Option<String>,
    pub password: Option<String>,
    pub destination_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct WebDAVConfigInner {
    #[serde(default = "WebDAVConfigInner::default_host_url")]
    pub host_url: String,
    #[serde(default = "WebDAVConfigInner::default_account")]
    pub account: String,
    #[serde(default = "WebDAVConfigInner::default_password")]
    pub password: String,
    #[serde(default = "WebDAVConfigInner::default_destination_dir")]
    pub destination_dir: String,
}

impl Default for WebDAVConfigInner {
    fn default() -> Self {
        Self {
            host_url: Self::default_host_url(),
            account: Self::default_account(),
            password: Self::default_password(),
            destination_dir: Self::default_destination_dir(),
        }
    }
}

impl WebDAVConfigInner {
    // 默认值方法
    pub(crate) fn default_host_url() -> String {
        String::new()
    }

    pub(crate) fn default_account() -> String {
        String::new()
    }

    pub(crate) fn default_password() -> String {
        String::new()
    }

    pub(crate) fn default_destination_dir() -> String {
        "/default/upload/path".into() // 可根据需求修改默认路径
    }

    // 更新方法
    pub fn update(&mut self, partial_config: PartialWebDAVConfig) {
        if let Some(host_url) = partial_config.host_url {
            self.host_url = host_url;
        }
        if let Some(account) = partial_config.account {
            self.account = account;
        }
        if let Some(password) = partial_config.password {
            self.password = password;
        }
        if let Some(dir) = partial_config.destination_dir {
            self.destination_dir = dir;
        }
    }

    // 转换方法
    pub fn to_partial(&self) -> PartialWebDAVConfig {
        PartialWebDAVConfig {
            host_url: Some(self.host_url.clone()),
            account: Some(self.account.clone()),
            password: Some(self.password.clone()),
            destination_dir: Some(self.destination_dir.clone()),
        }
    }

    // 访问方法
    pub fn get_host_url(&self) -> &str {
        &self.host_url
    }

    pub fn get_account(&self) -> &str {
        &self.account
    }

    pub fn get_password(&self) -> &str {
        &self.password
    }

    pub fn get_destination_dir(&self) -> &str {
        &self.destination_dir
    }
}

#[derive(Debug)]
pub struct WebDAVConfig {
    inner: RwLock<WebDAVConfigInner>,
}

impl Default for WebDAVConfig {
    fn default() -> Self {
        Self {
            inner: RwLock::new(WebDAVConfigInner::default()),
        }
    }
}

impl WebDAVConfig {
    pub fn update(&self, partial_config: PartialWebDAVConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }

    pub fn to_partial(&self) -> PartialWebDAVConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    // 新增的访问方法
    pub fn get_destination_dir(&self) -> String {
        let inner = self.inner.read();
        inner.destination_dir.clone()
    }

    // 保留原有访问方法
    pub fn get_host_url(&self) -> String {
        let inner = self.inner.read();
        inner.host_url.clone()
    }

    pub fn get_account(&self) -> String {
        let inner = self.inner.read();
        inner.account.clone()
    }

    pub fn get_password(&self) -> String {
        let inner = self.inner.read();
        inner.password.clone()
    }
}
