use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    Plugin, PluginContext, PluginError, PluginMetadata, Query, QueryResponse, ResultAction,
};

pub struct CalculatorPlugin {
    metadata: PluginMetadata,
    inner: RwLock<CalculatorSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalculatorSettings {
    #[serde(rename = "enabled", default = "default_enabled_true")]
    enabled: bool,
}

fn default_enabled_true() -> bool {
    true
}

impl Default for CalculatorSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Default for CalculatorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CalculatorPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "calculator".to_string(),
                name: "计算器".to_string(),
                version: "1.0.0".to_string(),
                description: "支持基本数学表达式求值的计算器插件".to_string(),
                author: "ZeroLaunch".to_string(),
                trigger_keywords: vec!["=".to_string()],
                supported_os: vec![
                    "windows".to_string(),
                    "macos".to_string(),
                    "linux".to_string(),
                ],
                priority: 100,
            },
            inner: RwLock::new(CalculatorSettings::default()),
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

impl Configurable for CalculatorPlugin {
    fn component_id(&self) -> &str {
        "calculator"
    }

    fn component_name(&self) -> &str {
        "计算器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Plugin
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: CalculatorSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }

    fn get_default_settings(&self) -> serde_json::Value {
        json!({ "enabled": true })
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

    /// CalculatorPlugin 无需异步初始化，所有状态在构造时已就绪
    async fn init(
        &self,
        _ctx: &PluginContext,
        _handle: Arc<PluginHandle>,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    async fn query(
        &self,
        _ctx: &PluginContext,
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
                // The frontend handles clipboard access via Tauri APIs
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

use crate::plugin_system::builtin_registry::PluginEntry;

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
