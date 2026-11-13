//! 系统托盘：独立消息循环 + 原生菜单事件。
//!
//! 仅在 GPUI 外壳内使用。宿主把菜单文案与可见性变化投递到托盘线程，
//! 菜单点击经 `app_command` 回到内核，避免托盘直接持有业务对象。

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE, WM_QUIT,
};
use zerolaunch_rs_lib::core::app_command::{self, AppCommand};

/// 托盘菜单当前语言的文案快照。
///
/// 由宿主按 `I18nManager` 生成，整体投递给托盘线程替换菜单文本；
/// 不参与持久化，语言或更新状态变化时重新生成。
#[derive(Debug, Clone)]
pub struct TrayLabels {
    /// 打开设置菜单项。
    pub show_settings: String,
    /// 刷新数据库菜单项。
    pub refresh: String,
    /// 重新注册快捷键菜单项。
    pub reregister: String,
    /// 游戏模式勾选项。
    pub game_mode: String,
    /// 检查更新菜单项（可能带检查结果）。
    pub update: String,
    /// 退出菜单项。
    pub exit: String,
}

/// 投递给托盘线程的操作。
///
/// 仅在本模块内构造；宿主通过 `Tray` 的三个 setter 发送。
enum TrayCommand {
    /// 替换菜单文案。
    Labels(TrayLabels),
    /// 显示或隐藏托盘图标。
    Visible(bool),
    /// 更新游戏模式勾选状态。
    Game(bool),
    /// 结束托盘线程并移除图标。
    Exit,
}

/// 托盘菜单句柄。
///
/// 由托盘线程持有，用于后续更新菜单文案与勾选状态；
/// 线程退出时随菜单一起释放。
struct TrayItems {
    /// 打开设置菜单项。
    show_settings: MenuItem,
    /// 刷新数据库菜单项。
    refresh: MenuItem,
    /// 重新注册快捷键菜单项。
    reregister: MenuItem,
    /// 游戏模式勾选项。
    game_mode: CheckMenuItem,
    /// 检查更新菜单项。
    update: MenuItem,
    /// 退出菜单项。
    exit: MenuItem,
}

/// 系统托盘入口。
///
/// 持有托盘线程的发送端；由 `GpuiHost` 持有并在退出时释放。
pub struct Tray {
    /// 托盘线程命令通道。
    commands: Sender<TrayCommand>,
}

impl Tray {
    /// 创建托盘线程并显示初始图标。
    /// 参数：icon_path - 托盘图标文件；labels - 初始菜单文案；
    ///       visible - 是否显示图标；game - 游戏模式初始勾选状态。
    /// 返回：创建线程成功时为 Tray；线程启动失败为 Err。
    pub fn new(
        icon_path: String,
        labels: TrayLabels,
        visible: bool,
        game: bool,
    ) -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("zerolaunch-tray".to_string())
            .spawn(move || run(icon_path, labels, visible, game, receiver))?;
        Ok(Self { commands })
    }

    /// 替换菜单文案。
    /// 参数：labels - 新语言/新更新状态对应的完整文案。
    /// 返回：无；托盘线程已退出时忽略。
    pub fn set_labels(&self, labels: TrayLabels) {
        let _ = self.commands.send(TrayCommand::Labels(labels));
    }

    /// 显示或隐藏图标。
    /// 参数：visible - 目标可见性。
    /// 返回：无；托盘线程已退出时忽略。
    pub fn set_visible(&self, visible: bool) {
        let _ = self.commands.send(TrayCommand::Visible(visible));
    }

    /// 更新游戏模式勾选状态。
    /// 参数：checked - 是否勾选。
    /// 返回：无；托盘线程已退出时忽略。
    pub fn set_game(&self, checked: bool) {
        let _ = self.commands.send(TrayCommand::Game(checked));
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        let _ = self.commands.send(TrayCommand::Exit);
    }
}

