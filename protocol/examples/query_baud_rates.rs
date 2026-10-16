use freemdu::{BaudRate, Interface};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut port = freemdu::serial::open("/dev/ttyACM0")?;
    let mut intf = Interface::new(port);

    // Kötelező első lépés
    let software_id = intf.query_software_id().await?;
    println!("Software ID: {software_id}");

    // A query_max_baud_rate újabb eszközökön unlock_read_access
    // vagy unlock_smart_home_access után érhető el.
    // Próbáljuk először a smart home unlock-ot (kulcs nélküli),
    // ha az nem megy, esünk vissza a klasszikus read unlock-ra.
    // (A read unlock-hoz eszköz-specifikus kulcs kellene – itt
    // a device::connect-et használjuk, ha van támogatott ID.)

    match try_query_via_smart_home(&mut intf).await {
        Ok(max) => print_supported_rates(max),
        Err(_) => {
            println!("Smart-home unlock failed, trying device::connect…");
            // Újracsatlakozás a magasabb szintű API-val
            let mut port = freemdu::serial::open("/dev/ttyACM0")?;
            let mut dev = freemdu::device::connect(&mut port).await?;
            let max = dev.interface().query_max_baud_rate().await?;
            print_supported_rates(max);
        }
    }

    Ok(())
}

async fn try_query_via_smart_home(
    intf: &mut Interface<impl freemdu::Read + freemdu::Write>,
) -> Result<BaudRate, Box<dyn Error>> {
    intf.unlock_smart_home_access().await?;
    let max = intf.query_max_baud_rate().await?;
    Ok(max)
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
        let marker = if rate == max {
            "  ← maximum"
        } else if (rate as u8) <= (max as u8) {
            ""
        } else {
            continue; // magasabb, mint a max → kihagyjuk
        };
        println!("  {:>6} baud{}", rate.as_baud(), marker);
    }
}
