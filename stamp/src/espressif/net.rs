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

async fn wifi_up(_wifimode: Option<WifiMode>) -> bool {
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
