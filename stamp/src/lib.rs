#![no_std]
#![no_main]

use core::task::Poll;
extern crate embassy_futures;
use embassy_futures::poll_once;
pub mod espressif;
pub mod serial;
pub mod serve;
pub mod settings;

// #[cfg(test)]
pub fn poll_power_on() -> Poll<bool> {
    poll_once(power_on())
}

async fn power_on() -> bool {
    true
}

pub fn poll_uart_task() -> Poll<bool> {
    poll_once(uart_task())
}

async fn uart_task() -> bool {
    true
}

pub fn poll_init_peripherals() -> Poll<bool> {
    poll_once(init_peripherals())
}

async fn init_peripherals() -> bool {
    // System init
    // let peripherals = esp_hal::init(esp_hal::Config::default());
    // let mut rng = Rng::new(peripherals.RNG);
    // let timg0 = TimerGroup::new(peripherals.TIMG0);

    // rng::register_custom_rng(rng);

    cfg_if::cfg_if! {
       if #[cfg(feature = "esp32")] {
            let timg1 = TimerGroup::new(peripherals.TIMG1);
            esp_hal_embassy::init(timg1.timer0);
       } else {
           // use esp_hal::timer::systimer::SystemTimer;
           // let systimer = SystemTimer::new(peripherals.SYSTIMER);
           // esp_hal_embassy::init(systimer.alarm0);
       }
    }
    true
}
