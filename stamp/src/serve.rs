use core::task::Poll;
extern crate embassy_futures;
use embassy_futures::poll_once;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

// use embassy_futures::select::{Either3, select3};
use crate::keys;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::channel::Sender;
use embassy_sync::mutex::Mutex;
use embedded_io_async::Read;
use esp_hal::peripherals::WIFI;
use heapless::String;
use sunset::{ChanHandle, ServEvent, SignKey, error};
// use crate::espressif::buffered_uart::BufferedUart;
use esp_println::{dbg, println};

enum SessionType {
    Bridge(ChanHandle),
    Env(ChanHandle),
    Sftp(ChanHandle),
}
use embassy_net::tcp::TcpSocket;
use sunset_async::{ChanInOut, ProgressHolder, SSHServer};
static RSOCK_MUTEX: Mutex<CriticalSectionRawMutex, Option<&SSHServer<'_>>> = Mutex::new(None);
static WSOCK_PIPE_MUTEX: Channel<CriticalSectionRawMutex, SessionType, 1> = Channel::new();
static SSH_SERVER_MUTEX: Mutex<CriticalSectionRawMutex, Option<SSHServer>> = Mutex::new(None);
static SERVER_MUTEX: Mutex<CriticalSectionRawMutex, Option<SSHServer>> = Mutex::new(None);
// static SSH_SERVER_MUTEX: Mutex<CriticalSectionRawMutex, Option<&SSHServer<'_>>> = Mutex::new(None);
static CHAN_PIPE: Channel<CriticalSectionRawMutex, SessionType, 1> = Channel::new();
// use crate::UART_BUFFER_MUTEX;
use crate::espressif::net::TCP_SOCKET_MUTEX;
// use crate::espressif::net::TCP_STACK_MUTEX;
static WIFI_MUTEX: Mutex<CriticalSectionRawMutex, Option<WIFI>> = Mutex::new(None);
pub static STDIO_MUTEX: Mutex<CriticalSectionRawMutex, Option<ChanInOut<'_>>> = Mutex::new(None);
pub static STDIO2_MUTEX: Mutex<CriticalSectionRawMutex, Option<ChanInOut<'_>>> = Mutex::new(None);

pub fn poll_connection_loop() -> Poll<Result<(), sunset::Error>> {
    poll_once(connection_loop())
}

pub async fn connection_loop() -> Result<(), sunset::Error> {
    // let serv: &SSHServer<'_> = SSH_SERVER_MUTEX.lock().await.as_ref().unwrap();
    let chan_pipe: Sender<'_, CriticalSectionRawMutex, SessionType, 1> = CHAN_PIPE.sender();
    // CHAN_PIPE_MUTEX.lock().await.take().unwrap();

    let username = Mutex::<NoopRawMutex, _>::new(String::<20>::new());
    let mut session: Option<ChanHandle> = None;

    println!("Entering connection_loop and prog_loop is next...");

    loop {
        let mut ph = ProgressHolder::new();

        let serv: SSHServer = SSH_SERVER_MUTEX.lock().await.take().unwrap();
        let ev: ServEvent = { serv.progress(&mut ph).await? };
        dbg!(&ev);
        #[allow(unreachable_patterns)]
        match ev {
            ServEvent::SessionShell(a) => {
                if let Some(ch) = session.take() {
                    debug_assert!(ch.num() == a.channel());

                    a.succeed()?;
                    dbg!("We got shell");
                    let _ = chan_pipe.try_send(SessionType::Bridge(ch));
                } else {
                    a.fail()?;
                }
            }
            ServEvent::FirstAuth(ref a) => {
                // record the username
                if username.lock().await.push_str(a.username()?).is_err() {
                    println!("Too long username")
                }
            }
            ServEvent::Hostkeys(h) => {
                let signkey: SignKey = SignKey::from_openssh(keys::HOST_SECRET_KEY)?;
                h.hostkeys(&[&signkey])?;
            }
            ServEvent::PasswordAuth(a) => {
                a.allow()?;
            }
            ServEvent::PubkeyAuth(a) => {
                a.allow()?;
            }
            ServEvent::OpenSession(a) => {
                match session {
                    Some(_) => {
                        todo!("Can't have two sessions");
                    }
                    None => {
                        // Track the session
                        session = Some(a.accept()?);
                    }
                }
            }
            ServEvent::SessionShell(a) => {
                // TODO: Logic to serialise/validate env vars? I.e:
                // config.validate(a)
                // config.save(a)
                // SSHConfig c = a.validate(); // Checks the input variable, sanitizes, assigns a target subsystem
                // a.config_change(c)?;

                // Obtain the current session channel handle and use it to get stdio.
                //  if let Some(ch) = session.take() {
                //     debug_assert!(ch.num() == a.channel());

                //     a.succeed()?;
                //     let _ = chan_pipe.try_send(SessionType::Env(ch));
                // } else {
                //     a.fail()?;
                // }
                a.succeed()?;
            }
            ServEvent::SessionPty(a) => {
                a.succeed()?;
            }
            ServEvent::SessionExec(a) => {
                a.fail()?;
            }
            ServEvent::Defunct | ServEvent::SessionShell(_) => {
                println!("Expected caller to handle event");
                error::BadUsage.fail()?
            }
            _ => (),
        };
    }
}

