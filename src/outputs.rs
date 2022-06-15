use embedded_hal::spi::{Mode, MODE_0};
use fixed::types::U16F16;
use fugit::*;
use mcp49xx::marker::{DualChannel, Resolution12Bit, Unbuffered};
use mcp49xx::{Channel, Command, Mcp49xx};
use stm32f1xx_hal::gpio::{gpioa, gpiob, Alternate, ErasedPin, Output, PinState, PushPull};
use stm32f1xx_hal::pac::SPI1;
use stm32f1xx_hal::spi::{NoMiso, Spi, Spi1NoRemap};

pub struct Outputs {
    gate_pins: [ErasedPin<Output<PushPull>>; 6],
    spi: OutputsSpi,
    dac: Dac,
}

impl Outputs {
    pub fn new(gates: PinsGate, spi: OutputsSpi, dac: Dac) -> Self {
        let mut gate_pins = [
            gates.0.erase(),
            gates.1.erase(),
            gates.2.erase(),
            gates.3.erase(),
            gates.4.erase(),
            gates.5.erase(),
        ];

        for gate_pin in &mut gate_pins {
            gate_pin.set_high();
        }

        return Self { gate_pins, spi, dac };
    }

    const ROOT_NOTE: u8 = 24; // C1
    pub fn set_cv_note(&mut self, channel: Cv, note: u8) {
        let note_voltage = U16F16::from_num(note.saturating_sub(Self::ROOT_NOTE)) / 12;
        self.dac.set_voltage(note_voltage, channel.into(), &mut self.spi);
    }

    pub fn set_cv7(&mut self, channel: Cv, value: u8) {
        let voltage = U16F16::from_num(value) / 127 * 8;
        self.dac.set_voltage(voltage, channel.into(), &mut self.spi);
    }

    pub fn set_gate(&mut self, gate: Gate, value: bool) {
        let state = if value { PinState::Low } else { PinState::High };
        self.gate_pins[gate as usize].set_state(state);
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Gate {
    G1,
    G2,
    G3,
    G4,
    G5,
    G6,
}

impl From<u8> for Gate {
    fn from(n: u8) -> Self {
        return [Self::G1, Self::G2, Self::G3, Self::G4, Self::G5, Self::G6][n as usize];
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Cv {
    Cv1,
    Cv2,
    Cv3,
    Cv4,
}

impl From<u8> for Cv {
    fn from(n: u8) -> Self {
        return [Self::Cv1, Self::Cv2, Self::Cv3, Self::Cv4][n as usize];
    }
}

impl Into<DacChannel> for Cv {
    fn into(self) -> DacChannel {
        return match self {
            Self::Cv1 => DacChannel::C0,
            Self::Cv2 => DacChannel::C1,
            Self::Cv3 => DacChannel::C2,
            Self::Cv4 => DacChannel::C3,
        };
    }
}

type OutputsSpi = Spi<SPI1, Spi1NoRemap, (PinSpiSck, NoMiso, PinSpiMopi), u8>;

type PinGate1 = gpiob::PB0<Output<PushPull>>;
type PinGate2 = gpiob::PB1<Output<PushPull>>;
type PinGate3 = gpiob::PB12<Output<PushPull>>;
type PinGate4 = gpiob::PB13<Output<PushPull>>;
type PinGate5 = gpiob::PB14<Output<PushPull>>;
type PinGate6 = gpiob::PB15<Output<PushPull>>;
type PinsGate = (PinGate1, PinGate2, PinGate3, PinGate4, PinGate5, PinGate6);

pub struct Dac {
    dac_1: Mcp49xx<PinDac1Cs, OutputsSpi, Resolution12Bit, DualChannel, Unbuffered>,
    dac_2: Mcp49xx<PinDac2Cs, OutputsSpi, Resolution12Bit, DualChannel, Unbuffered>,
    calibration: [[u16; Self::CAL_LEVELS.len()]; 4],
}

impl Dac {
    pub const N_CHANNELS: u8 = 4;
    pub const CAL_LEVELS: [u8; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
    pub const DEFAULT_CAL: [u16; 9] = [0, 500, 1000, 1500, 2000, 2500, 3000, 3500, 4000];
    pub const SPI_MODE: Mode = MODE_0;
    pub const SPI_FREQ: HertzU32 = HertzU32::MHz(9);

    pub fn new(cs1: PinDac1Cs, cs2: PinDac2Cs) -> Self {
        return Self {
            dac_1: Mcp49xx::new_mcp4822(cs1),
            dac_2: Mcp49xx::new_mcp4822(cs2),
            calibration: [Self::DEFAULT_CAL; 4],
        };
    }

    pub fn set(&mut self, value: u16, channel: DacChannel, spi: &mut OutputsSpi) {
        let cmd = Command::default().double_gain().value(value);
        match channel {
            DacChannel::C0 => self.dac_1.send(spi, cmd.channel(Channel::Ch0)).unwrap(),
            DacChannel::C1 => self.dac_1.send(spi, cmd.channel(Channel::Ch1)).unwrap(),
            DacChannel::C2 => self.dac_2.send(spi, cmd.channel(Channel::Ch0)).unwrap(),
            DacChannel::C3 => self.dac_2.send(spi, cmd.channel(Channel::Ch1)).unwrap(),
        }
    }

    pub fn set_voltage(&mut self, value: U16F16, channel: DacChannel, spi: &mut OutputsSpi) {
        let cal = self.calibration[channel as usize];
        let index: usize = value.to_num();
        if index >= (Self::CAL_LEVELS.len() - 1) {
            return self.set(cal[8], channel, spi);
        }

        let x0 = U16F16::from_num(cal[index]);
        let x1 = U16F16::from_num(cal[index + 1]);
        self.set((value % 1).lerp(x0, x1).to_num(), channel, spi);
    }
}

type PinDac1Cs = gpioa::PA4<Output<PushPull>>;
type PinDac2Cs = gpioa::PA3<Output<PushPull>>;
type PinSpiSck = gpioa::PA5<Alternate<PushPull>>;
type PinSpiMopi = gpioa::PA7<Alternate<PushPull>>;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum DacChannel {
    C0,
    C1,
    C2,
    C3,
}
