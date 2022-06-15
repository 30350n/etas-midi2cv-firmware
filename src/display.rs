use crate::binary_display::AnalogOutputPinArray;

use stm32f1xx_hal::afio::MAPR;
use stm32f1xx_hal::gpio::{gpiob, Alternate, PushPull};
use stm32f1xx_hal::pac::{TIM3, TIM4};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::rcc::Clocks;
use stm32f1xx_hal::timer::{
    Ch, Channel, PwmHz, Tim3PartialRemap, Tim4NoRemap, Timer, C1, C2, C3, C4,
};

pub struct DisplayPins {
    pwm_tim3: PwmHz<TIM3, Tim3PartialRemap, Ch<C2>, Led1>,
    pwm_tim4: PwmHz<TIM4, Tim4NoRemap, TIM4Channels, TIM4Pins>,
}

impl DisplayPins {
    pub fn new(pins: Leds, tim3: TIM3, tim4: TIM4, clocks: &Clocks, mapr: &mut MAPR) -> Self {
        let mut pwm_tim3 = Timer::new(tim3, &clocks).pwm_hz(pins.0, mapr, 100.kHz());
        let tim4_pins = (pins.1, pins.2, pins.3, pins.4);
        let mut pwm_tim4 = Timer::new(tim4, &clocks).pwm_hz(tim4_pins, mapr, 100.kHz());

        pwm_tim3.enable(Channel::C2);
        pwm_tim4.enable(Channel::C1);
        pwm_tim4.enable(Channel::C2);
        pwm_tim4.enable(Channel::C3);
        pwm_tim4.enable(Channel::C4);

        return Self { pwm_tim3, pwm_tim4 };
    }
}

impl AnalogOutputPinArray<5> for DisplayPins {
    fn set(&mut self, index: u8, value: u16) {
        let max_duty = match index {
            0 => match self.pwm_tim3.get_max_duty() {
                0 => 2u32.pow(16),
                x => x as u32,
            },
            1 | 2 | 3 | 4 => match self.pwm_tim4.get_max_duty() {
                0 => 2u32.pow(16),
                x => x as u32,
            },
            _ => panic!("index {} out of range (0-4)", index),
        };
        let duty = ((u16::MAX - value) as u32 * max_duty / u16::MAX as u32) as u16;
        match index {
            0 => self.pwm_tim3.set_duty(Channel::C2, duty),
            1 => self.pwm_tim4.set_duty(Channel::C1, duty),
            2 => self.pwm_tim4.set_duty(Channel::C2, duty),
            3 => self.pwm_tim4.set_duty(Channel::C3, duty),
            4 => self.pwm_tim4.set_duty(Channel::C4, duty),
            _ => (),
        }
    }
}

type Led1 = gpiob::PB5<Alternate<PushPull>>;
type Led2 = gpiob::PB6<Alternate<PushPull>>;
type Led3 = gpiob::PB7<Alternate<PushPull>>;
type Led4 = gpiob::PB8<Alternate<PushPull>>;
type Led5 = gpiob::PB9<Alternate<PushPull>>;
type Leds = (Led1, Led2, Led3, Led4, Led5);

type TIM4Pins = (Led2, Led3, Led4, Led5);
type TIM4Channels = (Ch<C1>, Ch<C2>, Ch<C3>, Ch<C4>);
