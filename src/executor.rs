// Heavily inspired by
// https://github.com/rust-embedded-community/async-on-embedded/blob/master/async-embedded/src/executor.rs

use core::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use crate::dbgprint;

use pin_utils::pin_mut;

pub struct Executor {}

// NOTE `*const ()` is &AtomicBool
static VTABLE: RawWakerVTable = {
    unsafe fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    unsafe fn wake(p: *const ()) {
        wake_by_ref(p)
    }
    unsafe fn wake_by_ref(p: *const ()) {
        (*(p as *const AtomicBool)).store(true, Ordering::Release)
    }
    unsafe fn drop(_: *const ()) {
        // no-op
    }

    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};

impl Executor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn block_on<T: Default>(&self, f: impl Future<Output = T>) -> T {
        pin_mut!(f);
        let ready = AtomicBool::new(true);
        let waker =
            unsafe { Waker::from_raw(RawWaker::new(&ready as *const _ as *const _, &VTABLE)) };
        let val = loop {
            // dbgprint!("Executor loop");
            let mut task_woken = false;
            if ready.load(Ordering::Acquire) {
                // dbgprint!("Task ready");
                task_woken = true;
                ready.store(false, Ordering::Release);

                let mut cx = Context::from_waker(&waker);
                // dbgprint!("Polling");
                if let Poll::Ready(val) = f.as_mut().poll(&mut cx) {
                    // dbgprint!("Task complete");
                    break val;
                }
                // dbgprint!("Task not complete");
            }

            if task_woken {
                // If at least one task was woken up, do not sleep, try again
                // dbgprint!("Trying once more as task woke up");
                continue;
            }
            // dbgprint!("Going to sleep");
            avr_device::asm::sleep();
            // dbgprint!("Waking up from sleep");
        };
        val
    }
}
