use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use zerolaunch_plugin_api::config::SettingDefinition;
use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, ConfigError, Configurable};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    PanelInteraction, PanelKeyAction, PanelKeyBinding, Plugin, PluginContext, PluginError,
    PluginKind, PluginMetadata, PluginMode, Query, QueryChannel, QueryResponse, ResultAction,
};

pub struct CalculatorPlugin {
    core: ComponentCore,
    metadata: PluginMetadata,
    inner: RwLock<CalculatorSettings>,
    /// 最近一次计算的结果文本，供 execute_action 写入剪贴板。
    last_result: RwLock<Option<String>>,
    /// PluginHandle（init 时发放），供 execute_action 经句柄访问平台能力。
    handle: RwLock<Option<Arc<PluginHandle>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CalculatorSettings {}

impl Default for CalculatorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CalculatorPlugin {
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "calculator".to_string(),
                t_key!("calculator", "name").to_string(),
                t_key!("calculator", "description").to_string(),
                ComponentType::Plugin,
                0,
            ),
            metadata: PluginMetadata {
                id: "calculator".to_string(),
                // name/description 与 ComponentCore 同用 i18n key（key-or-literal），
                // 消除插件级与组件级元数据的双源硬编码
                name: t_key!("calculator", "name").to_string(),
                // 内置插件无独立版本/作者（随应用分发），UI 按内置标识展示
                version: String::new(),
                description: t_key!("calculator", "description").to_string(),
                author: String::new(),
                trigger_keywords: vec!["=".to_string()],
                supported_os: vec![
                    "windows".to_string(),
                    "macos".to_string(),
                    "linux".to_string(),
                ],
                priority: 100,
                kind: PluginKind::Builtin,
                // 行内插件：仅关键词（=）唤醒，无全局热键，不展示图标
                hotkey: None,
                icon: None,
                mode: PluginMode::Inline,
            },
            inner: RwLock::new(CalculatorSettings::default()),
            last_result: RwLock::new(None),
            handle: RwLock::new(None),
        }
    }

    /// 对数学表达式求值，返回计算结果。
    /// 错误时返回描述性字符串。
    fn evaluate(&self, expr: &str) -> Result<f64, String> {
        let mut parser = ExprParser::new(expr);
        parser.parse()
    }
}

// ---- Configurable impl ----

#[async_trait]
impl Configurable for CalculatorPlugin {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: CalculatorSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

// ---- Plugin impl ----

#[async_trait]
impl Plugin for CalculatorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    /// CalculatorPlugin 无需异步初始化，所有状态在构造时已就绪；
    /// 仅保存 init 发放的服务句柄，供 execute_action 访问平台能力。
    async fn init(
        &self,
        _ctx: &PluginContext,
        handle: Option<Arc<PluginHandle>>,
    ) -> Result<(), PluginError> {
        *self.handle.write() = handle;
        Ok(())
    }

    fn interaction_policy(&self) -> PanelInteraction {
        PanelInteraction {
            // 计算面板为行内形态（保留搜索栏），按键声明即接管：
            // - Enter：Confirm —— 宿主三分支：面板有可执行动作（计算成功，actions=[复制结果]）
            //   时执行默认动作（复制结果写入剪贴板）；无动作（空/错误面板）时发起确认查询（重新计算）；
            // - Escape：GoBack —— 退出计算面板（返回默认搜索）。
            bindings: vec![
                PanelKeyBinding {
                    key: "Enter".to_string(),
                    action: PanelKeyAction::Confirm,
                },
                PanelKeyBinding {
                    key: "Escape".to_string(),
                    action: PanelKeyAction::GoBack,
                },
            ],
            ..Default::default()
        }
    }

