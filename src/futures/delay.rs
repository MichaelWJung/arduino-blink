use avr_device::interrupt::Mutex;
use core::{
    cell::Cell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::timers::{millis, WAKERS};

static NEXT_DELAY_ID: Mutex<Cell<u16>> = Mutex::new(Cell::new(0));

pub struct Delay {
    wake_time: u32,
    id: u16,
}

impl Delay {
    pub fn wait_for(delay: u32) -> Self {
        let wake_time = millis() + delay;
        let id = avr_device::interrupt::free(|cs| {
            let next_id = NEXT_DELAY_ID.borrow(cs);
            let id = next_id.get();
            next_id.set(id.wrapping_add(1));
            id
        });
        Self { wake_time, id }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let now = millis();
        if now < self.wake_time {
            avr_device::interrupt::free(|cs| {
                let mut wakers = WAKERS.borrow(cs).borrow_mut();
                if wakers.has_capacity() {
                    match wakers.replace_or_push(self.wake_time, self.id, cx.waker().clone()) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                } else {
                }
            });
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
