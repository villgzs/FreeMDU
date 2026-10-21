use freemdu::{BaudRate, Interface, serial::Port};
use serial2_tokio::{Parity, SerialPort, Settings};
use std::{
    error::Error,
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
};
//use tokio::time;
use tokio::time::{timeout, Duration};

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
    // let port = open_at(path, 115_200 )?;
    let port = freemdu::serial::open(path)?;
    let mut intf = Interface::new(port);

    use tokio::time::{timeout, Duration};
    
    let mut do_speed_change: bool = false;
    let mut enable_baudchange: bool = true;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;            

    // 1st Query

//    println!("Software ID whithout timeout: {}", intf.query_software_id().await?);            

    println!("Trying to speed up...");
    println!("~~~ FIRST QUERY ~~~");    

    match timeout(
        Duration::from_millis(100),
        intf.query_software_id(),
    )
    .await
    {
        Ok(Ok(swid)) => {
            println!("ID from timed query): {:?}", swid);
            println!("0-Software ID: {}", intf.query_software_id().await?);            
//            tokio::time::sleep(std::time::Duration::from_millis(100)).await;            
        }
        Ok(Err(err)) => {
            println!("FreeMDU hiba: {:?}", err);
        }
        Err(_) => {
            println!("1st timeout");
        }
    }

    println!("~~~ SECOND QUERY ~~~");    

 
    match timeout(
        Duration::from_millis(100),
        intf.query_software_id(),
    )
    .await
    {
        Ok(Ok(swid)) => {
            println!("ID from 2nd query: {:?}", swid);
            println!("1-Software ID: {}", intf.query_software_id().await?);            
            intf.unlock_read_access(0xbde7).await?;
            intf.unlock_full_access(0x902f).await?;
//            intf.unlock_smart_home_access().await?;
            println!("|- Unlocked -|");
            println!("2-Software ID: {}", intf.query_software_id().await?);
 //           tokio::time::sleep(std::time::Duration::from_millis(100)).await;            
//            println!("speed up");
        }
        Ok(Err(err)) => {
            println!("FreeMDU hiba: {:?}", err);
        }
        Err(_) => {
            println!("Timeout: 100 ms alatt nem érkezett válasz. Baudrate change!");
        }
    }

    // 1st Query
    do_speed_change = true;

    println!("~~~ FIRST SPEED CHANGE ~~~");    
        
    if do_speed_change {
        match timeout(
            Duration::from_millis(100),
            intf.set_baud_rate(BaudRate::Baud9600),
        )
        .await
        {
            Ok(Ok(())) => {
                println!("Baudrate change to 19200 on device successfully.");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                println!("Tokio wait done.");
                println!("3-Software ID: {}", intf.query_software_id().await?);            
                intf.unlock_read_access(0xbde7).await?;
                intf.unlock_full_access(0x902f).await?;
                println!("At speed 9600 |- Unlocked -|");
                println!("4-Software ID: {}", intf.query_software_id().await?);                
                println!("5-Software ID: {}", intf.query_software_id().await?);

                enable_baudchange = false;
            }
            Ok(Err(err)) => {
                println!("Baudrate change failed: {:?}", err);
            }
            Err(_) => {
                println!("Baudrate changed on bridge only. Timeout... No answer from device.");

            }
        }
    }

    println!("~~~ QUERY ID AFTER FIRST SPEED CHANGE ~~~");    

    //        do_speed_change = true;
//    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    match timeout(
        Duration::from_millis(100),
        intf.query_software_id(),
    )
    .await
    {
        Ok(Ok(swid)) => {
            println!("Retry queried ID(at probably 9600 baud): {:?}", swid);
        }
        Ok(Err(err)) => {
            println!("FreeMDU hiba: {:?}", err);
        }
        Err(_) => {
            println!("Timeout: 10 ms alatt nem érkezett válasz.");
        }
    }

    println!("~~~ QUERY ID AGAIN AFTER FIRST SPEED CHANGE ~~~");    
        
    match timeout(
        Duration::from_millis(100),
        intf.query_software_id(),
    )
    .await
    {
        Ok(Ok(swid)) => {
            println!("ID again (at 9600 baud): {:?}", swid);
        }
        Ok(Err(err)) => {
            println!("FreeMDU hiba: {:?}", err);
        }
        Err(_) => {
            println!("Timeout: 10 ms alatt nem érkezett válasz.");            
        }
    }        


    println!("~~~ ... AND AGAIN ...  ~~~");    
        
    match timeout(
        Duration::from_millis(100),
        intf.query_software_id(),
    )
    .await
    {
        Ok(Ok(swid)) => {
            println!("ID again (at 9600 baud): {:?}", swid);
        }
        Ok(Err(err)) => {
            println!("FreeMDU hiba: {:?}", err);
        }
        Err(_) => {
            println!("Timeout: 10 ms alatt nem érkezett válasz.");            
        }
    }        



    println!("~~~ NORMAL UNLOCK ~~~");    

    intf.unlock_read_access(0xbde7).await?;
    intf.unlock_full_access(0x902f).await?;
    println!("|- Unlock -|");
    let max = intf.query_max_baud_rate().await?;
    println!("Max baud rate: {:?}", max);
    println!("baud rate: {:?}", 9600);
    
    // -----------------------------------------
    println!("~~~ BAUD RATE CHANGE AGAIN  ~~~");    
    
    intf.set_baud_rate(BaudRate::Baud9600).await?;   
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // -----------------------------------------
    println!("~~~ QUERY AFTER NORMAL UNLOCK ~~~");    
    
    match timeout(
        Duration::from_millis(100),
        intf.query_software_id(),
    )
    .await
    {
        Ok(Ok(swid)) => {
            println!("ID: {:?}", swid);
        }
        Ok(Err(err)) => {
            println!("FreeMDU hiba: {:?}", err);
        }
        Err(_) => {
            println!("Timeout: 500 ms alatt nem érkezett válasz.");
            do_speed_change = true;
        }
    }
    
    intf.unlock_read_access(0xbde7).await?;
    intf.unlock_full_access(0x902f).await?;
    println!("- Unlock -");

    let max = intf.query_max_baud_rate().await?;
    println!("Max baud rate: {:?}", max);
    
    // -----------------------------------------
    
    // -----------------------------------------

    println!("~~~ END OF SPEEDING UP ~~~");    
    
    Ok((intf, max))
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
   
    let path = "/dev/ttyACM0";

    // 1) Próbáljunk felgyorsítani (újabb eszközökön)
    let mut intf = match try_speedup(path).await {
        Ok((intf, rate)) => {
            println!("- Possible maximum is {} baud", rate.as_baud());
            intf
        }
        Err(e) => {
            eprintln!("Speedup failed (old device?): {e} – staying at 2400");
            let port = freemdu::serial::open(path)?;
            Interface::new(port)
        }
    };

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    println!("- Software ID: {}", intf.query_software_id().await?);
    
    intf.unlock_read_access(0xbde7).await?;
    intf.unlock_full_access(0x902f).await?;    
    
    println!("- Max baud rate: {:?}", intf.query_max_baud_rate().await? );

    println!("- Connection Successful");  

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("memory_dump.bin")?;

    // Resume dumping process if previously interrupted
    let start: u32 = file.seek(SeekFrom::End(0))?.try_into()?;

    for addr in (start..=0x000fffff).step_by(0x80) {
        println!("- Reading memory address {addr:08x}");

        let data: [u8; 0x80] = intf.read_memory(addr.into()).await?;

        file.write_all(&data)?;
    }

    Ok(())
}
