#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![macro_use]

use defmt_rtt as _; // global logger
use panic_probe as _;

pub use defmt::*;
use embassy::executor::Spawner;
use embassy::time::{Duration, Timer};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::Peripherals;
use embedded_hal::digital::v2::OutputPin;

async fn blink_led(mut led: Output<'_, embassy_stm32::peripherals::PC1>) {
    const DURATION: Duration = Duration::from_millis(300);

    loop {
        unwrap!(led.set_high());
        Timer::after(DURATION).await;
        unwrap!(led.set_low());
        Timer::after(DURATION).await;
    }
}

async fn blink_info() {
    const DURATION: Duration = Duration::from_millis(1000);

    loop {
        info!("high");
        Timer::after(DURATION).await;
        info!("low");
        Timer::after(DURATION).await;
    }
}

#[embassy::main]
async fn main(_spawner: Spawner, p: Peripherals) {
    let led = Output::new(p.PC1, Level::High, Speed::Low);
    futures::join!(blink_led(led), blink_info());
}
