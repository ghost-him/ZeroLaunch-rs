// 存放辅助型的小类型
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone)]
pub enum LaunchMethod {
    /// 通过文件路径来启动
    Path(String),
    /// 通过包族名来启动
    PackageFamilyName(String),
    /// 使用默认的启动方式来打开一个文件
    File(String),
}

impl LaunchMethod {
    /// 这个是用于在文件中存储的全局唯一标识符
    pub fn get_text(&self) -> String {
        match &self {
            LaunchMethod::Path(path) => path.clone(),
            LaunchMethod::PackageFamilyName(name) => name.clone(),
            LaunchMethod::File(path) => path.clone(),
        }
    }

    pub fn is_uwp(&self) -> bool {
        match &self {
            LaunchMethod::Path(_) => false,
            LaunchMethod::PackageFamilyName(_) => true,
            LaunchMethod::File(_) => false,
        }
    }
}

/// 表示一个数据
#[derive(Debug)]
pub struct Program {
    /// 全局唯一标识符，用于快速索引，用于内存中存储
    pub program_guid: u64,
    /// 展示给用户看的名字
    pub show_name: String,
    /// 这个程序的启动方法
    pub launch_method: LaunchMethod,
    /// 用于计算的字符串
    pub search_keywords: Vec<String>,
    /// 权重固定偏移量
    pub stable_bias: f64,
    /// 应用程序应该展示的图片的地址
    pub icon_path: String,
}

/// 表示搜索测试的结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchTestResult {
    /// 程序的名称
    pub program_name: String,
    /// 程序的关键字
    pub program_keywords: String,
    /// 程序的路径
    pub program_path: String,
    /// 匹配的权重值
    pub score: f64,
}
