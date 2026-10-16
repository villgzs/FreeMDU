

---

A protocol-ban a device eszközökhöz tartozó fájlokban meg van határozva az egyes eszközök memória mérete?

---

**Nem, a protocol device fájljaiban nincs meghatározva az eszközök teljes memória mérete.**

### Mit tartalmaznak a device fájlok?

A `protocol/src/device/idXXX.rs` fájlok (pl. `id218.rs`, `id629.rs` stb.) a következőket definiálják:

- **Property-k** (pl. `operating_time`, `temperature`, `water_level`)
- **Action-ök** (pl. `start_program`, `set_program_options`)
- A property-k és action-ök mögötti **konkrét memória címek** (amiket a `query_property` / `trigger_action` metódusok használnak)
- Eszköz-specifikus enumok, bitflag-ek, dekódoló logika

Ezek a címek általában kis, ismert területekre mutatnak (pl. egy 1–4 bájtos érték egy adott címről), de **nincs** benne olyan konstans, ami az egész RAM / ROM / EEPROM méretét vagy a teljes címtartományt leírná.

### Hol van a memória méret?

Sehol sem központilag.

- A `dump_memory` példa **hardcode-olja** a tartományt:
  ```rust
  const START: u32 = 0x0000_0000;
  const END: u32   = 0x0000_ffff;   // 64 KiB
  ```
- A `dump_eeprom` hasonlóan fix tartományt használ.
- A low-level `Interface::read_memory(addr)` bármilyen címet elfogad, amit a készülék támogat — a méretet a hívó dönti el.

### Összefoglalva

| Dolog                          | Van a device fájlokban? | Hol van? |
|--------------------------------|--------------------------|----------|
| Teljes RAM/ROM méret           | **Nem**                  | Sehol (példákban hardcode) |
| Egyedi property címek          | **Igen**                 | `idXXX.rs` fájlokban |
| EEPROM méret                   | **Nem**                  | Példákban hardcode |
| Általános memória-térkép       | **Nem**                  | Nincs |

Tehát ha memória dumpot akarsz csinálni, a címtartományt neked kell megadnod (ahogy a `dump_memory` példában is van).
---

Milyen memória hivatkozások vannak az eszközfájlokban?

---

A protocol eszközfájlokban (`idXXX.rs`) a memória-hivatkozások **nem a teljes memória térképet**, hanem a konkrét diagnosztikai értékek címeit tartalmazzák. Ezek a következők:

### 1. Unlock kulcsok (minden eszköznél)
```rust
intf.unlock_read_access(0x43ea).await?;
intf.unlock_full_access(0x1f02).await?;
```
(Ezek a leggyakoribbak, de eszközönként eltérhetnek.)

### 2. RAM címek (`read_memory` / `write_memory`)

Ezek a leggyakoribb hivatkozások. Példák az `id218` és `id629` fájlokból:

| Cél | Tipikus cím (példa) | Megjegyzés |
|-----|---------------------|----------|
| Üzemidő (perc + óra) | `0x0010` / `0x0052` | 4 bájt (1 bináris perc + 3 BCD óra) |
| Hibakódok (aktív) | `0x002f`, `0x0085`, `0x0039`… | Bitmask-kel |
| Hibakódok (tárolt) | `0x000e`, `0x004e` | Bitmask-kel |
| Operating state | `0x008a` / `0x00cd` | Enum érték |
| Kiválasztott program | `0x0072` / `0x00b5` | |
| Program típus | `0x009d` / `0x00de` | |
| Program hőmérséklet | `0x009e` / `0x00df` | |
| Program opciók | `0x0016` / `0x0058` | Írható is |
| Spin setting | `0x0015` / `0x0057` | Írható is |
| Program fázis | `0x005f` / `0x00a2` | |
| ROM code | `0xffdf` / `0xffdb` | Magas cím |
| Aktuátorok | különböző | Bitflag-ek |
| NTC, vízszint, motor sebesség stb. | különböző Io címek | |

### 3. EEPROM címek (`read_eeprom`)

Főleg az újabb eszközöknél (pl. `id629`):

| Cél | Tipikus cím |
|-----|-------------|
| Gyári szám (serial number) | `0x01ba` |
| Serial number index | `0x01be` |
| Modellszám | `0x01bf` |
| Gyártási dátum | `0x01ce` |

### 4. Speciális / védelmi címek
Néhány eszköznél extra írások vannak a ROM védelem kikapcsolására:
```rust
// id629
intf.write_memory(0x02c2, 0x01u8).await?;   // ROM readout protection disable

// id360
intf.write_memory(0x0289, 0x01u8).await?;
```

### 5. Lookup táblák (kommentekben)
A kód gyakran megemlíti, hogy a firmware-ben hol vannak a lookup táblák (ezek **nem** olvasható címek a property-khez, csak dokumentáció):
- Program típus tábla: pl. `0xa760`, `0x2f66`, `0x5913`
- Hőmérséklet tábla: pl. `0xa776`, `0x2f92`
- Program fázis tábla: pl. `0xea99`, `0x93a3`, `0xad2d`

### Összefoglalva

Az eszközfájlokban lévő memória-hivatkozások:
- **Konkrét, kis méretű** (1–16 bájt) címek a diagnosztikai property-khez és action-ökhöz
- **RAM** (`read_memory` / `write_memory`) és **EEPROM** (`read_eeprom`) címek
- Unlock kulcsok
- Néhány speciális védelmi bit

**Nincs** bennük a teljes RAM/ROM/EEPROM méret vagy egy összefüggő memória-térkép. A címek eszközönként (Software ID-nként) eltérnek.

---

A memory dump 32bites olvasást említ. A fent megemlített címek 16biten elférnek. Melyik eszközfájlban van 16bitnél szélesebb memória hivatkozás?

---

**Egyik eszközfájlban sincs 16 bitnél szélesebb (32 bites) memória-hivatkozás.**

### Részletek

A low-level API (`Interface`) **támogatja** a 32 bites címeket:

```rust
pub async fn read_memory<...>(&mut self, addr: u32) -> ...
```

A dokumentáció szerint:
> „Newer devices support reading up to 65535 bytes from a **32-bit** memory address, while older devices are limited to 255 bytes and **16-bit** addresses.”

Ha a cím > `0xffff`, a protocol automatikusan elküldi az `ExtendAddress` parancsot.

A unit tesztekben is van példa 32 bites címre:
```rust
intf.read_memory(0x1234_abcd).await?;
```

### Az eszközfájlokban viszont

Az összes jelenlegi device implementáció (`id218`, `id324`, `id360`, `id419`, `id469`, `id517`, `id605`, `id629`, `id2088`, `id2895` stb.) **kizárólag 16 bites** címeket használ:

- Legmagasabbak: `0xffdf`, `0xffdb` (ROM code)
- A többi: `0x00xx` – `0x02xx` tartományban
- A kommentekben említett lookup táblák (`0xa760`, `0xea99`, `0x2f66` stb.) szintén beleférnek a 16 bitbe (`≤ 0xffff`)

### Összefoglalva

| Szint | Cím szélesség | Hol van |
|-------|---------------|---------|
| `Interface::read_memory` / `write_memory` | **32 bit** (`u32`) | `protocol/src/lib.rs` |
| Eszközfájlok property/action címei | **16 bit** | `protocol/src/device/idXXX.rs` |
| `dump_memory` példa | 32 bites típus, de 16 bites tartomány | `0x0000`–`0xffff` |

Tehát a 32 bites címkezelés a protokoll rétegben készen áll az újabb eszközökhöz, de a jelenlegi high-level device implementációk még nem használják ki.
