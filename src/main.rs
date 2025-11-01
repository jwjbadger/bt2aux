use esp_idf_svc::bt::{
    a2dp::{A2dpEvent, EspA2dp},
    avrc::{controller::EspAvrcc, KeyCode},
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
    gpio::{PinDriver, Pull},
    gpio::Gpio23,
    task::notification::Notification,
    prelude::*,
    sys::{esp, esp_bt_sp_param_t_ESP_BT_SP_IOCAP_MODE, ESP_BT_IO_CAP_IO, esp_bt_gap_set_security_param, esp_bt_gap_ssp_confirm_reply},
};

use core::num::NonZero;

use std::sync::{Arc, Mutex};

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

    let mut high = PinDriver::output(peripherals.pins.gpio21).unwrap();
    high.set_high().unwrap();

    let mut skip = PinDriver::input(peripherals.pins.gpio22).unwrap();
    let mut back = PinDriver::input(peripherals.pins.gpio23).unwrap();
    let mut pause = PinDriver::input(peripherals.pins.gpio18).unwrap();
    let mut play = PinDriver::input(peripherals.pins.gpio4).unwrap();

    skip.set_pull(Pull::Down).unwrap();
    back.set_pull(Pull::Down).unwrap();
    pause.set_pull(Pull::Down).unwrap();
    play.set_pull(Pull::Down).unwrap();

    skip.set_interrupt_type(esp_idf_hal::gpio::InterruptType::PosEdge).unwrap();
    back.set_interrupt_type(esp_idf_hal::gpio::InterruptType::PosEdge).unwrap();
    pause.set_interrupt_type(esp_idf_hal::gpio::InterruptType::PosEdge).unwrap();
    play.set_interrupt_type(esp_idf_hal::gpio::InterruptType::PosEdge).unwrap();

    let notification = Notification::new();
    let waker = notification.notifier();
    let action: Arc<Mutex<Option<KeyCode>>> = Arc::new(Mutex::new(None));

    unsafe {
        let skip_waker = waker.clone();
        let skip_action = action.clone();
        skip
            .subscribe(move || {
                *skip_action.lock().unwrap() = Some(KeyCode::Forward);
                skip_waker.notify(NonZero::new(1).unwrap());
            })
        .unwrap();

        let back_waker = waker.clone();
        let back_action = action.clone();
        back
            .subscribe(move || {
                *back_action.lock().unwrap() = Some(KeyCode::Backward);
                back_waker.notify(NonZero::new(2).unwrap());
            })
        .unwrap();

        let pause_waker = waker.clone();
        let pause_action = action.clone();
        pause
            .subscribe(move || {
                *pause_action.lock().unwrap() = Some(KeyCode::Pause);
                pause_waker.notify(NonZero::new(3).unwrap());
            })
        .unwrap();

        let play_waker = waker.clone();
        let play_action = action.clone();
        play
            .subscribe(move || {
                *play_action.lock().unwrap() = Some(KeyCode::Play);
                play_waker.notify(NonZero::new(4).unwrap());
            })
        .unwrap();
    }

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

    let avrc_driver = EspAvrcc::new(&bt_driver).unwrap();
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

    let mut transaction_label: u8 = 0;
    
    loop {
        skip.enable_interrupt().unwrap();
        back.enable_interrupt().unwrap();
        pause.enable_interrupt().unwrap();
        play.enable_interrupt().unwrap();

        notification.wait_any();
        
        let mut action = action.lock().unwrap();

        if let Some(action) = *action {
            avrc_driver.send_passthrough(transaction_label, action, true).unwrap();
        }

        *action = None;

        transaction_label += 1;
        if transaction_label > 15 {
            transaction_label = 0;
        }

        FreeRtos::delay_ms(200); // debounce
    }
}
