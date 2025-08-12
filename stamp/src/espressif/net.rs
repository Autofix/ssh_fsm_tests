use core::task::Poll;
use embassy_futures::poll_once;
#[derive(Debug, PartialEq, Clone)]
pub enum WifiMode {
    ApMode,
    StaMode,
}

pub fn poll_if_up() -> Poll<bool> {
    poll_once(if_up())
}

async fn if_up() -> bool {
    true
}

pub fn poll_accept_requests() -> Poll<bool> {
    poll_once(accept_requests())
}

async fn accept_requests() -> bool {
    true
}

pub fn poll_wifi_up(wifimode: Option<WifiMode>) -> Poll<bool> {
    poll_once(wifi_up(wifimode))
}

// use crate::PERIPHERAL_RADIO_CLOCK;
// use crate::PERIPHERAL_WIFI;
// use crate::PERIPHERAL_RNG;
// use crate::PERIPHERAL_TIMG0;
use crate::RADIO_CLOCK_CHANNEL;
use crate::RNG_CHANNEL;
use crate::TIMG0_CHANNEL;
use crate::WIFI_CHANNEL;
// use static_cell::StaticCell; //no-std

async fn wifi_up(_wifimode: Option<WifiMode>) -> bool {
    // let _radio_clk = *PERIPHERAL_RADIO_CLOCK;
    // let wifi = *PERIPHERAL_WIFI;
    // let sw_interupt = *PERIPHERAL_SW_INTERRUPT;
    // let rng = *PERIPHERAL_RNG;
    // let timg0 = *PERIPHERAL_TIMG0;
    //
    let radio_clk = RADIO_CLOCK_CHANNEL.receiver().receive().await;
    let _wifi = WIFI_CHANNEL.receiver().receive().await;
    let rng = RNG_CHANNEL.receiver().receive().await;
    let timg0 = TIMG0_CHANNEL.receiver().receive().await;
    let _wifi_controller = esp_wifi::init(timg0.timer0, rng, radio_clk).unwrap();

    true
}

pub fn poll_net_up() -> Poll<bool> {
    poll_once(net_up())
}

async fn net_up() -> bool {
    true
}

pub fn poll_dhcp_server() -> Poll<bool> {
    poll_once(dhcp_server())
}

async fn dhcp_server() -> bool {
    true
}
