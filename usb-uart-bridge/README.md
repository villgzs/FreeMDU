Version:

Upload delayed forward brige with echo suppression by buffer:
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
