use crate::button::{Button, Event};
use crate::outputs::Dac;
use crate::N_MODES;

use stm32f1xx_hal::gpio::{gpiob, Input, PullUp};
use stm32f1xx_hal::pac::{interrupt, Interrupt, TIM2};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::rcc::Clocks;
use stm32f1xx_hal::timer::{CounterHz, Event as TimerEvent};

use cortex_m::interrupt::Mutex;

use core::cell::Cell;

pub static PERIPHERALS: Mutex<Cell<Option<Peripherals>>> = Mutex::new(Cell::new(None));
pub static CONTEXT: Mutex<Cell<Context>> = Mutex::new(Cell::new(Context::new(Menu::Main)));

const LONG_PRESS_DELAY_MS: u32 = 600;

#[interrupt]
fn TIM2() {
    let mut periphs = cortex_m::interrupt::free(|cs| PERIPHERALS.borrow(cs).take()).unwrap();
    periphs.timer.clear_interrupt(TimerEvent::Update);

    let mut context = cortex_m::interrupt::free(|cs| CONTEXT.borrow(cs).get());

    let button_event_a = periphs.button_a.poll();
    let button_event_b = periphs.button_b.poll();
    match context.menu {
        Menu::Main => {
            match button_event_a {
                Event::Up => context.mode -= 1,
                Event::UpLong => context.menu = Menu::Settings,
                _ => (),
            }
            match button_event_b {
                Event::Up => context.mode += 1,
                Event::UpLong => context.menu = Menu::MidiLearn,
                _ => (),
            }
            context.mode = context.mode.rem_euclid(N_MODES as i8);
        },
        Menu::Calibration => {
            match button_event_a {
                Event::Up => context.cal_level -= 1,
                Event::UpLong => context.menu = Menu::Main,
                _ => (),
            }
            match button_event_b {
                Event::Up => context.cal_level += 1,
                Event::UpLong => context.cal_channel += 1,
                _ => (),
            }
            context.cal_level = context.cal_level.rem_euclid(Dac::CAL_LEVELS.len() as i8);
            context.cal_channel = context.cal_channel.rem_euclid(Dac::N_CHANNELS as i8);
        },
        Menu::MidiLearn => {
            match button_event_a {
                Event::UpLong => context.menu = Menu::Main,
                _ => (),
            }
            match button_event_b {
                _ => (),
            }
        },
        Menu::Settings => {
            match button_event_a {
                Event::Up => context.setting -= 1,
                Event::UpLong => context.menu = Menu::Main,
                _ => (),
            }
            match button_event_b {
                Event::Up => context.setting += 1,
                Event::UpLong => context.menu = Menu::SettingEdit,
                _ => (),
            }
            context.setting = context.setting.rem_euclid(5);
        },
        Menu::SettingEdit => {
            match button_event_a {
                Event::UpLong => context.menu = Menu::Settings,
                _ => (),
            }
            match button_event_b {
                _ => (),
            }
        },
    }

    cortex_m::interrupt::free(|cs| CONTEXT.borrow(cs).set(context));
    cortex_m::interrupt::free(|cs| PERIPHERALS.borrow(cs).set(Some(periphs)));
}

pub struct Peripherals {
    timer: CounterHz<TIM2>,
    button_a: Button<PinButtonA>,
    button_b: Button<PinButtonB>,
}

impl Peripherals {
    pub fn new(button_pins: (PinButtonA, PinButtonB), tim2: TIM2, clocks: &Clocks) -> Self {
        let mut timer = tim2.counter_hz(clocks);
        timer.start(1.kHz()).unwrap();
        timer.listen(TimerEvent::Update);
        return Self {
            timer,
            button_a: Button::new(button_pins.0, Some(LONG_PRESS_DELAY_MS)),
            button_b: Button::new(button_pins.1, Some(LONG_PRESS_DELAY_MS)),
        };
    }

    pub fn do_calibrate(&mut self) -> bool {
        if !self.button_a.read() && !self.button_b.read() {
            self.button_a.ignore_next_press();
            self.button_b.ignore_next_press();
            return true;
        }
        return false;
    }

    pub unsafe fn enable_isr() {
        cortex_m::peripheral::NVIC::unmask(Interrupt::TIM2);
    }
}

type PinButtonA = gpiob::PB3<Input<PullUp>>;
type PinButtonB = gpiob::PB4<Input<PullUp>>;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Menu {
    Main,
    Calibration,
    MidiLearn,
    Settings,
    SettingEdit,
}

#[derive(Copy, Clone, Debug)]
pub struct Context {
    pub menu: Menu,
    pub mode: i8,
    pub setting: i8,
    pub cal_level: i8,
    pub cal_channel: i8,
}

impl Context {
    pub const fn new(default_menu: Menu) -> Self {
        return Self { menu: default_menu, mode: 0, setting: 0, cal_level: 1, cal_channel: 0 };
    }
}
