#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
// #![no_std]
// #![no_main]

//pub mod fsm;

use heapless::String;
// // Inspired by https://play.rust-lang.org/?version=stable&mode=debug&edition=2015&gist=ee3e4df093c136ced7b394dc7ffb78e1
// // Originally described in https://hoverbear.org/blog/rust-state-machine-pattern/
// // Resurfaced at HN: https://news.ycombinator.com/item?id=43741051

// // Tenets:
// //  1. Lightweight and easy to understand/change.
// //  2. Should not interfere in performance, only "big" state transitions should be tracked (not micromanage on bytes sent, etc...).
// //  3. Non intrusive in application code.

use core::{
    option::Option::{self, None, Some},
    task::Poll,
};

pub mod fake_stamp {
    use core::task::Poll;

    pub fn poll_power_on() -> Poll<bool> {
        Poll::Pending
    }
    pub fn poll_init_peripherals() -> Poll<bool> {
        Poll::Pending
    }
    pub fn poll_uart_task() -> Poll<bool> {
        Poll::Pending
    }
    pub mod serial {
        use core::task::Poll;
        pub fn poll_serial_bridge() -> Poll<bool> {
            Poll::Pending
        }
    }
    pub mod serve {
        use core::task::Poll;
        pub fn poll_connection_loop() -> Poll<bool> {
            Poll::Pending
        }
        pub fn poll_authentication() -> Poll<bool> {
            Poll::Pending
        }
        pub fn poll_handle_ssh_client() -> Poll<bool> {
            Poll::Pending
        }
        pub fn poll_connect_ssh_client() -> Poll<bool> {
            Poll::Pending
        }
        pub fn poll_ssh_client_connected() -> Poll<bool> {
            Poll::Pending
        }
        pub fn poll_ssh_client_connected_no_bridge() -> Poll<bool> {
            Poll::Pending
        }
        pub fn poll_notify_client(_client_notified: &str) -> Poll<bool> {
            Poll::Pending
        }
    }
    pub mod settings {
        use core::task::Poll;
        pub fn poll_read_env_vars() -> Poll<bool> {
            Poll::Pending
        }
        pub fn poll_store_env_vars() -> Poll<bool> {
            Poll::Pending
        }
    }
    pub mod espressif {
        pub mod net {
            use core::task::Poll;
            #[derive(Debug, PartialEq, Clone)]
            pub enum WifiMode {
                ApMode,
                StaMode,
            }
            pub fn poll_if_up() -> Poll<bool> {
                Poll::Pending
            }
            pub fn poll_accept_requests() -> Poll<bool> {
                Poll::Pending
            }
            pub fn poll_wifi_mode_up(_wifimode: Option<WifiMode>) -> Poll<bool> {
                Poll::Pending
            }
            pub fn poll_net_up() -> Poll<bool> {
                Poll::Pending
            }
            pub fn poll_dhcp_server() -> Poll<bool> {
                Poll::Pending
            }
        }
    }
}

#[cfg(feature = "test")]
pub use crate::fake_stamp as stamp;
#[cfg(not(feature = "test"))]
use stamp;

#[cfg(feature = "test")]
pub use crate::fake_stamp::espressif::net;
#[cfg(not(feature = "test"))]
use stamp::espressif::net;

#[cfg(feature = "test")]
use crate::fake_stamp::espressif::net::WifiMode;
#[cfg(not(feature = "test"))]
use net::WifiMode;

#[cfg(test)]
extern crate std;

// use strum::IntoEnumIterator;
// use strum_macros::EnumIter;

pub struct TaskOk {
    ssh: bool,
    uart: bool,
    bridge: bool,
}

