#![no_std]
#![no_main]

#[allow(unused_extern_crates)] // NOTE(allow) bug rust-lang/rust#53964
extern crate panic_itm; // panic handler

use cortex_m::{asm, iprintln, peripheral::ITM};
use rtic::app;
use rtic::cyccnt::U32Ext;

use core::f32::consts::PI;
use micromath::F32Ext;
use stm32f3_discovery::{
    leds::Leds,
    lsm303dlhc,
    lsm303dlhc::I16x3,
    stm32f3xx_hal::{
        self as hal,
        gpio::gpiob::{PB6, PB7},
        gpio::gpioe,
        gpio::{Output, PushPull, AF4},
        i2c::I2c,
        pac::{self, I2C1},
        prelude::*,
        time::*,
        timer::{Event, Timer},
    },
    switch_hal::*,
};

use stm32f3_discovery::stm32f3xx_hal::stm32;

/// Cardinal directions. Each one matches one of the user LEDs.
#[derive(Debug)]
pub enum Direction {
    /// North / LD3
    North,
    /// Northeast / LD5
    Northeast,
    /// East / LD7
    East,
    /// Southeast / LD9
    Southeast,
    /// South / LD10
    South,
    /// Southwest / LD8
    Southwest,
    /// West / LD6
    West,
    /// Northwest / LD4
    Northwest,
}

fn set_led_for_angle(theta: f32, leds: &mut [Led; 8], itm: &mut ITM) {
    leds.iter_mut().for_each(|led| led.off().unwrap());
    let dir = if theta < -7. * PI / 8. {
        Direction::North
    } else if theta < -5. * PI / 8. {
        Direction::Northwest
    } else if theta < -3. * PI / 8. {
        Direction::West
    } else if theta < -PI / 8. {
        Direction::Southwest
    } else if theta < PI / 8. {
        Direction::South
    } else if theta < 3. * PI / 8. {
        Direction::Southeast
    } else if theta < 5. * PI / 8. {
        Direction::East
    } else if theta < 7. * PI / 8. {
        Direction::Northeast
    } else {
        Direction::North
    };

    iprintln!(&mut itm.stim[0], "Direction: {:?}", dir);
    leds.iter_mut().for_each(|led| led.off().unwrap());
    leds[dir as usize].on().unwrap();
}

fn init_clocks(cfgr: hal::rcc::CFGR, mut flash: hal::flash::Parts) -> hal::rcc::Clocks {
    cfgr.use_hse(8.mhz()) // Use external 8Mhz crystal
        .sysclk(48.mhz()) // System clock
        .hclk(48.mhz())
        .pclk1(12.mhz()) // Low speed bus
        .pclk2(12.mhz()) // High speed bus
        .freeze(&mut flash.acr)
}

pub type Led = Switch<gpioe::PEx<Output<PushPull>>, ActiveHigh>;
pub type Lsm303dlhc = lsm303dlhc::Lsm303dlhc<I2c<I2C1, (PB6<AF4>, PB7<AF4>)>>;

#[app(device = stm32f3_discovery::stm32f3xx_hal::stm32, monotonic = rtic::cyccnt::CYCCNT,  peripherals = true)]
const APP: () = {
    struct Resources {
        leds: [Led; 8],
        lsm303dlhc: Lsm303dlhc,
        timer: Timer<pac::TIM7>,
        itm: ITM,
        #[init(0)]
        tick_count: u32,
    }

    #[init(spawn=[tick])]
    fn init(cx: init::Context) -> init::LateResources {
        let mut core: rtic::Peripherals = cx.core;
        let device: stm32::Peripherals = cx.device;

        core.DCB.enable_trace();
        core.DWT.enable_cycle_counter();
        core.SCB.set_sleepdeep();

        let flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        let clocks = init_clocks(rcc.cfgr, flash);

        let mut gpioe = device.GPIOE.split(&mut rcc.ahb);
        let leds = Leds::new(
            gpioe.pe8,
            gpioe.pe9,
            gpioe.pe10,
            gpioe.pe11,
            gpioe.pe12,
            gpioe.pe13,
            gpioe.pe14,
            gpioe.pe15,
            &mut gpioe.moder,
            &mut gpioe.otyper,
        );

        let mut gpiob = device.GPIOB.split(&mut rcc.ahb);
        let scl = gpiob.pb6.into_af4(&mut gpiob.moder, &mut gpiob.afrl);
        let sda = gpiob.pb7.into_af4(&mut gpiob.moder, &mut gpiob.afrl);

        // Can't exchange scl with sda or use other pins
        let i2c = I2c::new(
            device.I2C1,
            (scl, sda),
            Hertz(400_000),
            clocks,
            &mut rcc.apb1,
        );

        let lsm303dlhc = Lsm303dlhc::new(i2c).unwrap();

        //unsafe { NVIC::unmask(hal::stm32::Interrupt::TIM7) };
        let mut timer = Timer::tim7(device.TIM7, 2.hz(), clocks, &mut rcc.apb1);
        timer.listen(Event::Update);

        init::LateResources {
            leds: leds.into_array(),
            lsm303dlhc,
            timer,
            itm: core.ITM,
        }
    }
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            asm::wfi();
        }
    }

    #[task(resources = [itm, tick_count], schedule=[tick])]
    fn tick(mut cx: tick::Context) {
        let tick_count = *cx.resources.tick_count;
        cx.resources
            .itm
            .lock(|itm| iprintln!(&mut itm.stim[0], "tick {}", tick_count));
        *cx.resources.tick_count = tick_count.wrapping_add(1);
        cx.schedule
            .tick(cx.scheduled + 168_000_000.cycles())
            .expect("to reschedule");
    }

    #[task(binds = TIM7, priority = 2, resources = [timer, lsm303dlhc, leds, itm])]
    fn tim7(cx: tim7::Context) {
        let mut resources = cx.resources;
        iprintln!(&mut resources.itm.stim[0], "TICK");
        resources.timer.clear_update_interrupt_flag();
        let I16x3 { x, y, .. } = resources.lsm303dlhc.mag().unwrap();

        let theta = (y as f32).atan2(x as f32); // in radians -PI..PI

        set_led_for_angle(theta, &mut resources.leds, &mut resources.itm);
    }

    extern "C" {
        fn FMC();
    }
};
