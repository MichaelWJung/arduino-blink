use core::cell::RefCell;

use ag_lcd::LcdDisplay;
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::OutputPin;
use heapless::String;

use crate::futures::delay::Delay;

const DISPLAY_WIDTH: usize = 16;

pub async fn show_moving_text<T, D>(text: &str, row: u8, lcd: &RefCell<LcdDisplay<T, D>>)
where
    T: OutputPin + Sized,
    D: DelayUs<u16> + Sized,
{
    let space: usize = DISPLAY_WIDTH - text.len();
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
        {
            let mut lcd = lcd.borrow_mut();
            lcd.set_position(0, row);
            lcd.print(&line);
        }
        Delay::wait_for(1000).await;
    }
}
