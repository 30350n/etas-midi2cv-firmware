use cordic::{exp, sin};
use fixed::const_fixed_from_int;
use fixed::types::{I18F14, I20F12};
use fugit::*;

pub type Millihertz = Rate<u32, 1, 1000>;

#[derive(Debug)]
pub struct BinaryDisplay<const N_BITS: u8, PINS> {
    pins: PINS,
    value: u8,
    off_level: u16,
    on_level: u16,
    is_breathing: bool,
    breathing_amplitude: I20F12,
    frequency: Millihertz,
    time: u32,
}

#[allow(dead_code)]
impl<const N_BITS: u8, PINS> BinaryDisplay<N_BITS, PINS>
where
    PINS: AnalogOutputPinArray<N_BITS>,
{
    pub fn new(output_pins: PINS) -> Self {
        return Self {
            pins: output_pins,
            value: 0,
            off_level: 0,
            on_level: u16::MAX,
            is_breathing: false,
            breathing_amplitude: I20F12::ZERO,
            frequency: 1u32.Hz(),
            time: 0,
        };
    }

    pub fn set(&mut self, n: u8) {
        self.value = n;
    }

    pub fn set_bit(&mut self, n: u8) {
        self.value |= 1 << n;
    }

    pub fn clear_bit(&mut self, n: u8) {
        self.value &= !(1 << n);
    }

    pub fn update(&mut self, delta_time: MillisDurationU32) {
        let mut breathing_offset = 0;
        if self.is_breathing {
            self.time = self.time.wrapping_add(delta_time.to_millis());
            let f_rec = self.frequency.into_duration::<1, 1000>().to_millis();
            let x = I20F12::from_num(self.time) / I20F12::from_num(f_rec);
            let y = (sin(x * I20F12::PI * 2) + I20F12::ONE) / 2;
            breathing_offset = (y * self.breathing_amplitude).to_num::<u16>();
        }

        for i in 0..N_BITS {
            let bit = (self.value >> i) & 1;
            let level = if bit == 0 { self.off_level } else { self.on_level };
            self.pins.set(i, Self::correct_brightness(level + breathing_offset));
        }
    }

    pub fn brightness_test(&mut self, delta_time: MillisDurationU32) {
        self.time = self.time.wrapping_add(delta_time.to_millis());
        let y = (self.time as u32 % 2000) * u16::MAX as u32 / 2000;
        for i in 0..N_BITS {
            self.pins.set(i, Self::correct_brightness(y as u16));
        }
    }

    fn correct_brightness(value: u16) -> u16 {
        const_fixed_from_int! {
            const U16_MAX: I18F14 = u16::MAX as i32;
            const ONE: I18F14 = 1;
            const SLOPE: I18F14 = 5;
            const LOW_ADJUST_SLOPE: I18F14 = 15;
            const LOW_ADJUST_AMOUNT: I18F14 = 100;
        }
        let x = I18F14::from_num(value) / U16_MAX;
        let mut y = (exp(x * SLOPE) - ONE) / (exp(SLOPE) - ONE);
        y += (ONE - x) * (ONE - ONE / (ONE + x * LOW_ADJUST_SLOPE)) / LOW_ADJUST_AMOUNT;
        return (y * U16_MAX).to_num();
    }

    pub fn enable_breathing(&mut self, frequency: Millihertz, amplitude: u16, margin: u16) {
        self.is_breathing = true;
        self.frequency = frequency;
        self.breathing_amplitude = I20F12::from_num(amplitude);
        self.off_level = margin;
        self.on_level = u16::MAX - (amplitude + margin);
    }

    pub fn disable_breathing(&mut self) {
        self.is_breathing = false;
        self.breathing_amplitude = I20F12::ZERO;
        self.off_level = 0;
        self.on_level = u16::MAX;
    }
}

pub trait AnalogOutputPinArray<const N: u8>: Sized {
    fn set(&mut self, index: u8, value: u16);
}
