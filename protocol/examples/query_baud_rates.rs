use freemdu::{BaudRate, Interface};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let port = freemdu::serial::open("/dev/ttyACM0")?;
    let mut intf = Interface::new(port);

    let software_id = intf.query_software_id().await?;
    println!("Software ID: {software_id}");

    // 1. próbálkozás: smart-home unlock (kulcs nélküli, újabb eszközök)
    match intf.unlock_smart_home_access().await {
        Ok(()) => {
            match intf.query_max_baud_rate().await {
                Ok(max) => {
                    print_supported_rates(max);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("query_max_baud_rate failed after smart-home unlock: {e}");
                }
            }
        }
        Err(e) => {
            eprintln!("smart-home unlock failed: {e}");
        }
    }

    // 2. próbálkozás: támogatott eszköz esetén device::connect
    println!("Falling back to device::connect…");
    let mut port = freemdu::serial::open("/dev/ttyACM0")?;
    let mut dev = freemdu::device::connect(&mut port).await?;

    match dev.interface().query_max_baud_rate().await {
        Ok(max) => print_supported_rates(max),
        Err(e) => {
            eprintln!("query_max_baud_rate not supported on this device: {e}");
            println!("This is normal on older appliances (only 2400 / possibly 9600 baud).");
        }
    }

    Ok(())
}

fn print_supported_rates(max: BaudRate) {
    println!("Maximum supported baud rate: {} baud", max.as_baud());
    println!();
    println!("Supported rates (≤ maximum):");

    let all = [
        BaudRate::Baud2400,
        BaudRate::Baud9600,
        BaudRate::Baud19200,
        BaudRate::Baud38400,
        BaudRate::Baud57600,
        BaudRate::Baud76800,
        BaudRate::Baud115200,
    ];

    for rate in all {
        if (rate as u8) > (max as u8) {
            continue;
        }
        let marker = if rate == max { "  ← maximum" } else { "" };
        println!("  {:>6} baud{}", rate.as_baud(), marker);
    }
}
