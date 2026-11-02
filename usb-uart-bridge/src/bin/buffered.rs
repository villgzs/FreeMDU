#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

// use core::sync::atomic::{AtomicBool, Ordering};
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_executor::Spawner;
// use embassy_sync::{
//    blocking_mutex::raw::CriticalSectionRawMutex,
//    signal::Signal,
// };
use embassy_time::{Duration, Instant, Timer, TimeoutError};
use embedded_io_async::{Read, Write};
use esp_hal::{
    timer::timg::TimerGroup,
    uart::{Config as UartConfig, Parity, Uart},
    gpio::{Output},
    gpio::interconnect::{InputSignal, OutputSignal,},
    usb::usb_serial_jtag::UsbSerialJtag,
    Async,
};
use esp_backtrace as _;

// Minimum LED on-time after each transmission
const LED_MIN_ON_DURATION: Duration = Duration::from_millis(10);

static LED_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

const BUFFER_SIZE: usize = 64;
const UART_BAUD: u32 = 2400;

/// Az UART → USB irányban alkalmazott késleltetés (ms).
/// Állítsd a kívánt értékre.
const DELAY_MS: u64 = 1000;

/// Egy bejegyzés a körkörös bufferben
struct TimedByte {
    data: u8,
    timestamp: Instant,
}

/// Körkörös buffer (no_std, lock-free, single-producer / single-consumer)
struct CircularBuffer {
    buf: [TimedByte; BUFFER_SIZE],
    head: AtomicUsize, // író pozíció
    tail: AtomicUsize, // olvasó pozíció
}

impl CircularBuffer {
    const fn new() -> Self {
        // Instant::now() nem konstans, ezért dummy értékkel inicializálunk
        const DUMMY: TimedByte = TimedByte {
            data: 0,
            timestamp: Instant::from_ticks(0),
        };
        Self {
            buf: [DUMMY; BUFFER_SIZE],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Új bájt berakása időbélyeggel (UART task hívja)
    fn push(&self, byte: u8, now: Instant) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let next = (head + 1) % BUFFER_SIZE;

        // Buffer tele van?
        if next == self.tail.load(Ordering::Acquire) {
            return false; // drop
        }

        // Biztonságos, mert SPSC és a head/tail atomikus
        unsafe {
            let ptr = self.buf.as_ptr() as *mut TimedByte;
            (*ptr.add(head)).data = byte;
            (*ptr.add(head)).timestamp = now;
        }

        self.head.store(next, Ordering::Release);
        true
    }

    /// A legöregebb bejegyzés kiolvasása (nem törli még)
    fn peek(&self) -> Option<(u8, Instant)> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return None;
        }

        unsafe {
            let entry = &*self.buf.as_ptr().add(tail);
            Some((entry.data, entry.timestamp))
        }
    }

    /// A legöregebb bejegyzés eldobása (után, hogy elküldtük)
    fn pop(&self) {
        let tail = self.tail.load(Ordering::Relaxed);
        let next = (tail + 1) % BUFFER_SIZE;
        self.tail.store(next, Ordering::Release);
    }
}

// Globális körkörös buffer
static CIRC_BUF: CircularBuffer = CircularBuffer::new();

static GOOUT: AtomicU8 = AtomicU8::new(0);
static COMEIN: AtomicU8 = AtomicU8::new(0);

#[embassy_executor::task]
async fn led_task(mut led: Output<'static>) -> ! {
    loop {
        match embassy_time::with_timeout(LED_MIN_ON_DURATION, LED_SIGNAL.wait()).await {
            Ok(()) => led.set_low(),
            Err(TimeoutError) => led.set_high(),
        }
    }
}

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
                    if Write::write_all(&mut writer, core::slice::from_ref(byte))
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

/// UART → körkörös buffer (csak tárol, nem küld azonnal)
#[embassy_executor::task]
async fn uart_rx_task(mut reader: esp_hal::uart::UartRx<'static, Async>) {
    let mut byte = [0u8; 1];

    loop {
        match Read::read(&mut reader, &mut byte).await {
            Ok(1) => {
                let now = Instant::now();

                // Echo-szűrés: ha még várunk echo-t, ne tegyük a bufferbe
                if COMEIN.load(Ordering::Relaxed) != GOOUT.load(Ordering::Relaxed) {
                    COMEIN.store(
                        COMEIN.load(Ordering::Relaxed).wrapping_add(1),
                        Ordering::Relaxed,
                    );
                    continue;
                }

                // Berakjuk a körkörös bufferbe időbélyeggel
                let _ = CIRC_BUF.push(byte[0], now);
            }
            _ => {}
        }
    }
}

/// Körkörös buffer → USB (késleltetéssel)
#[embassy_executor::task]
async fn delayed_usb_tx_task(
    mut writer: esp_hal::usb::usb_serial_jtag::UsbSerialJtagTx<'static, Async>,
) {
    loop {
        if let Some((data, timestamp)) = CIRC_BUF.peek() {
            
            LED_SIGNAL.signal(());
            
            let elapsed = Instant::now().duration_since(timestamp);

            if elapsed >= Duration::from_micros(DELAY_MS) {
                // Eltelt a késleltetés → küldjük
                let _ = Write::write_all(&mut writer, core::slice::from_ref(&data)).await;
                CIRC_BUF.pop();
            } else {
                // Még nem telt el → várunk a hátralévő időt
                let remaining = Duration::from_micros(DELAY_MS) - elapsed;
                Timer::after(remaining).await;
            }
        } else {
            // Üres a buffer → kis szünet, hogy ne pörögjön feleslegesen
            Timer::after(Duration::from_micros(100)).await;
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
        .with_rx(InputSignal::from(peripherals.GPIO0).with_input_inverter(true))
        .with_tx(OutputSignal::from(peripherals.GPIO1).with_output_inverter(true))
        .into_async();

    let (uart_rx, uart_tx) = uart.split();
    
    let led = usb_uart_bridge::new_status_led();

    // Három task:
    // 1. USB → UART (eredeti)
    // 2. UART → körkörös buffer (időbélyeggel)
    // 3. körkörös buffer → USB (késleltetéssel)
    spawner.spawn(usb_to_uart_task(usb_rx, uart_tx).unwrap());
    spawner.spawn(uart_rx_task(uart_rx).unwrap());
    spawner.spawn(delayed_usb_tx_task(usb_tx).unwrap());
    spawner.spawn(led_task(led).unwrap());
}
