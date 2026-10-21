#![no_std]
#![no_main]

use core::panic::PanicInfo;
use embassy_executor::Spawner;
// use embassy_futures::select::{self, Either};
use embassy_futures::select::{self, Either3};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant};
use embedded_io_async::{Read, Write};
use esp_alloc as _;
use esp_hal::{
    Async, Config, gpio::Output, interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup, usb_serial_jtag::UsbSerialJtag,
};
use freemdu_home::OpticalPort;

// Buffer size for USB serial and optical port reads
const BUF_SIZE: usize = 32;

// Minimum LED on-time after each transmission
const LED_MIN_ON_DURATION: Duration = Duration::from_millis(10);

// Protocol locks after ~3 s of inactivity → we use a slightly longer timeout
const IDLE_TIMEOUT: Duration = Duration::from_millis(4000);
const DO_RETARD: Duration = Duration::from_millis(100);

static LED_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

esp_bootloader_esp_idf::esp_app_desc!();

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

#[embassy_executor::task]
async fn led_task(mut led: Output<'static>) -> ! {
    loop {
        match embassy_time::with_timeout(LED_MIN_ON_DURATION, LED_SIGNAL.wait()).await {
            Ok(()) => led.set_low(),
            Err(_) => led.set_high(),
        }
    }
}

/// Bridge state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BridgeState {
    /// Basic state – only here can a new handshake start
    WaitLoop,
    LockedHiSpeed,
}

/// Very simple detector for the short set-baud request frames.
/// Request format: [cmd, param_lo, param_hi, len, checksum]
fn detect_set_baud(data: &[u8]) -> Option<u32> {
//    if data.len() < 5 {
//        return None;
//    }

    match data[0] {
        0x46 => Some(2400), // SetBaudRate2400
        0x47 => Some(9600), // SetBaudRate9600
        0x4b => {
            // SetBaudRate (newer devices) – param is the BaudRate enum value
            match data[1] {
                0 => Some(2400),
                1 => Some(9600),
                2 => Some(19200),
                3 => Some(38400),
                4 => Some(57600),
                5 => Some(76800),
                6 => Some(115_200),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Detects the beginning of a new connection (software-id query)
fn is_software_id_query(data: &[u8]) -> bool {
    data.len() >= 5 && data[0] == 0x11
}

/// Detects any unlock command
fn is_unlock(data: &[u8]) -> bool {
    data.len() >= 5 && matches!(data[0], 0x20 | 0x21 | 0x32)
}

#[embassy_executor::task]
async fn bridge_task(
    mut serial: UsbSerialJtag<'static, Async>,
    mut opt: OpticalPort<'static>,
) -> ! {
    let mut serial_buf = [0u8; BUF_SIZE];
    let mut opt_buf = [0u8; BUF_SIZE];

    let mut state = BridgeState::WaitLoop;
    let mut last_activity = Instant::now();
    let mut do_delay = Instant::now();
    
    let mut do_baud_change: bool = false;
    
    // if let Ok(()) = opt.set_baudrate(9600) {  }; 

    loop {
	    // Próbálunk olvasni (ha a driver támogatja a non-blockingot)
	    // vagy nagyon rövid timeout-os with_timeout-ot használunk

	    // 1. PC → Opt
	    if let Ok(Ok(len)) = embassy_time::with_timeout(
	        Duration::from_micros(500),
	        serial.read(&mut serial_buf)
	    ).await {
	        // feldolgozás + last_activity frissítés
                last_activity = Instant::now();                               

	        let _ = opt.write(&serial_buf[..len]).await;
                LED_SIGNAL.signal(());

                if state == BridgeState::WaitLoop {
                   // if let Some(baud_rate) = detect_set_baud(&serial_buf[..len]) {
                   if serial_buf[0] == 0x47 { // 0x4b                      
                      do_baud_change = true;
                      do_delay = Instant::now();
                   }                   
                }

	    }

	    // 2. Opt → PC
	    if let Ok(Ok(len)) = embassy_time::with_timeout(
	        Duration::from_micros(500),
	        opt.read(&mut opt_buf)
	    ).await {
	        let _ = Write::write(&mut serial, &opt_buf[..len]).await;
	        last_activity = Instant::now();
	    }

	    // 3. Idle / baud logika (mindig lefut)
	    // ...
	    if ( last_activity.elapsed() >= IDLE_TIMEOUT ) 
	            && ( state == BridgeState::LockedHiSpeed ) {
//
	            let _ = opt.flush().await;                        
	            let _ = opt.set_baudrate(2400);
	            state = BridgeState::WaitLoop;
	            LED_SIGNAL.signal(());
//		            
	    }
	        
	    if do_baud_change && ( do_delay.elapsed() >= DO_RETARD ) {

	            let _ = opt.flush().await;
                    let _ = opt.set_baudrate(9600); 
	            state = BridgeState::LockedHiSpeed;
	            
	            do_baud_change = false;
	    }


	    // Kis szünet, hogy ne pörögjön 100%-on
	    embassy_time::Timer::after(Duration::from_micros(100)).await;                   
        
    }

}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let led = freemdu_home::new_status_led();
    let serial = UsbSerialJtag::new(peripherals.USB_DEVICE).into_async();
    let opt = freemdu_home::new_optical_port(peripherals.UART1,2400).unwrap();

    spawner.spawn(led_task(led).unwrap());
    spawner.spawn(bridge_task(serial, opt).unwrap());
}
