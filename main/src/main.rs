// #![cfg_attr(not(test), no_std)]
// #![cfg_attr(not(test), no_main)]
#![no_std]
#![no_main]

// use heapless::String;
// // Inspired by https://play.rust-lang.org/?version=stable&mode=debug&edition=2015&gist=ee3e4df093c136ced7b394dc7ffb78e1
// // Originally described in https://hoverbear.org/blog/rust-state-machine-pattern/
// // Resurfaced at HN: https://news.ycombinator.com/item?id=43741051

// // Tenets:
// //  1. Lightweight and easy to understand/change.
// //  2. Should not interfere in performance, only "big" state transitions should be tracked (not micromanage on bytes sent, etc...).
// //  3. Non intrusive in application code.

// #[cfg(not(test))]
// #[cfg(not(feature = "std"))]
// use core::panic::PanicInfo;
// use strum::IntoEnumIterator;
// use strum_macros::EnumIter;

// use crate::settings;

// impl Settings {
//     fn read_uart_settings(&self) -> Option<u32> {
//         self.uart_baud
//     }
//     fn read_wifi_settings(&self) -> Option<WifiMode> {
//         self.wifi_mode.clone()
//     }
//     fn read_ssh_password_settings(&self) -> Option<String<20>> {
//         self.ssh_password.clone()
//     }

//     fn store_settings(
//         &mut self,
//         new_wifi_mode: Option<WifiMode>,
//         new_ssh_password: Option<String<20>>,
//         new_uart_baud: Option<u32>,
//     ) {
//         if new_wifi_mode != None {
//             self.wifi_mode = new_wifi_mode;
//         }
//         if new_ssh_password != None {
//             self.ssh_password = new_ssh_password;
//         }
//         if new_uart_baud != None {
//             self.uart_baud = new_uart_baud;
//         }
//     }
// }

// use fsm;
// static mut SETTINGS: Settings = Settings{
//   wifi_mode: None,
//   ssh_password: None,
//   uart_baud: None,
// };
// use core::marker::Sized;
// use esp_alloc as _;
// use esp_backtrace as _;
// use esp_hal::{
// gpio::AnyPin,
// interrupt::{Priority, software::SoftwareInterruptControl},
// peripherals::UART1,
// rng::Rng,
// timer::timg::TimerGroup,
// uart::{Config, RxConfig, Uart},
// };
// pub mod fsm;

// use esp_hal_embassy::InterruptExecutor;

// use embassy_executor::Spawner;
// use static_cell::StaticCell;

// #[unsafe(no_mangle)]
// #[cfg(feature = "std")]
// #[entry]
// #[cfg(feature = "std")]
// #[cfg(test)]
// #[cfg_attr(not(test), entry)]
// #[cfg(not(test))]
// cfg_if::cfg_if! {
// if #[cfg(any(feature = "test"))] {
// #[cfg(not(test))]
// #[entry]
// fn main() {
// #[cfg(not(test))]
//
// use core::marker::Sized;
// use esp_alloc as _;
use esp_backtrace as _;
// use esp_hal::{
//     gpio::AnyPin,
//     interrupt::{Priority, software::SoftwareInterruptControl},
//     peripherals::UART1,
//     rng::Rng,
//     timer::timg::TimerGroup,
//     uart::{Config, RxConfig, Uart},
// };
// use esp_hal_embassy::InterruptExecutor;

use core::task::Poll;
use embassy_executor::Spawner;
use fsm::EventResponse;
// use static_cell::StaticCell;
//
pub static SPAWNER_MUTEX: Mutex<CriticalSectionRawMutex, Option<Spawner>> = Mutex::new(None);
// #[main]
#[esp_hal_embassy::main]
pub async fn main(spawner: Spawner) {
    *(SPAWNER_MUTEX.lock().await) = Some(spawner);
    // System init
    // let peripherals = esp_hal::init({ esp_hal::Config::default() });
    // let mut rng = Rng::new(peripherals.RNG);
    // let timg0 = TimerGroup::new(peripherals.TIMG0);

    // let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    // use esp_hal::timer::systimer::SystemTimer;
    // let systimer = SystemTimer::new(peripherals.SYSTIMER);
    // esp_hal_embassy::init(systimer.alarm0);

    let mut state_machine = fsm::StateMachine::new();
    // Sequence of events (might be dynamic based on what State::run did)
    // TODO: Declare this array automatically from the enum definition above.
    // let mut iter = Event::iter();

    let mut event_response: EventResponse<bool>;

    loop {
        if let fsm::State::Failure(_string) = state_machine.state {
            // #[cfg(feature = "std")]
            #[cfg(test)]
            std::println!("Failure {}", string);
            break;
        } else {
            // You might want to do somethin while in a state
            // You could also add State::enter() and State::exit()
            loop {
                event_response = state_machine.run::<bool>();

                // keep polling until task complete
                if event_response.response == Poll::Pending {
                    continue;
                } else {
                    // response is ready
                    break;
                }
            }
        }

        //     // just a hack to get owned values, because I used an iterator
        //     // let event = iter.next().unwrap().clone();
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::print!("__ Transition from {:?}", state_machine.state);
        state_machine.next(event_response.event);
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!(" to {:?}", state_machine.state);
    }
    // #[cfg(feature = "std")]
    #[cfg(test)]
    std::println!("end loop");
}

// static UART_BUF: StaticCell<BufferedUart> = StaticCell::new();
//
// static INT_EXECUTOR: StaticCell<InterruptExecutor<0>> = StaticCell::new();
//     } else {
//         // fn main() -> ! {
//             // loop {}
//         // }
//     }
// }
// #[cfg(not(feature = "std"))]
//
//
// #[cfg_attr(not(test), entry)]

// #[cfg(not(feature = "std"))]
// #[cfg(not(test))]
// #[panic_handler]
// fn panic(_info: &PanicInfo) -> ! {
// Customize as needed. A common embedded-safe version:
// loop {}
// }

// use embassy_time::{Duration, Timer};
// use esp_backtrace as _;
// use esp_hal::{gpio, timer::timg::TimerGroup};

// #[esp_hal_embassy::main]
// async fn main(_spawner: embassy_executor::Spawner) {
//     esp_println::logger::init_logger_from_env();
//     let peripherals = esp_hal::init(esp_hal::Config::default());

//     esp_println::println!("Init!");

//     let timer_group_0 = TimerGroup::new(peripherals.TIMG0);
//     esp_hal_embassy::init(timer_group_0.timer0);

//     let mut led = gpio::Output::new(peripherals.GPIO0, gpio::Level::Low);
//     let mut led_state = false;
//     led.set_level(led_state.into());

//     loop {
//         Timer::after(Duration::from_millis(5000)).await;
//         led_state = !led_state;
//         led.set_level(led_state.into());
//     }
// }
