Versions:

Designed to measure the response time of an adapter connected to the ESP32-C3. It measures the elapsed time from sending on (0→1) and off (1→0) signals on the TX pin until they appear on the RX pin, and outputs the results to the jtagserial console. 
(Toggles TX every seconds, 100kHz on pin7 for chargepump)  
```
cargo run --release --bin reactiontime -- --monitor
```

Upload delayed forward brige with echo suppression by buffer:  
(100kHz output on pin 7; delayed forward; delayed backwards)  
```
cargo run --release --bin usb-uart-bridge
```

Suppress sent data that is reflected back to the RX line from optical device. Anyway it is transparent.  
Upload local echo suppressor:  
```
cargo run --release --bin usb-uart-bridge-suppressor
```

Simple transparent bridge between UART1 and USBserialJTAG  
    There is a buffer for receiving data from UART1. Short delay before forward to USB.  

Upload local echo with delayed forward to USB port:  
```
cargo run --release --bin usb-uart-bridge-delayed
```

Simple transparent bridge between UART1 and USBserialJTAG  
Upload local transparent bridge ( no echo suppressed ):  
```
cargo run --release --bin usb-uart-transparent-bridge
```
