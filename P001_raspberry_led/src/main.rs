use linux_embedded_hal::sysfs_gpio::Direction;
use linux_embedded_hal::Pin;
use std::thread;
use std::time::Duration;

fn main() {
    let pin = Pin::new(26);
    pin.export().expect("To export pin");
    pin.set_direction(Direction::Out).expect("To set direction");
    loop {
        println!("tick");
        pin.set_value(1).expect("set high");
        thread::sleep(Duration::from_millis(500));
        pin.set_value(0).expect("set low");
        thread::sleep(Duration::from_millis(500));
    }
}
