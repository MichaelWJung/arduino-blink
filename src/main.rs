#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(iter_from_coroutine)]
#![feature(never_type)]

mod ag_lcd;
mod blinks;
mod executor;
mod futures;
mod lcd;
mod timers;

use core::cell::RefCell;

use crate::{
    blinks::{pulse, sos},
    executor::Executor,
    futures::{delay::Delay, join::join3},
    timers::millis_init,
};

use ag_lcd::{Cursor, LcdDisplay, Lines};
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm};
use panic_halt as _;
use port_expander::dev::pcf8574::Pcf8574;

type Serial = arduino_hal::Usart<
    arduino_hal::pac::USART0,
    arduino_hal::port::Pin<arduino_hal::port::mode::Input, arduino_hal::hal::port::PD0>,
    arduino_hal::port::Pin<arduino_hal::port::mode::Output, arduino_hal::hal::port::PD1>,
>;

static mut SERIAL_PTR: *mut Serial = core::ptr::null_mut();

macro_rules! dbgprint {
    ($($args:expr),*) => {{
        unsafe { ufmt::uwriteln!(&mut *crate::SERIAL_PTR, $($args)*).unwrap(); }
    }};
}
pub(crate) use dbgprint;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    unsafe {
        SERIAL_PTR = &mut serial;
    }
    ufmt::uwriteln!(&mut serial, "Booting up").unwrap();

    let delay = arduino_hal::Delay::new();

    let sda = pins.a4.into_pull_up_input();
    let scl = pins.a5.into_pull_up_input();

    let i2c_bus = arduino_hal::i2c::I2c::new(dp.TWI, sda, scl, 50000);
    let mut i2c_expander = Pcf8574::new(i2c_bus, true, true, true);

    let lcd: LcdDisplay<_, _> = LcdDisplay::new_pcf8574(&mut i2c_expander, delay)
        .with_cursor(Cursor::Off)
        .with_lines(Lines::TwoLines)
        .build();
    let lcd = RefCell::new(lcd);

    // Configure INT0 for rising edge. 0x02 would be falling edge.
    dp.EXINT.eicra.modify(|_, w| w.isc0().bits(0x03));
    // Enable the INT0 interrupt source.
    dp.EXINT.eimsk.modify(|_, w| w.int0().set_bit());

    pins.d2.into_pull_up_input();

    ufmt::uwriteln!(&mut serial, "A").unwrap();
    millis_init(&dp.TC0);
    let timer = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut pwm_led = pins.d3.into_output().into_pwm(&timer);
    let mut onboard_led = pins.d13.into_output();
    onboard_led.set_high();

    ufmt::uwriteln!(&mut serial, "B").unwrap();
    dbgprint!("ABC");
    arduino_hal::delay_ms(300);
    unsafe { avr_device::interrupt::enable() };

    ufmt::uwriteln!(&mut serial, "C").unwrap();
    let executor = Executor::new();

    executor.block_on(join3(
        async {
            loop {
                sos(&mut onboard_led).await;
            }
        },
        async {
            Delay::wait_for(1000).await;
            loop {
                pulse(&mut pwm_led).await;
            }
        },
        async {
            Delay::wait_for(2000).await;
            lcd::show_moving_text(("Steuern", "sind Raub!"), &lcd).await;
        },
    ));

    loop {}
}
