use std::{
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Poll, Waker},
};

use gpui_kit::App;
use rstate::StoreError;

#[derive(Clone, Default)]
pub struct Gate {
    ready: Rc<Cell<bool>>,
    waker: Rc<RefCell<Option<Waker>>>,
}

impl Gate {
    #[expect(
        clippy::future_not_send,
        reason = "the deterministic foreground gate intentionally retains UI-thread Rc state"
    )]
    pub fn wait(&self) -> impl Future<Output = ()> + use<'_> {
        std::future::poll_fn(|cx| {
            if self.ready.get() {
                Poll::Ready(())
            } else {
                self.waker.replace(Some(cx.waker().clone()));
                Poll::Pending
            }
        })
    }

    pub fn release(&self) {
        self.ready.set(true);
        let waker = self.waker.take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

#[derive(Clone, Default)]
pub struct BackgroundGate {
    ready: Arc<AtomicBool>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl BackgroundGate {
    pub fn wait(&self) -> impl Future<Output = ()> + use<'_> {
        std::future::poll_fn(|cx| {
            let mut waker = self.waker.lock().unwrap();
            if self.ready.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                *waker = Some(cx.waker().clone());
                Poll::Pending
            }
        })
    }

    pub fn release(&self) {
        let waker = {
            let mut waker = self.waker.lock().unwrap();
            self.ready.store(true, Ordering::Release);
            waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

pub struct OnDrop(pub Option<Box<dyn FnOnce()>>);

impl Drop for OnDrop {
    fn drop(&mut self) {
        if let Some(callback) = self.0.take() {
            callback();
        }
    }
}

pub fn unexpected(error: StoreError, _: &mut App) {
    let message = error.to_string();
    drop(error);
    panic!("unexpected command delivery error: {message}");
}
