# Bach-Fugen: Korpora

Die Korpora, an denen `fux-fuge` geeicht ist und an denen `bach-probe` zählt, geladen am
2026-11-24 mit [`tools/fetch_bach.sh`](../../../tools/fetch_bach.sh). Dieses Repository gibt sie
nicht weiter (siehe die Rechte unten); `tools/fetch_bach.sh` lädt sie von ihren Quellen hierher.
Die Programme brauchen sie nicht, nur `bach-probe` und die Tests, die ohne sie nichts prüfen.

| Verzeichnis | Inhalt | Quelle | Rechte |
|---|---|---|---|
| `wtc/` | Die 48 Fugen des Wohltemperierten Klaviers in Humdrum `**kern`, eine Spine je Stimme, nach der Ausgabe der Bach-Gesellschaft, Bd. 14: `wtc1f01.krn` bis `wtc2f24.krn` | D. Huron, [humdrum-tools/bach-wtc-fugues](https://github.com/humdrum-tools/bach-wtc-fugues), Verzeichnis `kern/` | „Copyright 1994, David Huron“, „Rights to all derivative electronic formats reserved.“ |
| `wir/` | Sieben Stufenanalysen von WTC-Fugen (I/19, I/22, II/7, II/11, II/16, II/23, II/24) im Format RomanText, „by Dmitri Tymoczko and his computer“ | [When in Rome](https://github.com/MarkGotham/When-in-Rome), `Corpus/Keyboard_Other/Bach,_Johann_Sebastian/The_Well-Tempered_Clavier_*/*_fugue/analysis.txt` | CC BY-SA 4.0 |
| `wir/choraele/`, `wir/praeludien/` | Stufenanalysen der 371 Choräle (`001.txt` bis `371.txt`, die meisten von D. Tymoczko) und der 24 Präludien von WTK I (`01.txt` bis `24.txt`, M. Gotham und andere), RomanText | [When in Rome](https://github.com/MarkGotham/When-in-Rome), `Corpus/Early_Choral/Bach,_Johann_Sebastian/Chorales/*/analysis.txt` und `Corpus/Keyboard_Other/Bach,_Johann_Sebastian/The_Well-Tempered_Clavier_I/*/analysis.txt` | CC BY-SA 4.0 |
| `kdf/` | Die Kunst der Fuge, BWV 1080: 20 Dateien `artfugue-001.krn` bis `artfugue-019.krn` (Contrapunctus 1 bis 14, Kanons, beide Formen der Spiegelfugen, die unvollendete Fuge), eine Spine je Stimme | C. S. Sapp, [craigsapp/art-of-the-fugue](https://github.com/craigsapp/art-of-the-fugue), Verzeichnis `kern/` (Zweig `main`) | keine Lizenzangabe im Repository |
| `opfer/` | Musikalisches Opfer, BWV 1079: die Ricercare a 3 und a 6 (`offering-001.krn`, `offering-002.krn`) und die vier Sätze der Triosonate mit Generalbass-Spine (`offering-013a.krn` bis `013d.krn`, vom Leser nicht gelesen) | C. S. Sapp, [craigsapp/bach-musical-offering](https://github.com/craigsapp/bach-musical-offering), Verzeichnis `kern/` (Zweig `main`) | keine Lizenzangabe im Repository |
| `algomus/` | Etiketten zu 23 Fugen von WTC I (Thema, Kontrasubjekte, Kadenzen, Orgelpunkte), `01-bwv846-ref.dez` bis `24-bwv869-ref.dez`; die Datei zu I/11 gibt es nicht | Algomus, [algomus-data](https://gitlab.com/algomus.fr/algomus-data), `fugues/bach-wtc-i/` | ODbL 1.0, Inhalte DbCL 1.0 |

- **Sapps Dateien** tragen keine Lizenz, **Hurons Dateien** behalten alle Rechte vor. Beide
  dienen nur dem eigenen Gebrauch (Zählung und Prüfung) und werden nicht weitergegeben, ebenso
  wenig Notenausgaben, die Töne aus ihnen enthalten (die Neusätze von `bach-probe`). Veröffentlicht
  werden aus ihnen nur Zahlen (`crates/fux-bach/data/`). `artfugue-012.krn` liest der Leser nicht
  (ein Ton überlappt in seiner Spine den vorigen).
- **When in Rome** (CC BY-SA 4.0) und **algomus-data** (ODbL 1.0) dürften weitergegeben werden,
  werden der Einfachheit halber aber ebenfalls geladen statt mitgeliefert. Aus algomus-data stammt
  `crates/fux-bach/data/plans.json`, das deshalb unter der ODbL 1.0 steht.
- **Eigenheiten der Dateien**, die der Leser `fux_bach::kern` kennt: geteilte Spines (`*^`, `*v`),
  abweichende Lesarten (`*S/sic`, `*S/ossia`), Akkorde in einer Spine, in `wtc1f24.krn` ein
  überzähliger Tabulator in jeder Zeile, in `wtc1f20.krn` die letzten sieben Takte nur als
  Kommentar (die fünfstimmige Stelle; die Datei endet nach Takt 80).
- **Algomus:** `start` und `duration` zählen in Vierteln ab dem Anfang der Datei, `staff` zählt die
  Stimmen von oben.
