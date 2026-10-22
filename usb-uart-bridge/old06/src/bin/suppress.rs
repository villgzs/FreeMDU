#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use core::sync::atomic::{AtomicBool, Ordering};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_io_async::{Read, Write};
use esp_hal::{
    timer::timg::TimerGroup,
    uart::{Config as UartConfig, Uart, Parity},
    usb::usb_serial_jtag::UsbSerialJtag,
    Async,
};
use esp_backtrace as _;
use heapless::Deque;

const BUFFER_SIZE: usize = 64;
const UART_BAUD: u32 = 2400;
const ECHO_CAP: usize = 2; // max ennyi bájt "várható" echo

type EchoQueue = Mutex<CriticalSectionRawMutex, Deque<u8, ECHO_CAP>>;

static ECHO: EchoQueue = Mutex::new(Deque::new());
/// Ha true: echo szűrés bekapcsolva
static ECHO_SUPPRESS: AtomicBool = AtomicBool::new(true);

#[embassy_executor::task]
async fn usb_to_uart_task(
    mut reader: esp_hal::usb::usb_serial_jtag::UsbSerialJtagRx<'static, Async>,
    mut writer: esp_hal::uart::UartTx<'static, Async>,
) {
    let mut buf = [0u8; BUFFER_SIZE];
    loop {
        match Read::read(&mut reader, &mut buf).await {
            Ok(n) if n > 0 => {
                if ECHO_SUPPRESS.load(Ordering::Relaxed) {
                    let mut q = ECHO.lock().await;
                    for &b in &buf[..n] {
                        let _ = q.push_back(b); // ha tele van, a legrégebbi kieshet
                        if q.is_full() {
                            let _ = q.pop_front();
                            let _ = q.push_back(b);
                        }
                    }
                }
                let _ = Write::write_all(&mut writer, &buf[..n]).await;
            }
            _ => {}
        }
    }
}

#[embassy_executor::task]
async fn uart_to_usb_task(
    mut reader: esp_hal::uart::UartRx<'static, Async>,
    mut writer: esp_hal::usb::usb_serial_jtag::UsbSerialJtagTx<'static, Async>,
) {
    let mut buf = [0u8; BUFFER_SIZE];
    let mut out = [0u8; BUFFER_SIZE];
    loop {
        match Read::read(&mut reader, &mut buf).await {
            Ok(n) if n > 0 => {
                let mut out_len = 0;
                if ECHO_SUPPRESS.load(Ordering::Relaxed) {
                    let mut q = ECHO.lock().await;
                    for &b in &buf[..n] {
                        if q.front().copied() == Some(b) {
                            let _ = q.pop_front(); // echo → eldob
                        } else {
                            out[out_len] = b;
                            out_len += 1;
                        }
                    }
                } else {
                    out[..n].copy_from_slice(&buf[..n]);
                    out_len = n;
                }
                if out_len > 0 {
                    let _ = Write::write_all(&mut writer, &out[..out_len]).await;
                }
            }
            _ => {}
        }
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let usb = UsbSerialJtag::new(peripherals.USB_DEVICE).into_async();
    let (usb_rx, usb_tx) = usb.split();

    let uart_config = UartConfig::default()
        .with_baudrate(UART_BAUD)
        .with_parity(Parity::Even);
    let uart = Uart::new(peripherals.UART1, uart_config)
        .unwrap()
        .with_rx(peripherals.GPIO0)
        .with_tx(peripherals.GPIO1)
        .into_async();
    let (uart_rx, uart_tx) = uart.split();

    spawner.spawn(usb_to_uart_task(usb_rx, uart_tx).unwrap());
    spawner.spawn(uart_to_usb_task(uart_rx, usb_tx).unwrap());
}
