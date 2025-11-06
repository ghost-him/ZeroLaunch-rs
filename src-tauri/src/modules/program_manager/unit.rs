// 存放辅助型的小类型
use crate::core::image_processor::ImageIdentity;
use crate::program_manager::PartialProgramManagerConfig;
use crate::program_manager::builtin_commands::PREFIX;
use bincode::{Decode, Encode};
pub type EmbeddingVec = Vec<f32>;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LaunchMethodKind {
    Path,
    PackageFamilyName,
    File,
    Command,
    BuiltinCommand,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash, Encode, Decode)]
pub enum LaunchMethod {
    /// 通过文件路径来启动
    Path(String),
    /// 通过包族名来启动
    PackageFamilyName(String),
    /// 使用默认的启动方式来打开一个文件
    File(String),
    /// 一个自定义的命令
    Command(String),
    /// 内置设置的命令
    BuiltinCommand(String),
}

impl LaunchMethod {
    fn template_text(&self) -> &str {
        match self {
            LaunchMethod::Path(path) => path,
            LaunchMethod::PackageFamilyName(name) => name,
            LaunchMethod::File(path) => path,
            LaunchMethod::Command(command) => command,
            LaunchMethod::BuiltinCommand(command) => command,
        }
    }

    fn map_text(&self, text: String) -> LaunchMethod {
        match self {
            LaunchMethod::Path(_) => LaunchMethod::Path(text),
            LaunchMethod::PackageFamilyName(_) => LaunchMethod::PackageFamilyName(text),
            LaunchMethod::File(_) => LaunchMethod::File(text),
            LaunchMethod::Command(_) => LaunchMethod::Command(text),
            LaunchMethod::BuiltinCommand(_) => {
                if text.starts_with(PREFIX) {
                    LaunchMethod::BuiltinCommand(text)
                } else {
                    panic!("编码错误！内置命令必须以 内置命令前缀 PREFIX 开头");
                }
            },
        }
    }

    /// 这个是用于在文件中存储的全局唯一标识符
    pub fn get_text(&self) -> String {
        self.template_text().to_string()
    }

    /// 统计启动模板中的"{}"占位符数量
    pub fn placeholder_count(&self) -> usize {
        self.template_text().matches("{}").count()
    }

    /// 返回启动方式的具体类型
    pub fn kind(&self) -> LaunchMethodKind {
        match self {
            LaunchMethod::Path(_) => LaunchMethodKind::Path,
            LaunchMethod::PackageFamilyName(_) => LaunchMethodKind::PackageFamilyName,
            LaunchMethod::File(_) => LaunchMethodKind::File,
            LaunchMethod::Command(_) => LaunchMethodKind::Command,
            LaunchMethod::BuiltinCommand(_) => LaunchMethodKind::BuiltinCommand,
        }
    }

    /// 用用户输入替换模板占位符并生成新的启动方式
    pub fn fill_placeholders(&self, args: &[String]) -> Result<LaunchMethod, String> {
        let filled = fill_template(self.template_text(), args)?;
        Ok(self.map_text(filled))
    }

    pub fn is_uwp(&self) -> bool {
        matches!(self, LaunchMethod::PackageFamilyName(_))
    }
}

// 根据占位符顺序依次填充模板并校验参数数量
fn fill_template(template: &str, args: &[String]) -> Result<String, String> {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;
    let mut index = 0;

    while let Some(pos) = remaining.find("{}") {
        let (before, after_placeholder) = remaining.split_at(pos);
        result.push_str(before);

        let replacement = args.get(index).ok_or_else(|| {
            format!(
                "not enough arguments: expected at least {}, got {}",
                index + 1,
                args.len()
            )
        })?;
        result.push_str(replacement);

        remaining = &after_placeholder[2..];
        index += 1;
    }

    if index != args.len() {
        return Err(format!(
            "too many arguments: expected {}, got {}",
            index,
            args.len()
        ));
    }

    result.push_str(remaining);
    Ok(result)
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
    pub icon_path: ImageIdentity,
    /// 用于语义搜索的相关内容(可选)
    pub embedding: EmbeddingVec,
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

/// 表示语义信息的存储项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticStoreItem {
    /// 程序的显示名字
    pub show_name: String,
    /// 是否为 UWP 应用
    pub is_uwp: bool,
    /// 描述信息
    pub description: String,
}

impl SemanticStoreItem {
    pub fn new(program: Arc<Program>) -> Self {
        Self {
            show_name: program.show_name.clone(),
            is_uwp: program.launch_method.is_uwp(),
            description: String::new(),
        }
    }
}

pub struct ProgramManagerRuntimeData {
    pub semantic_store_str: String,
    pub runtime_data: PartialProgramManagerConfig,
    pub semantic_cache_bytes: Vec<u8>,
}
