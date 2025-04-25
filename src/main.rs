//#![no_std]
#![no_main]

// Inspired by https://play.rust-lang.org/?version=stable&mode=debug&edition=2015&gist=ee3e4df093c136ced7b394dc7ffb78e1
// Originally described in https://hoverbear.org/blog/rust-state-machine-pattern/
// Resurfaced at HN: https://news.ycombinator.com/item?id=43741051

// Tenets:
//  1. Lightweight and easy to understand/change.
//  2. Should not interfere in performance, only "big" state transitions should be tracked (not micromanage on bytes sent, etc...).
//  3. Non intrusive in application code.

#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;
use strum_macros::EnumIter;
use strum::IntoEnumIterator;

#[derive(Debug)]
pub enum State<'a> {
    PowerOn, // Both PowerOn and Reset represent states where peripherals are not initialised yet.
    Reset,
    Start,   // Represents state where peripherals and basics are initialised
    Idle,
    Timeout,
    Failure(&'a str),
    BridgeUp,
    InitPeripherals,
    TcpStackUp,
    TaskSpawning { name: &'a str },
    TaskFailed { name: &'a str },
    TaskRunning { name: &'a str },
    AllTasksOk,
    ClientConnecting,
    ClientConnected,
    AuthzChecks,
    SshConnEstablished,
    ReadEnvVars,
    UartReconf,
    SshUartBridgeEstablished,
}

#[derive(Debug, Copy, Clone, EnumIter)]
enum Event {
    AllGood,
    Fail,
    UartReconf,
    ClientConnect,
    SshDisconnect,
}

impl<'a> State<'a> {
    fn next(self, event: Event) -> State<'a> {
        match (self, event) {
            (State::PowerOn, Event::AllGood) => State::TaskSpawning { name: "G'day" },
            (State::TaskSpawning { .. }, Event::AllGood) => State::TaskRunning { name: "A task?" },
            (State::AllTasksOk, Event::AllGood) => State::BridgeUp,
            (State::BridgeUp, Event::AllGood) => State::Idle,
            (State::Idle, Event::AllGood) => State::Idle,
            (State::Idle, Event::ClientConnect) => State::ClientConnecting,
            (State::ClientConnecting, Event::AllGood) => State::ReadEnvVars,
            (State::ReadEnvVars, Event::AllGood) => State::ClientConnected,
            (State::ClientConnected, Event::AllGood) => State::SshUartBridgeEstablished,
            (_s, _e) => {
                // TODO: Implement appropriate formatters/display trait
                //State::Start(println!("Wrong state, event combination: {} {}", s, e))
                State::Start
            }
        }
    }
    fn run(&self) {
        match *self {
            State::Idle |
            State::Failure(_) => {}
            State::Start => {}
            _ => todo!()
        }
    }
}

#[cfg(feature = "std")]
#[unsafe(no_mangle)]
fn main() {
    let mut state = State::Idle;

    // Sequence of events (might be dynamic based on what State::run did)
    // TODO: Declare this array automatically from the enum definition above.
    let mut iter = Event::iter();

    loop {
        // just a hack to get owned values, because I used an iterator
        let event = iter.next().unwrap().clone();
        #[cfg(feature = "std")]
        print!("__ Transition from {:?}", state);
        state = state.next(event);
        #[cfg(feature = "std")]
        println!(" to {:?}", state);

        if let State::Failure(string) = state {
            #[cfg(feature = "std")]
            println!("{}", string);
            break;
        } else {
            // You might want to do somethin while in a state
            // You could also add State::enter() and State::exit()
            state.run();
        }
    }

}


#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Customize as needed. A common embedded-safe version:
    loop {}
}
