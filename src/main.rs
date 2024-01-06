#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod blinks;
mod executor;
mod futures;
mod timers;

use crate::{
    blinks::{pulse, sos},
    executor::Executor,
    futures::{Delay, r#yield},
    timers::{millis, millis_init},
};

use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm};
use panic_halt as _;

type Serial =  arduino_hal::Usart<
      arduino_hal::pac::USART0,
      arduino_hal::port::Pin<arduino_hal::port::mode::Input,arduino_hal::hal::port::PD0>,
      arduino_hal::port::Pin<arduino_hal::port::mode::Output,arduino_hal::hal::port::PD1>,
  >;

static mut SERIAL_PTR: *mut Serial = core::ptr::null_mut();
// unsafe {
//     ufmt::uwriteln!(&mut *SERIAL_PTR, "wakers len: {}, cap: {}", wakers.len(), wakers.capacity()).unwrap();
// }

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    unsafe { SERIAL_PTR = &mut serial; }
    ufmt::uwriteln!(&mut serial, "Booting up").unwrap();

    // Configure INT0 for rising edge. 0x02 would be falling edge.
    dp.EXINT.eicra.modify(|_, w| w.isc0().bits(0x03));
    // Enable the INT0 interrupt source.
    dp.EXINT.eimsk.modify(|_, w| w.int0().set_bit());

    pins.d2.into_pull_up_input();

    millis_init(&dp.TC0);
    let timer = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut pwm_led = pins.d3.into_output().into_pwm(&timer);

    unsafe { avr_device::interrupt::enable() };

    ufmt::uwriteln!(&mut serial, "Nach init: {}", millis()).unwrap();
    // loop {
    {
        pulse(&mut pwm_led);
        sos(&mut pwm_led);
    }
    ufmt::uwriteln!(&mut serial, "Nach erstem Geblinke: {}", millis()).unwrap();

    let executor = Executor::new();
    // let delay = executor.block_on(
    executor.block_on(
        async {
            // r#yield().await;
            // 10_000
            Delay::wait_for(3000).await;
        });

    // arduino_hal::delay_ms(delay);
    loop {
        ufmt::uwriteln!(&mut serial, "In main loop: {}", millis()).unwrap();
        pulse(&mut pwm_led);
        sos(&mut pwm_led);
    }
}
