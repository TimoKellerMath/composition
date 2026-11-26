# Palestrinas Messen (Humdrum)

- **Inhalt:** 1318 Dateien in `kern/`, eine je Satz oder Abschnitt einer Messe (Kyrie, Christe,
  Gloria, Credo, Sanctus, Osanna, Benedictus, Agnus Dei), aus 101 Messen. 3–8 Stimmen, im Format
  Humdrum `**kern`: Noten, moderne Schlüssel, Tonart mit Modus (`*G:mix`), Takt 4/2 oder 3/2,
  Referenzzeilen mit Messe (`!!!OPR`), Satz und Abschnitt (`!!!OMD`, `!!!OTL`, siehe unten) und
  Kadenzzielen (`!!!RNB`). Kein Text.
- **Satz und Abschnitt:** Wie Humdrum die Zeilen festlegt, nennt `!!!OTL` den Inhalt der Datei und
  `!!!OMD` den Satz. Eine Datei mit einem ganzen Satz hat nur `!!!OTL` (`!!!OTL: Kyrie`, 449
  Dateien). Eine Datei mit einem Abschnitt eines geteilten Satzes hat beide: `!!!OMD` nennt den Satz,
  `!!!OTL` den Abschnitt (`!!!OMD: Credo` mit `!!!OTL: Crucifixus`, `!!!OMD: Kyrie` mit
  `!!!OTL: Christe`; 869 Dateien). So liest es `fux_core::kern` (Satz: `!!!OMD`, sonst `!!!OTL`;
  Abschnitt: `!!!OTL` neben `!!!OMD`), und so zeigt `fux corpus --palestrina` beide.
- **Herkunft:** kodiert von John Miller nach *Le Opere Complete* (Rom, Fratelli Scalera), in
  Humdrum übertragen von Bret Aarden. Kopie aus dem Korpus von
  [music21](https://github.com/cuthbertLab/music21) (`music21/corpus/palestrina`),
  Commit `1e68f5fc08970e8cee1e25627039dc0b64275367`, geladen am 2026-10-19 mit
  `python3 tools/fetch_palestrina.py`.
- **Rechte:** Jede Datei trägt „Copyright 2000, John Miller … Rights to all derivative electronic
  formats reserved. … This file must be accompanied by the licensing agreement.“ Der Lizenztext lag
  unter `http://www.music-cog.ohio-state.edu/HumdrumDatabases/Palestrina/license.txt` und ist dort
  nicht mehr abrufbar. music21 verbreitet die Dateien nach eigener Angabe „with the permission of
  the encoders“. Dieses Repository gibt die Dateien deshalb nicht weiter. Wer sie für Prüfung und
  Statistik braucht, lädt sie zum eigenen Gebrauch mit `python3 tools/fetch_palestrina.py` von
  music21 (fester Commit) nach `kern/`; die Tests, die sie brauchen, übergehen ihre Prüfung ohne sie.
