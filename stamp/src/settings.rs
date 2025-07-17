use core::task::Poll;
extern crate embassy_futures;
use embassy_futures::poll_once;

pub fn poll_read_env_vars() -> Poll<bool> {
    poll_once(read_env_vars())
}

pub async fn read_env_vars() -> bool {
    true
}

pub fn poll_store_env_vars() -> Poll<bool> {
    poll_once(store_env_vars())
}

pub async fn store_env_vars() -> bool {
    true
}
