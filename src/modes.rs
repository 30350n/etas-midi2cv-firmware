use crate::outputs::{Cv, Gate, Outputs};
use crate::settings::{NotePriority, Settings};

use embedded_midi::MidiMessage as Midi;
use fugit::*;
use rtt_target::rprintln;

const MOD_WHEEL_CC: u8 = 1;

#[derive(Default, Debug)]
pub struct Mono {
    settings: MonoSettings,
    voice: Voice<{ Gate::G1 as u8 }, { Cv::Cv1 as u8 }, 8>,
    trigger: Trigger<{ Gate::G2 as u8 }>,
    learn_visualizer: Trigger<{ Gate::G4 as u8 }>,
}

#[derive(Default, Debug)]
struct MonoSettings {
    midi_channel: u8,
    midi_cc: u8,
}

impl Mode for Mono {
    fn handle_midi_event(&mut self, msg: Midi, outputs: &mut Outputs, settings: &Settings) {
        let midi_channel = self.settings.midi_channel.into();
        let midi_cc = self.settings.midi_cc;
        match msg {
            Midi::NoteOn(ch, note, vel) if ch == midi_channel => {
                if self.voice.note_on(note.into(), outputs, settings) {
                    self.trigger.trigger(settings.trigger_length.into(), outputs);
                    outputs.set_cv7(Cv::Cv2, vel.into());
                }
            },
            Midi::NoteOff(ch, note, _) if ch == midi_channel => {
                if self.voice.note_off(note.into(), outputs, settings) {
                    self.trigger.trigger(settings.trigger_length.into(), outputs);
                }
            },
            Midi::ControlChange(ch, cc, val) if ch == midi_channel => match cc.into() {
                MOD_WHEEL_CC => outputs.set_cv7(Cv::Cv3, val.into()),
                cc if cc == midi_cc => outputs.set_cv7(Cv::Cv4, val.into()),
                _ => (),
            },
            _ => (),
        }
    }

    fn handle_midi_learn(&mut self, msg: Midi, outputs: &mut Outputs) {
        let midi_channel = self.settings.midi_channel.into();
        match msg {
            Midi::NoteOn(channel, _, _) => {
                self.settings.midi_channel = channel.into();
            },
            Midi::ControlChange(channel, cc, _)
                if channel == midi_channel && cc != MOD_WHEEL_CC.into() =>
            {
                self.settings.midi_cc = cc.into();
                self.learn_visualizer.trigger(100u32.millis(), outputs);
            },
            _ => (),
        }
    }

    fn update(&mut self, delta_time: MicrosDurationU32, outputs: &mut Outputs) {
        self.trigger.update(delta_time, outputs);
        self.learn_visualizer.update(delta_time, outputs);
    }
}

pub trait Mode {
    fn handle_midi_event(&mut self, msg: Midi, outputs: &mut Outputs, settings: &Settings);
    fn handle_midi_learn(&mut self, msg: Midi, outputs: &mut Outputs);
    #[allow(unused_variables)]
    fn update(&mut self, delta_time: MicrosDurationU32, outputs: &mut Outputs) {}
}

#[derive(Debug)]
struct Voice<const GATE: u8, const CV: u8, const MEMORY: usize> {
    memory: [u8; MEMORY],
    size: usize,
    active: usize,
}

impl<const GATE: u8, const CV: u8, const MEMORY: usize> Default for Voice<GATE, CV, MEMORY> {
    fn default() -> Self {
        return Self { memory: [0; MEMORY], size: 0, active: 0 };
    }
}

impl<const GATE: u8, const CV: u8, const MEMORY: usize> Voice<GATE, CV, MEMORY> {
    fn note_on(&mut self, note: u8, outputs: &mut Outputs, settings: &Settings) -> bool {
        if self.memory[..self.size as usize].contains(&note) {
            // TODO: react differenly
            return false;
        }
        if (self.size as usize) == self.memory.len() {
            return false;
        }

        self.memory[self.size as usize] = note;
        let is_higher = self.memory[self.active as usize] < self.memory[self.size as usize];
        let new_active = match settings.note_priority {
            NotePriority::Latest => self.size,
            NotePriority::First => 0,
            NotePriority::Highest => if is_higher { self.size } else { self.active },
            NotePriority::Lowest => if !is_higher { self.size } else { self.active },
        };
        if self.size == 0 {
            outputs.set_gate(GATE.into(), true);
        }
        if self.size == 0 || new_active != self.active {
            outputs.set_cv_note(CV.into(), note);
        }
        self.active = new_active;
        self.size += 1;

        rprintln!("{:?} {}", self.memory, self.size);
        return !settings.legato || self.size == 1;
    }

    fn note_off(&mut self, note: u8, outputs: &mut Outputs, settings: &Settings) -> bool {
        let mut found = usize::MAX;
        for i in 0..self.size {
            if found != usize::MAX {
                self.memory[i - 1] = self.memory[i];
            }
            else if self.memory[i] == note {
                found = i;
                self.size -= 1;
                break;
            }
        }

        if found == self.active {
            self.active = match settings.note_priority {
                NotePriority::Latest => self.size,
                NotePriority::First => 0,
                NotePriority::Highest => {
                    let mut highest = 0;
                    for i in 0..self.size {
                        if self.memory[i] > self.memory[highest] {
                            highest = i;
                        }
                    }
                    highest
                },
                NotePriority::Lowest => {
                    let mut lowest = 0;
                    for i in 0..self.size {
                        if self.memory[i] < self.memory[lowest] {
                            lowest = i;
                        }
                    }
                    lowest
                },
            };

            outputs.set_cv_note(CV.into(), self.memory[self.active]);
            if self.size == 0 {
                outputs.set_gate(GATE.into(), false);
            }
        }

        rprintln!("{:?} {}", self.memory, self.size);
        return found != usize::MAX && !settings.legato && self.size > 0;
    }
}

#[derive(Default, Debug)]
struct Trigger<const GATE: u8> {
    time: u32,
    length: u32,
    is_active: bool,
}

impl<const GATE: u8> Trigger<GATE> {
    fn trigger(&mut self, length: MicrosDurationU32, outputs: &mut Outputs) {
        self.time = 0;
        self.length = length.to_micros();
        self.is_active = true;
        outputs.set_gate(Gate::from(GATE), true);
        rprintln!("trigger on");
    }

    fn update(&mut self, delta_time: MicrosDurationU32, outputs: &mut Outputs) {
        if self.is_active {
            self.time += delta_time.to_micros();
            if self.time > self.length {
                self.is_active = false;
                outputs.set_gate(Gate::from(GATE), false);
                rprintln!("trigger off");
            }
        }
    }
}
