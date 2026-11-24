Version:

Upload delayed forward brige with echo suppression by buffer:
(100kHz output on pin 7; delayed forward; delayed backwards)
```
cargo run --release --bin usb-uart-bridge
```


Faster way, but it has some problem. I think the host send the ACK in a short time and communication stops sometime.

Upload local echo suppressor:
```
cargo run --release --bin usb-uart-bridge-suppressor
```

Upload local echo with delayed forward to USB port:
```
cargo run --release --bin usb-uart-bridge-delayed
```

Upload local transparent bridge ( no echo suppressed ):
```
cargo run --release --bin usb-uart-transparent-bridge
```
