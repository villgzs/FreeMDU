use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{AnyIOPin, PinDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_hal::uart::{config::Config, UartDriver};
use esp_idf_hal::units::Hertz;
use esp_idf_sys as _; // szükséges a linkeléshez
use log::*;
use std::io::Write;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    // ESP-IDF inicializálás
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("ESP32-C3 UART → USB Serial/JTAG bridge indítása...");

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    // --- UART konfiguráció ---
    let uart_config = Config::new().baudrate(Hertz(115_200));

    // UART0 (alapértelmezett pinek: TX=GPIO21, RX=GPIO20)
    // Ha más pineket akarsz, cseréld ki
    let uart0 = UartDriver::new(
        peripherals.uart0,
        pins.gpio21,          // TX
        pins.gpio20,          // RX
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &uart_config,
    )?;

    // UART1 - tetszőleges pinek (példa: GPIO4 = TX, GPIO5 = RX)
    let uart1 = UartDriver::new(
        peripherals.uart1,
        pins.gpio4,           // TX
        pins.gpio5,           // RX
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &uart_config,
    )?;

    info!("UART0 és UART1 inicializálva (115200 baud)");

    // Két külön szál a két UART-hoz
    let _handle0 = thread::Builder::new()
        .name("uart0_fwd".into())
        .stack_size(4096)
        .spawn(move || {
            forward_uart(uart0, 0);
        })?;

    let _handle1 = thread::Builder::new()
        .name("uart1_fwd".into())
        .stack_size(4096)
        .spawn(move || {
            forward_uart(uart1, 1);
        })?;

    // Fő szál csak élve marad
    loop {
        FreeRtos::delay_ms(1000);
    }
}

/// Egy UART-ról érkező adatokat továbbít a stdout-ra (USB Serial/JTAG)
/// előtaggal ellátva.
fn forward_uart(mut uart: UartDriver, id: u8) {
    let mut buf = [0u8; 256];
    let prefix = format!("{}:", id);

    loop {
        match uart.read(&mut buf, 50) { // 50 ms timeout
            Ok(0) => {
                // nincs adat
            }
            Ok(len) => {
                // Előtag + adat küldése a USB Serial/JTAG-ra
                // A println! / print! a USB Serial/JTAG-ra megy,
                // ha a console USB Serial/JTAG-ra van állítva.
                print!("{}", prefix);
                // Nyers bájtok kiírása
                let _ = std::io::stdout().write_all(&buf[..len]);
                let _ = std::io::stdout().flush();
            }
            Err(e) => {
                // timeout vagy egyéb hiba – nem baj
                if e.code() != esp_idf_sys::ESP_ERR_TIMEOUT as i32 {
                    error!("UART{} hiba: {:?}", id, e);
                }
            }
        }
    }
}
