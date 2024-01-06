use crate::{futures::delay::Delay, timers::millis};

use crate::SERIAL_PTR;
use arduino_hal::port::mode::{Output, PwmOutput};
use arduino_hal::port::{Pin, PinOps};
use arduino_hal::simple_pwm::{PwmPinOps, Timer2Pwm};
use avr_device::interrupt::Mutex;
use core::cell::Cell;
use core::sync::atomic::{AtomicBool, Ordering};

// static BUTTON_PRESSED: AtomicBool = AtomicBool::new(false);
// static LAST_TIME_PRESSED: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
// const DEBOUNCE_DELAY: u32 = 200;

// #[avr_device::interrupt(atmega328p)]
// fn INT0() {
//     let now = millis();
//     avr_device::interrupt::free(|cs| {
//         let last = LAST_TIME_PRESSED.borrow(cs);
//         if now - last.get() > DEBOUNCE_DELAY {
//             BUTTON_PRESSED.store(true, Ordering::SeqCst);
//             last.set(now);
//         }
//     });
// }

// fn interrupted() -> bool {
//     let pressed = BUTTON_PRESSED.load(Ordering::SeqCst);
//     if !pressed {
//         return false;
//     }
//     BUTTON_PRESSED.store(false, Ordering::SeqCst);
//     return true;
// }

const MORSE_UNIT: u32 = 250;

async fn blink<X>(led: &mut Pin<Output, X>, factor: u8)
where
    X: PinOps,
{
    //     dbgprint!("Running blink");
    led.set_high();
    Delay::wait_for(MORSE_UNIT * factor as u32).await;
    //     dbgprint!("Running blink A");
    led.set_low();
    Delay::wait_for(MORSE_UNIT).await;
    //     dbgprint!("Running blink B");
}

const SOS_BLINKS: [u8; 9] = [1, 1, 1, 3, 3, 3, 1, 1, 1];

pub async fn sos<X>(led: &mut Pin<Output, X>)
where
    X: PinOps,
{
    loop {
        //     dbgprint!("Running sos");
        for factor in SOS_BLINKS.iter() {
            blink(led, *factor).await;
        }
        Delay::wait_for(6 * MORSE_UNIT).await;
    }
}

pub async fn pulse<X>(led: &mut Pin<PwmOutput<Timer2Pwm>, X>)
where
    X: PwmPinOps<Timer2Pwm>,
{
    led.enable();
    loop {
        for x in (0..=255).chain((0..=254).rev()) {
            led.set_duty(x);
            Delay::wait_for(10).await;
        }
    }
}
