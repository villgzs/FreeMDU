use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_hal::uart::{config::{Config, DataBits, Parity, StopBits}, UartDriver};
use esp_idf_hal::units::Hertz;
use esp_idf_sys::{self as _, esp_timer_get_time};
use log::*;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;

/// Egy beérkezett byte időbélyeggel
#[derive(Debug, Clone, Copy)]
struct TimedByte {
    timestamp_us: i64, // mikrosekundum a boot óta
    uart_id: u8,       // 0 vagy 1
    data: u8,
}

impl PartialEq for TimedByte {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp_us == other.timestamp_us
    }
}

impl Eq for TimedByte {}

impl PartialOrd for TimedByte {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimedByte {
    fn cmp(&self, other: &Self) -> Ordering {
        // Kisebb timestamp előbb jön
        self.timestamp_us.cmp(&other.timestamp_us)
    }
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("ESP32-C3 dual UART → USB Serial/JTAG bridge (timestamped, hex)");

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    // --- UART konfiguráció 9600 baud ---
    let uart_config = Config::new()
        .baudrate(Hertz(9600))
        .data_bits(DataBits::DataBits8)
        .parity_none()
        .stop_bits(StopBits::STOP1);

    // UART0 – normál (TX=GPIO21, RX=GPIO20)
    let uart0 = UartDriver::new(
        peripherals.uart0,
        pins.gpio21, // TX
        pins.gpio20, // RX
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &uart_config,
    )?;

    // UART1 – invertált RX (példa pinek: TX=GPIO4, RX=GPIO5)
    // Az invertálást a driver létrehozása után állítjuk
    let mut uart1 = UartDriver::new(
        peripherals.uart1,
        pins.gpio4,  // TX
        pins.gpio5,  // RX
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &uart_config,
    )?;

    // UART1 RX invertálása
    // esp-idf-hal-ben a set_rx_invert metódus elérhető
    uart1.set_rx_invert(true)?;

    info!("UART0 (normal) és UART1 (inverted RX) inicializálva @ 9600 baud");

    // Közös körkörös buffer (időbélyeggel)
    // A VecDeque + Mutex elegendő, mert nincs folyamatos forgalom
    let buffer: Arc<Mutex<VecDeque<TimedByte>>> = Arc::new(Mutex::new(VecDeque::with_capacity(1024)));

    // UART0 fogadó szál
    {
        let buf = Arc::clone(&buffer);
        thread::Builder::new()
            .name("uart0_rx".into())
            .stack_size(4096)
            .spawn(move || {
                uart_receiver(uart0, 0, buf);
            })?;
    }

    // UART1 fogadó szál
    {
        let buf = Arc::clone(&buffer);
        thread::Builder::new()
            .name("uart1_rx".into())
            .stack_size(4096)
            .spawn(move || {
                uart_receiver(uart1, 1, buf);
            })?;
    }

    // USB-re küldő szál (időbélyeg szerinti sorrendben)
    {
        let buf = Arc::clone(&buffer);
        thread::Builder::new()
            .name("usb_tx".into())
            .stack_size(4096)
            .spawn(move || {
                usb_sender(buf);
            })?;
    }

    // Fő szál csak élve marad
    loop {
        FreeRtos::delay_ms(1000);
    }
}

/// Egy UART-ról érkező byte-okat időbélyeggel a közös bufferbe teszi
fn uart_receiver(mut uart: UartDriver, uart_id: u8, buffer: Arc<Mutex<VecDeque<TimedByte>>>) {
    let mut byte_buf = [0u8; 1];

    loop {
        // Blokkoló olvasás 1 byte-ra (timeout nélkül, vagy nagyon hosszú timeouttal)
        match uart.read(&mut byte_buf, 100) {
            // 100 ms timeout
            Ok(1) => {
                let ts = unsafe { esp_timer_get_time() }; // mikrosekundum

                let entry = TimedByte {
                    timestamp_us: ts,
                    uart_id,
                    data: byte_buf[0],
                };

                if let Ok(mut guard) = buffer.lock() {
                    // Körkörös: ha megtelt volna, a legrégebbit dobjuk (bár a spec szerint nem kell)
                    if guard.len() >= 1024 {
                        guard.pop_front();
                    }
                    guard.push_back(entry);
                }
            }
            Ok(_) => {} // 0 byte
            Err(_) => {
                // timeout – normális
            }
        }
    }
}

/// A közös bufferből időbélyeg szerinti sorrendben küldi az adatokat az USB-re
fn usb_sender(buffer: Arc<Mutex<VecDeque<TimedByte>>>) {
    loop {
        // Kis késleltetés, hogy több byte is összegyűljön, ha közel egyszerre jönnek
        FreeRtos::delay_ms(5);

        let mut to_send = Vec::new();

        if let Ok(mut guard) = buffer.lock() {
            if guard.is_empty() {
                continue;
            }

            // A VecDeque-t rendezzük időbélyeg szerint (bár általában már közel sorrendben van)
            // Egyszerűbb és biztonságosabb: kiveszünk mindent és rendezzük
            let mut items: Vec<TimedByte> = guard.drain(..).collect();
            items.sort(); // Ord implementáció miatt timestamp szerint

            to_send = items;
        }

        // Kiírás a host felé
        for item in to_send {
            // Formátum: "0: 0xAA\n" vagy "1: 0xAA\n"
            let _ = writeln!(
                std::io::stdout(),
                "{}: 0x{:02X}",
                item.uart_id,
                item.data
            );
        }

        let _ = std::io::stdout().flush();
    }
}
