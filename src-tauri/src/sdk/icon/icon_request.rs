use bincode_next::Encode;
use serde::{Deserialize, Serialize};

/// 图标请求类型，表示不同来源的图标提取需求。
/// 各类型使用各自的提取逻辑完成图标提取。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Encode)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum IconRequest {
    /// 本地文件路径 (exe, lnk, url, ico, png) -> 提取文件图标
    Path(String),
    /// 网址 -> 下载或查找本地域名图标库
    Url(String),
    /// 文件扩展名 (.txt, .doc) -> 获取系统关联图标
    Extension(String),
}

impl IconRequest {
    /// 返回内部字符串值，用于 bridge 层到前端的向后兼容转换。
    pub fn value(&self) -> &str {
        match self {
            IconRequest::Path(s) | IconRequest::Url(s) | IconRequest::Extension(s) => s.as_str(),
        }
    }

    /// 判断图标请求是否为空（无意义的图标数据）。
    pub fn is_empty(&self) -> bool {
        self.value().is_empty()
    }

    /// 计算图标请求的 blake3 哈希值，用作缓存键。
    pub fn get_hash_string(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        let _ = bincode_next::encode_into_std_write(
            self,
            &mut hasher,
            bincode_next::config::standard(),
        );
        hasher.finalize().to_hex().to_string()
    }
}
