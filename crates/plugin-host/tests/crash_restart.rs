//! 崩溃自动重启链路的端到端集成测试。
//!
//! 使用真实 fixture 插件二进制（`src/bin/fixture_plugin.rs`）走完整链路：
//! load（spawn + 握手 + discover）→ 测试用 taskkill 模拟崩溃 → watchdog 检测
//! → crash_loop → handle_crash（崩溃即解注册）→ 复用 load 重启。
//!
//! 覆盖场景：
//! 1. 崩溃 → 自动重启成功，组件重新注册（on_crash/on_restart 各触发一次）
//! 2. max_restart 超限 → 放弃重启，登记清空且不再有重注册
//! 3. 组件 id 冲突 → load 被预检拒绝（ComponentIdCollision）

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::time::sleep;

use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_host::host_dispatch::HostCallHandler;
use zerolaunch_plugin_host::manager::{
    CrashCallback, PluginHostManager, PluginLoadError, PluginRegistration, RestartCallback,
};
use zerolaunch_plugin_protocol::codes::METHOD_NOT_FOUND;
use zerolaunch_plugin_protocol::error::JsonRpcError;

/// fixture 插件可执行文件路径（cargo 为集成测试注入的编译期常量）。
const FIXTURE_BIN: &str = env!("CARGO_BIN_EXE_fixture_plugin");
/// fixture 插件声明的组件 id（与 fixture_plugin.rs 保持一致）。
const FIXTURE_COMPONENT_ID: &str = "fixture.hello";

/// 测试用 HostCallHandler stub：fixture 不发起 host/* 调用，恒返回 method not found。
struct StubHandler;

#[async_trait::async_trait]
impl HostCallHandler for StubHandler {
    async fn handle_host_call(&self, _method: &str, _params: Value) -> Result<Value, JsonRpcError> {
        Err(JsonRpcError::new(METHOD_NOT_FOUND, "stub"))
    }
}

/// 从 fixtures 目录组装完整插件布局到临时目录，返回插件目录路径。
///
/// `tests/fixtures/crash-restart-plugin/` 是仓库内的完整插件清单
/// （manifest.toml + bin/ 布局）；运行时只补充编译产物二进制
/// （exe 无法静态入库），并按测试场景改写 plugin.id 与 runtime.maxRestart。
fn prepare_plugin_dir(test_name: &str, plugin_id: &str, max_restart: u32) -> PathBuf {
    let fixture_src =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/crash-restart-plugin");
    let plugin_dir = std::env::temp_dir()
        .join(format!(
            "zl-crash-test-{}-{}",
            std::process::id(),
            test_name
        ))
        .join("plugin");
    let bin_dir = plugin_dir.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("create bin dir");

    // 复制 manifest 并放入编译产物（与 manifest 的 command 路径对应）
    std::fs::copy(
        fixture_src.join("manifest.toml"),
        plugin_dir.join("manifest.toml"),
    )
    .expect("copy manifest");
    std::fs::copy(FIXTURE_BIN, bin_dir.join("fixture_plugin.exe")).expect("copy fixture binary");

    // 按测试场景改写 manifest：插件 id 与重启上限（仅这两行，字段唯一）
    let manifest_path = plugin_dir.join("manifest.toml");
    let manifest = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest = manifest.replace(
        "id = \"com.example.crash-restart-fixture\"",
        &format!("id = \"{}\"", plugin_id),
    );
    let manifest = manifest.replace("maxRestart = 3", &format!("maxRestart = {}", max_restart));
    std::fs::write(&manifest_path, manifest).expect("rewrite manifest");
    plugin_dir
}

/// 加载一个测试插件，返回 (管理器, 崩溃计数, 重启计数)。
async fn setup_loaded_plugin(
    test_name: &str,
    plugin_id: &str,
    max_restart: u32,
) -> (Arc<PluginHostManager>, Arc<AtomicU32>, Arc<AtomicU32>) {
    let root = std::env::temp_dir().join(format!(
        "zl-crash-test-{}-{}",
        std::process::id(),
        test_name
    ));
    let plugin_dir = prepare_plugin_dir(test_name, plugin_id, max_restart);
    let mgr = PluginHostManager::new(root.join("plugins"), root.join("data"), root.join("logs"));

    let crashed = Arc::new(AtomicU32::new(0));
    let restarted = Arc::new(AtomicU32::new(0));
    let on_crash: CrashCallback = {
        let c = Arc::clone(&crashed);
        Arc::new(move |_prev: PluginRegistration| {
            c.fetch_add(1, Ordering::SeqCst);
        })
    };
    let on_restart: RestartCallback = {
        let r = Arc::clone(&restarted);
        Arc::new(move |_new: PluginRegistration| {
            let r = Arc::clone(&r);
            Box::pin(async move {
                r.fetch_add(1, Ordering::SeqCst);
            })
        })
    };

    let reg = mgr
        .load(
            &plugin_dir,
            Arc::new(StubHandler),
            on_restart,
            on_crash,
            0,
            "zh-CN",
        )
        .await
        .expect("initial load succeeds");
    assert_eq!(reg.components.len(), 1, "fixture 声明一个组件");
    assert_eq!(reg.components[0].component_id(), FIXTURE_COMPONENT_ID);
    (mgr, crashed, restarted)
}

/// 取插件当前进程的 PID。
fn plugin_pid(mgr: &PluginHostManager, plugin_id: &str) -> u32 {
    mgr.processes
        .get(plugin_id)
        .and_then(|p| p.pid)
        .expect("plugin process registered")
}

