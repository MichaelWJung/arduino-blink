
use core::cell::RefCell;
use core::ops::Coroutine;
use core::iter;

use crate::ag_lcd::LcdDisplay;
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::OutputPin;
use heapless::String;

use crate::futures::delay::Delay;

const DISPLAY_WIDTH: usize = 16;

fn generate_moving_text(text: &str) -> impl Coroutine<Yield = String<DISPLAY_WIDTH>, Return = ()> + '_
{
    || {
        let space: usize = DISPLAY_WIDTH - text.len();
        loop {
            for x in (0..space).chain((1..=space).rev()) {
                let space_left = x;
                let space_right = space - x;
                let mut line: String<DISPLAY_WIDTH> = String::new();
                for _ in 0..space_left {
                    let _ = line.push(' ');
                }
                let _ = line.push_str(text);
                for _ in 0..space_right {
                    let _ = line.push(' ');
                }
                yield line;
            }
        }
    }
}

pub async fn show_moving_text<T, D>(text: (&str, &str), lcd: &RefCell<LcdDisplay<T, D>>)
where
    T: OutputPin + Sized,
    D: DelayUs<u16> + Sized,
{
    let line1 = iter::from_coroutine(generate_moving_text(text.0));
    let line2 = iter::from_coroutine(generate_moving_text(text.1));
    for (l1, l2) in iter::zip(line1, line2) {
        let mut lcd = lcd.borrow_mut();
        lcd.set_position(0, 0).await;
        lcd.print(&l1).await;
        lcd.set_position(0, 1).await;
        lcd.print(&l2).await;
        Delay::wait_for(500).await;
    }
}
