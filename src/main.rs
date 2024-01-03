#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod blinks;
mod timers;

use crate::{
    blinks::{pulse, sos},
    timers::millis_init,
};

use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm};
use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Configure INT0 for rising edge. 0x02 would be falling edge.
    dp.EXINT.eicra.modify(|_, w| w.isc0().bits(0x03));
    // Enable the INT0 interrupt source.
    dp.EXINT.eimsk.modify(|_, w| w.int0().set_bit());

    pins.d2.into_pull_up_input();

    millis_init(&dp.TC0);
    let timer = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut pwm_led = pins.d3.into_output().into_pwm(&timer);

    unsafe { avr_device::interrupt::enable() };

    loop {
        pulse(&mut pwm_led);
        sos(&mut pwm_led);
    }
}
