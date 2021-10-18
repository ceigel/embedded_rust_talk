#![no_std]
#![no_main]

#[allow(unused_extern_crates)] // NOTE(allow) bug rust-lang/rust#53964
extern crate panic_itm; // panic handler

use core::cell::RefCell;
use core::ops::DerefMut;
use cortex_m::{
    asm,
    interrupt::Mutex,
    iprintln,
    peripheral::{ITM, NVIC},
};
use cortex_m_rt::entry;

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
        interrupt,
        pac::{self, I2C1},
        prelude::*,
        time::*,
        timer::{Event, Timer},
    },
    switch_hal::*,
};

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

struct Resources {
    pub leds: [Led; 8],
    pub lsm303dlhc: Lsm303dlhc,
    pub timer: Timer<pac::TIM7>,
    pub itm: ITM,
}

type Led = Switch<gpioe::PEx<Output<PushPull>>, ActiveHigh>;
type Lsm303dlhc = lsm303dlhc::Lsm303dlhc<I2c<I2C1, (PB6<AF4>, PB7<AF4>)>>;

fn init() -> Resources {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let clocks = init_clocks(rcc.cfgr, flash);

    let mut gpioe = dp.GPIOE.split(&mut rcc.ahb);
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

    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    let scl: PB6<AF4> = gpiob.pb6.into_af4(&mut gpiob.moder, &mut gpiob.afrl);
    let sda: PB7<AF4> = gpiob.pb7.into_af4(&mut gpiob.moder, &mut gpiob.afrl);

    // Can't exchange scl with sda or use other pins
    let i2c = I2c::new(dp.I2C1, (scl, sda), Hertz(400_000), clocks, &mut rcc.apb1);

    let lsm303dlhc = Lsm303dlhc::new(i2c).unwrap();

    unsafe { NVIC::unmask(hal::stm32::Interrupt::TIM7) };
    let mut timer = Timer::tim7(dp.TIM7, 2.hz(), clocks, &mut rcc.apb1);
    timer.listen(Event::Update);

    Resources {
        leds: leds.into_array(),
        lsm303dlhc,
        timer,
        itm: cp.ITM,
    }
}

#[entry]
fn main() -> ! {
    cortex_m::interrupt::free(|cs| RESOURCES.borrow(cs).replace(Some(init())));

    loop {
        asm::wfi();
    }
}

static RESOURCES: Mutex<RefCell<Option<Resources>>> = Mutex::new(RefCell::new(None));

#[interrupt]
fn TIM7() {
    cortex_m::interrupt::free(|cs| {
        if let Some(ref mut resources) = RESOURCES.borrow(cs).borrow_mut().deref_mut() {
            resources.timer.clear_update_interrupt_flag();
            let I16x3 { x, y, .. } = resources.lsm303dlhc.mag().unwrap();

            let theta = (y as f32).atan2(x as f32); // in radians -PI..PI

            set_led_for_angle(theta, &mut resources.leds, &mut resources.itm);
        }
    });
}
