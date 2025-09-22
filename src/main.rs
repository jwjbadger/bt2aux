use esp_idf_svc::bt::{
    a2dp::{A2dpEvent, EspA2dp},
    gap::{DiscoveryMode, EspGap, GapEvent},
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
    sys::{esp, esp_bt_sp_param_t_ESP_BT_SP_IOCAP_MODE, ESP_BT_IO_CAP_IO, esp_bt_gap_set_security_param, esp_bt_gap_ssp_confirm_reply},
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

    bt_driver.set_device_name("MY CAR").unwrap();

    let gap = EspGap::new(&bt_driver).unwrap();
    // gap.set_pin("1234").unwrap();

    // Requires esp_idf_bt_ssp_enabled to be enabled, but there's no clear way to enable this
    // gap.set_ssp_io_cap(gap::IOCapabilities::DisplayInput).unwrap();

    // Update: Turns out this is intentionally extremely difficult
    // https://github.com/esp-rs/esp-idf-svc/blob/master/examples/bt_spp_acceptor.rs (line 346 as
    // of a37ce9d)
    esp!(unsafe {
        esp_bt_gap_set_security_param(
            esp_bt_sp_param_t_ESP_BT_SP_IOCAP_MODE,
            &ESP_BT_IO_CAP_IO as *const _ as *mut std::ffi::c_void, // Display Input
            1,
        )
    }).unwrap();

    gap.request_variable_pin().unwrap();

    gap.subscribe(|event| {
        match event {
            GapEvent::PairingUserConfirmationRequest { bd_addr, number } => {
                log::info!("Received pairing verification number: {number}");

                // At some point we should actually verify before confirming but we need to build
                // up the hardware before doing that
                // cfg elides the more proper version of this as well
                esp!(unsafe {
                    esp_bt_gap_ssp_confirm_reply(&bd_addr as *const _ as *mut _, true)
                }).unwrap();
            },
            _ => { log::info!("Got GAP event: {:?}", event)}
        };
    }).unwrap();

    let a2dp = EspA2dp::new_sink(&bt_driver).unwrap();
    //a2dp.set_delay(core::time::Duration::from_millis(10000));

    let mut i2s_driver = I2sDriver::new_std_tx(
        peripherals.i2s0,
        &StdConfig::philips(44100, DataBitWidth::Bits16),
        peripherals.pins.gpio26,
        peripherals.pins.gpio19, //25
        None as Option<Gpio23>,
        peripherals.pins.gpio25, //22
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
