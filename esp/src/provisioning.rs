use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::sys::*;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use std::ffi::c_void;
use std::ffi::CString;
use std::ptr;

pub fn connect_to_wifi(
    modem: Modem,
    sys_loop: &EspSystemEventLoop,
    nvs: &EspDefaultNvsPartition,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs.clone()))?,
        sys_loop.clone(),
    )?;

    let prov = WifiProvisioning::new()?;
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
    Ok(wifi)
}

// From here bellow i took from a github repo and made just some changes
// Its working so i dont dare to touch it anymore
pub struct WifiProvisioning;

impl WifiProvisioning {
    pub fn new() -> Result<Self, EspError> {
        unsafe {
            // Updated struct initialization
            let config = wifi_prov_mgr_config_t {
                scheme: wifi_prov_scheme_ble, // BLE provisioning
                scheme_event_handler: wifi_prov_event_handler_t {
                    event_cb: None, // No custom callback
                    user_data: ptr::null_mut(),
                },
                app_event_handler: wifi_prov_event_handler_t {
                    event_cb: None, // No custom callback
                    user_data: ptr::null_mut(),
                },
            };
            esp!(wifi_prov_mgr_init(config))?;
        }
        Ok(WifiProvisioning)
    }

    pub fn start_provisioning(
        &self,
        security: wifi_prov_security_t,
        pop: &str,
        service_name: &str,
        service_key: Option<&str>,
    ) -> Result<(), EspError> {
        let pop = CString::new(pop).unwrap();
        let service_name = CString::new(service_name).unwrap();
        let service_key = service_key.map(|key| CString::new(key).unwrap());
        let pop_ptr: *const c_void = pop.as_ptr() as *const c_void;
        unsafe {
            esp!(wifi_prov_mgr_start_provisioning(
                security,
                pop_ptr,
                service_name.as_ptr(),
                service_key.map_or(ptr::null(), |k| k.as_ptr()),
            ))?;
        }
        Ok(())
    }

    pub fn wait(&self) {
        unsafe {
            wifi_prov_mgr_wait();
        }
    }

    pub fn is_provisioned(&self) -> Result<bool, EspError> {
        let mut provisioned: bool = false;
        let result: esp_err_t = unsafe { wifi_prov_mgr_is_provisioned(&mut provisioned) };
        if result == 0 {
            Ok(provisioned)
        } else {
            Err(EspError::from(result).unwrap())
        }
    }

    pub fn stop(&self) {
        unsafe {
            wifi_prov_mgr_stop_provisioning();
        }
    }
}

impl Drop for WifiProvisioning {
    fn drop(&mut self) {
        unsafe {
            wifi_prov_mgr_deinit();
        }
    }
}