struct Settings {
    wifi_mode: Option<WifiMode>,
    ssh_password: Option<String<20>>,
    uart_baud: Option<u32>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum State<'a> {
    PowerOn, // Both PowerOn and Reset represent states where peripherals are not initialised yet.
    Reset,
    Start, // Represents state where peripherals and basics are initialised
    Idle,
    Timeout,
    Failure(&'a str),
    BridgeInit,
    BridgeUp,
    InitPeripherals,
    TcpCheckSettings,
    TcpInit,
    TcpStartDefault,
    TcpStartApMode,
    TcpStartStaMode,
    TcpStackUp, // Network stack bringup/init
    TaskSpawning { name: &'a str },
    TaskFailed { name: &'a str },
    TaskRunning { name: &'a str },
    AllTasksOk,
    ClientConnecting,
    ClientConnected,
    AuthzChecks,
    SshConnInit,
    SshConnEstablished,
    ReadEnvVars,
    StoreEnvVars,
    UartReconf,
    WifiReconf,
    SshReconf,
    CheckBridge,
    SshUartBridgeEstablished,
    ClientConnectedNoBridge,
    ClientNotify { error: &'a str },
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Response {
    Wait,
    Complete,
}

// #[derive(Debug, Copy, Clone, EnumIter)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Event {
    AllGood,
    Fail,
    StartDefaultAp,
    StartApMode,
    StartStaMode,
    // UartReconf,
    ClientConnect,
    Timeout,
    AccessDenied,
    SshDisconnect,
    UartSettingsChanged,
    SshSettingsChanged,
    WifiSettingsChanged,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EventResponse<T> {
    pub event: Event,
    pub response: Poll<T>,
}

pub struct StateMachine<'a> {
    pub state: State<'a>,
    settings: Settings,
    task_ok: TaskOk,
}

impl<'a> StateMachine<'a> {
    pub fn new() -> Self {
        StateMachine {
            state: State::PowerOn,
            settings: Settings {
                wifi_mode: None,
                ssh_password: None,
                uart_baud: None,
            },
            task_ok: TaskOk {
                ssh: false,
                uart: false,
                bridge: false,
            },
        }
    }

    pub fn next(&mut self, event: Event) {
        self.state = match (&self.state, event) {
            (State::PowerOn, Event::AllGood) => State::InitPeripherals,
            (State::Reset, Event::AllGood) => State::InitPeripherals,
            (State::InitPeripherals, Event::AllGood) => State::TcpInit,
            (State::InitPeripherals, Event::Fail) => State::Reset,
            (State::TcpInit, Event::StartApMode) => State::TcpStartApMode,
            (State::TcpInit, Event::StartStaMode) => State::TcpStartStaMode,
            (State::TcpInit, Event::StartDefaultAp) => State::TcpStartDefault,
            (State::TcpInit, Event::Fail) => State::Reset,
            (State::TcpStartDefault, Event::AllGood) => State::TaskSpawning { name: "ssh" },
            (State::TcpStartDefault, Event::Fail) => State::Reset,
            (State::TcpStartApMode, Event::AllGood) => State::TaskSpawning { name: "ssh" },
            (State::TcpStartApMode, Event::Fail) => State::Reset,
            (State::TcpStartStaMode, Event::AllGood) => State::TaskSpawning { name: "ssh" },
            (State::TcpStartStaMode, Event::Fail) => State::Reset,
            (State::TaskSpawning { name: "ssh" }, Event::AllGood) => {
                State::TaskRunning { name: "ssh" }
            }
            (State::TaskRunning { name: "ssh" }, Event::AllGood) => {
                State::TaskSpawning { name: "uart" }
            }
            (State::TaskSpawning { name: "uart" }, Event::AllGood) => {
                State::TaskRunning { name: "uart" }
            }
            (State::TaskRunning { name: "uart" }, Event::AllGood) => State::BridgeInit,
            (State::TaskRunning { name: "uart" }, Event::Fail) => State::Idle,
            (State::BridgeInit, Event::AllGood) => State::Idle,
            (State::BridgeInit, Event::Fail) => State::Idle,
            (State::Idle, Event::ClientConnect) => State::ClientConnecting,
            (State::ClientConnecting, Event::AllGood) => State::AuthzChecks,
            (State::ClientConnecting, Event::Timeout) => State::Idle,
            (State::AuthzChecks, Event::Timeout) => State::Idle,
            (State::AuthzChecks, Event::AccessDenied) => State::Idle,
            (State::AuthzChecks, Event::AllGood) => State::SshConnInit,
            (State::SshConnInit, Event::AllGood) => State::ReadEnvVars,
            (State::SshConnInit, Event::SshDisconnect) => State::Idle,
            (State::ReadEnvVars, Event::AllGood) => State::StoreEnvVars,
            (State::ReadEnvVars, Event::Fail) => State::ClientNotify {
                error: ("Input validation error"),
            },
            (
                State::ClientNotify {
                    error: "Input validation error",
                },
                Event::AllGood,
            ) => State::Idle,
            (State::StoreEnvVars, Event::AllGood) => State::CheckBridge,
            (State::CheckBridge, Event::AllGood) => State::SshUartBridgeEstablished,
            (State::CheckBridge, Event::Fail) => State::ClientConnectedNoBridge,
            (State::SshUartBridgeEstablished, Event::SshDisconnect) => State::Idle,
            (State::ClientConnectedNoBridge, Event::AllGood) => State::ClientNotify {
                error: "Bridge error",
            },
            (
                State::ClientNotify {
                    error: "Bridge error",
                },
                Event::SshDisconnect,
            ) => State::Idle,
            (State::StoreEnvVars, Event::SshSettingsChanged) => State::SshReconf,
            (State::SshReconf, Event::AllGood) => State::TaskSpawning { name: ("ssh") },
            (State::StoreEnvVars, Event::UartSettingsChanged) => State::UartReconf,
            (State::UartReconf, Event::AllGood) => State::TaskSpawning { name: ("uart") },
            (State::StoreEnvVars, Event::WifiSettingsChanged) => State::WifiReconf,
            (State::WifiReconf, Event::AllGood) => State::TcpInit,
            (_s, _e) => {
                // TODO: Implement appropriate formatters/display trait
                //State::Start(println!("Wrong state, event combination: {} {}", s, e))
                // State::Start
                State::Failure("fail")
            }
        };
    }

    pub fn run<Bool>(&mut self) -> EventResponse<bool> {
        let event_response: EventResponse<bool> = match self.state {
            // State::Failure(_) => Event::Fail,
            State::Start => EventResponse {
                event: Event::AllGood,
                response: Poll::Ready(true),
            },
            State::PowerOn => self.power_on(),
            State::InitPeripherals => self.init_peripherals(),
            State::TcpInit => self.tcp_init(),
            State::TcpStartDefault => self.tcp_start(None),
            State::TcpStartApMode => self.tcp_start(Some(WifiMode::ApMode)),
            State::TcpStartStaMode => self.tcp_start(Some(WifiMode::StaMode)),
            State::TaskSpawning { name: "ssh" } => self.spawn_tasks("ssh"),
            State::TaskRunning { name: "ssh" } => self.run_tasks("ssh"),
            State::TaskSpawning { name: "uart" } => self.spawn_tasks("uart"),
            State::TaskRunning { name: "uart" } => self.run_tasks("uart"),
            State::AllTasksOk => self.all_tasks_ok(),
            State::BridgeInit => self.bridge_up(),
            State::Idle => self.idle(),
            State::ClientConnecting => self.connect_client(),
            State::AuthzChecks => self.authorization_checks(),
            State::SshConnInit => self.ssh_connection_initialisation(),
            State::ReadEnvVars => self.read_env_vars(),
            State::StoreEnvVars => self.store_env_vars(),
            State::SshReconf => self.send_notification(Event::SshSettingsChanged),
            State::UartReconf => self.send_notification(Event::UartSettingsChanged),
            State::WifiReconf => self.send_notification(Event::WifiSettingsChanged),
            State::CheckBridge => self.check_bridge(),
            State::SshUartBridgeEstablished => self.ssh_uart_bridge_established(),
            State::ClientConnectedNoBridge => self.client_connected_no_bridge(),
            State::ClientNotify {
                error: "Bridge error",
            } => self.notify_client("no bridge"),
            _ => EventResponse {
                event: Event::Fail,
                response: Poll::Pending,
            },
        };

        event_response
    }

    fn power_on(&self) -> EventResponse<bool> {
        #[cfg(test)]
        std::println!("Powering up");

        let response: Poll<bool> = stamp::poll_power_on();
        // let response: Poll<bool> = poll_once(stamp::power_on());
        // poll_once(power_on())
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn init_peripherals(&self) -> EventResponse<bool> {
        #[cfg(test)]
        std::println!("Starting peripherals up");

        let response: Poll<bool> = stamp::poll_init_peripherals();
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn tcp_init(&self) -> EventResponse<bool> {
        #[cfg(test)]
        std::println!("Initalising TCP");
        // #[cfg(feature = "std")]
        let event = match self.settings.wifi_mode {
            Some(WifiMode::ApMode) => Event::StartApMode,
            Some(WifiMode::StaMode) => Event::StartStaMode,
            _ => Event::StartDefaultAp,
        };
        let response: Poll<bool> = match event {
            Event::StartDefaultAp => Poll::Ready(true),
            Event::StartApMode => Poll::Ready(true),
            Event::StartStaMode => Poll::Ready(true),
            _ => Poll::Pending,
        };
        EventResponse {
            event: event,
            response: response,
        }
    }

    fn tcp_start(&self, wifimode: Option<WifiMode>) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Starting TCP Stack {:?}", wifimode);
        let response: Poll<bool> = net::poll_wifi_mode_up(wifimode);
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }
    fn spawn_tasks(&self, task: &str) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Spawning a task");
        if task == "ssh" {
            // #[cfg(feature = "std")]
            #[cfg(test)]
            std::println!("Spawning SSH task");
            EventResponse {
                event: Event::AllGood,
                response: Poll::Ready(true),
            }
        } else if task == "uart" {
            // #[cfg(feature = "std")]
            #[cfg(test)]
            std::println!("Spawning Uart task");
            EventResponse {
                event: Event::AllGood,
                response: Poll::Ready(true),
            }
        } else {
            EventResponse {
                event: Event::Fail,
                response: Poll::Ready(false),
            }
        }
    }

    fn run_tasks(
        &mut self,
        task: &str,
        // settings: Settings,
        // mut tasks_ok: TaskOk,
    ) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Running a task");
        if task == "ssh" {
            // #[cfg(feature = "std")]
            #[cfg(test)]
            std::println!("Running SSH task");

            let response: Poll<bool>;
            response = stamp::serve::poll_handle_ssh_client();
            let event: Event = match response {
                Poll::Ready(value) => {
                    if value {
                        self.task_ok.ssh = true;
                        Event::AllGood
                    } else {
                        Event::Fail
                    }
                }
                Poll::Pending => Event::Fail,
            };

            EventResponse {
                event: event,
                response: response,
            }
        } else if task == "uart" {
            // #[cfg(feature = "std")]
            #[cfg(test)]
            std::println!("Running Uart task");

            let response: Poll<bool>;
            response = stamp::poll_uart_task();
            let event: Event = match response {
                Poll::Ready(value) => {
                    if value {
                        self.task_ok.uart = true;
                        Event::AllGood
                    } else {
                        Event::Fail
                    }
                }
                Poll::Pending => Event::Fail,
            };

            EventResponse {
                event: event,
                response: response,
            }
        } else {
            EventResponse {
                event: Event::Fail,
                response: Poll::Ready(false),
            }
        }
    }

    fn all_tasks_ok(&self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Connecting to client");
        if self.task_ok.uart && self.task_ok.ssh {
            EventResponse {
                event: Event::AllGood,
                response: Poll::Ready(true),
            }
        } else {
            EventResponse {
                event: Event::Fail,
                response: Poll::Ready(false),
            }
        }
    }

    fn bridge_up(&mut self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Bridging SSH to UART");
        // serial::poll_serial_bridge();

        let response: Poll<bool> = stamp::serial::poll_serial_bridge();

        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    self.task_ok.bridge = true;
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn idle(&self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Connecting to client");

        let response = net::poll_accept_requests();
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::ClientConnect
                } else {
                    Event::Timeout
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn connect_client(&self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Connecting to client");
        let response = stamp::serve::poll_handle_ssh_client();
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn authorization_checks(&self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Checking ssh authorisation");
        let response = stamp::serve::poll_connection_loop();
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::AccessDenied
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn ssh_connection_initialisation(&self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Initialising ssh connection to client");
        let response = stamp::serve::poll_connect_ssh_client();
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn read_env_vars(&self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Reading env vars from client");

        let response = stamp::settings::poll_read_env_vars();
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn store_env_vars(&mut self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Storing env vars from client");
        let response = stamp::settings::poll_store_env_vars();
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    self.settings.wifi_mode = Some(WifiMode::ApMode);
                    self.settings.uart_baud = Some(9600);
                    self.settings.ssh_password = Some(String::<20>::new());
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn send_notification(&self, notification_type: Event) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Sending notification to client");

        let response: Poll<bool>;
        if notification_type == Event::SshSettingsChanged {
            response = Poll::Ready(true);
        } else if notification_type == Event::UartSettingsChanged {
            response = Poll::Ready(true);
        } else if notification_type == Event::WifiSettingsChanged {
            response = Poll::Ready(true);
        } else {
            response = Poll::Pending;
        }
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn check_bridge(&self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Checking if bridge is up");

        if self.task_ok.bridge {
            EventResponse {
                event: Event::AllGood,
                response: Poll::Ready(true),
            }
        } else {
            EventResponse {
                event: Event::Fail,
                response: Poll::Ready(false),
            }
        }
    }

    fn ssh_uart_bridge_established(&self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Storing env vars from client");

        let response = stamp::serve::poll_ssh_client_connected();
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::SshDisconnect
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn client_connected_no_bridge(&self) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Prepare to notify client of no bridge");

        let response: Poll<bool> = stamp::serve::poll_ssh_client_connected_no_bridge();
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::AllGood
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }

    fn notify_client(&self, client_notified: &str) -> EventResponse<bool> {
        // #[cfg(feature = "std")]
        #[cfg(test)]
        std::println!("Notified client of no bridge");

        let response = stamp::serve::poll_notify_client(client_notified);
        let event: Event = match response {
            Poll::Ready(value) => {
                if value {
                    Event::SshDisconnect
                } else {
                    Event::Fail
                }
            }
            Poll::Pending => Event::Fail,
        };

        EventResponse {
            event: event,
            response: response,
        }
    }
}

#[cfg(test)]
use injectorpp::interface::injector::*; //used for sans-io testing
// use core::future::Future;

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    //
    // Testing Run Events
    //

    #[test]
    fn run_poweron_allgood() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn (stamp::poll_power_on)() -> Poll<bool>))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));
        let response = state_machine.power_on();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_power_on_fail() {
        let state_machine = StateMachine::new();

        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::poll_power_on)() ->Poll<bool> ))
            // .when_called(injectorpp::func!(fn(poll_once)() ->Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.power_on();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_init_peripherals_success() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::poll_init_peripherals)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.init_peripherals();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_init_peripherals_fail() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::poll_init_peripherals)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.init_peripherals();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_init_peripherals_pending() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::poll_init_peripherals)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.init_peripherals();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_tcp_init_default() {
        let mut state_machine = StateMachine::new();
        state_machine.settings.wifi_mode = None;
        let response = state_machine.tcp_init();
        assert_eq!(response.event, Event::StartDefaultAp);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_tcp_init_apmode() {
        let mut state_machine = StateMachine::new();
        state_machine.settings.wifi_mode = Some(WifiMode::ApMode);
        let response = state_machine.tcp_init();
        assert_eq!(response.event, Event::StartApMode);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_tcp_init_stamode() {
        let mut state_machine = StateMachine::new();
        state_machine.settings.wifi_mode = Some(WifiMode::StaMode);
        let response = state_machine.tcp_init();
        assert_eq!(response.event, Event::StartStaMode);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_tcp_init_empty() {
        let state_machine = StateMachine::new();
        let response = state_machine.tcp_init();
        assert_eq!(response.event, Event::StartDefaultAp);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_tcp_start_default_success() {
        let state_machine = StateMachine::new();

        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(net::poll_wifi_mode_up)(Option<WifiMode>) -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn(_wifimode: Option<WifiMode>) -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.tcp_start(None);
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_tcp_start_apmode_success() {
        let state_machine = StateMachine::new();

        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(net::poll_wifi_mode_up)(Option<WifiMode>) -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn(_wifimode: Option<WifiMode>) -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.tcp_start(Some(WifiMode::ApMode));
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_tcp_start_stamode_success() {
        let state_machine = StateMachine::new();

        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(net::poll_wifi_mode_up)(Option<WifiMode>) -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn(_wifimode: Option<WifiMode>) -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.tcp_start(Some(WifiMode::StaMode));
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_tcp_start_fail() {
        let state_machine = StateMachine::new();

        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(net::poll_wifi_mode_up)(Option<WifiMode>) -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn(_wifimode: Option<WifiMode>) -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.tcp_start(None);
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_tcp_start_pending() {
        let state_machine = StateMachine::new();

        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(net::poll_wifi_mode_up)(Option<WifiMode>) -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn(_wifimode: Option<WifiMode>) -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.tcp_start(None);
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_spawn_tasks_ssh_success() {
        let state_machine = StateMachine::new();
        let response = state_machine.spawn_tasks("ssh");
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_spawn_tasks_uart_success() {
        let state_machine = StateMachine::new();
        let response = state_machine.spawn_tasks("ssh");
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_spawn_tasks_fail() {
        let state_machine = StateMachine::new();
        let response = state_machine.spawn_tasks("");
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_run_tasks_ssh_success() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_handle_ssh_client)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.run_tasks("ssh");
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
        assert_eq!(state_machine.task_ok.ssh, true);
    }

    #[test]
    fn run_run_tasks_ssh_fail() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_handle_ssh_client)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.run_tasks("ssh");
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
        assert_eq!(state_machine.task_ok.ssh, false);
    }

    #[test]
    fn run_run_tasks_ssh_pending() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_handle_ssh_client)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.run_tasks("ssh");
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
        assert_eq!(state_machine.task_ok.ssh, false);
    }

    #[test]
    fn run_run_tasks_uart_success() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::poll_uart_task)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.run_tasks("uart");
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
        assert_eq!(state_machine.task_ok.uart, true);
    }

    #[test]
    fn run_run_tasks_uart_fail() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::poll_uart_task)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.run_tasks("uart");
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
        assert_eq!(state_machine.task_ok.uart, false);
    }

    #[test]
    fn run_run_tasks_uart_pending() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::poll_uart_task)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.run_tasks("uart");
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
        assert_eq!(state_machine.task_ok.uart, false);
    }

    #[test]
    fn run_run_tasks_fail() {
        let mut state_machine = StateMachine::new();
        let response = state_machine.run_tasks("");
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_all_tasks_ok_success() {
        let mut state_machine = StateMachine::new();
        state_machine.task_ok.uart = true;
        state_machine.task_ok.ssh = true;
        let response = state_machine.all_tasks_ok();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_all_tasks_ok_uart_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.task_ok.ssh = true;
        let response = state_machine.all_tasks_ok();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_all_tasks_ok_ssh_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.task_ok.uart = true;
        let response = state_machine.all_tasks_ok();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_all_tasks_ok_all_fail() {
        let state_machine = StateMachine::new();
        let response = state_machine.all_tasks_ok();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_bridge_up_fail() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::serial::poll_serial_bridge)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.bridge_up();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
        assert_eq!(state_machine.task_ok.bridge, false);
    }

    #[test]
    fn run_bridge_up_success() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::serial::poll_serial_bridge)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.bridge_up();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
        assert_eq!(state_machine.task_ok.bridge, true);
    }

    #[test]
    fn run_bridge_up_pending() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::serial::poll_serial_bridge)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.bridge_up();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
        assert_eq!(state_machine.task_ok.bridge, false);
    }

