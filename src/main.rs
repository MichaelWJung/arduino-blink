#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod timers;

use crate::timers::{millis, millis_init};
use arduino_hal::port::mode::PwmOutput;
use arduino_hal::port::Pin;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, PwmPinOps, Timer2Pwm};
use avr_device::interrupt::Mutex;
use core::cell::Cell;
use core::sync::atomic::{AtomicBool, Ordering};
use panic_halt as _;

static BUTTON_PRESSED: AtomicBool = AtomicBool::new(false);
static LAST_TIME_PRESSED: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
const DEBOUNCE_DELAY: u32 = 200;

#[avr_device::interrupt(atmega328p)]
fn INT0() {
    let now = millis();
    avr_device::interrupt::free(|cs| {
        let last = LAST_TIME_PRESSED.borrow(cs);
        if now - last.get() > DEBOUNCE_DELAY {
            BUTTON_PRESSED.store(true, Ordering::SeqCst);
            last.set(now);
        }
    });
}

fn interrupted() -> bool {
    let pressed = BUTTON_PRESSED.load(Ordering::SeqCst);
    if !pressed {
        return false;
    }
    BUTTON_PRESSED.store(false, Ordering::SeqCst);
    return true;
}

const MORSE_UNIT: u16 = 250;

fn blink<X>(led: &mut Pin<PwmOutput<Timer2Pwm>, X>, factor: u16)
where
    X: PwmPinOps<Timer2Pwm>,
{
    led.enable();
    arduino_hal::delay_ms(MORSE_UNIT * factor);
    led.disable();
    arduino_hal::delay_ms(MORSE_UNIT);
}

const SOS_BLINKS: [u16; 9] = [1, 1, 1, 3, 3, 3, 1, 1, 1];

fn sos<X>(led: &mut Pin<PwmOutput<Timer2Pwm>, X>)
where
    X: PwmPinOps<Timer2Pwm>,
{
    led.set_duty(255);
    loop {
        for factor in SOS_BLINKS.iter() {
            blink(led, *factor);
            if interrupted() {
                return;
            }
        }
        for _ in 0..6 {
            arduino_hal::delay_ms(MORSE_UNIT);
            if interrupted() {
                return;
            }
        }
    }
}

fn pulse<X>(led: &mut Pin<PwmOutput<Timer2Pwm>, X>)
where
    X: PwmPinOps<Timer2Pwm>,
{
    led.enable();
    loop {
        for x in (0..=255).chain((0..=254).rev()) {
            led.set_duty(x);
            arduino_hal::delay_ms(10);
            if interrupted() {
                led.disable();
                return;
            }
        }
    }
}

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
