//! 版本更新检查。
//!
//! 只做「有没有更新」的判定与跳转，不下载安装包。
//! 检查不走 `api.github.com`（匿名额度常被打空返回 403）：直接请求
//! `releases/latest`，禁用重定向后从 `Location` 里取 tag。

use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use reqwest::redirect::Policy;
use reqwest::Client as HttpClient;
use serde::Serialize;
use tracing::{info, warn};

use crate::core::host::UpdateMenuState;
use crate::sdk::OpenTarget;
use crate::state::app_state::AppState;

/// 仓库 Releases 的 latest 地址（302 到具体 tag）。
const LATEST_RELEASE_URL: &str = "https://github.com/ghost-him/ZeroLaunch-rs/releases/latest";
/// `Location` 中 tag 页路径片段；不含它表示仓库还没有发布。
const RELEASE_TAG_PATH: &str = "/releases/tag/";
/// `Location` 中 tag 页的绝对前缀（拼发布页地址用）。
const RELEASE_TAG_BASE: &str = "https://github.com/ghost-him/ZeroLaunch-rs/releases/tag/";

/// 一次更新检查的结果 —— `update_check` / `update_get_status` 的返回体，经 IPC 原样下发设置页。
///
/// 仅在 `updater` 模块内产生，前端「关于」页消费；不作为持久化结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    /// 当前运行版本（`CARGO_PKG_VERSION`，不含 `v` 前缀）。
    pub current_version: String,
    /// 最新发布版本（不含 `v` 前缀）；`None` 表示尚未成功检查过或仓库无发布。
    pub latest_version: Option<String>,
    /// 最新版本是否高于当前版本；检查失败时保留上次成功检查的结论。
    pub has_update: bool,
    /// 最新版本的发布页地址；`None` 表示没有可跳转的目标。
    pub release_url: Option<String>,
    /// 最近一次检查完成时间（本地时间 `YYYY-MM-DD HH:MM:SS`）；`None` 表示从未检查。
    pub checked_at: Option<String>,
    /// 最近一次检查的失败原因（HTTP 错误、解析失败等）；成功后清空。
    pub error: Option<String>,
}

/// 更新检查器 —— 持有 HTTP 客户端与最近一次检查结果。
///
/// 由 `AppState` 持有单实例（进程内共享），供托盘、启动自检与设置页 IPC 复用；
/// 其他模块不得另建实例，以免状态分叉。
pub struct UpdateChecker {
    /// 禁用重定向的 HTTP 客户端（需要读取 302 的 `Location`）。
    client: HttpClient,
    /// 最近一次检查结果；仅在 `check` 成功/失败时整体替换。
    status: RwLock<UpdateStatus>,
}

impl Default for UpdateChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateChecker {
    /// 创建检查器实例。
    ///
    /// 参数：无。返回：初始状态（未检查）；HTTP 客户端构建失败时 panic
    /// （构建只依赖编译期常量，失败属不可恢复的环境错误）。
    pub fn new() -> Self {
        let client = HttpClient::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("ZeroLaunch-rs/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("构建更新检查 HTTP 客户端失败");

        Self {
            client,
            status: RwLock::new(UpdateStatus {
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                latest_version: None,
                has_update: false,
                release_url: None,
                checked_at: None,
                error: None,
            }),
        }
    }

    /// 读取最近一次检查结果（不发网络请求）。
    pub fn status(&self) -> UpdateStatus {
        self.status.read().clone()
    }

    /// 实网检查一次并写回状态。
    ///
    /// 返回：本次检查结果。网络或解析失败时不返回 Err，而是把原因写入 `error`
    /// 并保留上次成功检查的版本结论，避免短暂断网把「有新版本」清空。
    pub async fn check(&self) -> UpdateStatus {
        let fetched = self.fetch_latest_release().await;
        // 网络请求期间不持有锁，结果一次性写回，避免读锁跨 await。
        let status = match fetched {
            Ok(latest) => {
                let mut status = self.status.read().clone();
                match latest {
                    Some((version, url)) => {
                        status.has_update = is_newer(&version, &status.current_version);
                        status.latest_version = Some(version);
                        status.release_url = Some(url);
                    }
                    None => {
                        status.has_update = false;
                        status.latest_version = None;
                        status.release_url = None;
                    }
                }
                status.error = None;
                status.checked_at = Some(now_string());
                status
            }
            Err(e) => {
                warn!("检查更新失败: {}", e);
                let mut status = self.status.read().clone();
                status.error = Some(e);
                status.checked_at = Some(now_string());
                status
            }
        };

        *self.status.write() = status.clone();
        info!(
            "更新检查完成: 当前 v{}, 最新 {:?}, 有新版本: {}",
            status.current_version, status.latest_version, status.has_update
        );
        status
    }

    /// 请求 `releases/latest` 并解析最新 tag。
    ///
    /// 返回：`Ok(Some((版本, 发布页地址)))` 表示有发布；`Ok(None)` 表示仓库无发布；
    /// `Err` 为网络错误、响应头缺失或 tag 无法解析。
    async fn fetch_latest_release(&self) -> Result<Option<(String, String)>, String> {
        let response = self
            .client
            .get(LATEST_RELEASE_URL)
            .send()
            .await
            .map_err(|e| format!("请求 GitHub Releases 失败: {e}"))?;

        let http_status = response.status();
        if !http_status.is_redirection() {
            return Err(format!(
                "GitHub Releases 未返回重定向（HTTP {}）",
                http_status.as_u16()
            ));
        }

        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| "重定向响应缺少 Location 头".to_string())?;

        // 无发布时 Location 指向 /releases，不含 tag 路径。
        let Some(tag) = location.split(RELEASE_TAG_PATH).nth(1) else {
            return Ok(None);
        };
        let tag = tag.trim_end_matches('/');
        let version = tag.trim_start_matches('v');
        if version.is_empty() {
            return Err(format!("无法从 Location 解析版本号: {location}"));
        }

        Ok(Some((
            version.to_string(),
            format!("{RELEASE_TAG_BASE}{tag}"),
        )))
    }
}

