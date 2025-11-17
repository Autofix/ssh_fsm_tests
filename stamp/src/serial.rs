use core::task::Poll;
extern crate embassy_futures;
use crate::espressif::buffered_uart::BufferedUart;
use embassy_futures::poll_once;
use embassy_futures::select::select;
use embedded_io_async::{Read, Write};
use esp_println::println;
use sunset_async::ChanInOut;

pub fn poll_serial_bridge() -> Poll<Result<(), sunset::Error>> {
    poll_once(serial_bridge())
}
use crate::UART_BUFFER_MUTEX;
use crate::serve::STDIO_MUTEX;
use crate::serve::STDIO2_MUTEX;

pub(crate) async fn serial_bridge() -> Result<(), sunset::Error> {
    let chanr: ChanInOut<'_> = STDIO_MUTEX.lock().await.take().unwrap();
    let chanw: ChanInOut<'_> = STDIO2_MUTEX.lock().await.take().unwrap();
    let uart: BufferedUart = UART_BUFFER_MUTEX.lock().await.take().unwrap();
    println!("Starting serial <--> SSH bridge");

    select(uart_to_ssh(&uart, chanw), ssh_to_uart(chanr, &uart)).await;
    println!("Stopping serial <--> SSH bridge");
    Ok(())
}

async fn uart_to_ssh(
    uart_buf: &BufferedUart,
    mut chanw: impl Write<Error = sunset::Error>,
) -> Result<(), sunset::Error> {
    let mut ssh_tx_buf = [0u8; 512];
    loop {
        let dropped = uart_buf.check_dropped_bytes();
        if dropped > 0 {
            // TODO: should this also go to the SSH client?
            println!("UART RX dropped {} bytes", dropped);
        }

        let n = uart_buf.read(&mut ssh_tx_buf).await;
        chanw.write_all(&ssh_tx_buf[..n]).await?;
    }
}

async fn ssh_to_uart(
    mut chanr: impl Read<Error = sunset::Error>,
    uart_buf: &BufferedUart,
) -> Result<(), sunset::Error> {
    let mut uart_tx_buf = [0u8; 64];
    loop {
        let n = chanr.read(&mut uart_tx_buf).await?;
        if n == 0 {
            return Err(sunset::Error::ChannelEOF);
        }
        uart_buf.write(&uart_tx_buf[..n]).await;
    }
}