/// 托盘线程主体：创建菜单/图标并泵 Windows 消息。
///
/// 参数：icon_path - 图标路径；labels - 初始文案；visible/game - 初始状态；
///       commands - 宿主操作通道。
/// 返回：无；图标创建失败时仍保留消息循环，避免通道发送端报错。
fn run(
    icon_path: String,
    labels: TrayLabels,
    visible: bool,
    game: bool,
    commands: Receiver<TrayCommand>,
) {
    MenuEvent::set_event_handler(Some(|event: MenuEvent| {
        let command = match event.id().0.as_str() {
            "show_settings" => AppCommand::ShowSettings,
            "refresh" => AppCommand::RefreshCandidates,
            "reregister" => AppCommand::ReregisterHotkeys,
            "toggle_game_mode" => AppCommand::ToggleGameMode,
            "check_update" => AppCommand::CheckUpdate,
            "exit" => AppCommand::ExitProgram,
            _ => return,
        };
        app_command::send(command);
    }));
    TrayIconEvent::set_event_handler(Some(|event: TrayIconEvent| {
        if matches!(event, TrayIconEvent::DoubleClick { .. }) {
            app_command::send(AppCommand::ToggleSearchBar);
        }
    }));

    let mut tray = build(&icon_path, &labels, visible, game);
    if tray.is_none() {
        tracing::warn!("创建系统托盘失败，图标保持隐藏");
    }

    loop {
        while let Ok(command) = commands.try_recv() {
            match command {
                TrayCommand::Exit => {
                    drop(tray.take());
                    return;
                }
                TrayCommand::Labels(next) => {
                    if let Some((_, items)) = tray.as_mut() {
                        items.show_settings.set_text(&next.show_settings);
                        items.refresh.set_text(&next.refresh);
                        items.reregister.set_text(&next.reregister);
                        items.game_mode.set_text(&next.game_mode);
                        items.update.set_text(&next.update);
                        items.exit.set_text(&next.exit);
                    }
                }
                TrayCommand::Visible(visible) => {
                    if let Some((icon, _)) = tray.as_ref() {
                        if let Err(error) = icon.set_visible(visible) {
                            tracing::warn!("切换托盘图标失败: {error}");
                        }
                    }
                }
                TrayCommand::Game(checked) => {
                    if let Some((_, items)) = tray.as_ref() {
                        items.game_mode.set_checked(checked);
                    }
                }
            }
        }

        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() } {
            if message.message == WM_QUIT {
                return;
            }
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// 创建托盘图标与菜单。
///
/// 参数：icon_path - 图标文件；labels - 菜单文案；visible/game - 初始状态。
/// 返回：图标与菜单句柄；任一创建步骤失败时为 None。
fn build(
    icon_path: &str,
    labels: &TrayLabels,
    visible: bool,
    game: bool,
) -> Option<(TrayIcon, TrayItems)> {
    let show_settings = MenuItem::with_id("show_settings", &labels.show_settings, true, None);
    let refresh = MenuItem::with_id("refresh", &labels.refresh, true, None);
    let reregister = MenuItem::with_id("reregister", &labels.reregister, true, None);
    let game_mode = CheckMenuItem::with_id("toggle_game_mode", &labels.game_mode, true, game, None);
    let update = MenuItem::with_id("check_update", &labels.update, true, None);
    let exit = MenuItem::with_id("exit", &labels.exit, true, None);

    let menu = Menu::new();
    let built = menu
        .append(&show_settings)
        .and_then(|_| menu.append(&PredefinedMenuItem::separator()))
        .and_then(|_| menu.append(&refresh))
        .and_then(|_| menu.append(&reregister))
        .and_then(|_| menu.append(&game_mode))
        .and_then(|_| menu.append(&PredefinedMenuItem::separator()))
        .and_then(|_| menu.append(&update))
        .and_then(|_| menu.append(&exit));
    if let Err(error) = built {
        tracing::warn!("构建托盘菜单失败: {error}");
        return None;
    }

    let icon = Icon::from_path(icon_path, None)
        .map_err(|error| tracing::warn!("加载托盘图标失败: {error}"))
        .ok()?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("ZeroLaunch-rs")
        .with_menu_on_left_click(false)
        .build()
        .map_err(|error| tracing::warn!("创建托盘图标失败: {error}"))
        .ok()?;
    if let Err(error) = tray.set_visible(visible) {
        tracing::warn!("设置托盘初始可见性失败: {error}");
    }
    tracing::info!("系统托盘已创建");

    Some((
        tray,
        TrayItems {
            show_settings,
            refresh,
            reregister,
            game_mode,
            update,
            exit,
        },
    ))
}
