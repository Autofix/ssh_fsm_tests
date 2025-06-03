//#![no_std]
// #![no_main]

use heapless::String;
// Inspired by https://play.rust-lang.org/?version=stable&mode=debug&edition=2015&gist=ee3e4df093c136ced7b394dc7ffb78e1
// Originally described in https://hoverbear.org/blog/rust-state-machine-pattern/
// Resurfaced at HN: https://news.ycombinator.com/item?id=43741051

// Tenets:
//  1. Lightweight and easy to understand/change.
//  2. Should not interfere in performance, only "big" state transitions should be tracked (not micromanage on bytes sent, etc...).
//  3. Non intrusive in application code.

#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;
// use strum::IntoEnumIterator;
// use strum_macros::EnumIter;

#[derive(Debug, PartialEq, Clone)]
enum WifiMode {
    ApMode,
    StaMode,
}

struct Settings {
    wifi_mode: Option<WifiMode>,
    ssh_password: Option<String<20>>,
    uart_baud: Option<u32>,
}

struct TaskOk {
    ssh: bool,
    uart: bool,
    bridge: bool,
}

#[derive(Debug, PartialEq)]
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
    SshUartBridgeEstablished,
    ClientConnectedNoBridge,
    ClientNotify { error: &'a str },
}

// #[derive(Debug, Copy, Clone, EnumIter)]
#[derive(Debug, Copy, Clone, PartialEq)]
enum Event {
    AllGood,
    Fail,
    DefaultModeUp,
    ApModeUp,
    StaModeUp,
    // UartReconf,
    ClientConnect,
    Timeout,
    AccessDenied,
    SshDisconnect,
    UartSettingsChanged,
    SshSettingsChanged,
    WifiSettingsChanged,
}

struct StateMachine<'a> {
    state: State<'a>,
    settings: Settings,
    task_ok: TaskOk,
}

