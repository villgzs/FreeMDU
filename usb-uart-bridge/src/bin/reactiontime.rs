#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use core::sync::atomic::{AtomicU8, Ordering};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer, TimeoutError};
use embedded_io_async::Write;
use esp_hal::{
    timer::timg::TimerGroup,
    gpio::{AnyPin, Input, InputConfig, Level, Output, OutputConfig, Pull, DriveMode},
    ledc::{
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
        LSGlobalClkSource, Ledc, LowSpeed,
    },
    time::Rate,
    usb::usb_serial_jtag::{UsbSerialJtag, UsbSerialJtagTx},
    Async,
};
use esp_backtrace as _;

// Minimum LED on-time after each transmission
const LED_MIN_ON_DURATION: Duration = Duration::from_millis(10);

static LED_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// 0 → 1 átmenet késleltetése (µs)
static DELTA_ON: AtomicU8 = AtomicU8::new(0);
/// 1 → 0 átmenet késleltetése (µs)
static DELTA_OFF: AtomicU8 = AtomicU8::new(0);

#[embassy_executor::task]
async fn led_task(mut led: Output<'static>) -> ! {
    loop {
        match embassy_time::with_timeout(LED_MIN_ON_DURATION, LED_SIGNAL.wait()).await {
            Ok(()) => led.set_low(),
            Err(TimeoutError) => led.set_high(),
        }
    }
}

/// GPIO1 másodpercenkénti állapotváltása
/// - 0→1 átmenetnél méri a DELTA_ON értéket
/// - 1→0 átmenetnél méri a DELTA_OFF értéket
#[embassy_executor::task]
async fn toggle_task(mut pin1: Output<'static>, pin0: Input<'static>) -> ! {
    let mut high = false;

    loop {
        if high {
            // ========== 1 → 0 átmenet (falling edge) ==========
            let start = Instant::now();
            pin1.set_low();

            let deadline = start + Duration::from_micros(200);
            let mut seen = false;

            while Instant::now() < deadline {
                if pin0.is_low() {
                    let delta = Instant::now()
                        .duration_since(start)
                        .as_micros()
                        .min(255) as u8;
                    DELTA_OFF.store(delta, Ordering::Relaxed);
                    seen = true;
                    break;
                }
            }
            if !seen {
                DELTA_OFF.store(255, Ordering::Relaxed); // timeout
            }
        } else {
            // ========== 0 → 1 átmenet (rising edge) ==========
            let start = Instant::now();
            pin1.set_high();

            let deadline = start + Duration::from_micros(200);
            let mut seen = false;

            while Instant::now() < deadline {
                if pin0.is_high() {
                    let delta = Instant::now()
                        .duration_since(start)
                        .as_micros()
                        .min(255) as u8;
                    DELTA_ON.store(delta, Ordering::Relaxed);
                    seen = true;
                    break;
                }
            }
            if !seen {
                DELTA_ON.store(255, Ordering::Relaxed); // timeout
            }
        }

        high = !high;

        // LED villanás
        LED_SIGNAL.signal(());

        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Másodpercenként elküldi mindkét delta értéket az USB Serial/JTAG-on
/// Formátum: "on:42 off:37\r\n"
#[embassy_executor::task]
async fn usb_tx_task(mut usb_tx: UsbSerialJtagTx<'static, Async>) -> ! {
    loop {
        let on  = DELTA_ON.load(Ordering::Relaxed);
        let off = DELTA_OFF.load(Ordering::Relaxed);

        let mut buf = [0u8; 24];
        let msg = format_deltas(&mut buf, on, off);

        let _ = usb_tx.write_all(msg).await;
        let _ = usb_tx.flush().await;

        Timer::after(Duration::from_secs(1)).await;
    }
}

/// "on:XX off:YY\r\n" formátum alloc nélkül
fn format_deltas(buf: &mut [u8; 24], on: u8, off: u8) -> &[u8] {
    let mut i = 0;

    // " on(us):"
    buf[i..i+8].copy_from_slice(b" on(us):");
    i += 8;
    i = write_u8(buf, i, on);

    // " off(us):"
    buf[i..i+9].copy_from_slice(b" off(us):");
    i += 9;
    i = write_u8(buf, i, off);

    // "\r\n"
    buf[i] = b'\r';
    i += 1;
    buf[i] = b'\n';
    i += 1;

    &buf[..i]
}

fn write_u8(buf: &mut [u8], mut i: usize, value: u8) -> usize {
    if value >= 100 {
        buf[i] = b'0' + value / 100;
        i += 1;
    }
    if value >= 10 {
        buf[i] = b'0' + (value / 10) % 10;
        i += 1;
    }
    buf[i] = b'0' + value % 10;
    i + 1
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // =====================================================
    // 100 kHz, 50% duty PWM a GPIO7-en (LEDC)
    // =====================================================
    {
        let mut ledc = Ledc::new(peripherals.LEDC);
        ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

        let mut lstimer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
        lstimer0
            .configure(timer::config::Config {
                duty: timer::config::Duty::Duty5Bit,
                clock_source: timer::LSClockSource::APBClk,
                frequency: Rate::from_hz(100_000),
            })
            .unwrap();

        let mut channel0 = ledc.channel(
            channel::Number::Channel0,
            unsafe { AnyPin::steal(7) },
        );
        channel0
            .configure(channel::config::Config {
                timer: &lstimer0,
                duty_pct: 50,
                drive_mode: DriveMode::PushPull,
            })
            .unwrap();
    }

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // USB Serial JTAG
    let usb = UsbSerialJtag::new(peripherals.USB_DEVICE).into_async();
    let (_usb_rx, usb_tx) = usb.split();

    // =====================================================
    // GPIO1 = kimenet, GPIO0 = bemenet
    // =====================================================
    let pin1 = Output::new(
        peripherals.GPIO1,
        Level::Low,
        OutputConfig::default(),
    );

    let pin0 = Input::new(
        peripherals.GPIO0,
        InputConfig::default().with_pull(Pull::None),
    );

    let led = usb_uart_bridge::new_status_led();

    spawner.spawn(led_task(led).unwrap());
    spawner.spawn(toggle_task(pin1, pin0).unwrap());
    spawner.spawn(usb_tx_task(usb_tx).unwrap());
}
