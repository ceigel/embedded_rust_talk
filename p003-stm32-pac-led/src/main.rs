#![no_std]
#![no_main]
extern crate panic_rtt_target;

use cortex_m::asm;
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init_print};
use stm32f3::stm32f303 as pac;

#[entry]
fn entrypoint() -> ! {
    rtt_init_print!();
    let dp = pac::Peripherals::take().unwrap();

    let rcc = dp.RCC;
    rcc.ahbenr.write(|w| w.iopeen().set_bit());

    let gpioe = dp.GPIOE;
    gpioe.moder.write(|w| w.moder9().output());

    rprintln!("Setup done");
    loop {
        rprintln!("blink");
        gpioe.bsrr.write(|w| w.bs9().set_bit());
        asm::delay(2_000_000);
        gpioe.bsrr.write(|w| w.br9().set_bit());
        asm::delay(2_000_000);
    }
}
