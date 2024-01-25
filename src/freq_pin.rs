use arduino_hal::{pac::TC2, simple_pwm::Prescaler, port::{Pin, mode::Output}, hal::port::PD3};

pub struct Timer2Freq {
    timer: TC2,
}

impl Timer2Freq {
    pub fn new(timer: TC2, prescaler: Prescaler) -> Timer2Freq {
        let mut t = Timer2Freq { timer };

        {
            let tim = &mut t.timer;
            let prescaler = prescaler;
            tim.tccr2a.modify(|_r, w| w.wgm2().ctc());
            tim.tccr2b.modify(|_r, w| match prescaler {
                    Prescaler::Direct => w.cs2().direct(),
                    Prescaler::Prescale8 => w.cs2().prescale_8(),
                    Prescaler::Prescale64 => w.cs2().prescale_64(),
                    Prescaler::Prescale256 => w.cs2().prescale_256(),
                    Prescaler::Prescale1024 => w.cs2().prescale_1024(),
            });
        }

        t
    }
}

pub struct FreqPinPD3<'a> {
    _pin: Pin<Output, PD3>,
    timer: &'a Timer2Freq,
}

impl FreqPinPD3<'_> {
    pub fn new(timer: &Timer2Freq, pin: Pin<Output, PD3>) -> FreqPinPD3 {
        FreqPinPD3 { timer, _pin: pin }
    }

    pub fn enable(&mut self) {
        self.timer.timer.tccr2a.modify(|_r, w| w.com2b().match_toggle());
    }

    pub fn disable(&mut self) {
        self.timer.timer.tccr2a.modify(|_r, w| w.com2b().disconnected());
    }

    pub fn set_freq(&mut self, freq: u16) {
        let (prescaler, numerator) = match freq {
            40..=199 => (Prescaler::Prescale1024, 15625),
            200..=999 => (Prescaler::Prescale256, 62500),
            1000..=4799 => (Prescaler::Prescale64, 250_000),
            4800..=35000 => (Prescaler::Prescale8, 2_000_000),
            _ => panic!(),
        };
        self.timer.timer.tccr2b.modify(|_r, w| match prescaler {
            Prescaler::Direct => w.cs2().direct(),
            Prescaler::Prescale8 => w.cs2().prescale_8(),
            Prescaler::Prescale64 => w.cs2().prescale_64(),
            Prescaler::Prescale256 => w.cs2().prescale_256(),
            Prescaler::Prescale1024 => w.cs2().prescale_1024(),
        });
        let reg = (numerator / (2 * freq) as u32) as u8;
        self.timer.timer.ocr2a.write(|w| w.bits(reg));
    }
}
