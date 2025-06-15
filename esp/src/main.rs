mod provisioning;
mod servo;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use servo::Servo;
use std::time::Duration;

use esp_idf_hal::ledc::{LedcChannel, LedcTimerDriver};
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::LedcDriver;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let peripherals = Peripherals::take()?;

    let wifi = provisioning::connect_to_wifi(peripherals.modem, &sys_loop, &nvs)?;

    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::new().frequency(50u32.into()),
    )?;

    let channel_driver = LedcDriver::new(
        peripherals.ledc.channel0,
        &timer_driver,
        peripherals.pins.gpio19,
    )?;

    let mut servo = Servo { channel_driver };

    loop {
        servo.close();
        std::thread::sleep(Duration::from_secs(1));
        servo.open();
        std::thread::sleep(Duration::from_secs(1));
    }
}
