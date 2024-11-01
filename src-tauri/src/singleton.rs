use std::sync::{Mutex, Once};
use std::thread;

/// 创建一个单列模式
trait Singleton: Any + Send + Sync {
    fn instance() -> Arc<Mutex<Self>>
    where
        Self: Sized;
}

/// 实现一个宏来简化 Singleton 的实现
macro_rules! impl_singleton {
    ($t:ty) => {
        impl Singleton for $t {
            fn instance() -> Arc<Mutex<Self>> {
                static ONCE: Once = Once::new();
                static mut INSTANCE: Option<Arc<Mutex<$t>>> = None;

                ONCE.call_once(|| {
                    let instance = <$t>::new();
                    unsafe {
                        INSTANCE = Some(Arc::new(Mutex::new(instance)));
                    }
                });

                unsafe { INSTANCE.clone().unwrap() }
            }
        }
    };
}