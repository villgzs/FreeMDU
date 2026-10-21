#![no_std]

use embassy_time::{with_timeout, Duration};
use embedded_io_async::{ErrorType, Read, ReadExactError, Write};
use esp_hal::{
    Async,
    gpio::{AnyPin, Input, InputConfig, Level, Output, OutputConfig},
    uart::{Config, ConfigError, Instance, IoError, Parity, Uart},
};

#[macro_export]
macro_rules! num_from_env {
    ($name:literal, $type:ty) => {
        match <$type>::from_str_radix(env!($name), 10) {
            Ok(val) => val,
            Err(_) => panic!("failed to parse environment variable as number"),
        }
    };
}

pub struct OpticalPort<'a>(Uart<'a, Async>);

impl ErrorType for OpticalPort<'_> {
    type Error = IoError;
}

impl Read for OpticalPort<'_> {
//    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
//        // Retry on error
//        loop {
//            if let Ok(len) = self.0.read_async(buf).await {
//                return Ok(len);
//            }
//        }
//    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
       self.0.read_async(buf).await.map_err(IoError::Rx)
    }

    async fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), ReadExactError<Self::Error>> {
        while !buf.is_empty() {
            let len = self.read(buf).await?;

            buf = &mut buf[len..];
        }

        Ok(())
    }
}

impl Write for OpticalPort<'_> {
//    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
//        let len = self.0.write_async(buf).await?;
//
//        // Discard data that is read back by the optical receiver
//        for _ in 0..len {
//            self.read(&mut [0x00]).await?;
//        }
//
//        Ok(len)
//    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> { 
        let len = self.0.write_async(buf).await.map_err(IoError::Tx)?;

        // Echo bájtok eldobása timeout-tal (ne ragadjon be)
        for _ in 0..len {
            // Maximum ~2 bájt ideje 2400 baud-on (~8-10 ms), adjunk rá tartalékot
            match with_timeout(Duration::from_millis(20), self.read(&mut [0u8])).await {
                Ok(Ok(_)) => {}          // megjött az echo, eldobtuk
                Ok(Err(_)) => break,     // hiba történt → ne ragadjunk
                Err(_) => break,         // timeout → ne ragadjunk
            }
        }

        Ok(len)
    }	
 
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(self.0.flush_async().await?)
    }
}

#[must_use]
pub fn new_status_led<'a>() -> Output<'a> {
    const PIN: u8 = num_from_env!("PIN_LED_STATUS", u8);
    let led = unsafe { AnyPin::steal(PIN) };

    Output::new(led, Level::High, OutputConfig::default())
}

pub fn new_optical_port<'a>(
    uart: impl Instance + 'a,
    baudrate: u32,
) -> Result<OpticalPort<'a>, ConfigError> {
    const PIN_RX: u8 = num_from_env!("PIN_OPTICAL_RX", u8);
    const PIN_TX: u8 = num_from_env!("PIN_OPTICAL_TX", u8);

    let rx = Input::new(
        unsafe { AnyPin::steal(PIN_RX) },
        InputConfig::default(),
    );

    let tx = Output::new(
        unsafe { AnyPin::steal(PIN_TX) },
        Level::Low,
        OutputConfig::default(),
    );

    let cfg = Config::default()
        .with_baudrate(baudrate)
        .with_parity(Parity::Even);

    let uart = Uart::new(uart, cfg)?
        .with_rx(rx.peripheral_input().with_input_inverter(true))
        .with_tx(tx.into_peripheral_output().with_output_inverter(true))
        .into_async();

    Ok(OpticalPort(uart))
}

// home/src/lib.rs – OpticalPort bővítése

impl OpticalPort<'_> {
    /// Átállítja az optikai UART baud-sebességét.
    /// Figyelem: ne hívd, amíg adás folyamatban van (előtte flush).
    pub fn set_baudrate(&mut self, baud: u32) -> Result<(), ConfigError> {
        let cfg = Config::default()
            .with_baudrate(baud)
            .with_parity(Parity::Even);

        self.0.apply_config(&cfg)?;
        Ok(())
    }
}
