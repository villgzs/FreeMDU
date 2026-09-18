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
