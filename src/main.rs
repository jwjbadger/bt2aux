use esp_idf_svc::bt::{
    a2dp::{A2dpEvent, EspA2dp},
    gap::{DiscoveryMode, EspGap},
    BtClassic, BtDriver,
};
use esp_idf_svc::nvs::EspDefaultNvsPartition;

use esp_idf_svc::hal::{
    delay::{FreeRtos, TickType},
    i2s::{
        config::{DataBitWidth, StdConfig},
        I2sDriver,
    },
    gpio::Gpio23,
    prelude::*,
};

#[cfg(not(feature = "experimental"))]
fn main() {
    panic!("Use `--features experimental` to build this project");
}

#[cfg(feature = "experimental")]
fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let bt_driver: BtDriver<'_, BtClassic> = BtDriver::new(peripherals.modem, Some(nvs)).unwrap();

    bt_driver.set_device_name("Car-Go").unwrap();

    let gap = EspGap::new(&bt_driver).unwrap();
    gap.set_pin("1234").unwrap();

    // Requires esp_idf_bt_ssp_enabled to be enabled, but there's no clear way to enable this
    // gap.set_ssp_io_cap(gap::IOCapabilities::DisplayInput).unwrap();

    let a2dp = EspA2dp::new_sink(&bt_driver).unwrap();

    let mut i2s_driver = I2sDriver::new_std_tx(
        peripherals.i2s0,
        &StdConfig::philips(44100, DataBitWidth::Bits16),
        peripherals.pins.gpio26,
        peripherals.pins.gpio25,
        None as Option<Gpio23>,
        peripherals.pins.gpio22,
    ).unwrap();
    i2s_driver.tx_enable().unwrap();

    a2dp.subscribe(move |event: A2dpEvent| {
        match event {
            A2dpEvent::SinkData(arr) => {
                i2s_driver.write_all(arr, TickType::new_millis(10000).into()).unwrap(); 
            }
            _ => {
                log::info!("Got a2dp event: {:?}", event)
            }
        }
        0
    })
    .unwrap();

    gap.set_scan_mode(true, DiscoveryMode::Discoverable)
        .unwrap();

    loop {
        FreeRtos::delay_ms(10000);
    }
}