pub fn poll_authentication() -> Poll<Result<(), sunset::Error>> {
    poll_once(authentication())
}

pub async fn authentication() -> Result<(), sunset::Error> {
    Ok(())
}

pub fn poll_handle_ssh_client() -> Poll<Result<(), sunset::Error>> {
    poll_once(handle_ssh_client())
}

pub async fn handle_ssh_client() -> Result<(), sunset::Error> {
    let stream: &mut TcpSocket<'_> = TCP_SOCKET_MUTEX.lock().await.as_mut().unwrap();
    // let uart: &BufferedUart = *(UART_BUFFER_MUTEX.lock().await.as_ref().unwrap());
    let mut inbuf = [0u8; 4096];
    let mut outbuf = [0u8; 4096];

    let ssh_server: SSHServer = SSHServer::new(&mut inbuf, &mut outbuf);
    let (mut rsock, mut wsock) = stream.split();

    // let chan_pipe = Channel::<NoopRawMutex, ChanHandle, 1>::new();

    println!("Calling connection_loop from handle_ssh_client");
    // let conn_loop = connection_loop(&ssh_server, &chan_pipe);
    println!("Running server from handle_ssh_client()");
    let server = ssh_server.run(&mut rsock, &mut wsock);

    *(SSH_SERVER_MUTEX.lock().await) = Some(ssh_server);
    *(SERVER_MUTEX.lock().await) = Some(server);

    println!("Main select() in handle_ssh_client()");
    // match select3(conn_loop, server, bridge).await {
    //     Either3::First(r) => r,
    //     Either3::Second(r) => r,
    //     Either3::Third(r) => r,
    // }?;
    Ok(())
}

pub fn poll_connect_ssh_client() -> Poll<Result<(), sunset::Error>> {
    poll_once(connect_ssh_client())
}

pub async fn connect_ssh_client() -> Result<(), sunset::Error> {
    // println!("Setting up serial bridge");
    let ssh_server: SSHServer<'_> = SSH_SERVER_MUTEX.lock().await.take().unwrap();
    let session_type = CHAN_PIPE.receive().await;
    match session_type {
        SessionType::Bridge(ch) => {
            let stdio = ssh_server.stdio(ch).await?;
            let stdio2 = stdio.clone();
            // let stdio: ChanInOut<'static> = ssh_server.stdio(ch).await?;
            // let stdio2: ChanInOut<'_> = stdio.clone();
            *(STDIO_MUTEX.lock().await) = Some(stdio);
            *(STDIO2_MUTEX.lock().await) = Some(stdio2);
            // *(SSH_SERVER_MUTEX.lock().await) = Some(ssh_server);
            // serial_bridge(stdio, stdio2, uart).await?
        }
        SessionType::Env(ch) => {
            // Handle environment variable session
            let mut stdio = ssh_server.stdio(ch).await?;
            let mut buf = [0u8; 256];
            dbg!("Waiting to read ENV session data");
            let n = stdio.read(&mut buf).await?;
            dbg!("Got ENV session");
            dbg!("Name/Value of ENV is: ", &buf[..n]);
        }
        SessionType::Sftp(_ch) => {
            // Handle SFTP session
            todo!()
        }
    };
    Ok(())
}

pub fn poll_ssh_client_connected() -> Poll<Result<(), sunset::Error>> {
    poll_once(ssh_client_connected())
}

pub async fn ssh_client_connected() -> Result<(), sunset::Error> {
    // let server = ssh_server.run(&mut rsock, &mut wsock);
    Ok(())
}

pub fn poll_ssh_client_connected_no_bridge() -> Poll<Result<(), sunset::Error>> {
    poll_once(ssh_client_connected_no_bridge())
}

pub async fn ssh_client_connected_no_bridge() -> Result<(), sunset::Error> {
    Ok(())
}

pub fn poll_notify_client(client_notified: &str) -> Poll<Result<(), sunset::Error>> {
    poll_once(notify_client(client_notified))
}

pub async fn notify_client(client_notified: &str) -> Result<(), sunset::Error> {
    if client_notified == "no bridge" {
        Ok(())
    } else {
        Err(sunset::Error::Custom { msg: ("no bridge") })
    }
}
