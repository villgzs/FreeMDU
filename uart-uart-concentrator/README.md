uart-uart-concentrator
-------------------------------------------
Íme a javított verzió a kért architektúrával:

- **Külön buffer** UART0-hoz és UART1-hez
- A fogadó szálak **függetlenül** írnak a saját bufferükbe
- A küldő folyamat mindkét buffert figyeli, és az **időbélyeg alapján** dönti el, hogy melyik byte-ot küldi el előbb

```rust
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_hal::uart::{
    config::{Config, DataBits, StopBits},
    UartDriver,
};
use esp_idf_hal::units::Hertz;
use esp_idf_sys::{self as _, esp_timer_get_time};
use log::*;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;

/// Egy beérkezett byte időbélyeggel
#[derive(Debug, Clone, Copy)]
struct TimedByte {
    timestamp_us: i64, // mikrosekundum a boot óta
    data: u8,
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("ESP32-C3 dual UART → USB Serial/JTAG bridge (separate buffers)");

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    // --- UART konfiguráció 9600 baud ---
    let uart_config = Config::new()
        .baudrate(Hertz(9600))
        .data_bits(DataBits::DataBits8)
        .parity_none()
        .stop_bits(StopBits::STOP1);

    // UART0 – normál
    let uart0 = UartDriver::new(
        peripherals.uart0,
        pins.gpio21, // TX
        pins.gpio20, // RX
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &uart_config,
    )?;

    // UART1 – invertált RX
    let mut uart1 = UartDriver::new(
        peripherals.uart1,
        pins.gpio4, // TX
        pins.gpio5, // RX
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &uart_config,
    )?;
    uart1.set_rx_invert(true)?;

    info!("UART0 (normal) és UART1 (inverted RX) kész @ 9600 baud");

    // Külön buffer a két UART-hoz
    let buffer0: Arc<Mutex<VecDeque<TimedByte>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(512)));
    let buffer1: Arc<Mutex<VecDeque<TimedByte>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(512)));

    // UART0 fogadó szál
    {
        let buf = Arc::clone(&buffer0);
        thread::Builder::new()
            .name("uart0_rx".into())
            .stack_size(4096)
            .spawn(move || {
                uart_receiver(uart0, buf);
            })?;
    }

    // UART1 fogadó szál
    {
        let buf = Arc::clone(&buffer1);
        thread::Builder::new()
            .name("uart1_rx".into())
            .stack_size(4096)
            .spawn(move || {
                uart_receiver(uart1, buf);
            })?;
    }

    // Küldő szál – mindkét buffert figyeli
    {
        let b0 = Arc::clone(&buffer0);
        let b1 = Arc::clone(&buffer1);
        thread::Builder::new()
            .name("usb_tx".into())
            .stack_size(4096)
            .spawn(move || {
                usb_sender(b0, b1);
            })?;
    }

    // Fő szál
    loop {
        FreeRtos::delay_ms(1000);
    }
}

/// Egy UART-ról érkező byte-okat a saját bufferébe teszi
fn uart_receiver(mut uart: UartDriver, buffer: Arc<Mutex<VecDeque<TimedByte>>>) {
    let mut byte_buf = [0u8; 1];

    loop {
        match uart.read(&mut byte_buf, 50) {
            // 50 ms timeout
            Ok(1) => {
                let ts = unsafe { esp_timer_get_time() };

                let entry = TimedByte {
                    timestamp_us: ts,
                    data: byte_buf[0],
                };

                if let Ok(mut guard) = buffer.lock() {
                    // Körkörös: ha megtelt, a legrégebbit dobjuk (bár nem várható)
                    if guard.len() >= 512 {
                        guard.pop_front();
                    }
                    guard.push_back(entry);
                }
            }
            _ => {
                // timeout vagy 0 byte – normális
            }
        }
    }
}

/// Mindkét buffert figyeli, és az időbélyeg alapján dönti el a sorrendet
fn usb_sender(
    buffer0: Arc<Mutex<VecDeque<TimedByte>>>,
    buffer1: Arc<Mutex<VecDeque<TimedByte>>>,
) {
    loop {
        FreeRtos::delay_ms(2); // kis késleltetés a CPU teher csökkentésére

        // Megnézzük a két buffer elejét
        let front0 = {
            let guard = buffer0.lock().unwrap();
            guard.front().copied()
        };
        let front1 = {
            let guard = buffer1.lock().unwrap();
            guard.front().copied()
        };

        match (front0, front1) {
            // Mindkét bufferben van adat
            (Some(b0), Some(b1)) => {
                if b0.timestamp_us <= b1.timestamp_us {
                    // UART0 régebbi → azt küldjük
                    if let Ok(mut guard) = buffer0.lock() {
                        if let Some(item) = guard.pop_front() {
                            send_byte(0, item.data);
                        }
                    }
                } else {
                    // UART1 régebbi
                    if let Ok(mut guard) = buffer1.lock() {
                        if let Some(item) = guard.pop_front() {
                            send_byte(1, item.data);
                        }
                    }
                }
            }

            // Csak UART0-ban van adat
            (Some(_), None) => {
                if let Ok(mut guard) = buffer0.lock() {
                    if let Some(item) = guard.pop_front() {
                        send_byte(0, item.data);
                    }
                }
            }

            // Csak UART1-ben van adat
            (None, Some(_)) => {
                if let Ok(mut guard) = buffer1.lock() {
                    if let Some(item) = guard.pop_front() {
                        send_byte(1, item.data);
                    }
                }
            }

            // Mindkettő üres
            (None, None) => {
                // semmi teendő
            }
        }
    }
}

/// Egy byte kiírása a host felé a megadott formátumban
fn send_byte(uart_id: u8, data: u8) {
    let _ = writeln!(std::io::stdout(), "{}: 0x{:02X}", uart_id, data);
    let _ = std::io::stdout().flush();
}
```

