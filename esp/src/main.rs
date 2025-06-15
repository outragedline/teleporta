use esp_idf_svc::sys::*;
mod provisioning;

use std::time::Duration;

use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};

const MQTT_URL: &str = env!("MQTT_URL");
fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    let prov = provisioning::WifiProvisioning::new()?;
    if !prov.is_provisioned()? {
        let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
            ..Default::default()
        });
        wifi.set_configuration(&wifi_configuration)?;
        wifi.start()?;
        prov.start_provisioning(
            wifi_prov_security_WIFI_PROV_SECURITY_1,
            "teleporta",      // Proof of Possession (POP)
            "PROV_TELEPORTA", // Service Name
            None,             // No Service Key
        )?;

        println!("Waiting for Wi-Fi provisioning...");
        prov.wait();

        println!("Provisioning completed. Stopping...");
        prov.stop();
    } else {
        wifi.start()?;
        wifi.connect()?;
    }
    wifi.wait_netif_up()?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    println!("Wifi DHCP info: {:?}", ip_info);

    let (mut mqtt_client, mut mqtt_conn) = EspMqttClient::new(
        MQTT_URL,
        &MqttClientConfiguration {
            client_id: Some("esp_teleporta"),
            ..Default::default()
        },
    )?;

    std::thread::scope(|s| {
        // This is required it seems
        std::thread::Builder::new()
            .stack_size(6000)
            .spawn_scoped(s, move || {
                println!("MQTT Listening for messages");

                while let Ok(event) = mqtt_conn.next() {
                    println!("[Queue] Event: {}", event.payload());
                }
            })
            .unwrap();

        loop {
            if let Err(_) = mqtt_client.subscribe("door/alert", QoS::AtMostOnce) {
                std::thread::sleep(Duration::from_millis(500));

                continue;
            }

            println!("Subscribed to topic \"door/alert\"");

            // Just to give a chance of our connection to get even the first published message
            std::thread::sleep(Duration::from_millis(500));

            loop {
                mqtt_client
                    .publish("door/alert", QoS::AtMostOnce, false, "".as_bytes())
                    .unwrap();

                let sleep_secs = 2;

                println!("Now sleeping for {sleep_secs}s...");
                std::thread::sleep(Duration::from_secs(sleep_secs));
            }
        }
    });
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
