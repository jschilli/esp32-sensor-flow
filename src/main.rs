use std::ops::Deref;
use std::sync::Arc;

use std::sync::Mutex;
use std::thread::sleep;
use std::time::SystemTime;

use anyhow::Result;
use buttons::ButtonTypes;
use buttons::Buttons;
use esp_idf_hal::adc::attenuation;
use esp_idf_hal::adc::config::Config;
use esp_idf_hal::adc::AdcChannelDriver;

use esp_idf_hal::adc::AdcDriver;
use esp_idf_hal::peripherals::Peripherals;
use futures_util::*;

use tokio::time::Duration;
// mod window;
// use window::SensorFlowExt;
mod buttons;
use sensor_stream::*;

#[derive(Clone, Debug)]
pub struct ButtonSensorData(SensorData<ButtonTypes>);

impl PartialEq for ButtonSensorData {
    fn eq(&self, other: &Self) -> bool {
        self.0.value == other.0.value
    }
}

impl ButtonSensorData {
    pub fn new(value: ButtonTypes) -> Self {
        Self(SensorData {
            value,
            timestamp: SystemTime::now(),
        })
    }
}

impl Deref for ButtonSensorData {
    type Target = SensorData<ButtonTypes>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[derive(Debug)]
struct KeyConfig {
    button: ButtonTypes,
    min_adc_val: u16,
    max_adc_val: u16,
}

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

// type Error = anyhow::Error;

// pub fn sensor_reading<'a>(
//     adc: &'a mut AdcDriver<'a, ADC1>,
//     adc_pin: &'a mut esp_idf_hal::adc::AdcChannelDriver<'a, 3, Gpio1>,
// ) -> impl Stream<Item = ButtonSensorData> + 'a {
//     async_stream::stream! {
//         let mut interval = tokio::time::interval(Duration::from_secs(1));
//             loop {
//                 let mut result = ButtonTypes::None;

//                      interval.tick().await;
//                 let button_adc = adc.read(adc_pin).unwrap();
//                 // println!("{:?}", button_adc);
//                 for button in BUTTONS.iter() {
//                     // println!("{:?}", button);
//                     if button_adc >= button.min_adc_val && button_adc <= button.max_adc_val {
//                         println!("Button pressed: {:?}", button.button);
//                         // push adc values into stream!
//                         result = button.button;
//                     }
//             }
//             yield ButtonSensorData::new(result)

//         }
//     }
// }

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

    // let mut led = PinDriver::output(peripherals.pins.gpio3)?;
    // let mut timer = TimerDriver::new(peripherals.timer00, &TimerConfig::new())?;

    let mut adc = AdcDriver::new(peripherals.adc1, &Config::new().calibration(true))?;

    let mut adc_pin: esp_idf_hal::adc::AdcChannelDriver<{ attenuation::DB_11 }, _> =
        AdcChannelDriver::new(peripherals.pins.gpio1)?;
    let buttons = Buttons {
        menu: false,
        play: false,
        vol_up: false,
        vol_dn: false,
    };

    let arc_buttons = Arc::new(Mutex::new(buttons));
    let arc_buttons_clone2 = Arc::clone(&arc_buttons);

    let menu_button_flow = async_stream::stream! {

        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
               interval.tick().await;
               let button_val = arc_buttons_clone2.lock().unwrap().menu;
            //    println!("yielding {:?} {:?}", button_val, &arc_buttons_clone2);
               yield button_val
       }
    };

    // let button_stream = sensor_reading(&mut adc, &mut adc_pin).window::<ButtonSensorData>();
    let arc_buttons_clone = Arc::clone(&arc_buttons);
    let button_reader = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1000));
        loop {
            interval.tick().await;
            let button_adc = adc.read(&mut adc_pin).unwrap();
            for button in BUTTONS.iter() {
                let arc_buttons_clone = Arc::clone(&arc_buttons_clone);

                if button_adc >= button.min_adc_val && button_adc <= button.max_adc_val {
                    arc_buttons_clone
                        .lock()
                        .unwrap()
                        .set_button_state(button.button, true);
                    println!("Button pressed: {:?}", button.button);
                    // push adc values into stream!
                    // result = button.button;
                } else {
                    arc_buttons_clone
                        .lock()
                        .unwrap()
                        .set_button_state(button.button, false);
                }
            }
            // println!("{:?}", &arc_buttons_clone);
        }
    });

    let menu_button_flow = menu_button_flow.window::<bool>();
    pin_mut!(menu_button_flow);
    while let Some(state) = menu_button_flow.next().await {
        println!("menu state: {:?}", state);
    }
    // loop {
    // pin_mut!(button_stream); // StreamExt::next requires that the stream be Unpin
    // while let Some(msg) = button_stream.next().await {
    //     println!("{:?}", msg);
    // }
    join!(button_reader);
    Ok(())
}
