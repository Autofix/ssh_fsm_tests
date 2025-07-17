use core::task::Poll;
extern crate embassy_futures;
use embassy_futures::poll_once;

pub fn poll_connection_loop() -> Poll<bool> {
    poll_once(connection_loop())
}

pub async fn connection_loop() -> bool {
    true
}

pub fn poll_authentication() -> Poll<bool> {
    poll_once(authentication())
}

pub async fn authentication() -> bool {
    true
}

pub fn poll_handle_ssh_client() -> Poll<bool> {
    poll_once(handle_ssh_client())
}

pub async fn handle_ssh_client() -> bool {
    true
}

pub fn poll_connect_ssh_client() -> Poll<bool> {
    poll_once(connect_ssh_client())
}

pub async fn connect_ssh_client() -> bool {
    true
}

pub fn poll_ssh_client_connected() -> Poll<bool> {
    poll_once(ssh_client_connected())
}

pub async fn ssh_client_connected() -> bool {
    true
}

pub fn poll_ssh_client_connected_no_bridge() -> Poll<bool> {
    poll_once(ssh_client_connected_no_bridge())
}

pub async fn ssh_client_connected_no_bridge() -> bool {
    true
}

pub fn poll_notify_client(client_notified: &str) -> Poll<bool> {
    poll_once(notify_client(client_notified))
}

pub async fn notify_client(client_notified: &str) -> bool {
    if client_notified == "no bridge" {
        true
    } else {
        false
    }
}
