use crate::futures::delay::Delay;

use arduino_hal::port::mode::{Output, PwmOutput};
use arduino_hal::port::{Pin, PinOps};
use arduino_hal::simple_pwm::{PwmPinOps, Timer1Pwm};

const MORSE_UNIT: u32 = 250;

async fn blink<X>(led: &mut Pin<Output, X>, factor: u8)
where
    X: PinOps,
{
    led.set_high();
    Delay::wait_for(MORSE_UNIT * factor as u32).await;
    led.set_low();
    Delay::wait_for(MORSE_UNIT).await;
}

const SOS_BLINKS: [u8; 9] = [1, 1, 1, 3, 3, 3, 1, 1, 1];

pub async fn sos<X>(led: &mut Pin<Output, X>)
where
    X: PinOps,
{
    loop {
        for factor in SOS_BLINKS.iter() {
            blink(led, *factor).await;
        }
        Delay::wait_for(6 * MORSE_UNIT).await;
    }
}

pub async fn pulse<X>(led: &mut Pin<PwmOutput<Timer1Pwm>, X>)
where
    X: PwmPinOps<Timer1Pwm>,
{
    led.enable();
    loop {
        for x in (0..=255).chain((1..=254).rev()) {
            led.set_duty(x);
            Delay::wait_for(10).await;
        }
    }
}
