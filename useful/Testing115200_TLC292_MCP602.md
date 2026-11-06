Az átvitel leromlott, ha letakartam az adó-vevő részt egy dobozzal. Lámpafény mellett működött hibamentesen.
A hibás átvitel esetén nem látszódott, hogy akár az erősítésre, akár az offset állításra reagált volna, azaz a kapcsolat nem áll helyre.

A TLC272 és at MCP602 ugyanazt mutatja, mindkettő működik, ha megvilágítás mellett tesztelem.

Feltételezés, hogy az erősítés begerjed, bár az oszcilloszkóp ábra nem igazolja.

Az MCP602 esetén az offszet felfelés tolása (0,4->0,6V) segített az átvitelen.

Az is tapasztalható, hogy a DSO152 működési (triggelési) határa az 57600baud. 115200-on már nagyon ritkán kapta el a jelet.

115200 Baud beállításnál:
Érzékenység: 
  12.4 kOhm -> ennél a működés egészen stabilnak látszódott.
  15.0 kOhm -> stabilan működik (letakarva hibázik)
  20.0 kOhm -> Comm.hiba határa, innen feljebb már sok a hiba, alatta még csak 1-1 sor
Offset állítás 0,75V-ra, engedte tovább növelni az érzékenységet.
  21.6 kOhm Comm.hiba határa

-------------
2400 baud, hogy a szkóp mutassa a jelszinteket (MCP602)

- Érzékenység 6,7kOhm már jól működő jelszintet ad, kimenet 0,43V /2.5V körül.
  
  
