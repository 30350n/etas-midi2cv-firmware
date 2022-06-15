use core::convert::Infallible;
use core::option::Option;
use embedded_hal::digital::v2::{InputPin, PinState};

#[derive(PartialEq, Eq, Debug)]
pub enum Event {
    Unpressed,
    Pressed,
    Down,
    DownLong,
    Up,
    UpLong,
}

pub struct Button<IPIN> {
    pin: IPIN,
    long_press_threshold: u32,
    state: PinState,
    polls_since_pressed: u32,
    is_long_press: bool,
    ignore_next: bool,
}

const ACTIVE: PinState = PinState::Low;

#[allow(dead_code)]
impl<IPIN> Button<IPIN>
where
    IPIN: InputPin<Error = Infallible>,
{
    pub fn new(pin: IPIN, long_press_threshold: Option<u32>) -> Self {
        return Self {
            pin,
            long_press_threshold: long_press_threshold.unwrap_or(u32::MAX),
            state: PinState::High,
            polls_since_pressed: 0,
            is_long_press: false,
            ignore_next: false,
        };
    }

    pub fn poll(&mut self) -> Event {
        let last_state = self.state;
        self.state = self.read().into();

        if self.ignore_next {
            if last_state == ACTIVE && self.state == !ACTIVE {
                self.ignore_next = false;
            }
            return Event::Unpressed;
        }

        self.polls_since_pressed += 1;

        if last_state == !ACTIVE && self.state == ACTIVE {
            self.polls_since_pressed = 0;
            self.is_long_press = false;
            return Event::Down;
        }
        else if last_state == ACTIVE && self.state == !ACTIVE {
            if self.is_long_press {
                return Event::UpLong;
            }
            else {
                return Event::Up;
            }
        }
        else if last_state == ACTIVE && self.state == ACTIVE {
            if self.polls_since_pressed > self.long_press_threshold && !self.is_long_press {
                self.is_long_press = true;
                return Event::DownLong;
            }
            else {
                return Event::Pressed;
            }
        }
        else {
            return Event::Unpressed;
        }
    }

    pub fn read(&self) -> bool {
        return self.pin.is_high().unwrap();
    }

    pub fn ignore_next_press(&mut self) {
        self.ignore_next = true;
    }
}
