mod provisioning;
mod sensor;
mod servo;

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver},
        prelude::Peripherals,
    },
    mqtt::client::{
        EventPayload::{self, Error},
        QoS, *,
    },
    nvs::EspDefaultNvsPartition,
};
use sensor::HcSr04;
use servo::Servo;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let peripherals = Peripherals::take()?;

    let wifi = provisioning::connect_to_wifi(peripherals.modem, &sys_loop, &nvs)?;

    let _wifi = wifi;
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

    let mut mqtt_client = EspMqttClient::new_cb(
        "mqtt://test.mosquitto.org:1883",
        &MqttClientConfiguration {
            client_id: Some("teleporta_esp"),
            ..Default::default()
        },
        move |message_event| match message_event.payload() {
            EventPayload::Received {
                id: _,
                topic: _,
                data,
                details: _,
            } => {
                if data.eq("open".as_bytes()) {
                    servo.open();
                } else {
                    servo.close();
                }
            }
            Error(e) => println!("MQTT Error: {}", e),
            _ => {}
        },
    )?;

    std::thread::sleep(Duration::from_secs(1));

    mqtt_client.subscribe("door/command", QoS::AtLeastOnce)?;

    let mut sensor = HcSr04::new(peripherals.pins.gpio4, peripherals.pins.gpio2)?;
    let mut counter = 0u8;
    loop {
        match sensor.measure_distance_cm() {
            Ok(distance) => {
                if distance < 10.0 {
                    if counter >= 5 {
                        mqtt_client.publish("door/alert", QoS::AtLeastOnce, false, "".as_bytes())?;
                        counter = 0u8;
                    } else {
                        counter += 1;
                    }
                }
            }
            Err(e) => {
                println!("Failed to measure distance: {}", e);
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
