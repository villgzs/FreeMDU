use freemdu::{BaudRate, Interface, serial::Port};
use serial2_tokio::{Parity, SerialPort, Settings};
use std::{error::Error, thread, time::Duration};
use tokio::time;

const UNLOCK_TIMEOUT: Duration = Duration::from_millis(150);
const ERROR_RETRY_DELAY: Duration = Duration::from_secs(4);
const CHECK_TIMEOUT: Duration = Duration::from_millis(50);

async fn find_read_access_key(intf: &mut Interface<Port>) -> Result<u16, Box<dyn Error>> {
    for i in 0x0000..=0xffff {
        println!("Trying read access key: {i:04x}");

        while let Err(err) = time::timeout(UNLOCK_TIMEOUT, async {
            intf.query_software_id().await?;
            intf.unlock_read_access(i).await
        })
        .await
        {
            eprintln!("Error: {err}");
            thread::sleep(ERROR_RETRY_DELAY);
        }

        // Check if read access was successfully unlocked
        if let Ok(Ok(_)) = time::timeout(CHECK_TIMEOUT, intf.read_memory::<u8, _>(0x0000)).await {
            return Ok(i);
        }
    }

    Err("Failed to find read access key".into())
}

async fn find_full_access_key(
    intf: &mut Interface<Port>,
    read_key: u16,
) -> Result<u16, Box<dyn Error>> {
    for i in 0x0000..=0xffff {
        println!("Trying read & full access keys: {read_key:04x}, {i:04x}");

        while let Err(err) = time::timeout(UNLOCK_TIMEOUT, async {
            intf.query_software_id().await?;
            intf.unlock_read_access(read_key).await?;
            intf.unlock_full_access(i).await
        })
        .await
        {
            eprintln!("Error: {err}");
            thread::sleep(ERROR_RETRY_DELAY);
        }

        // Check if full access was successfully unlocked
        if let Ok(Ok(())) = time::timeout(CHECK_TIMEOUT, intf.halt()).await {
            return Ok(i);
        }
    }

    Err("Failed to find full access key".into())
}

/// Serial port nyitás tetszőleges baud-dal (a stock open() mindig 2400).
fn open_at(path: &str, baud: u32) -> Result<Port, Box<dyn Error>> {
    let port = SerialPort::open(path, |mut settings: Settings| {
        settings.set_raw();
        settings.set_baud_rate(baud)?;
        settings.set_parity(Parity::Even);
        Ok(settings)
    })?;
    port.discard_buffers()?;
    Ok(embedded_io_adapters::tokio_1::FromTokio::new(port))
}

/// Próbál magasabb sebességre váltani (Smart Home unlock-kal, kulcs nélkül).
/// Siker esetén új Interface-t ad vissza az új baud-on.
async fn try_speedup(path: &str) -> Result<(Interface<Port>, BaudRate), Box<dyn Error>> {
    let port = open_at(path, 2400)?;
    let mut intf = Interface::new(port);

    intf.query_software_id().await?;
    intf.unlock_smart_home_access().await?;

    let max = intf.query_max_baud_rate().await?;
    println!("Max baud rate: {:?}", max);

    // Parancs a készüléknek (még 2400-on)
    intf.set_baud_rate(max).await?;

    // Új port az új sebességen
    let port = open_at(path, max.as_baud())?;
    let intf = Interface::new(port);

    Ok((intf, max))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let path = "/dev/ttyACM0";

    // 1) Próbáljunk felgyorsítani (újabb eszközökön)
    let mut intf = match try_speedup(path).await {
        Ok((intf, rate)) => {
            println!("Switched to {} baud", rate.as_baud());
            intf
        }
        Err(e) => {
            eprintln!("Speedup failed (old device?): {e} – staying at 2400");
            let port = freemdu::serial::open(path)?;
            Interface::new(port)
        }
    };

    // 2) Key search a (lehetőleg) magasabb sebességen
    let read_key = find_read_access_key(&mut intf).await?;
    let full_key = find_full_access_key(&mut intf, read_key).await?;

    println!("Found keys: {read_key:04x}, {full_key:04x}");
    Ok(())
}
