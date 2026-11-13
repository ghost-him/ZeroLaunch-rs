//! 深链接注册与分发。
//!
//! 把 `zerolaunch-rs://` 注册到当前用户，并把首实例/第二实例收到的 URL
//! 写入内核的等待表；插件通过同一张表读取外部打开参数。

use std::path::Path;
use std::sync::Arc;

use tracing::warn;
use url::Url;
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;
use zerolaunch_rs_lib::runtime;
use zerolaunch_rs_lib::state::app_state::AppState;

/// 深链接 scheme。
const SCHEME: &str = "zerolaunch-rs";

/// 把当前 exe 注册为 `zerolaunch-rs://` 的处理程序。
/// 参数：exe - 当前可执行文件路径。
/// 返回：注册成功为 Ok；写注册表失败为 Err。
pub fn register_protocol(exe: &Path) -> std::io::Result<()> {
    let classes =
        RegKey::predef(HKEY_CURRENT_USER).create_subkey(format!("Software\\Classes\\{SCHEME}"))?;
    let (key, _) = classes;
    key.set_value("", &"URL:ZeroLaunch Protocol")?;
    key.set_value("URL Protocol", &"")?;
    let (command, _) = key.create_subkey("shell\\open\\command")?;
    command.set_value("", &format!("\"{}\" \"%1\"", exe.display()))?;
    Ok(())
}

/// 把命令行里的深链接写入等待表。
/// 参数：state - 应用状态；args - 当前进程或第二实例的完整命令行。
/// 返回：是否至少识别到一个合法深链接。
pub fn dispatch(state: &Arc<AppState>, args: &[String]) -> bool {
    let mut found = false;
    for arg in args {
        let Ok(url) = Url::parse(arg) else {
            continue;
        };
        if url.scheme() != SCHEME {
            continue;
        }
        let Some(domain) = url.domain().map(str::to_string) else {
            warn!(scheme = url.scheme(), "深链接缺少域名，已忽略");
            continue;
        };
        let pairs = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();
        tracing::info!("收到深链接: {domain}, {} 个参数", pairs.len());
        let state = state.clone();
        runtime::spawn(async move {
            state.get_waiting_hashmap().insert(domain, pairs).await;
        });
        found = true;
    }
    found
}
