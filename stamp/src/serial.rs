use core::task::Poll;
extern crate embassy_futures;
use embassy_futures::poll_once;

pub fn poll_serial_bridge() -> Poll<bool> {
    poll_once(serial_bridge())
}

pub async fn serial_bridge() -> bool {
    true
}