    #[test]
    fn run_idle_connection() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(net::poll_accept_requests)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.idle();
        assert_eq!(response.event, Event::ClientConnect);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_idle_timeout() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(net::poll_accept_requests)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.idle();
        assert_eq!(response.event, Event::Timeout);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_idle_pending() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(net::poll_accept_requests)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.idle();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_connect_client_success() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_handle_ssh_client)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.connect_client();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_connect_client_fail() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_handle_ssh_client)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.connect_client();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_connect_client_pending() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_handle_ssh_client)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.connect_client();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_authorization_checks_success() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::serve::poll_connection_loop)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.authorization_checks();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_authorization_checks_fail() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::serve::poll_connection_loop)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.authorization_checks();
        assert_eq!(response.event, Event::AccessDenied);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_authorization_checks_pending() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(injectorpp::func!(fn(stamp::serve::poll_connection_loop)() -> Poll<bool> ))
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.authorization_checks();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::<bool>::Pending);
    }

    #[test]
    fn run_ssh_connection_initialisation_succcess() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_connect_ssh_client)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.ssh_connection_initialisation();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_ssh_connection_initialisation_fail() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_connect_ssh_client)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.ssh_connection_initialisation();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_ssh_connection_initialisation_pending() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_connect_ssh_client)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.ssh_connection_initialisation();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_read_env_vars_success() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::settings::poll_read_env_vars)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.read_env_vars();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_read_env_vars_fail() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::settings::poll_read_env_vars)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.read_env_vars();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_read_env_vars_pending() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::settings::poll_read_env_vars)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.read_env_vars();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_store_env_vars_success() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::settings::poll_store_env_vars)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.store_env_vars();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
        assert_eq!(state_machine.settings.wifi_mode, Some(WifiMode::ApMode));
        assert_eq!(state_machine.settings.uart_baud, Some(9600));
        assert_eq!(
            state_machine.settings.ssh_password,
            Some(String::<20>::new())
        );
    }

    #[test]
    fn run_store_env_vars_fail() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::settings::poll_store_env_vars)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.store_env_vars();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_store_env_vars_pending() {
        let mut state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::settings::poll_store_env_vars)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.store_env_vars();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_send_notification_ssh() {
        let state_machine = StateMachine::new();
        let response = state_machine.send_notification(Event::SshSettingsChanged);
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_send_notification_uart() {
        let state_machine = StateMachine::new();
        let response = state_machine.send_notification(Event::UartSettingsChanged);
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_send_notification_wifi() {
        let state_machine = StateMachine::new();
        let response = state_machine.send_notification(Event::WifiSettingsChanged);
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_send_notification_pending() {
        let state_machine = StateMachine::new();
        let response = state_machine.send_notification(Event::Fail);
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_check_bridge_success() {
        let mut state_machine = StateMachine::new();
        state_machine.task_ok.bridge = true;
        let response = state_machine.check_bridge();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_check_bridge_fail() {
        let state_machine = StateMachine::new();
        let response = state_machine.check_bridge();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_ssh_uart_bridge_established_success() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_ssh_client_connected)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.ssh_uart_bridge_established();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_ssh_uart_bridge_established_fail() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_ssh_client_connected)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.ssh_uart_bridge_established();
        assert_eq!(response.event, Event::SshDisconnect);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_ssh_uart_bridge_established_pending() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_ssh_client_connected)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.ssh_uart_bridge_established();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_client_connected_no_bridge_success() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_ssh_client_connected_no_bridge)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.client_connected_no_bridge();
        assert_eq!(response.event, Event::AllGood);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_client_connected_no_bridge_fail() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_ssh_client_connected_no_bridge)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.client_connected_no_bridge();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_client_connected_no_bridge_pending() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_ssh_client_connected_no_bridge)() -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn() -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.client_connected_no_bridge();
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    #[test]
    fn run_notify_client_success() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_notify_client)(&str) -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn(_string: &str) -> Poll<bool>,
                returns: Poll::Ready(true),
                times: 1
            ));

        let response = state_machine.notify_client("no bridge");
        assert_eq!(response.event, Event::SshDisconnect);
        assert_eq!(response.response, Poll::Ready(true));
    }

    #[test]
    fn run_notify_client_fail() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_notify_client)(&str) -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn(_string: &str) -> Poll<bool>,
                returns: Poll::Ready(false),
                times: 1
            ));

        let response = state_machine.notify_client("");
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Ready(false));
    }

    #[test]
    fn run_notify_client_pending() {
        let state_machine = StateMachine::new();
        let mut injector = InjectorPP::new();
        injector
            .when_called(
                injectorpp::func!(fn(stamp::serve::poll_notify_client)(&str) -> Poll<bool> ),
            )
            .will_execute(injectorpp::fake!(
                func_type: fn(_string: &str) -> Poll<bool>,
                returns: Poll::Pending,
                times: 1
            ));

        let response = state_machine.notify_client("");
        assert_eq!(response.event, Event::Fail);
        assert_eq!(response.response, Poll::Pending);
    }

    //
    //
    //
    // Testing State Changes
    //
    //
    //
    #[test]
    fn next_power_on_to_init_peripherals() {
        let mut state_machine = StateMachine::new();
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::InitPeripherals);
    }

    #[test]
    fn net_power_on_not_good() {
        let mut state_machine = StateMachine::new();
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Failure("fail"));
    }

    #[test]
    fn next_reset_to_init_peripherals() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::Reset;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::InitPeripherals);
    }

    #[test]
    fn next_init_peripherals_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::InitPeripherals;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Reset);
    }

    #[test]
    fn next_init_peripherals_to_tcp_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::InitPeripherals;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpInit);
    }

    #[test]
    fn next_tcp_init_to_tcp_start_default() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpInit;
        let event: Event = Event::StartDefaultAp;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpStartDefault);
    }

    #[test]
    fn next_tcp_start_default_to_ssh() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartDefault;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("ssh") });
    }

    #[test]
    fn next_tcp_start_default_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartDefault;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Reset);
    }

    #[test]
    fn next_tcp_init_to_tcp_start_ap_mode() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpInit;
        let event: Event = Event::StartApMode;
        state_machine.settings.wifi_mode = Some(WifiMode::ApMode);
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpStartApMode);
    }

    #[test]
    fn next_tcp_start_ap_mode_to_ssh() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartApMode;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("ssh") });
    }

    #[test]
    fn next_tcp_start_ap_mode_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartApMode;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Reset);
    }

    #[test]
    fn next_tcp_init_to_tcp_start_sta_mode() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpInit;
        let event: Event = Event::StartStaMode;
        state_machine.settings.wifi_mode = Some(WifiMode::StaMode);
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpStartStaMode);
    }

    #[test]
    fn next_tcp_start_sta_mode_to_ssh() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartStaMode;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("ssh") });
    }

    #[test]
    fn next_tcp_start_sta_mode_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartStaMode;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Reset);
    }

    #[test]
    fn next_ssh_init_to_ssh_running() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskSpawning { name: ("ssh") };
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskRunning { name: ("ssh") });
    }

    #[test]
    fn next_ssh_running_to_uart_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskRunning { name: ("ssh") };
        let event: Event = Event::AllGood;
        // let settings: Settings = setup_settings();
        // let tasks_ok: TaskOk = setup_tasks();
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("uart") });
    }

    #[test]
    fn next_uart_init_to_uart_running() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskSpawning { name: ("uart") };
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskRunning { name: ("uart") });
    }

    #[test]
    fn next_uart_running_to_bridge_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskRunning { name: ("uart") };
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::BridgeInit);
    }

    #[test]
    fn next_uart_running_fail_to_idle() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskRunning { name: ("uart") };
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn next_bridge_init_to_idle() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::BridgeInit;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn next_bridge_init_fail_to_idle() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::BridgeInit;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn next_idle_to_client_connecting() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::Idle;
        let event: Event = Event::ClientConnect;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::ClientConnecting);
    }

    #[test]
    fn next_client_connecting_to_timeout() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ClientConnecting;
        let event: Event = Event::Timeout;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn next_client_connecting_authorisation_checks() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ClientConnecting;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::AuthzChecks);
    }

    #[test]
    fn next_authorisation_checks_to_timeout() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::AuthzChecks;
        let event: Event = Event::Timeout;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn next_authorisation_checks_to_access_denied() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::AuthzChecks;
        let event: Event = Event::AccessDenied;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn next_authorisation_checks_to_access_granted() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::AuthzChecks;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::SshConnInit);
    }

    #[test]
    fn next_ssh_init_to_read_env_vars() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::SshConnInit;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::ReadEnvVars);
    }

    #[test]
    fn next_ssh_init_dropped() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::SshConnInit;
        let event: Event = Event::SshDisconnect;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn next_read_env_vars_validation_error() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ReadEnvVars;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(
            state_machine.state,
            State::ClientNotify {
                error: ("Input validation error")
            }
        );
    }
    #[test]
    fn next_notify_validation_error_to_idle() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ClientNotify {
            error: ("Input validation error"),
        };
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }
    #[test]
    fn next_read_env_vars_ok() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ReadEnvVars;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::StoreEnvVars);
    }

    #[test]
    fn next_store_env_vars_no_change_bridge_ok() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::StoreEnvVars;
        let event: Event = Event::AllGood;
        state_machine.task_ok.bridge = true;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::CheckBridge);
    }

    #[test]
    fn next_client_connected_ssh_uart_bridge_to_disconnect() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::SshUartBridgeEstablished;
        let event: Event = Event::SshDisconnect;
        state_machine.task_ok.bridge = true;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn next_bridge_check_ok() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::CheckBridge;
        let event: Event = Event::AllGood;
        state_machine.task_ok.bridge = true;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::SshUartBridgeEstablished);
    }

    #[test]
    fn next_bridge_check_error() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::CheckBridge;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::ClientConnectedNoBridge);
    }

    #[test]
    fn next_client_connected_bridge_error_to_notify() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ClientConnectedNoBridge;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(
            state_machine.state,
            State::ClientNotify {
                error: ("Bridge error")
            }
        );
    }

    #[test]
    fn next_bridge_error_notify_to_disconnect() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ClientNotify {
            error: ("Bridge error"),
        };
        let event: Event = Event::SshDisconnect;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn next_store_env_vars_ssh_changed() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::StoreEnvVars;
        let event: Event = Event::SshSettingsChanged;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::SshReconf);
    }

    #[test]
    fn next_ssh_changed_to_ssh_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::SshReconf;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("ssh") });
    }

    #[test]
    fn next_store_env_vars_wifi_changed() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::StoreEnvVars;
        let event: Event = Event::WifiSettingsChanged;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::WifiReconf);
    }

    #[test]
    fn next_wifi_changed_to_tcp_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::WifiReconf;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpInit);
    }

    #[test]
    fn next_store_env_vars_uart_changed() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::StoreEnvVars;
        let event: Event = Event::UartSettingsChanged;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::UartReconf);
    }

    #[test]
    fn next_uart_changed_to_uart_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::UartReconf;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("uart") });
    }

    // #[test]
    // fn task_spawning_to_task_running() {
    //     let mut state: State = State::TaskSpawning { name: "G'day" };
    //     let event: Event = Event::AllGood;
    //     let settings: Settings = setup_settings();
    //     let tasks_ok: TaskOk = setup_tasks();
    //     state = state.next(event, &settings, &tasks_ok);
    //     assert_eq!(state, State::TaskRunning { name: "A task?" });
    // }
}
