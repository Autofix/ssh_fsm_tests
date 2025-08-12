#![no_std]
#![no_main]

use core::task::Poll;
extern crate embassy_futures;
use embassy_futures::poll_once;
pub mod espressif;
pub mod serial;
pub mod serve;
pub mod settings;

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

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
// use embassy_sync::channel::{Channel, Receiver, Sender};

use esp_hal::peripherals::{RADIO_CLK, SW_INTERRUPT, TIMG0, WIFI};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use espressif::rng;
// use static_cell::StaticCell; //no-std - doesn't need to be initialised at compile time
// pub static PERIPHERAL_RADIO_CLOCK: StaticCell<CriticalSectionMutex<RADIO_CLK>> = StaticCell::new();
// pub static PERIPHERAL_WIFI: StaticCell<CriticalSectionMutex<WIFI>> = StaticCell::new();
// pub static PERIPHERAL_SW_INTERRUPT: StaticCell<CriticalSectionMutex<SW_INTERRUPT>> = StaticCell::new();
// pub static PERIPHERAL_RNG: StaticCell<CriticalSectionMutex<Rng>> = StaticCell::new();
// pub static PERIPHERAL_TIMG0: StaticCell<CriticalSectionMutex<TimerGroup<TIMG0>>> =    StaticCell::new();
pub static RADIO_CLOCK_CHANNEL: Channel<CriticalSectionRawMutex, RADIO_CLK, 1> = Channel::new();
pub static WIFI_CHANNEL: Channel<CriticalSectionRawMutex, WIFI, 1> = Channel::new();
pub static SW_INTERRUPT_CHANNEL: Channel<CriticalSectionRawMutex, SW_INTERRUPT, 1> = Channel::new();
pub static RNG_CHANNEL: Channel<CriticalSectionRawMutex, Rng, 1> = Channel::new();
pub static TIMG0_CHANNEL: Channel<CriticalSectionRawMutex, TimerGroup<TIMG0>, 1> = Channel::new();

async fn init_peripherals() -> bool {
    // System init
    let config = esp_hal::Config::default();
    let peripherals = esp_hal::init(config);
    let _radio_clk = RADIO_CLOCK_CHANNEL.send(peripherals.RADIO_CLK);
    let _wifi = WIFI_CHANNEL.send(peripherals.WIFI);
    let _sw_interrupt = SW_INTERRUPT_CHANNEL.send(peripherals.SW_INTERRUPT);
    let rng = Rng::new(peripherals.RNG);
    let _timg0 = TIMG0_CHANNEL.send(TimerGroup::new(peripherals.TIMG0));
    rng::register_custom_rng(rng);
    let _rng = RNG_CHANNEL.send(rng);

    // let _radio_clk = PERIPHERAL_RADIO_CLOCK.init(CriticalSectionMutex::new(peripherals.RADIO_CLK));
    // let _wifi = PERIPHERAL_WIFI.init(CriticalSectionMutex::new(peripherals.WIFI));
    // let _sw_interrupt =
    // PERIPHERAL_SW_INTERRUPT.init(CriticalSectionMutex::new(peripherals.SW_INTERRUPT));
    // let timg0 = TimerGroup::new(peripherals.TIMG0);
    // let _timg0 = PERIPHERAL_TIMG0.init(CriticalSectionMutex::new(timg0));
    // let _rng = PERIPHERAL_RNG.init(CriticalSectionMutex::new(rng));

    cfg_if::cfg_if! {
       if #[cfg(feature = "esp32")] {
            let timg1 = TimerGroup::new(peripherals.TIMG1);
            esp_hal_embassy::init(timg1.timer0);
       } else {
           use esp_hal::timer::systimer::SystemTimer;
           let systimer = SystemTimer::new(peripherals.SYSTIMER);
           esp_hal_embassy::init(systimer.alarm0);
       }
    }

    true
}