/// 执行一次检查并同步外部表现。
///
/// 参数：`state` —— 应用状态（取检查器、托盘与平台句柄）；
/// `user_initiated` —— 是否由用户主动触发（托盘点「检查更新」）。
/// 返回：本次检查结果。
/// 行为：始终刷新托盘菜单文案——主动触发时先显示「正在检查…」，结束后回显结果
/// （已是最新带版本号 / 失败），不让点击看起来毫无反馈；主动触发且发现新版本时
/// 直接打开发布页，否则（已是最新/失败）发系统通知。自动检查不弹任何窗口。
pub async fn check_and_sync(state: &Arc<AppState>, user_initiated: bool) -> UpdateStatus {
    if user_initiated {
        state
            .get_host()
            .tray_set_update_state(UpdateMenuState::Checking);
    }

    let status = state.get_update_checker().check().await;

    state
        .get_host()
        .tray_set_update_state(update_menu_state(&status, user_initiated));

    if user_initiated {
        match (status.error.as_ref(), status.release_url.as_deref()) {
            (None, Some(url)) if status.has_update => {
                if let Err(e) = state
                    .get_core_handle()
                    .shell_open(OpenTarget::Url(url.to_string()))
                    .await
                {
                    warn!("打开更新发布页失败: {:?}", e);
                }
            }
            _ => notify_check_result(state, &status),
        }
    }

    status
}

/// 用系统通知告知一次手动检查的结果（已是最新 / 检查失败）。
fn notify_check_result(state: &Arc<AppState>, status: &UpdateStatus) {
    let i18n = state.get_i18n_manager();
    let lang = i18n.current_language();
    let body = if status.error.is_some() {
        i18n.t(&lang, "update.checkFailed")
    } else {
        i18n.t(&lang, "update.upToDate")
            .replace("{version}", &status.current_version)
    };

    state
        .get_host()
        .notify(&i18n.t(&lang, "update.toastTitle"), &body);
}

/// 由检查结果推导托盘「检查更新」菜单的展示状态。
///
/// 参数：`status` —— 本次检查结果；`user_initiated` —— 是否用户主动触发。
/// 返回：菜单状态。手动检查以本次结果为准（失败即「失败」，与同时发出的通知一致）；
/// 自动检查只在发现新版本时改文案，其余情况复位成 Idle，避免启动时的静默检查在菜单里
/// 留下结果回显。检查失败时 `has_update` 会保留上次成功结论，因此自动检查仍能提示已知新版本。
fn update_menu_state(status: &UpdateStatus, user_initiated: bool) -> UpdateMenuState {
    if user_initiated && status.error.is_some() {
        return UpdateMenuState::Failed;
    }
    if status.has_update {
        if let Some(version) = status.latest_version.as_deref() {
            return UpdateMenuState::Available(version.to_string());
        }
    }
    if user_initiated {
        UpdateMenuState::UpToDate(status.current_version.clone())
    } else {
        UpdateMenuState::Idle
    }
}

