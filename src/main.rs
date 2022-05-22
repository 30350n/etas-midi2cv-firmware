#![no_std]
#![no_main]

extern crate panic_probe;

extern crate rtt_target;
use rtt_target::{rtt_init_print, rprintln};

extern crate cortex_m_rt;
use cortex_m_rt::entry;

extern crate stm32f1xx_hal;
use stm32f1xx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut flash = pac.FLASH.constrain();
    let rcc = pac.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(8.MHz()).sysclk(72.MHz()).freeze(&mut flash.acr);
    
    let mut delay = core.SYST.delay(&clocks);

    let mut gpioc = pac.GPIOC.split();
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    loop {
        rprintln!("blink");
        led.set_low();
        delay.delay_ms(100u16);
        led.set_high();
        delay.delay_ms(100u16);
        led.set_low();
        delay.delay_ms(100u16);
        led.set_high();
        delay.delay_ms(700u16);
    }
}
