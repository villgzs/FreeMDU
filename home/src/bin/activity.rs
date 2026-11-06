#![no_std]
#![no_main]

extern crate alloc;

use esp_backtrace as _;
use esp_hal::time::{Duration, Instant};

const TESTED_RX_PIN: u8 = match core::primitive::u8::from_str_radix(env!("PIN_OPTICAL_RX"), 10) {
    Ok(pin) => pin,
    Err(_) => panic!("A PIN_OPTICAL_RX is not valid."),
};

const TESTED_TX_PIN: u8 = match core::primitive::u8::from_str_radix(env!("PIN_OPTICAL_TX"), 10) {
    Ok(pin) => pin,
    Err(_) => panic!("A PIN_OPTICAL_TX is not valid."),
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
#[allow(unused_variables)]
async fn main(spawner: embassy_executor::Spawner) {
    esp_alloc::heap_allocator!(size: 32 * 1024);

    let peripherals = esp_hal::init(esp_hal::Config::default());

    // RTOS
    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);

    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(
            peripherals.SW_INTERRUPT
        );

    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    // Optikai TX kimenet
    let mut gpioforout = esp_hal::gpio::Output::new(
        unsafe {
            esp_hal::gpio::AnyPin::steal(TESTED_TX_PIN)
        },
        esp_hal::gpio::Level::Low,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Optikai RX bemenet
    let _gpioforinput = esp_hal::gpio::Input::new(
        unsafe {
            esp_hal::gpio::AnyPin::steal(TESTED_RX_PIN)
        },
        esp_hal::gpio::InputConfig::default(),
    );

    // GPIO10
    let mut gpio10 = esp_hal::gpio::Output::new(
        peripherals.GPIO10,
        esp_hal::gpio::Level::Low,
        esp_hal::gpio::OutputConfig::default(),
    );
    
    // ESP32-C3 SuperMini beépített LED
    // GPIO8, active LOW
    let mut led = esp_hal::gpio::Output::new(
        peripherals.GPIO8,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    let gpios = esp_hal::peripherals::GPIO::regs();

   // Impulzus időzítése
    let mut gpioforout_high = false;
    let mut gpioforout_timer = Instant::now().duration_since_epoch();

    loop {
        let level =
        (gpios.in_().read().bits() & (1 << TESTED_RX_PIN)) != 0;

        // GPIO10 invertálva követi az RX állapotát
        if level {
            gpio10.set_low();
        } else {
            gpio10.set_high();
        }

        // 1 másodperces impulzusgenerátor
        let now = Instant::now().duration_since_epoch();

        if gpioforout_high {
            // 100 ms HIGH után LOW
            if now - gpioforout_timer >= Duration::from_millis(100) {
                gpioforout.set_low();
                gpioforout_high = false;
                gpioforout_timer = now;
                
                // LED kikapcsol
                led.set_high();
            }
        } else {
            // 900 ms LOW után HIGH
            if now - gpioforout_timer >= Duration::from_millis(900) {
                gpioforout.set_high();
                gpioforout_high = true;
                gpioforout_timer = now;
                
                // LED bekapcsol
                led.set_low();
            }
        }
    }
}
