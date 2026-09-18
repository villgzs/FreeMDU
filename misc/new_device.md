Az **`Error: UnknownSoftwareId(1494)`** azt jelenti, hogy a FreeMDU protocol crate-ben még **nincs regisztrálva** a 1494-es Software ID, ezért a `dump_memory` (és a többi high-level eszköz) nem tudja kezelni.

### Mit kell tenned?

A leggyakoribb megoldás (ahogy mások is csinálták, pl. a 1998-as ID-nél):

1. **Másold le egy meglévő eszköz fájlját**  
   Pl. a `protocol/src/device/id419.rs`-t (vagy bármelyik létezőt):

   ```bash
   cp protocol/src/device/id419.rs protocol/src/device/id1494.rs
   ```

2. **Szerkeszd meg az új fájlt** (`id1494.rs`):
   - Cseréld ki az összes `419`-et `1494`-re.
   - Írd be a saját **read access** és **full access** kulcsodat (amit a `find_keys`-szel találtál).

3. **Regisztráld az eszközt**  
   Nyisd meg a `protocol/src/device/mod.rs` (vagy `devices.rs`) fájlt, és add hozzá az új ID-t a listához / match ághoz, hasonlóan a meglévőkhöz.

4. **Fordítsd újra** a projektet:

   ```bash
   cargo build --all-features
   ```

5. Próbáld újra a dump-ot:

   ```bash
   cargo run --all-features --example dump_memory
   # vagy ha már bin-ként van:
   cargo run --all-features --bin dump_memory
   ```

### Fontos megjegyzések
- Ha még **nincs meg a két kulcs** (read + full access), először a `find_keys`-t futtasd végig, és csak utána próbáld a dump-ot.
- A memóriacímek (RAM/ROM tartomány) eszközönként eltérhetnek, ezért elsőre lehet, hogy csak részleges dumpot kapsz – ezt később finomhangolni kell.
- Ha elakadsz a fájlok szerkezeténél, írd meg, hogy pontosan milyen fájlok vannak nálad a `protocol/src/device/` mappában, és segítek a pontos módosításokkal.

A kulcsokat **itt** kell beírnod az új fájlban (`id1494.rs` vagy ahogy elnevezted):

Keresd meg ezt a részt (az `initialize` függvényben):

```rust
intf.unlock_read_access(0xb4ee).await?;
intf.unlock_full_access(0x4e83).await?;
```

És cseréld ki a saját kulcsaidra, például:

```rust
intf.unlock_read_access(0xXXXX).await?;   // ← ide a read access key
intf.unlock_full_access(0xYYYY).await?;   // ← ide a full access key
```

(Ahol `XXXX` és `YYYY` a `find_keys` által kiírt értékek hexában.)

### Összefoglalva a pontos hely:
- Fájl: `protocol/src/device/id1494.rs` (vagy amit másoltál)
- Függvény: `initialize`
- A két sor, ahol `unlock_read_access` és `unlock_full_access` van hívva.

Utána mentés + újrafordítás, és próbáld újra a `dump_memory`-t.

A figyelmeztetés (**unused import**) önmagában nem hiba, csak warning. A program lefordul, de a 1494-es ID még mindig nem lesz felismerve, amíg a modul nincs megfelelően regisztrálva.

### Mit kell még megcsinálnod?

1. **Nyisd meg a `src/device/mod.rs` fájlt** (ez a szülőmodul).

2. **Add hozzá az új modul deklarációt** a többihez hasonlóan:

```rust
mod id1494;
```

3. **A `connect` (vagy hasonló) függvényben** add hozzá a 1494-es esetet a match-hez.  
   Általában valami ilyesmi van ott:

```rust
match id {
    id419::compatible_software_ids!() => id419::WashingMachine::initialize(...).await,
    // ... többi ID ...
    id1494::compatible_software_ids!() => id1494::WashingMachine::initialize(intf, id).await,  // ← ezt add hozzá
    _ => Err(Error::UnknownSoftwareId(id)),
}
```

4. **A `id1494.rs` fájlban** a 26. sor (`pub(super) use compatible_software_ids;`) megmaradhat – ez a re-export a szülőmodul számára. A warning azért jön, mert a fájlon belül nem használod közvetlenül, de a szülőnek kell.

### Gyors ellenőrzés
A `id1494.rs` elején legyen ez (a 419-esről másolva):

```rust
macro_rules! compatible_software_ids {
    () => {
        1494
    };
}
pub(super) use compatible_software_ids;
```

És az `initialize`-ben a saját kulcsaid:

```rust
intf.unlock_read_access(0xXXXX).await?;  // a te read key-ed
intf.unlock_full_access(0xYYYY).await?;  // a te full key-ed
```

Ha elküldöd a `src/device/mod.rs` releváns részét (a `mod` deklarációkat és a `match` részt), pontosan megírom, mit kell beírni.

A fájlstruktúrád alapján pontosan ezt kell tenned:

### 1. Nyisd meg a `device.rs` fájlt

Add hozzá a modul deklarációt a többi `pub mod` sor közé:

```rust
pub mod id1494;
```

(pl. a `pub mod id419;` után)

### 2. A `connect` függvény `match` ágába is add hozzá

Keresd meg ezt a részt:

```rust
match id {
    id218::compatible_software_ids!() => { ... }
    ...
    id2895::compatible_software_ids!() => { ... }
    _ => Err(Error::UnknownSoftwareId(id)),
}
```

És szúrd be a 1494-es ágat a `_` elé:

```rust
id1494::compatible_software_ids!() => {
    Ok(Box::new(id1494::WashingMachine::initialize(intf, id).await?) as Box<dyn Device<P>>)
}
```

### 3. Ellenőrizd az `id1494.rs` fájlt

- A makró legyen 1494:

```rust
macro_rules! compatible_software_ids {
    () => {
        1494
    };
}
pub(super) use compatible_software_ids;
```

- Az `initialize` függvényben legyenek a saját kulcsaid:

```rust
intf.unlock_read_access(0xXXXX).await?;  // a te read key-ed
intf.unlock_full_access(0xYYYY).await?;  // a te full key-ed
```

### 4. Fordítsd újra

```bash
cargo run --all-features --bin dump_memory
```

A warning (`unused import`) el fog tűnni, amint a `device.rs`-ben használod a `compatible_software_ids!()` makrót.

Ha a kulcsokat még nem írtad be, vagy a struktúra neve nem `WashingMachine` (hanem pl. `TumbleDryer`), szólj, és igazítjuk.