impl<'a> StateMachine<'a> {
    fn new() -> Self {
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

    fn next(&mut self, event: Event) {
        self.state = match (&self.state, event) {
            (State::PowerOn, Event::AllGood) => State::InitPeripherals,
            (State::Reset, Event::AllGood) => State::InitPeripherals,
            (State::InitPeripherals, Event::AllGood) => State::TcpInit,
            (State::TcpInit, Event::AllGood) => match self.settings.wifi_mode {
                None => State::TcpStartDefault,
                Some(WifiMode::ApMode) => State::TcpStartApMode,
                Some(WifiMode::StaMode) => State::TcpStartStaMode,
            },
            (State::InitPeripherals, Event::Fail) => State::Reset,
            (State::TcpStartDefault, Event::DefaultModeUp) => State::TaskSpawning { name: "ssh" },
            (State::TcpStartDefault, Event::Fail) => State::Reset,
            (State::TcpStartApMode, Event::ApModeUp) => State::TaskSpawning { name: "ssh" },
            (State::TcpStartApMode, Event::Fail) => State::Reset,
            (State::TcpStartStaMode, Event::StaModeUp) => State::TaskSpawning { name: "ssh" },
            (State::TcpStartStaMode, Event::Fail) => State::Reset,
            (State::TaskSpawning { name: "ssh" }, Event::AllGood) => {
                State::TaskRunning { name: "ssh" }
            }
            (State::TaskRunning { name: "ssh" }, Event::AllGood) => match self.task_ok.uart {
                true => State::BridgeInit,
                false => State::TaskSpawning { name: "uart" },
            },
            (State::TaskSpawning { name: "uart" }, Event::AllGood) => {
                State::TaskRunning { name: "uart" }
            }
            (State::TaskRunning { name: "uart" }, Event::AllGood) => State::BridgeInit,
            (State::TaskRunning { name: "uart" }, Event::Fail) => State::Idle,
            (State::BridgeInit, Event::AllGood) => State::Idle,
            (State::BridgeInit, Event::Fail) => State::Idle,
            // (State::TaskSpawning { .. }, Event::AllGood) => State::TaskRunning { name: "A task?" },
            // (State::AllTasksOk, Event::AllGood) => State::BridgeUp,
            // (State::BridgeUp, Event::AllGood) => State::Idle,
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
            (State::StoreEnvVars, Event::AllGood) => match self.task_ok.bridge {
                true => State::SshUartBridgeEstablished,
                false => State::ClientConnectedNoBridge,
            },
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
    fn run(&mut self) -> Event {
        let event: Event;

        event = match self.state {
            State::Failure(_) => Event::Fail,
            State::Start => Event::AllGood,
            State::PowerOn => self.power_on(true),
            State::InitPeripherals => self.init_peripherals(true),
            State::TcpInit => self.tcp_init(true),
            State::TcpStartDefault => self.tcp_start(None),
            State::TcpStartApMode => self.tcp_start(Some(WifiMode::ApMode)),
            State::TcpStartStaMode => self.tcp_start(Some(WifiMode::StaMode)),
            State::TaskSpawning { name: "ssh" } => self.spawn_tasks("ssh"),
            State::TaskRunning { name: "ssh" } => self.run_tasks("ssh"),
            State::TaskSpawning { name: "uart" } => self.spawn_tasks("uart"),
            State::TaskRunning { name: "uart" } => self.run_tasks("uart"),
            State::AllTasksOk => self.all_tasks_ok(true),
            State::BridgeInit => self.bridge_up(true),
            State::Idle => self.idle(true),
            State::ClientConnecting => self.connect_client(true),
            State::AuthzChecks => self.authorization_checks(true),
            State::SshConnInit => self.ssh_connection_initialisation(true),
            State::ReadEnvVars => self.read_env_vars(true),
            State::StoreEnvVars => self.store_env_vars(true),
            State::SshReconf => self.send_notification(Event::SshSettingsChanged),
            State::UartReconf => self.send_notification(Event::UartSettingsChanged),
            State::WifiReconf => self.send_notification(Event::WifiSettingsChanged),
            State::SshUartBridgeEstablished => self.ssh_uart_bridge_established(true),
            State::ClientConnectedNoBridge => self.client_connected_no_bridge(true),
            State::ClientNotify {
                error: "Bridge error",
            } => self.notify_client("no bridge"),
            _ => Event::Fail,
        };
        event
    }

    fn power_on(&self, powergood: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Powering up");
        if powergood {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn init_peripherals(&self, peripheralgood: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Starting peripherals up");
        if peripheralgood {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn tcp_init(&self, tcp_init_good: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Initalising TCP");
        if tcp_init_good {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn tcp_start(&self, wifimode: Option<WifiMode>) -> Event {
        #[cfg(feature = "std")]
        println!("Starting TCP Stack {:?}", wifimode);
        if wifimode == None {
            Event::DefaultModeUp
        } else if wifimode == Some(WifiMode::ApMode) {
            Event::ApModeUp
        } else if wifimode == Some(WifiMode::StaMode) {
            Event::StaModeUp
        } else {
            Event::Fail
        }
    }
    fn spawn_tasks(&self, task: &str) -> Event {
        #[cfg(feature = "std")]
        println!("Spawning a task");
        if task == "ssh" {
            #[cfg(feature = "std")]
            println!("Spawning SSH task");
            Event::AllGood
        } else if task == "uart" {
            #[cfg(feature = "std")]
            println!("Spawning Uart task");
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn run_tasks(
        &mut self,
        task: &str,
        // settings: Settings,
        // mut tasks_ok: TaskOk,
    ) -> Event {
        #[cfg(feature = "std")]
        println!("Running a task");
        if task == "ssh" {
            #[cfg(feature = "std")]
            println!("Running SSH task");
            self.task_ok.ssh = true;
            Event::AllGood
        } else if task == "uart" {
            #[cfg(feature = "std")]
            println!("Running Uart task");
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn all_tasks_ok(&self, all_tasks_ok: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Connecting to client");
        if all_tasks_ok {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn bridge_up(&self, bridge_up: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Bridging SSH to UART");
        if bridge_up {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn idle(&self, connect_client: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Connecting to client");
        if connect_client {
            Event::ClientConnect
        } else if connect_client == false {
            Event::Timeout
        } else {
            Event::AllGood
        }
    }

    fn connect_client(&self, connect_client: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Connecting to client");
        if connect_client {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn authorization_checks(&self, authz_checks: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Checking ssh authorisation");
        if authz_checks {
            Event::AllGood
        } else {
            Event::AccessDenied
        }
    }

    fn ssh_connection_initialisation(&self, ssh_connection: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Initialising ssh connection to client");
        if ssh_connection {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn read_env_vars(&self, read_env_vars_ok: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Reading env vars from client");
        if read_env_vars_ok {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn store_env_vars(
        &mut self,
        store_env_vars_ok: bool,
        // mut settings: Settings,
        // tasks_ok: TaskOk,
    ) -> Event {
        #[cfg(feature = "std")]
        println!("Storing env vars from client");
        if store_env_vars_ok {
            self.settings.wifi_mode = Some(WifiMode::ApMode);
            // SETTINGS.store_settings(
            //     Some(WifiMode::ApMode),
            //     Some(String::<20>::new()),
            //     Some(9600),
            // );
            // wifi_mode = Some(WifiMode::ApMode);
            self.settings.uart_baud = Some(9600);
            self.settings.ssh_password = Some(String::<20>::new());
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn send_notification(&self, notification_type: Event) -> Event {
        #[cfg(feature = "std")]
        println!("Sending notification to client");
        if notification_type == Event::SshSettingsChanged {
            Event::AllGood
        } else if notification_type == Event::UartSettingsChanged {
            Event::AllGood
        } else if notification_type == Event::WifiSettingsChanged {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn ssh_uart_bridge_established(&self, client_disconnect: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Storing env vars from client");
        if client_disconnect {
            Event::SshDisconnect
        } else {
            Event::AllGood
        }
    }

    fn client_connected_no_bridge(&self, notify_prepared: bool) -> Event {
        #[cfg(feature = "std")]
        println!("Prepare to notify client of no bridge");
        if notify_prepared {
            Event::AllGood
        } else {
            Event::Fail
        }
    }

    fn notify_client(&self, client_notified: &str) -> Event {
        #[cfg(feature = "std")]
        println!("Notified client of no bridge");
        if client_notified == "no bridge" {
            Event::SshDisconnect
        } else {
            Event::Fail
        }
    }
}

// #[unsafe(no_mangle)]
// #[cfg(feature = "std")]
fn main() {
    let mut state_machine = StateMachine::new();
    // Sequence of events (might be dynamic based on what State::run did)
    // TODO: Declare this array automatically from the enum definition above.
    // let mut iter = Event::iter();

    let mut event;

    loop {
        if let State::Failure(string) = state_machine.state {
            #[cfg(feature = "std")]
            println!("Failure {}", string);
            break;
        } else {
            // You might want to do somethin while in a state
            // You could also add State::enter() and State::exit()
            event = state_machine.run();
            // break;
        }
        //     // just a hack to get owned values, because I used an iterator
        //     // let event = iter.next().unwrap().clone();
        #[cfg(feature = "std")]
        print!("__ Transition from {:?}", state_machine.state);
        state_machine.next(event);
        #[cfg(feature = "std")]
        println!(" to {:?}", state_machine.state);
    }
    #[cfg(feature = "std")]
    println!("end loop");
}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Customize as needed. A common embedded-safe version:
    loop {}
}

#[cfg(test)]
mod tests {

    extern crate std;
    use super::*;

    #[test]
    fn power_on_to_init_peripherals() {
        let mut state_machine = StateMachine::new();
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::InitPeripherals);
    }

    #[test]
    fn power_on_not_good() {
        let mut state_machine = StateMachine::new();
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Failure("fail"));
    }

    #[test]
    fn reset_to_init_peripherals() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::Reset;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::InitPeripherals);
    }

    #[test]
    fn init_peripherals_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::InitPeripherals;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Reset);
    }

    #[test]
    fn init_peripherals_to_tcp_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::InitPeripherals;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpInit);
    }

    #[test]
    fn tcp_init_to_tcp_start_default() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpInit;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpStartDefault);
    }

    #[test]
    fn tcp_start_default_to_ssh() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartDefault;
        let event: Event = Event::DefaultModeUp;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("ssh") });
    }

    #[test]
    fn tcp_start_default_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartDefault;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Reset);
    }

    #[test]
    fn tcp_init_to_tcp_start_ap_mode() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpInit;
        let event: Event = Event::AllGood;
        state_machine.settings.wifi_mode = Some(WifiMode::ApMode);
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpStartApMode);
    }

    #[test]
    fn tcp_start_ap_mode_to_ssh() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartApMode;
        let event: Event = Event::ApModeUp;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("ssh") });
    }

    #[test]
    fn tcp_start_ap_mode_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartApMode;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Reset);
    }

    #[test]
    fn tcp_init_to_tcp_start_sta_mode() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpInit;
        let event: Event = Event::AllGood;
        state_machine.settings.wifi_mode = Some(WifiMode::StaMode);
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpStartStaMode);
    }

    #[test]
    fn tcp_start_sta_mode_to_ssh() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartStaMode;
        let event: Event = Event::StaModeUp;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("ssh") });
    }

    #[test]
    fn tcp_start_sta_mode_fail() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TcpStartStaMode;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Reset);
    }

    #[test]
    fn ssh_init_to_ssh_running() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskSpawning { name: ("ssh") };
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskRunning { name: ("ssh") });
    }

    #[test]
    fn ssh_running_to_uart_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskRunning { name: ("ssh") };
        let event: Event = Event::AllGood;
        // let settings: Settings = setup_settings();
        // let tasks_ok: TaskOk = setup_tasks();
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("uart") });
    }

    #[test]
    fn ssh_running_uart_ok_to_bridge() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskRunning { name: ("ssh") };
        let event: Event = Event::AllGood;
        state_machine.task_ok.uart = true;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::BridgeInit);
    }

    #[test]
    fn uart_init_to_uart_running() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskSpawning { name: ("uart") };
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskRunning { name: ("uart") });
    }

    #[test]
    fn uart_running_to_bridge_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskRunning { name: ("uart") };
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::BridgeInit);
    }

    #[test]
    fn uart_running_fail_to_idle() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::TaskRunning { name: ("uart") };
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn bridge_init_to_idle() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::BridgeInit;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn bridge_init_fail_to_idle() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::BridgeInit;
        let event: Event = Event::Fail;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn idle_to_client_connecting() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::Idle;
        let event: Event = Event::ClientConnect;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::ClientConnecting);
    }

    #[test]
    fn client_connecting_to_timeout() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ClientConnecting;
        let event: Event = Event::Timeout;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn client_connecting_authorisation_checks() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ClientConnecting;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::AuthzChecks);
    }

    #[test]
    fn authorisation_checks_to_timeout() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::AuthzChecks;
        let event: Event = Event::Timeout;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn authorisation_checks_to_access_denied() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::AuthzChecks;
        let event: Event = Event::AccessDenied;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn authorisation_checks_to_access_granted() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::AuthzChecks;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::SshConnInit);
    }

    #[test]
    fn ssh_init_to_read_env_vars() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::SshConnInit;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::ReadEnvVars);
    }

    #[test]
    fn ssh_init_dropped() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::SshConnInit;
        let event: Event = Event::SshDisconnect;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn read_env_vars_validation_error() {
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
    fn notify_validation_error_to_idle() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ClientNotify {
            error: ("Input validation error"),
        };
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }
    #[test]
    fn read_env_vars_ok() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ReadEnvVars;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::StoreEnvVars);
    }

    #[test]
    fn store_env_vars_no_change_bridge_ok() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::StoreEnvVars;
        let event: Event = Event::AllGood;
        state_machine.task_ok.bridge = true;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::SshUartBridgeEstablished);
    }

    #[test]
    fn client_connected_ssh_uart_bridge_to_disconnect() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::SshUartBridgeEstablished;
        let event: Event = Event::SshDisconnect;
        state_machine.task_ok.bridge = true;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn store_env_vars_no_change_bridge_error() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::StoreEnvVars;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::ClientConnectedNoBridge);
    }

    #[test]
    fn client_connected_bridge_error_to_notify() {
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
    fn bridge_error_notify_to_disconnect() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::ClientNotify {
            error: ("Bridge error"),
        };
        let event: Event = Event::SshDisconnect;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::Idle);
    }

    #[test]
    fn store_env_vars_ssh_changed() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::StoreEnvVars;
        let event: Event = Event::SshSettingsChanged;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::SshReconf);
    }

    #[test]
    fn ssh_changed_to_ssh_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::SshReconf;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("ssh") });
    }

    #[test]
    fn store_env_vars_wifi_changed() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::StoreEnvVars;
        let event: Event = Event::WifiSettingsChanged;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::WifiReconf);
    }

    #[test]
    fn wifi_changed_to_tcp_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::WifiReconf;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TcpInit);
    }

    #[test]
    fn store_env_vars_uart_changed() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::StoreEnvVars;
        let event: Event = Event::UartSettingsChanged;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::UartReconf);
    }

    #[test]
    fn uart_changed_to_uart_init() {
        let mut state_machine = StateMachine::new();
        state_machine.state = State::UartReconf;
        let event: Event = Event::AllGood;
        state_machine.next(event);
        assert_eq!(state_machine.state, State::TaskSpawning { name: ("uart") });
    }
}
