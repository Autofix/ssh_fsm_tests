#![no_std]
#![no_main]

use core::task::Poll;
extern crate embassy_futures;
use embassy_executor::SendSpawner;
use embassy_futures::poll_once;
pub mod config;
pub mod errors;
pub mod espressif;
pub mod keys;
pub mod serial;
pub mod serve;
pub mod settings;
pub mod storage;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use esp_hal::Config;
use crate::config::SSHStampConfig;
use crate::storage::Fl;
use esp_hal::interrupt::{Priority, software::SoftwareInterruptControl};
use esp_hal::peripherals::{Peripherals, RADIO_CLK, SW_INTERRUPT, TIMG0, WIFI};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_hal_embassy::InterruptExecutor;
use esp_storage::FlashStorage;
use espressif::rng;


use crate::espressif::buffered_uart::BufferedUart;

// ****
// Potenital future option to save static memory
// Use arc to store pointer inside mutex rather than entire values
// use core::ptr::addr_of_mut;
// use heapless::{
//     arc_pool,
//     pool::arc::{Arc, ArcBlock},
// };
arc_pool!(RadioClkPool: RADIO_CLK<'static>);
static mut RADIO_CLOCK_BLOCK: ArcBlock<RadioClkPool> = ArcBlock::new();
static RADIO_CLOCK_MUTEX: Mutex<CriticalSectionRawMutex, Option<Arc<RadioClkPool>>> =
    Mutex::new(None);
// ****

static RADIO_CLOCK_MUTEX: Mutex<CriticalSectionRawMutex, Option<RADIO_CLK>> = Mutex::new(None);
static WIFI_MUTEX: Mutex<CriticalSectionRawMutex, Option<WIFI>> = Mutex::new(None);
static RNG_MUTEX: Mutex<CriticalSectionRawMutex, Option<Rng>> = Mutex::new(None);
static TIMG0_MUTEX: Mutex<CriticalSectionRawMutex, Option<TimerGroup<'_, TIMG0<'static>>>> =
    Mutex::new(None);
static CONFIG_MUTEX: Mutex<CriticalSectionRawMutex, Option<SSHStampConfig>> = Mutex::new(None);
static SW_INTERRUPT_MUTEX: Mutex<CriticalSectionRawMutex, Option<SW_INTERRUPT>> = Mutex::new(None);
static UART_BUFFER_MUTEX: Mutex<CriticalSectionRawMutex, Option<BufferedUart>> = Mutex::new(None);
static INT_SPAWNER_MUTEX: Mutex<CriticalSectionRawMutex, Option<SendSpawner>> = Mutex::new(None);

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
    let config: Config = esp_hal::Config::default();
    let peripherals: Peripherals = esp_hal::init(config);
    let rng: Rng = Rng::new(peripherals.RNG);
    rng::register_custom_rng(rng);
    let timg0: TimerGroup<'_, TIMG0<'_>> = TimerGroup::new(peripherals.TIMG0);

    // ******
    // Potenital future option to save static memory
    // Use arc to store pointer inside mutex rather than entire values
    // let radio_clock_block: &'static mut ArcBlock<RADIO_CLK> = unsafe {
    //     static mut RADIO_CLOCK_BLOCK: ArcBlock<RADIO_CLK> = ArcBlock::new();
    //     addr_of_mut!(RADIO_CLOCK_BLOCK).as_mut().unwrap()
    // };
    // RadioClkPool.manage(radio_clock_block);
    // *(RADIO_CLOCK_MUTEX.lock().await) = Some(RadioClkPool.alloc(peripherals.RADIO_CLK).unwrap());
    // *******

    *(RADIO_CLOCK_MUTEX.lock().await) = Some(peripherals.RADIO_CLK);
    *(WIFI_MUTEX.lock().await) = Some(peripherals.WIFI);
    *(TIMG0_MUTEX.lock().await) = Some(timg0);
    // let _radio_clk = PERIPHERAL_RADIO_CLOCK.init(CriticalSectionMutex::new(peripherals.RADIO_CLK));
    // let _wifi = PERIPHERAL_WIFI.init(CriticalSectionMutex::new(peripherals.WIFI));
    // let _sw_interrupt =
    // PERIPHERAL_SW_INTERRUPT.init(CriticalSectionMutex::new(peripherals.SW_INTERRUPT));
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

    // Read SSH configuration from Flash (if it exists)
    let mut flash_storage = Fl::new(FlashStorage::new());
    let config: SSHStampConfig = storage::load_or_create(&mut flash_storage).await.unwrap();
    *(CONFIG_MUTEX.lock().await) = Some(config);

    // Set up software buffered UART to run in a higher priority InterruptExecutor
    let uart_buf: BufferedUart = BufferedUart::new();
    *(UART_BUFFER_MUTEX.lock().await) = Some(uart_buf);

    let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let mut interrupt_executor: InterruptExecutor<0> =
        (|| InterruptExecutor::new(software_interrupts.software_interrupt0))();
    let interrupt_spawner: SendSpawner;
    cfg_if::cfg_if! {
        if #[cfg(any(feature = "esp32", feature = "esp32s2", feature = "esp32s3"))] {
            interrupt_spawner   = interrupt_executor.start(Priority::Priority1);
        } else {
            interrupt_spawner = interrupt_executor.start(Priority::Priority10);
        }
    }
    *(INT_SPAWNER_MUTEX.lock().await) = Some(interrupt_spawner);

    true
}
