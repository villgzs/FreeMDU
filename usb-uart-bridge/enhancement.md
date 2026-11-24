Pontosan így van! **Az 5 bit még tökéletes egészre jön ki, a 6 bit viszont már nem.**

Nézzük meg a matematika hátterét a 100 kHz-es célnál (ahol az alapképlet: $\frac{800}{2^{\text{bit}}}$):

* **1 bit:** $800 / 2 = \mathbf{400}$ (Egész szám, 0 jitter)
* **2 bit:** $800 / 4 = \mathbf{200}$ (Egész szám, 0 jitter)
* **3 bit:** $800 / 8 = \mathbf{100}$ (Egész szám, 0 jitter)
* **4 bit:** $800 / 16 = \mathbf{50}$ (Egész szám, 0 jitter)
* **5 bit:** $800 / 32 = \mathbf{25}$ (**Tökéletes egész szám, 0 jitter!**)
* **6 bit:** $800 / 64 = \mathbf{12,5}$ (**Már nem egész**, frakcionális osztást igényel, így megjelenik a jitter)

### Miért kiváló választás az 5 bites felbontás?

Az 5 bit (32 lépés, 0–31-ig) az **utolsó olyan bitfelbontás** 100 kHz-en, amelynél az ESP32-C3 belső órajel-osztója még teljesen zéró jitterrel tud dolgozni.

* **50%-os kitöltés:** Pontosan a skála fele, azaz a **`16`**-os érték ($16 / 32 = 0,5$).
* **Finomabb állíthatóság:** Míg 3 bitnél csak 8 lépésed volt (12,5%-os lépésközökkel), 5 bitnél már **32 finom lépés** áll rendelkezésedre (lépésenként 3,125%-os pontossággal), miközben a frekvencia hajszálpontosan 100,000 kHz marad, mindenféle fázis-ingadozás nélkül.

### Példakód az 5 bites, jittermentes 100 kHz-hez:

```cpp
const int pwmPin = 6;           // GPIO6 (vagy GPIO5)
const uint32_t freq = 100000;   // Pontosan 100 kHz
const uint8_t resolution = 5;   // 5 bites felbontás (0-31 skála)

void setup() {
  // Hardveres PWM konfigurálása: 80 MHz / (32 * 100 kHz) = 25-ös tiszta egész osztó
  ledcAttach(pwmPin, freq, resolution);
  
  // 50%-os kitöltési tényező beállítása (a 32 lépésből a 16. jelenti a felet)
  ledcWrite(pwmPin, 16);
}

void loop() {
  // Hardveres, stabil négyszögjel 0 jitterrel és 32 fokozatú kitöltés-állíthatósággal
}

```
