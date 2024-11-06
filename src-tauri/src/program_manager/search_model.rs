use super::Program;
/// SearchModel 表示一个综合的搜索模型
///
/// Preprocessor 表示一个预处理函数，会在加载程序，和预处理用户输入时使用。
///
/// SearchAlgorithm 定义了搜索算法所需具备的核心功能和行为
///
/// ScoreAdjuster 代表一个函数 y = f(x)，通常用于调整权重或将一个值映射到另一个域
use std::sync::Arc;

trait Preprocessor {
    /// 预处理一个函数
    /// input: 要预处理的字符串
    fn preprocessing(&self, input: &String) -> String;
}

trait SearchAlgorithm {
    /// 计算两个字符串之间的权重
    /// source: 目标字符串
    /// user_input: 用户输入的字符串
    fn calculator(&self, source: &String, user_input: &String) -> f64;
}

trait ScoreAdjuster {
    /// 将一个值映射到另一个值上
    /// x: 要映射的值
    fn map(x: f64) -> f64;
}

pub trait SearchModel {
    fn new() -> Self where Self: Sized;
    /// 表示一个综合的，集成多种子算法的搜索算法
    /// source: 目标程序
    /// user_input: 用户输入的字符串
    fn calculator(&self, program: Arc<Program>, user_input: &String) -> f64;
}

pub struct StandardSearchModel {}

impl SearchModel for StandardSearchModel {
    fn new() -> Self {
        StandardSearchModel {}
    }

    fn calculator(&self, program: Arc<Program>, user_input: &String) -> f64 {
        // todo: 完成这个实现，如果使用到了什么子算法，用上面的模块实现出来再完成这个就可以了
        0.0
    }
}
