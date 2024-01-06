use crate::dbgprint;
use avr_device::interrupt::Mutex;
use core::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    task::Waker,
};
use heapless::binary_heap::Min;

const FREQ_CPU: u32 = 16_000_000;
const CLOCK_CYCLES_PER_MICROSECOND: u32 = FREQ_CPU / 1_000_000;
const PRESCALER: u32 = 64;

const fn clock_cycles_to_microseconds(cycles: u32) -> u32 {
    cycles / CLOCK_CYCLES_PER_MICROSECOND
}

// the prescaler is set so that timer0 ticks every 64 clock cycles, and the
// the overflow handler is called every 256 ticks.
// Should be 1024.
const MICROSECONDS_PER_TIMER0_OVERFLOW: u32 = clock_cycles_to_microseconds(PRESCALER * 256);

// the whole number of milliseconds per timer0 overflow
const MILLIS_INC: u32 = MICROSECONDS_PER_TIMER0_OVERFLOW / 1000;

// the fractional number of milliseconds per timer0 overflow. we shift right
// by three to fit these numbers into a byte. (for the clock speeds we care
// about - 8 and 16 MHz - this doesn't lose precision.)
const FRACT_INC: u8 = ((MICROSECONDS_PER_TIMER0_OVERFLOW % 1000) >> 3) as u8;
const FRACT_MAX: u8 = (1000 >> 3) as u8;

static TIMER0_OVERFLOW_COUNT: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
static TIMER0_MILLIS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

pub fn millis_init(tc0: &arduino_hal::pac::TC0) {
    // Configure the timer for the above interval (in CTC mode)
    // and enable its interrupt.
    tc0.tccr0a.write(|w| w.wgm0().pwm_fast());
    // tc0.ocr0a.write(|w| w.bits(TIMER_COUNTS as u8));
    tc0.tccr0b.write(|w| match PRESCALER {
        8 => w.cs0().prescale_8(),
        64 => w.cs0().prescale_64(),
        256 => w.cs0().prescale_256(),
        1024 => w.cs0().prescale_1024(),
        _ => panic!(),
    });
    tc0.timsk0.write(|w| w.toie0().set_bit());

    // Reset the global millisecond counter
    avr_device::interrupt::free(|cs| {
        TIMER0_MILLIS.borrow(cs).set(0);
    });
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_OVF() {
    avr_device::interrupt::free(|cs| {
        static TIMER0_FRACT: Mutex<Cell<u8>> = Mutex::new(Cell::new(0));
        let mut f = TIMER0_FRACT.borrow(cs).get();
        let mut m = TIMER0_MILLIS.borrow(cs).get();
        let overflow_count = TIMER0_OVERFLOW_COUNT.borrow(cs).get();

        m = m.wrapping_add(MILLIS_INC);
        f = f.wrapping_add(FRACT_INC);

        if f >= FRACT_MAX {
            f -= FRACT_MAX;
            m += 1;
        }
        TIMER0_FRACT.borrow(cs).set(f);
        TIMER0_MILLIS.borrow(cs).set(m);
        TIMER0_OVERFLOW_COUNT.borrow(cs).set(overflow_count + 1);

        WAKERS.borrow(cs).borrow_mut().wake_all_before(m);
    })
}

#[derive(Debug)]
pub struct WakersHeapEntry {
    wake_time: u32,
    id: u16,
    waker: Waker,
}

impl PartialEq for WakersHeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.wake_time == other.wake_time
    }
}

impl Eq for WakersHeapEntry {}

impl PartialOrd for WakersHeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.wake_time.partial_cmp(&other.wake_time)
    }
}

impl Ord for WakersHeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wake_time.cmp(&other.wake_time)
    }
}

pub struct WakersHeap {
    heap: heapless::BinaryHeap<WakersHeapEntry, Min, 10>,
}

fn wake_time_has_passed(entry: &WakersHeapEntry, now: u32) -> bool {
    now >= entry.wake_time
}

impl WakersHeap {
    const fn new() -> Self {
        Self {
            heap: heapless::BinaryHeap::new(),
        }
    }

    pub fn has_capacity(&self) -> bool {
        self.heap.len() < self.heap.capacity()
    }

    pub fn replace_or_push(&mut self, wake_time: u32, id: u16, waker: Waker) -> Result<(), ()> {
        for entry in self.heap.iter_mut() {
            if entry.id == id {
                entry.waker = waker;
                return Ok(());
            }
        }
        match self.heap.push(WakersHeapEntry {
            wake_time,
            id,
            waker,
        }) {
            Ok(_) => Ok(()),
            Err(_) => {
                dbgprint!("Push NOT Ok!");
                Err(())
            }
        }
    }

    fn wake_all_before(&mut self, now: u32) {
        while let Some(entry) = self.heap.peek() {
            if !wake_time_has_passed(entry, now) {
                break;
            }
            entry.waker.wake_by_ref();
            self.heap.pop();
        }
    }
}

pub static WAKERS: Mutex<RefCell<WakersHeap>> = Mutex::new(RefCell::new(WakersHeap::new()));

pub fn millis() -> u32 {
    avr_device::interrupt::free(|cs| TIMER0_MILLIS.borrow(cs).get())
}
