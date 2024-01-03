#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use arduino_hal::port::mode::PwmOutput;
use arduino_hal::port::Pin;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, PwmPinOps, Timer2Pwm};
use core::cell;
use core::sync::atomic::{AtomicBool, Ordering};

static BUTTON_PRESSED: AtomicBool = AtomicBool::new(false);
static LAST_TIME_PRESSED: avr_device::interrupt::Mutex<cell::Cell<u32>> =
    avr_device::interrupt::Mutex::new(cell::Cell::new(0));
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

const PRESCALER: u32 = 1024;
const TIMER_COUNTS: u32 = 125;

const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16000;

static MILLIS_COUNTER: avr_device::interrupt::Mutex<cell::Cell<u32>> =
    avr_device::interrupt::Mutex::new(cell::Cell::new(0));

fn millis_init(tc0: &arduino_hal::pac::TC0) {
    // Configure the timer for the above interval (in CTC mode)
    // and enable its interrupt.
    tc0.tccr0a.write(|w| w.wgm0().ctc());
    tc0.ocr0a.write(|w| w.bits(TIMER_COUNTS as u8));
    tc0.tccr0b.write(|w| match PRESCALER {
        8 => w.cs0().prescale_8(),
        64 => w.cs0().prescale_64(),
        256 => w.cs0().prescale_256(),
        1024 => w.cs0().prescale_1024(),
        _ => panic!(),
    });
    tc0.timsk0.write(|w| w.ocie0a().set_bit());

    // Reset the global millisecond counter
    avr_device::interrupt::free(|cs| {
        MILLIS_COUNTER.borrow(cs).set(0);
    });
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MILLIS_INCREMENT);
    })
}

fn millis() -> u32 {
    avr_device::interrupt::free(|cs| MILLIS_COUNTER.borrow(cs).get())
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Configure INT0 for falling edge. 0x03 would be rising edge.
    dp.EXINT.eicra.modify(|_, w| w.isc0().bits(0x03));
    // Enable the INT0 interrupt source.
    dp.EXINT.eimsk.modify(|_, w| w.int0().set_bit());

    pins.d2.into_pull_up_input();

    // let mut status_led = pins.d13.into_output();
    // status_led.set_low();

    millis_init(&dp.TC0);
    let timer = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut pwm_led = pins.d3.into_output().into_pwm(&timer);

    unsafe { avr_device::interrupt::enable() };

    loop {
        pulse(&mut pwm_led);
        sos(&mut pwm_led);

        // status_led.set_high();
        // arduino_hal::delay_ms(1000);
        // status_led.set_low();
    }
}