/// 用 Windows 原生命令强制终止插件进程，模拟崩溃。
fn kill_plugin(pid: u32) {
    let status = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .status()
        .expect("taskkill runs");
    assert!(status.success(), "taskkill failed for pid {}", pid);
}

/// 轮询等待条件成立（崩溃处理是异步链路，需要等待 watchdog/restart 完成）。
async fn wait_until(mut cond: impl FnMut() -> bool, timeout: Duration) {
    tokio::time::timeout(timeout, async {
        loop {
            if cond() {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("condition not met within timeout");
}

/// 场景 1：崩溃 → 自动重启成功，组件重新注册。
#[tokio::test]
async fn crash_triggers_restart_and_reregisters() {
    let plugin_id = "com.example.crash-test";
    let (mgr, crashed, restarted) = setup_loaded_plugin("restart-ok", plugin_id, 3).await;

    // 第一次崩溃 → 重启成功（新进程 pid 变化）
    let old_pid = plugin_pid(&mgr, plugin_id);
    kill_plugin(old_pid);
    wait_until(
        || {
            mgr.processes
                .get(plugin_id)
                .map(|p| p.pid != Some(old_pid))
                .unwrap_or(false)
        },
        Duration::from_secs(15),
    )
    .await;
    assert_eq!(crashed.load(Ordering::SeqCst), 1, "崩溃即解注册触发一次");
    assert_eq!(restarted.load(Ordering::SeqCst), 1, "重启成功重注册一次");
    assert!(mgr.plugins.get(plugin_id).is_some(), "重启后组件重新登记");

    // 第二次崩溃 → 再次重启（验证链路可重复）
    let new_pid = plugin_pid(&mgr, plugin_id);
    kill_plugin(new_pid);
    wait_until(
        || {
            mgr.processes
                .get(plugin_id)
                .map(|p| p.pid != Some(new_pid))
                .unwrap_or(false)
        },
        Duration::from_secs(15),
    )
    .await;
    assert_eq!(crashed.load(Ordering::SeqCst), 2);
    assert_eq!(restarted.load(Ordering::SeqCst), 2);

    // 清理：优雅卸载（关闭进程）
    mgr.unload(plugin_id).await.expect("unload succeeds");
}

/// 场景 2：max_restart 超限 → 放弃重启，登记清空、不再重注册。
#[tokio::test]
async fn crash_exhausts_max_restart_and_abandons() {
    let plugin_id = "com.example.crash-test";
    // maxRestart = 1：第一次崩溃重启成功，第二次放弃
    let (mgr, crashed, restarted) = setup_loaded_plugin("restart-limit", plugin_id, 1).await;

    let old_pid = plugin_pid(&mgr, plugin_id);
    kill_plugin(old_pid);
    wait_until(
        || {
            mgr.processes
                .get(plugin_id)
                .map(|p| p.pid != Some(old_pid))
                .unwrap_or(false)
        },
        Duration::from_secs(15),
    )
    .await;
    assert_eq!(restarted.load(Ordering::SeqCst), 1, "第一次崩溃重启成功");

    // 第二次崩溃 → 超出 max_restart，放弃重启：登记清空且不再恢复
    let new_pid = plugin_pid(&mgr, plugin_id);
    kill_plugin(new_pid);
    wait_until(
        || mgr.plugins.get(plugin_id).is_none(),
        Duration::from_secs(15),
    )
    .await;
    assert_eq!(crashed.load(Ordering::SeqCst), 2, "两次崩溃均解注册");
    assert_eq!(restarted.load(Ordering::SeqCst), 1, "超限后不再重启");
    assert!(mgr.processes.get(plugin_id).is_none(), "进程登记已清空");
}

/// 场景 3：组件 id 冲突 → 第二个插件 load 被预检拒绝（ComponentIdCollision）。
#[tokio::test]
async fn colliding_component_id_rejected_on_load() {
    let plugin_id_a = "com.example.crash-a";
    let plugin_id_b = "com.example.crash-b";
    let (mgr, _, _) = setup_loaded_plugin("collision-a", plugin_id_a, 3).await;

    // 插件 B 使用同一 fixture 插件（manifest id 被改写为 B）→ 声明相同组件 id fixture.hello
    let plugin_dir_b = prepare_plugin_dir("collision-b", plugin_id_b, 3);
    let crashed_b = Arc::new(AtomicU32::new(0));
    let restarted_b = Arc::new(AtomicU32::new(0));
    let on_crash_b: CrashCallback = {
        let c = Arc::clone(&crashed_b);
        Arc::new(move |_prev: PluginRegistration| {
            c.fetch_add(1, Ordering::SeqCst);
        })
    };
    let on_restart_b: RestartCallback = {
        let r = Arc::clone(&restarted_b);
        Arc::new(move |_new: PluginRegistration| {
            let r = Arc::clone(&r);
            Box::pin(async move {
                r.fetch_add(1, Ordering::SeqCst);
            })
        })
    };

    let err = mgr
        .load(
            &plugin_dir_b,
            Arc::new(StubHandler),
            on_restart_b,
            on_crash_b,
            0,
            "zh-CN",
        )
        .await
        .expect_err("冲突插件加载必须被拒绝");
    assert!(
        matches!(err, PluginLoadError::ComponentIdCollision { .. }),
        "应为 ComponentIdCollision，实际: {:?}",
        err
    );
    // 冲突插件进程已被 teardown，不留登记
    assert!(mgr.plugins.get(plugin_id_b).is_none());
    assert!(mgr.processes.get(plugin_id_b).is_none());
    // 原插件不受影响
    assert!(mgr.plugins.get(plugin_id_a).is_some());

    mgr.unload(plugin_id_a).await.expect("unload succeeds");
}
