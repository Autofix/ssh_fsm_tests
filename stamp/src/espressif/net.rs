use core::task::Poll;
use embassy_futures::poll_once;

use core::net::Ipv4Addr;
use core::net::SocketAddrV4;
use core::str::FromStr;

use embassy_executor::Spawner;
use embassy_net::{IpListenEndpoint, Ipv4Cidr, Runner, StaticConfigV4};
use embassy_net::{Stack, StackResources, tcp::TcpSocket};
use embassy_time::{Duration, Timer};

use esp_hal::peripherals::{RADIO_CLK, TIMG0, WIFI};

use esp_hal::rng::Rng;
use esp_println::{dbg, println};

use esp_wifi::EspWifiController;
use esp_wifi::wifi::{AccessPointConfiguration, Configuration, WifiController};
use esp_wifi::wifi::{WifiEvent, WifiState};

use edge_dhcp;
use edge_dhcp::{
    io::{self, DEFAULT_SERVER_PORT},
    server::{Server, ServerOptions},
};
use edge_nal::UdpBind;
use edge_nal_embassy::{Udp, UdpBuffers};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::timer::timg::TimerGroup;
use heapless::String;

const GW_IP_ADDR_ENV: Option<&'static str> = option_env!("GATEWAY_IP");

static WIFI_CONTROLLER_MUTEX: Mutex<CriticalSectionRawMutex, Option<EspWifiController>> =
    Mutex::new(None);
// use crate::ssh_stamp::SPAWNER_MUTEX;
use crate::{CONFIG_MUTEX, WIFI_MUTEX};
static TCP_STACK_MUTEX: Mutex<CriticalSectionRawMutex, Option<Stack>> = Mutex::new(None);
static RUNNER_MUTEX: Mutex<CriticalSectionRawMutex, Option<Runner>> = Mutex::new(None);
static CONTROLLER_MUTEX: Mutex<CriticalSectionRawMutex, Option<WifiController<'static>>> =
    Mutex::new(None);
static AP_STACK_MUTEX: Mutex<CriticalSectionRawMutex, Option<Stack>> = Mutex::new(None);
static GW_IP_ADDR_MUTEX: Mutex<CriticalSectionRawMutex, Option<Ipv4Addr>> = Mutex::new(None);
use crate::SPAWNER_MUTEX;
// use crate::UART_BUFFER_MUTEX;
pub static TCP_SOCKET_MUTEX: Mutex<CriticalSectionRawMutex, Option<TcpSocket>> = Mutex::new(None);

use crate::RADIO_CLOCK_MUTEX;
use crate::RNG_MUTEX;
use crate::TIMG0_MUTEX;
// use crate::SW_INTERRUPT_MUTEX;

//
// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[derive(Debug, PartialEq, Clone)]
pub enum WifiMode {
    ApMode,
    StaMode,
}

pub fn poll_if_up() -> Poll<bool> {
    poll_once(if_up())
}