    async fn query(
        &self,
        ctx: &PluginContext,
        query: &Query,
    ) -> Result<QueryResponse, PluginError> {
        let expr = query.search_term.trim().to_string();

        if expr.is_empty() {
            return Ok(QueryResponse::CustomPanel {
                panel_type: "calculator".to_string(),
                data: json!({
                    "expression": "",
                    "result": null,
                    "history": []
                }),
                actions: vec![],
                keep_search_bar: true,
            });
        }

        match self.evaluate(&expr) {
            Ok(result) => {
                // 使用 epsilon 比较避免浮点精度问题
                let result_str = if (result - result.round()).abs() < 1e-10 {
                    format!("{}", result.round() as i64)
                } else {
                    format!("{}", result)
                };

                // 缓存结果文本，供 execute_action 写入剪贴板。
                // 仅 GUI 通道且查询仍最新可写入：CLI/调试查询为只读辅助路径，
                // 不得改写 GUI 剪贴板缓存（复制动作无通道区分）。
                if ctx.is_query_current() && ctx.query_channel == QueryChannel::Ui {
                    *self.last_result.write() = Some(result_str.clone());
                }

                Ok(QueryResponse::CustomPanel {
                    panel_type: "calculator".to_string(),
                    data: json!({
                        "expression": expr,
                        "result": result_str,
                        "rawValue": result,
                    }),
                    actions: vec![ResultAction {
                        id: "copy_result".to_string(),
                        label: "复制结果".to_string(),
                        icon: IconRequest::Path("copy".to_string()),
                        is_default: true,
                        shortcut_key: "Enter".to_string(),
                    }],
                    keep_search_bar: true,
                })
            }
            Err(error) => Ok(QueryResponse::CustomPanel {
                panel_type: "calculator".to_string(),
                data: json!({
                    "expression": expr,
                    "result": null,
                    "error": error,
                }),
                actions: vec![],
                keep_search_bar: true,
            }),
        }
    }

    async fn execute_action(
        &self,
        _ctx: &PluginContext,
        action_id: &str,
        _payload: serde_json::Value,
    ) -> Result<(), PluginError> {
        match action_id {
            "copy_result" => {
                let text = self.last_result.read().clone();
                if let Some(text) = text {
                    // 经 PluginHandle 访问剪贴板能力（init 时发放）。
                    let handle = self.handle.read().clone().ok_or_else(|| {
                        PluginError::ActionFailed("插件服务句柄不可用".to_string())
                    })?;
                    handle
                        .set_clipboard_text(&text)
                        .map_err(|e| PluginError::ActionFailed(format!("剪贴板写入失败: {}", e)))?;
                }
                Ok(())
            }
            _ => Err(PluginError::ActionFailed(format!(
                "Unknown action: {}",
                action_id
            ))),
        }
    }
}

// ---- Expression Parser ----

struct ExprParser {
    chars: Vec<char>,
    pos: usize,
}

impl ExprParser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        let result = self.parse_expr()?;
        self.skip_whitespace();
        if self.peek().is_some() {
            return Err(format!(
                "Unexpected character '{}' at position {}",
                self.peek().unwrap(),
                self.pos
            ));
        }
        Ok(result)
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut left = self.parse_term()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('+') => {
                    self.advance();
                    left += self.parse_term()?;
                }
                Some('-') => {
                    self.advance();
                    left -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut left = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('*') => {
                    self.advance();
                    left *= self.parse_factor()?;
                }
                Some('/') => {
                    self.advance();
                    let rhs = self.parse_factor()?;
                    if rhs == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    left /= rhs;
                }
                Some('%') => {
                    self.advance();
                    let rhs = self.parse_factor()?;
                    if rhs == 0.0 {
                        return Err("Modulo by zero".to_string());
                    }
                    left %= rhs;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        self.skip_whitespace();
        if self.peek() == Some('^') {
            self.advance();
            let exp = self.parse_factor()?;
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        match self.peek() {
            Some('-') => {
                self.advance();
                Ok(-self.parse_unary()?)
            }
            Some('+') => {
                self.advance();
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        match self.peek() {
            Some('(') => {
                self.advance();
                let result = self.parse_expr()?;
                self.skip_whitespace();
                match self.advance() {
                    Some(')') => Ok(result),
                    _ => Err("Missing closing ')'".to_string()),
                }
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.parse_number(),
            Some(c) => Err(format!("Unexpected character '{}'", c)),
            None => Err("Unexpected end of expression".to_string()),
        }
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        let mut has_dot = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else if c == '.' && !has_dot {
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }
        let num_str: String = self.chars[start..self.pos].iter().collect();
        num_str
            .parse::<f64>()
            .map_err(|_| format!("Invalid number: {}", num_str))
    }
}

use crate::plugin_framework::builtin_registry::PluginEntry;

fn build_calculator_plugin() -> (Arc<dyn Configurable>, Arc<dyn Plugin>) {
    let plugin: Arc<dyn Plugin> = Arc::new(CalculatorPlugin::new());
    let configurable: Arc<dyn Configurable> = plugin.clone();
    (configurable, plugin)
}

::inventory::submit! {
    PluginEntry {
        component_id: "calculator",
        priority: 0,
        factory: build_calculator_plugin,
    }
}
