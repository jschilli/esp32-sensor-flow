use anyhow::Result;
use esp_idf_hal::adc::attenuation;
use esp_idf_hal::adc::config::Config;
use esp_idf_hal::adc::AdcChannelDriver;

use esp_idf_hal::adc::AdcDriver;
use esp_idf_hal::adc::ADC1;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
// use esp_idf_hal::task::*;
use esp_idf_hal::timer::*;
// use futures::lock::Mutex;
use futures_util::*;
use tokio::time::sleep;
// use std::ops::DerefMut;
// use std::sync::{Arc, Mutex};
// use std::thread;
// use std::time::Duration;
use tokio::time::{self, Duration};

#[derive(Debug, PartialEq, Clone, Copy)]
enum ButtonTypes {
    None,
    VolUp,
    VolDown,
    Play,
    Menu,
}
#[derive(Debug)]
struct KeyConfig {
    button: ButtonTypes,
    min_adc_val: u16,
    max_adc_val: u16,
}

// impl Debug for KeyConfig {}

const BUTTONS: [KeyConfig; 4] = [
    KeyConfig {
        button: ButtonTypes::VolUp,
        min_adc_val: 0,
        max_adc_val: 375,
    },
    KeyConfig {
        button: ButtonTypes::VolDown,
        min_adc_val: 750,
        max_adc_val: 850,
    },
    KeyConfig {
        button: ButtonTypes::Play,
        min_adc_val: 1900,
        max_adc_val: 2000,
    },
    KeyConfig {
        button: ButtonTypes::Menu,
        min_adc_val: 2350,
        max_adc_val: 2450,
    },
];

type Error = anyhow::Error;

pub fn sensor_reading<'a>(
    adc: &'a mut AdcDriver<'a, ADC1>,
    adc_pin: &'a mut esp_idf_hal::adc::AdcChannelDriver<'a, 3, Gpio1>,
) -> impl Stream<Item = Result<ButtonTypes, Error>> + 'a {
    async_stream::stream! {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
            // let delay = time::sleep(Duration::from_secs(1));
            loop {
                let mut result = ButtonTypes::None;

                            interval.tick().await;
                let button_adc = adc.read(adc_pin).unwrap();
                println!("{:?}", button_adc);
                for button in BUTTONS.iter() {
                    println!("{:?}", button);
                    if button_adc >= button.min_adc_val && button_adc <= button.max_adc_val {
                        println!("Button pressed: {:?}", button.button);
                        result = button.button;
                        // push adc values into stream!

                        // let buttons and other interested parties consume stream and react `on_change`
                    }
            }
            yield Ok(result);

        }
    }
}

// async fn sensor_readings<'a>(
//     adc: Arc<AdcDriver<'_, ADC1>>,
//     adc_pin: Arc<Mutex<esp_idf_hal::adc::AdcChannelDriver<'_, 3, Gpio1>>>,
//     id: u8,
// ) -> impl futures_util::Stream<Item = ButtonTypes> {
//     stream::unfold(0, move |count| {
//         let adc = Arc::clone(&adc);
//         // let adc_pin = Arc::clone(adc_pin);
//         let mut adc_pin = adc_pin.lock().unwrap();
//         let delay = time::sleep(Duration::from_secs(1));
//         async move {
//             delay.await;
//             let mut result = ButtonTypes::None;
//             // thread::sleep(Duration::from_millis(1000));
//             let button_adc = adc.read(adc_pin.deref_mut()).unwrap();
//             for button in BUTTONS.iter() {
//                 if button_adc >= button.min_adc_val && button_adc <= button.max_adc_val {
//                     println!("Button pressed: {:?}", button.button);
//                     result = button.button;
//                     // push adc values into stream!

//                     // let buttons and other interested parties consume stream and react `on_change`
//                 }
//             }
//             // let reading = format!("Sensor {}: Temperature {}°C", id, 20 + count % 5);
//             Some((result, count + 1))
//         }
//     })
// }
// #[tokio::main]
fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    esp_idf_svc::io::vfs::initialize_eventfd(1).expect("Failed to initialize eventfd");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");

    match rt.block_on(async { async_main().await }) {
        Ok(()) => println!("main() finished, reboot."),
        Err(err) => {
            println!("{err:?}");
            // Let them read the error message before rebooting
            sleep(std::time::Duration::from_secs(3));
        }
    }

    esp_idf_hal::reset::restart();
    Ok(())
}
async fn async_main() -> Result<()> {
    let peripherals = Peripherals::take()?;

    let mut led = PinDriver::output(peripherals.pins.gpio3)?;
    let mut timer = TimerDriver::new(peripherals.timer00, &TimerConfig::new())?;

    let mut adc = AdcDriver::new(peripherals.adc1, &Config::new().calibration(true))?;

    // let mut pin1 = adc1_config.enable_pin_with_cal::<_, AdcCal>(io.pins.gpio1.into_analog(), atten);
    let mut adc_pin: esp_idf_hal::adc::AdcChannelDriver<{ attenuation::DB_11 }, _> =
        AdcChannelDriver::new(peripherals.pins.gpio1)?;
    // let mut adc_pin = Arc::new(Mutex::new(adc_pin));
    let button_stream = sensor_reading(&mut adc, &mut adc_pin);
    // loop {
    pin_mut!(button_stream); // StreamExt::next requires that the stream be Unpin
    while let Some(Ok(msg)) = button_stream.next().await {
        println!("{:?}", msg);
    }
    Ok(())
    // you can change the sleep duration depending on how often you want to sample
    // thread::sleep(Duration::from_millis(1000));
    // let button_adc = adc.read(&mut adc_pin)?;
    // for button in BUTTONS.iter() {
    //     if button_adc >= button.min_adc_val && button_adc <= button.max_adc_val {
    //         println!("Button pressed: {:?}", button.button);
    //         // push adc values into stream!

    //         // let buttons and other interested parties consume stream and react `on_change`
    //     }
    // }

    // println!("ADC value: {}", adc.read(&mut adc_pin)?);
    // }
    // block_on(async {
    //     loop {
    //         led.set_high()?;

    //         timer.delay(timer.tick_hz()).await?;

    //         led.set_low()?;

    //         timer.delay(timer.tick_hz()).await?;
    //     }
    // })
    // log::info!("Hello, world!");
}