/// 当前本地时间字符串（`YYYY-MM-DD HH:MM:SS`）。
fn now_string() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 判断 `latest` 是否高于 `current`（忽略 `v` 前缀与 `-` 之后的预发布后缀）。
///
/// 逐段按数字比较，缺失段按 0 处理（`1.6` 与 `1.6.0` 相等）。
pub fn is_newer(latest: &str, current: &str) -> bool {
    let latest = version_parts(latest);
    let current = version_parts(current);
    let len = latest.len().max(current.len());
    for i in 0..len {
        let l = latest.get(i).copied().unwrap_or(0);
        let c = current.get(i).copied().unwrap_or(0);
        if l != c {
            return l > c;
        }
    }
    false
}

/// 把 `v1.5.4-beta.1` 解析为 `[1, 5, 4]`。
fn version_parts(version: &str) -> Vec<u64> {
    version
        .trim()
        .trim_start_matches('v')
        .split('-')
        .next()
        .unwrap_or_default()
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 版本比较：更高版本、相等版本与预发布后缀。
    #[test]
    fn is_newer_compares_numeric_segments() {
        assert!(is_newer("1.6.0", "1.5.4"));
        assert!(is_newer("v1.5.10", "1.5.9"));
        assert!(is_newer("1.6", "1.5.9"));
        assert!(!is_newer("1.5.4", "1.5.4"));
        assert!(!is_newer("v1.5.4", "1.5.4"));
        assert!(!is_newer("1.5", "1.5.0"));
        assert!(!is_newer("1.4.9", "1.5.0"));
        // 预发布后缀按基础版本比较
        assert!(is_newer("1.6.0-beta.1", "1.5.4"));
    }

    /// 构造检查结果，供托盘状态推导测试使用。
    fn status_with(
        current: &str,
        latest: Option<&str>,
        has_update: bool,
        error: Option<&str>,
    ) -> UpdateStatus {
        UpdateStatus {
            current_version: current.to_string(),
            latest_version: latest.map(str::to_string),
            has_update,
            release_url: None,
            checked_at: Some("2026-01-01 00:00:00".to_string()),
            error: error.map(str::to_string),
        }
    }

    /// 手动检查：已是最新时托盘菜单要回显「已是最新」而不是退回「检查更新」。
    #[test]
    fn manual_check_without_update_reports_up_to_date() {
        let status = status_with("1.5.4", Some("1.5.4"), false, None);
        assert_eq!(
            update_menu_state(&status, true),
            UpdateMenuState::UpToDate("1.5.4".to_string())
        );
    }

    /// 手动检查失败时回显失败，不能静默复位。
    #[test]
    fn manual_check_failure_reports_failed() {
        let status = status_with("1.5.4", None, false, Some("network down"));
        assert_eq!(update_menu_state(&status, true), UpdateMenuState::Failed);
    }

    /// 发现新版本时菜单带版本号（手动与自动一致）。
    #[test]
    fn available_version_wins_for_both_triggers() {
        let status = status_with("1.5.4", Some("1.6.0"), true, None);
        let expected = UpdateMenuState::Available("1.6.0".to_string());
        assert_eq!(update_menu_state(&status, true), expected);
        assert_eq!(
            update_menu_state(&status, false),
            UpdateMenuState::Available("1.6.0".to_string())
        );
    }

    /// 启动自动检查无更新时复位为「检查更新」，避免菜单留下上次操作的结果。
    #[test]
    fn auto_check_without_update_resets_to_idle() {
        let status = status_with("1.5.4", Some("1.5.4"), false, None);
        assert_eq!(update_menu_state(&status, false), UpdateMenuState::Idle);
    }

    /// 手动检查失败但已知新版：菜单以本次失败为准（与同时发出的通知一致），
    /// 自动检查则继续提示已知的新版本。
    #[test]
    fn manual_error_overrides_known_update_but_auto_keeps_it() {
        let status = status_with("1.5.4", Some("1.6.0"), true, Some("network down"));
        assert_eq!(update_menu_state(&status, true), UpdateMenuState::Failed);
        assert_eq!(
            update_menu_state(&status, false),
            UpdateMenuState::Available("1.6.0".to_string())
        );
    }
}
