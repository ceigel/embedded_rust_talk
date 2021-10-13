#![no_std]
#![no_main]
extern crate panic_itm;

use cortex_m::{asm, iprintln};
use cortex_m_rt::entry;
use stm32f3xx_hal::{gpio, pac, prelude::*};

#[entry]
fn entrypoint() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut itm = cp.ITM;
    let stim = &mut itm.stim[0];
    let mut rcc = dp.RCC.constrain();
    let mut gpioe = dp.GPIOE.split(&mut rcc.ahb);
    let mut led: gpio::gpioe::PE9<gpio::Output<gpio::PushPull>> = gpioe
        .pe9
        .into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);

    iprintln!(stim, "Setup done");
    loop {
        iprintln!(stim, "blink");
        led.toggle().expect("to toggle");
        asm::delay(1_000_000);
    }
}