async fn if_up() -> bool {
    let spawner: Spawner = SPAWNER_MUTEX.lock().await.take().unwrap();
    let wifi_controller = WIFI_CONTROLLER_MUTEX.lock().await.take().unwrap();
    let wifi: WIFI = WIFI_MUTEX.lock().await.take().unwrap();
    let rng: Rng = *(RNG_MUTEX.lock().await.as_ref().unwrap());

    let wifi_init = &*mk_static!(EspWifiController<'static>, wifi_controller);
    let (controller, interfaces) = esp_wifi::wifi::new(wifi_init, wifi).unwrap();

    // Stack<'static>
    let gw_ip_addr_str = GW_IP_ADDR_ENV.unwrap_or("192.168.0.1");
    let gw_ip_addr = Ipv4Addr::from_str(gw_ip_addr_str).expect("failed to parse gateway ip");

    let net_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(gw_ip_addr, 24),
        gateway: Some(gw_ip_addr),
        dns_servers: Default::default(),
    });

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let (ap_stack, runner) = embassy_net::new(
        interfaces.ap,
        net_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    *(AP_STACK_MUTEX.lock().await) = Some(ap_stack);
    *(GW_IP_ADDR_MUTEX.lock().await) = Some(gw_ip_addr);
    // spawner.spawn(wifi_up(controller, config)).ok();
    // spawner.spawn(net_up(runner)).ok();
    // spawner.spawn(dhcp_server(ap_stack, gw_ip_addr)).ok();

    loop {
        // println!("Checking if link is up...\n");
        if ap_stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    // TODO: Use wifi_manager instead?
    // println!("Connect to the AP `ssh-stamp` as a DHCP client with IP: {}", gw_ip_addr_str);

    true
}

pub fn poll_accept_requests() -> Poll<bool> {
    poll_once(accept_requests())
}

async fn accept_requests() -> bool {
    let tcp_stack: Stack<'static> = TCP_STACK_MUTEX.lock().await.take().unwrap();
    // let uart_buf: &BufferedUart = *(UART_BUFFER_MUTEX.lock().await.as_ref().unwrap());
    let rx_buffer = mk_static!([u8; 1536], [0; 1536]);
    let tx_buffer = mk_static!([u8; 1536], [0; 1536]);

    let mut socket: TcpSocket<'_> = TcpSocket::new(tcp_stack, rx_buffer, tx_buffer);

    // println!("Waiting for SSH client...");

    if let Err(_e) = socket
        .accept(IpListenEndpoint {
            addr: None,
            port: 22,
        })
        .await
    {
        return false; // println!("connect error: {:?}", e);
        // continue;
    }
    *(TCP_SOCKET_MUTEX.lock().await) = Some(socket);

    // println!("Connected, port 22");

    true
}

pub fn poll_wifi_up() -> Poll<bool> {
    poll_once(wifi_up())
}

async fn wifi_up() -> bool {
    let mut controller: WifiController<'static> = CONTROLLER_MUTEX.lock().await.take().unwrap();
    println!("Device capabilities: {:?}", controller.capabilities());
    let wifi_ssid: String<32> = {
        // let guard = CONFIG_MUTEX.lock().await;
        let guard = CONFIG_MUTEX.lock().await;
        guard.as_ref().unwrap().wifi_ssid.clone()
        // drop guard
    };
    // TODO: No wifi password(s) yet...
    //let wifi_password = config.lock().await.wifi_pw;

    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::ApStarted {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::ApStop).await;
            Timer::after(Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::AccessPoint(AccessPointConfiguration {
                ssid: wifi_ssid.to_ascii_lowercase(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");
        }
        Timer::after(Duration::from_millis(10)).await;
    }
}

pub fn poll_wifi_mode_up(wifimode: Option<WifiMode>) -> Poll<bool> {
    poll_once(wifi_mode_up(wifimode))
}

async fn wifi_mode_up(_wifimode: Option<WifiMode>) -> bool {
    // *****
    // Potenital future option to save static memory
    // Use arc to store pointer inside mutex rather than entire values
    // let radio_clk = unsafe {
    //     RADIO_CLOCK_MUTEX
    //         .lock()
    //         .await
    //         .take()
    //         .unwrap()
    //         .clone_unchecked()
    // };
    // *****

    let radio_clk: RADIO_CLK = RADIO_CLOCK_MUTEX.lock().await.take().unwrap();
    let rng: Rng = *(RNG_MUTEX.lock().await.as_ref().unwrap());
    let timg0: TimerGroup<'_, TIMG0<'static>> = TIMG0_MUTEX.lock().await.take().unwrap();
    let wifi_controller = esp_wifi::init(timg0.timer0, rng, radio_clk).unwrap();
    *(WIFI_CONTROLLER_MUTEX.lock().await) = Some(wifi_controller);
    true
}

pub fn poll_net_up() -> Poll<bool> {
    poll_once(net_up())
}

async fn net_up() -> bool {
    // let rng: Rng = *(RNG_MUTEX.lock().await.as_ref().unwrap());
    // let _wifi: WIFI = WIFI_MUTEX.lock().await.take().unwrap();
    // let _wifi_controller: EspWifiController = WIFI_CONTROLLER_MUTEX.lock().await.take().unwrap();
    // Bring up the network interface and start accepting SSH connections.
    // Clone the reference to config to avoid borrow checker issues.
    // let tcp_stack = if_up(wifi_controller, wifi, &mut rng).await.unwrap();
    println!("Bringing up network stack...\n");
    let runner = RUNNER_MUTEX.lock().await.take().unwrap();

    runner.run().await;
    true
}

pub fn poll_dhcp_server() -> Poll<bool> {
    poll_once(dhcp_server())
}

async fn dhcp_server() -> bool {
    let stack: Stack = AP_STACK_MUTEX.lock().await.take().unwrap();
    let ip: Ipv4Addr = GW_IP_ADDR_MUTEX.lock().await.take().unwrap();
    let mut buf = [0u8; 1500];

    let mut gw_buf = [Ipv4Addr::UNSPECIFIED];

    let buffers = UdpBuffers::<3, 1024, 1024, 10>::new();
    let unbound_socket = Udp::new(stack, &buffers);
    let mut bound_socket = unbound_socket
        .bind(core::net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_SERVER_PORT,
        )))
        .await
        .unwrap();

    let res = io::server::run(
        &mut Server::<_, 64>::new_with_et(ip),
        &ServerOptions::new(ip, Some(&mut gw_buf)),
        &mut bound_socket,
        &mut buf,
    )
    .await
    .inspect_err(|e| log::warn!("DHCP server error: {e:?}"));
    // Timer::after(Duration::from_millis(500)).await;

    dbg!(res.unwrap());
    *(TCP_STACK_MUTEX.lock().await) = Some(stack);

    true
}
