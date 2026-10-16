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

A következő lépés a `Command::SetBaudRate` és a `query_max_baud_rate()` konkrét implementációjának áttekintése. Ezekből már pontosan megadható a parancs felismerése, a sebesség paraméterének dekódolása és az UART biztonságos átállításának helye.

----

