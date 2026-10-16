Van egy időtartam a forgalom mentes időszakra, amit ha átlépünk, akkor a mosógép (slave) lezárja a kommunikációt és újból kell kezdeményezni. A kezdeményezés mindig 2400 baud sebességgel történik. Ha ilyen time-out történik, vissza kell állnia a bridge-nek a 2400-as sebességre. A nagyobb sebesség beállítása kizárólag a kapcsolat kezdeményezésekor fogadható el. Tehát ahogy a protocol szerint felépül a kapcsolat, akkor az ebben az üzenetsorban megjelenő bitsebesség váltást szabad csak a bridge-nek saját magára is értelmeznie. A kapcsolat felépítésének lépéseit meg kell nézni a freemdu implementációjában, de nagyjóból a megszólítás, id lekérés, unlock - és itt jön be a lehetséges maximális baud lekérdezése, majd ez alapján a beállítsa. Ha ezzel kezdődik a párbeszéd a host és a slave között, akkor a bridge átállhat a készüléktől kért baud-ra. A magasabb sebesség beállítás után már nem kell figyelni a fogalmat, csak továbbítani az optical port felé, további baud állítást nem engedünk meg. Újabb beállításra (baud sebesség állításra) csak a time-out letelte után lenne lehetőség, amikor újból a 2400 baud sebességgel kezdeményezi a host a kapcsolatot a slave-vel. A time-out leteltével a bridge visszaáll a 2400 baud sebességre.

---

Ez alapján a FreeMDU bridge működését egy kapcsolatfelépítéshez kötött, állapotvezérelt baud rate automatikával érdemes kiegészíteni. Ez jobb megoldás, mint a nyers adatfolyamban minden baud-beállító parancsot figyelni, mert a sebességváltás csak a kapcsolatfelépítési folyamat megfelelő pontján engedélyezett.

A szabályok így foglalhatók össze:

- A bridge induláskor és új kapcsolat kezdeményezésekor 2400 baud sebességgel működik.
- A baud rate módosítása csak a kapcsolatfelépítési protokoll során, a maximális sebesség lekérdezése és az unlock után engedélyezett.
- A sikeres sebességváltás után a bridge már nem értelmezi újra a baud-beállító üzeneteket, hanem átlátszóan továbbítja a forgalmat.
- Ha a forgalommentes időszak túllépi a protokoll timeoutját, a bridge visszaáll 2400 baudra, lezárja az aktuális állapotot, és új kapcsolatfelépítést engedélyez.

## Javasolt állapotgép

`Idle2400`

UART1 = 2400 baud · Új kapcsolat kezdeményezése

Megszólítás → ID-lekérés → unlock

`Negotiating`

A protokoll lépéseinek követése

Maximális baud lekérése → sebesség beállítása

Érvényes sebességváltás a megfelelő protokollállapotban

`ConnectedFast`

UART1 = egyeztetett sebesség

Átlátszó továbbítás, további sebességváltás tiltva

A protokoll szerinti forgalommentes timeout

Visszaállítás

UART1 = 2400 baud · Állapot = `Idle2400`

## Két megvalósítási részletre figyelnék

1\. A timeoutot mindkét irány forgalmának figyelembevételével kell kezelni. A hosttól vagy a slave-től érkező forgalom egyaránt frissítse az utolsó aktivitás időpontját. A timeout csak akkor járjon le, ha egyik irányban sem történt adatforgalom a protokoll által meghatározott ideig.

2\. Az UART visszaállítása előtt a régi sebességen folyó műveleteket le kell zárni. Az `OpticalPort` jelenlegi implementációja az `esp_hal::uart::Uart` objektumot burkolja, ezért a futásidejű átállítást ebben a rétegben célszerű megvalósítani.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://raw.githubusercontent.com\&sz=32)

GitHub



