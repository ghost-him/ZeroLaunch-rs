use windows::Win32::System::Com::{CoInitialize, CoUninitialize};

/// COM 初始化守卫。以 STA 模式初始化 COM，析构时按需调用 CoUninitialize。
/// 自动处理 RPC_E_CHANGED_MODE（线程已用其他模式初始化 COM）— 此情况下 COM 仍可使用。
pub struct ComGuard {
    should_uninit: bool,
}

impl ComGuard {
    /// 以 STA 模式初始化当前线程的 COM。
    /// 成功时析构阶段会调用 CoUninitialize 平衡引用计数。
    /// 若 COM 已用其他模式初始化（RPC_E_CHANGED_MODE），不报错也不做清理。
    ///
    /// # Safety
    /// 必须在调用 COM API 的线程上调用。
    pub unsafe fn init() -> Self {
        const RPC_E_CHANGED_MODE: i32 = -2147417850; // 0x80010106
        let hr = CoInitialize(None);
        if hr.is_ok() {
            return Self {
                should_uninit: true,
            };
        }
        if hr.0 == RPC_E_CHANGED_MODE {
            return Self {
                should_uninit: false,
            };
        }
        tracing::warn!("COM 初始化失败：{:?}", hr);
        Self {
            should_uninit: false,
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninit {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