### Működés röviden

1. **UART0 szál** → csak a `buffer0`-ba ír  
2. **UART1 szál** → csak a `buffer1`-be ír  
3. **Küldő szál** folyamatosan nézi mindkét buffer elejét:
   - Ha mindkettőben van adat → a **kisebb időbélyegű** megy előbb
   - Ha csak az egyikben van → azt küldi
   - Ha mindkettő üres → vár

Ez pontosan megfelel a kért modellnek.


----------------------------------------
---------------------------
Itt egy működő **Rust (esp-idf-hal)** megoldás ESP32-C3-ra, ami a UART0 és UART1 adatokat továbbítja a USB Serial/JTAG-ra, és előtaggal látja el őket (`0:` / `1:`).

### 1. `Cargo.toml`

```toml
[package]
name = "uart_bridge"
version = "0.1.0"
edition = "2021"

[dependencies]
esp-idf-sys = { version = "0.36", features = ["binstart"] }
esp-idf-hal = "0.45"
esp-idf-svc = "0.51"
log = "0.4"
anyhow = "1"

[build-dependencies]
embuild = "0.33"
```

### 2. Teljes `src/main.rs`

```rust
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
```

### 3. Fontos beállítások (sdkconfig / menuconfig)

A projektben a console-t **USB Serial/JTAG**-ra kell állítani, különben a `print!` nem a USB-re megy:

```bash
idf.py menuconfig
```

Majd:

```
Component config → ESP System Settings → Channel for console output
→ USB Serial/JTAG Controller
```

Vagy a `sdkconfig.defaults` fájlba:

```ini
CONFIG_ESP_CONSOLE_USB_SERIAL_JTAG=y
CONFIG_ESP_CONSOLE_UART=n
```

### 4. Pinok (ajánlott)

| Funkció     | Ajánlott pin | Megjegyzés                  |
|-------------|--------------|-----------------------------|
| UART0 TX    | GPIO21       | Alapértelmezett             |
| UART0 RX    | GPIO20       | Alapértelmezett             |
| UART1 TX    | GPIO4        | Szabadon választható        |
| UART1 RX    | GPIO5        | Szabadon választható        |
| USB D−      | GPIO18       | Natív USB Serial/JTAG       |
| USB D+      | GPIO19       | Natív USB Serial/JTAG       |

### 5. Használat

1. Fordítsd és flash-eld:
   ```bash
   cargo build --release
   # vagy
   idf.py build flash monitor
   ```

2. A PC-n a USB Serial/JTAG porton (általában `/dev/ttyACM0` vagy `COMx`) ezt fogod látni:

```
0:hello from uart0
1:data from uart1
0:másik csomag
```

### Extra lehetőségek

- Ha **soronként** akarod az előtagot (nem bájtonként), akkor buffereld a sort `\n`-ig.
- Ha mindkét irányba kell (PC → UART is), akkor a fő szálban olvashatod a stdin-t és továbbíthatod a megfelelő UART-ra.
- Nagyobb sebességhez növeld a buffer méretét és csökkentsd a timeoutot.

Szeretnéd, hogy kiegészítsem **kétirányú** (PC → UART) támogatással is, vagy sor-alapú előtagozással?
