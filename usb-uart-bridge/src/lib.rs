#![no_std]

use esp_hal::gpio::{AnyPin, Level, Output, OutputConfig};

#[must_use]
pub fn new_status_led<'a>() -> Output<'a> {
    const PIN: u8 = 10;
    let led = unsafe { AnyPin::steal(PIN) };

    Output::new(led, Level::High, OutputConfig::default())
}
