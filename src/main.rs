#![no_std]
#![no_main]

use binary_display::{BinaryDisplay, Millihertz};
use display::DisplayPins;
use interrupt::{Context, Menu, CONTEXT, PERIPHERALS};
use modes::*;
use outputs::{Dac, Outputs};
use settings::Settings;

use cortex_m_rt::entry;
use dwt_systick_monotonic::DwtSystick;
use embedded_midi::MidiIn;
use fugit::MicrosDurationU32;
use rtic_monotonic::Monotonic;
use rtt_target::{rprintln, rtt_init_print};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::spi::{NoMiso, Spi};
use stm32f1xx_hal::{pac, serial};

const N_MODES: usize = 1;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();

    let mut flash = pac.FLASH.constrain();
    let rcc = pac.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(8.MHz()).sysclk(72.MHz()).freeze(&mut flash.acr);

    let mut gpioa = pac.GPIOA.split();
    let mut gpiob = pac.GPIOB.split();
    let mut afio = pac.AFIO.constrain();

    let settings = Settings::default();

    let mut modes: [&mut dyn Mode; N_MODES] = [&mut Mono::default()];

    let led_pins = (
        gpiob.pb5.into_alternate_push_pull(&mut gpiob.crl),
        gpiob.pb6.into_alternate_push_pull(&mut gpiob.crl),
        gpiob.pb7.into_alternate_push_pull(&mut gpiob.crl),
        gpiob.pb8.into_alternate_push_pull(&mut gpiob.crh),
        gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh),
    );
    let display_pins = DisplayPins::new(led_pins, pac.TIM3, pac.TIM4, &clocks, &mut afio.mapr);
    let mut display = BinaryDisplay::new(display_pins);

    let (_pa15, pb3, pb4) = afio.mapr.disable_jtag(gpioa.pa15, gpiob.pb3, gpiob.pb4);
    let pb3 = pb3.into_pull_up_input(&mut gpiob.crl);
    let pb4 = pb4.into_pull_up_input(&mut gpiob.crl);
    let mut isr_peripherals = interrupt::Peripherals::new((pb3, pb4), pac.TIM2, &clocks);
    let menu = if isr_peripherals.do_calibrate() { Menu::Calibration } else { Menu::Main };
    cortex_m::interrupt::free(|cs| PERIPHERALS.borrow(cs).set(Some(isr_peripherals)));
    unsafe {
        interrupt::Peripherals::enable_isr();
    }

    let context = Context::new(menu);
    cortex_m::interrupt::free(|cs| CONTEXT.borrow(cs).set(context));

    let gate_pins = (
        gpiob.pb0.into_push_pull_output(&mut gpiob.crl),
        gpiob.pb1.into_push_pull_output(&mut gpiob.crl),
        gpiob.pb12.into_push_pull_output(&mut gpiob.crh),
        gpiob.pb13.into_push_pull_output(&mut gpiob.crh),
        gpiob.pb14.into_push_pull_output(&mut gpiob.crh),
        gpiob.pb15.into_push_pull_output(&mut gpiob.crh),
    );
    let cs1 = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
    let cs2 = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);
    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let mopi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
    let spi = Spi::spi1(
        pac.SPI1,
        (sck, NoMiso, mopi),
        &mut afio.mapr,
        Dac::SPI_MODE,
        Dac::SPI_FREQ,
        clocks,
    );
    let dac = Dac::new(cs1, cs2);
    let mut outputs = Outputs::new(gate_pins, spi, dac);

    let sysclk = clocks.sysclk().raw();
    let mut timer = DwtSystick::<72_000_000>::new(&mut core.DCB, core.DWT, core.SYST, sysclk);
    unsafe {
        timer.reset();
    }

    let pin_tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
    let pin_rx = gpioa.pa10;
    let usart = serial::Serial::usart1(
        pac.USART1,
        (pin_tx, pin_rx),
        &mut afio.mapr,
        serial::Config::default().baudrate(31250.bps()).parity_none(),
        clocks,
    );
    let (_tx, rx) = usart.split();
    let mut midi_in = MidiIn::new(rx);

    let mut last_time = timer.now();
    let mut last_context = context;
    loop {
        let now = timer.now();
        let delta_time = MicrosDurationU32::micros((now - last_time).to_micros() as u32);
        last_time = now;

        let context = cortex_m::interrupt::free(|cs| CONTEXT.borrow(cs).get());
        let mode = &mut modes[context.mode as usize];

        match midi_in.read() {
            Ok(message) => {
                rprintln!("message {:?}", message);
                match context.menu {
                    Menu::Calibration => (),
                    Menu::MidiLearn => mode.handle_midi_learn(message, &mut outputs),
                    _ => mode.handle_midi_event(message, &mut outputs, &settings),
                }
            },
            Err(_) => (),
        }
        mode.update(delta_time.convert(), &mut outputs);

        if last_context.menu != context.menu {
            match context.menu {
                Menu::Main => {
                    display.disable_breathing();
                },
                Menu::Calibration => {
                    display.enable_breathing(7.Hz(), 15000, 4000);
                },
                Menu::MidiLearn => {
                    display.enable_breathing(2.Hz(), 10000, 4000);
                },
                Menu::Settings => {
                    display.enable_breathing(Millihertz::from_raw(500), 20000, 4000);
                },
                Menu::SettingEdit => {
                    display.enable_breathing(1.Hz(), 20000, 4000);
                },
            }
        }
        last_context = context;

        match context.menu {
            Menu::Main | Menu::MidiLearn => {
                display.set(0);
            },
            Menu::Calibration => {
                display.set(context.cal_level as u8);
            },
            Menu::Settings => {
                display.set(context.setting as u8 + 1);
            },
            Menu::SettingEdit => {
                display.set(0);
            },
        }
        display.update(delta_time.convert());
    }
}

extern crate cortex_m;
extern crate cortex_m_rt;
extern crate rtt_target;

extern crate embedded_hal;

extern crate stm32f1xx_hal;

extern crate panic_probe;

extern crate embedded_midi;

extern crate cordic;
extern crate fixed;

extern crate dwt_systick_monotonic;
extern crate fugit;
extern crate rtic_monotonic;

extern crate mcp49xx;

mod binary_display;
mod button;
mod display;
mod interrupt;
mod modes;
mod outputs;
mod settings;
