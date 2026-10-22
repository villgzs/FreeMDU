#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embedded_io_async::{Read, Write};
use esp_hal::{
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    uart::{Config as UartConfig, Uart},
    usb_serial_jtag::UsbSerialJtag,
    Async,
};
use esp_backtrace as _;

const BUFFER_SIZE: usize = 64;
const UART_BAUD: u32 = 115_200;

#[embassy_executor::task]
async fn usb_to_uart_task(
    mut reader: esp_hal::usb_serial_jtag::UsbSerialJtagRx<'static, Async>,
    mut writer: esp_hal::uart::UartTx<'static, Async>,
) {
    let mut buf = [0u8; BUFFER_SIZE];
    loop {
        match reader.read(&mut buf).await {
            Ok(n) if n > 0 => {
                let _ = writer.write(&buf[..n]).await;
            }
            _ => {}
        }
    }
}

#[embassy_executor::task]
async fn uart_to_usb_task(
    mut reader: esp_hal::uart::UartRx<'static, Async>,
    mut writer: esp_hal::usb_serial_jtag::UsbSerialJtagTx<'static, Async>,
) {
    let mut buf = [0u8; BUFFER_SIZE];
    loop {
        match reader.read(&mut buf).await {
            Ok(n) if n > 0 => {
                let _ = writer.write(&buf[..n]).await;
            }
            _ => {}
        }
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    // USB Serial JTAG
    let usb_serial = UsbSerialJtag::new(peripherals.USB_DEVICE).into_async();
    let (usb_rx, usb_tx) = usb_serial.split();

    // UART1 – pin-eket a hardveredhez igazítsd!
    let uart_config = UartConfig::default().with_baudrate(UART_BAUD);
    let uart = Uart::new(peripherals.UART1, uart_config)
        .unwrap()
        .with_rx(peripherals.GPIO0)
        .with_tx(peripherals.GPIO1)
        .into_async();
    let (uart_rx, uart_tx) = uart.split();

    spawner.spawn(usb_to_uart_task(usb_rx, uart_tx)).unwrap();
    spawner.spawn(uart_to_usb_task(uart_rx, usb_tx)).unwrap();
}
