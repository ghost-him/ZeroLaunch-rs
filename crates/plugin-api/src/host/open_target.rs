/// Shell 打开目标类型。
/// 将不同打开语义统一为枚举，平台层根据类型选择不同的系统调用。
#[derive(Debug, Clone)]
pub enum OpenTarget {
    /// 使用系统默认程序打开文件
    File(String),
    /// 使用默认浏览器打开网址
    Url(String),
    /// 使用文件资源管理器打开文件夹
    Folder(String),
}
