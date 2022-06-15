#![allow(dead_code)]

use fugit::*;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub voicing: Voicing,
    pub note_priority: NotePriority,
    pub legato: bool,
    pub trigger_length: TriggerLength,
    pub trigger_scaling: bool,
    pub trigger_shape: TriggerShape,
    pub tuning: Tuning,
}

impl Default for Settings {
    fn default() -> Self {
        return Self {
            voicing: Voicing::Poly,
            note_priority: NotePriority::Latest,
            legato: false,
            trigger_length: TriggerLength::T5ms,
            trigger_scaling: false,
            trigger_shape: TriggerShape::Square,
            tuning: Tuning::EqualTemperament,
        };
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Voicing {
    Poly,
    Cyclic,
    Random,
    Velocity,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum NotePriority {
    Latest,
    First,
    Highest,
    Lowest,
}

#[derive(Clone, Copy, Debug)]
pub enum TriggerLength {
    T50us,
    T500us,
    T1ms,
    T5ms,
    T25ms,
}

impl Into<MicrosDurationU32> for TriggerLength {
    fn into(self) -> MicrosDurationU32 {
        return match self {
            Self::T50us  =>  50u32.micros(),
            Self::T500us => 500u32.micros(),
            Self::T1ms   =>   1u32.millis(),
            Self::T5ms   =>   5u32.millis(),
            Self::T25ms  =>  25u32.millis(),
        };
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TriggerShape {
    Square,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Tuning {
    EqualTemperament,
}
