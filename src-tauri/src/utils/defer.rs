/// 实现类似于go语言中的defer函数
use std::ops::Drop;

pub struct Defer<F: FnOnce()> {
    f: Option<F>,
}

impl<F: FnOnce()> Defer<F> {
    fn new(f: F) -> Self {
        Defer { f: Some(f) }
    }
}

impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            f();
        }
    }
}

pub fn defer<F: FnOnce()>(f: F) -> Defer<F> {
    Defer::new(f)
}
