#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

// use core::sync::atomic::{AtomicBool, Ordering};
use core::sync::atomic::{AtomicU8, Ordering};
use embassy_executor::Spawner;
// use embassy_sync::{
//    blocking_mutex::raw::CriticalSectionRawMutex,
//    signal::Signal,
// };
use embedded_io_async::{Read, Write};
use esp_hal::{
    timer::timg::TimerGroup,
    uart::{Config as UartConfig, Parity, Uart},
    usb::usb_serial_jtag::UsbSerialJtag,
    Async,
};
use esp_backtrace as _;

const BUFFER_SIZE: usize = 64;
const UART_BAUD: u32 = 2400;

// true: az UART TX-en kiküldött bájt echo-jára várunk.
// static ECHO_PENDING: AtomicBool = AtomicBool::new(false);
// static PENDING_ACK: AtomicBool = AtomicBool::new(true);
// static ECHO_PENDING: Signal<CriticalSectionRawMutex, bool> = Signal::new();
// static PENDING_ACK: Signal<CriticalSectionRawMutex, bool> = Signal::new();
// static ECHO_PENDING: Signal<CriticalSectionRawMutex, bool> =
//    Signal::new();
//
// static PENDING_ACK: Signal<CriticalSectionRawMutex, bool> =
//    Signal::new();
static GOOUT: AtomicU8 = AtomicU8::new(0);
static COMEIN: AtomicU8 = AtomicU8::new(0);

#[embassy_executor::task]
async fn usb_to_uart_task(
    mut reader: esp_hal::usb::usb_serial_jtag::UsbSerialJtagRx<'static, Async>,
    mut writer: esp_hal::uart::UartTx<'static, Async>,
) {
    let mut buf = [0u8; BUFFER_SIZE];

    loop {
        match Read::read(&mut reader, &mut buf).await {
            Ok(n) if n > 0 => {
                for byte in &buf[..n] {

                    if Write::write_all(
                        &mut writer,
                        core::slice::from_ref(byte),
                    )
                    .await
                    .is_err()
                    {
                        break;
                    }
                    GOOUT.store(
                        GOOUT.load(Ordering::Relaxed).wrapping_add(1),
                        Ordering::Relaxed,
                    );
                }
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
    let mut byte = [0u8; 1];

    loop {
        match Read::read(&mut reader, &mut byte).await {
            Ok(1) => {
                if COMEIN.load(Ordering::Relaxed) == GOOUT.load(Ordering::Relaxed) {
                    let _ = Write::write_all(&mut writer, &byte).await;
                }
                else { 
                    COMEIN.store(
                        COMEIN.load(Ordering::Relaxed).wrapping_add(1),
                        Ordering::Relaxed,
                    );
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