A jelenlegi [`bridge.rs`](https://github.com/medusalix/FreeMDU/blob/master/home/src/bin/bridge.rs?utm_source=chatgpt.com) csak nyers bájtokat továbbít, nem követi a kapcsolatfelépítési protokoll állapotát.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub

&#x20;Emiatt a módosításhoz először azonosítani kell a protocol implementáció konkrét üzeneteit és az unlock, a maximális baud lekérése, valamint a baud beállítása közötti sorrendet. Ezután lehet a bridge-ben biztonságosan megvalósítani az állapotgépet.



Egy fontos pontosítás: a timeout értékét ne találjuk ki, hanem a FreeMDU protokollban már használt időkorlátot vegyük át. Így a bridge és a host ugyanazt a kapcsolat-élettartamot fogja követni.

---

Ez már elegendő ahhoz, hogy a bridge-ben felismerhető protokollparancsokra építsük a baud rate-váltást. Különösen fontos a két parancs:

- `QueryMaxBaudRate = 0x38` – a maximális sebesség lekérdezése.
- `SetBaudRate = 0x4b` – az újabb készülékek sebességbeállító parancsa.

A régebbi parancsok (`SetBaudRate2400 = 0x46` és `SetBaudRate9600 = 0x47`) miatt a bridge-nek a régi protokollváltozatot is figyelembe kell vennie.

## Javasolt bridge-logika

| Állapot               | Felismert esemény                | Teendő                                         |
| --------------------- | -------------------------------- | ---------------------------------------------- |
| `Idle2400`            | `QuerySoftwareId` (`0x11`)       | Kapcsolatfelépítés követése                    |
| `Negotiating`         | `UnlockSmartHomeAccess` (`0x21`) | Feloldási szakasz                              |
| `Negotiating`         | `QueryMaxBaudRate` (`0x38`)      | A válasz figyelése                             |
| `Negotiating`         | Érvényes `SetBaudRate` (`0x4b`)  | Új sebesség alkalmazása                        |
| `ConnectedFast`       | Újabb baud-parancs               | Figyelmen kívül hagyás, változatlan továbbítás |
| Bármely aktív állapot | Forgalommentes timeout           | Visszaállás 2400 baudra                        |

Fontos: a `0x4b` parancskód önmagában nem feltétlenül tartalmazza a sebességet; a parancs paramétereit és keretezését is meg kell vizsgálni a `protocol` implementációban. Ugyanígy a `0x38` válaszának formátuma is szükséges ahhoz, hogy helyesen lehessen ellenőrizni a maximális támogatott sebességet.

A bridge-nek az eredeti bájtokat változtatás nélkül kell továbbítania. A parancsfelismerés csak a belső állapotot módosíthatja, nem nyelheti el és nem alakíthatja át a protokollüzeneteket.

A timeout pillanatában az UART-ot 2400-ra kell állítani, és újra engedélyezni a sebességegyeztetést. Így minden új kapcsolat az alapértelmezett sebességgel indul.

---

A kérésed alapján a `bridge.rs`-t úgy kell módosítani, hogy a protokoll szerinti baud-váltást (csak a kapcsolatfelépítés során) detektálja, a slave OK válasza után átálljon, timeout után pedig mindig visszaálljon 2400-ra.

### Összefoglaló a szükséges logikáról

A protokollból (FreeMDU `protocol` crate):

- Indulás **mindig 2400 baud**.
- Kapcsolatfelépítés: `query_software_id` → unlock (read / smart-home / full) → (opcionálisan) `query_max_baud_rate` → `set_baud_rate`.
- Baud-váltó parancsok:
  - `0x46` → 2400
  - `0x47` → 9600
  - `0x4b` + paraméter → magasabb sebesség (device válaszol a tényleges értékkel)
- A slave **~3 másodperc inaktivitás** után lezárja a kommunikációt → újraindítás csak 2400-zal lehetséges.
- A host oldalon a `set_baud_rate` után a portot újra kell nyitni az új sebességgel. A bridge-nek az **optikai UART**-ot is át kell állítania, különben a magasabb sebesség nem működik.

### Szükséges módosítások

1. **`OpticalPort` bővítése** (`home/src/lib.rs`) – baud váltás támogatása  
2. **`bridge_task` átírása** – kimenő forgalom figyelése, állapotgép, timeout

#### 1. `OpticalPort` – baud váltás

```rust
// home/src/lib.rs – OpticalPort bővítése

impl OpticalPort<'_> {
    /// Átállítja az optikai UART baud-sebességét.
    /// A TX/RX FIFO-t is üríti.
    pub fn set_baudrate(&mut self, baud: u32) -> Result<(), ConfigError> {
        let cfg = Config::default()
            .with_baudrate(baud)
            .with_parity(Parity::Even);
        // esp-hal Uart-on change_baud / apply_config jellegű metódus
        // (a pontos API a használt esp-hal verziótól függ)
        self.0.change_baud(&cfg)?;   // vagy a megfelelő API
        Ok(())
    }
}
```

(A pontos metódusnév az aktuális `esp-hal` verziótól függ – `change_baud` / `apply_config` / hasonló.)

#### 2. `bridge.rs` – állapotgép + figyelés

A lényeg:

- **Állapotok**:
  - `Idle2400` – alapállapot, figyeljük a baud-váltó parancsokat
  - `WaitingOk(baud)` – kiküldtük a set-baud parancsot, várjuk a `0x00` (Success) választ
  - `HighSpeed` – már átálltunk, csak továbbítunk, további baud-váltást **nem** fogadunk el
- Timeout (pl. 4 s inaktivitás) → mindig vissza 2400-ra + `Idle2400` állapotba.
- Csak a host→opt irányban figyelünk (a kiküldött adat).
- A válasz (opt→host) oldalán csak a `WaitingOk` állapotban várjuk a `0x00`-t.

Vázlat a `bridge_task`-re:

```rust
#[embassy_executor::task]
async fn bridge_task(
    mut serial: UsbSerialJtag<'static, Async>,
    mut opt: OpticalPort<'static>,
) -> ! {
    let mut serial_buf = [0u8; BUF_SIZE];
    let mut opt_buf = [0u8; BUF_SIZE];

    // Állapot
    enum State {
        Idle2400,
        WaitingOk { target: u32 },
        HighSpeed,
    }
    let mut state = State::Idle2400;
    let mut last_activity = Instant::now();
    const TIMEOUT: Duration = Duration::from_secs(4); // > 3 s a protokoll szerint

    // Egyszerű frame-parser a kimenő oldalra (cmd + param + len + checksum)
    // A legtöbb set-baud parancs rövid, default chunk_size=4

    loop {
        // Timeout ellenőrzés
        if last_activity.elapsed() > TIMEOUT {
            if !matches!(state, State::Idle2400) {
                let _ = opt.set_baudrate(2400);
                state = State::Idle2400;
            }
            last_activity = Instant::now();
        }

        match select::select(serial.read(&mut serial_buf), opt.read(&mut opt_buf)).await {
            Either::First(Ok(len)) => {
                // Host → Optical
                last_activity = Instant::now();

                // Figyeljük a set-baud parancsokat csak Idle2400-ban
                if matches!(state, State::Idle2400) {
                    if let Some(target_baud) = detect_set_baud_command(&serial_buf[..len]) {
                        // Először továbbküldjük a parancsot
                        let _ = opt.write(&serial_buf[..len]).await;
                        state = State::WaitingOk { target: target_baud };
                        LED_SIGNAL.signal(());
                        continue;
                    }
                }

                let _ = opt.write(&serial_buf[..len]).await;
                LED_SIGNAL.signal(());
            }
            Either::Second(Ok(len)) => {
                // Optical → Host
                last_activity = Instant::now();

                if let State::WaitingOk { target } = state {
                    // Várjuk a Success (0x00) választ
                    if len > 0 && opt_buf[0] == 0x00 {
                        let _ = opt.set_baudrate(target);
                        state = if target == 2400 {
                            State::Idle2400
                        } else {
                            State::HighSpeed
                        };
                    }
                }

                let _ = Write::write(&mut serial, &opt_buf[..len]).await;
            }
            _ => {}
        }
    }
}

/// Egyszerű detektor a set-baud parancsokhoz.
/// A Request formátum: [cmd, param_lo, param_hi, len, checksum]
fn detect_set_baud_command(data: &[u8]) -> Option<u32> {
    if data.len() < 5 {
        return None;
    }
    match data[0] {
        0x46 => Some(2400),                     // SetBaudRate2400
        0x47 => Some(9600),                     // SetBaudRate9600
        0x4b => {                               // SetBaudRate (magasabb)
            let rate_code = data[1];            // a BaudRate enum értéke
            match rate_code {
                0 => Some(2400),
                1 => Some(9600),
                2 => Some(19200),
                3 => Some(38400),
                4 => Some(57600),
                5 => Some(76800),
                6 => Some(115200),
                _ => None,
            }
        }
        _ => None,
    }
}
```

### Fontos megjegyzések

- A frame-parsing jelenleg nagyon egyszerű. A protokoll chunkolhat, és a checksum is benne van. Ha pontosabb detektálás kell, érdemes a `Request` struktúrát (vagy legalább a cmd + len mezőket) felismerni.
- A `set_baud_rate` a protokoll szerint **reseteli az access level-t**, ezért a host is újra unlock-ol. A bridge-nek ez nem gond, mert a timeout is visszaállít 2400-ra.
- A magasabb sebesség után (`HighSpeed`) **nem** fogadunk el újabb baud-váltást – csak timeout után.
- A 3 másodperces protokoll-timeout miatt a bridge timeoutját érdemes 3,5–4 másodpercre tenni.

Ha szeretnéd, elkészíthetem a teljes, konkrét `bridge.rs` + `lib.rs` diffet is (az aktuális FreeMDU forrás alapján), vagy finomíthatjuk a frame-parser logikát. Mondd el, melyik irányt szeretnéd.

A következő lépés a `Command::SetBaudRate` és a `query_max_baud_rate()` konkrét implementációjának áttekintése. Ezekből már pontosan megadható a parancs felismerése, a sebesség paraméterének dekódolása és az UART biztonságos átállításának helye.

----

