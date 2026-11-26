# fux-bach

Fugen im Stil Bachs zu einem gegebenen Thema, dazu die Zählungen an Bachs Fugen, an denen die
Regeln geeicht sind. Die Crate steht neben dem Fux-Code und ändert nichts an ihm: eigenes
Zeitraster (96 Ticks je Ganze), Tonarten statt Modi, der Akkord als Eingabe jeder Prüfung.

Zwei Programme:

- **`fux-fuge`** setzt Fugen (`fugue`), mehrere Fassungen einer Fuge zum Vergleichen (`variants`,
  `choose`), prüft Sätze gegen gegebene Akkorde (`check`) und zeigt die Regeln (`rules`).
  Anleitung: [`docs/fux-fuge.md`](../../docs/fux-fuge.md).
- **`bach-probe`** zählt und misst an Bachs Fugen und an den Fugen des Programms; aus seinen
  Zählungen stammen die Tabellen in `data/`
  (`cargo run --release -p fux-bach --bin bach-probe -- --help`). Es braucht die Korpora, die
  `tools/fetch_bach.sh` lädt (siehe
  [`references/datasets/bach-fugen/`](../../references/datasets/bach-fugen/README.md)).

## Aufbau

| Module | Aufgabe |
|---|---|
| `pitch`, `time`, `key`, `score` | Töne, Ticks, Tonarten und Akkorde, Stücke |
| `kern`, `roman`, `chordline`, `musicxml_read`, `algomus` | Leser: Noten, Akkorde, Etiketten |
| `rules` | die Regeln mit Kennung, Stufe und Meldung; Profile ([Katalog](../../docs/regeln-bach.md)) |
| `engine` | die Maschine: ein Übergang für Suche und Prüfung |
| `check` | Prüfung eines fertigen Satzes, Bericht, Zählung je Gelegenheit |
| `cells`, `melody`, `draft` | Rhythmus und Intervallwünsche der gesuchten Stimmen; vom Entwurf zur Aufgabe |
| `scaffold`, `harmonize`, `bass` | das Gerüst: Akkorde aus einem Plan, auf Wunsch mit einem Basston je Akkord |
| `answer`, `stretto`, `finder` | Antwort nach der Regel, Engführungen, Themeneinsätze finden |
| `compose` | der Weg einer Fuge: Plan, Kontrasubjekt, Zwischenspiele, Figuren der freien Stimmen, Suche in zwei Sätzen |
| `hands`, `export`, `musicxml` | zwei Hände, LilyPond, MIDI, MusicXML |
| `api`, `tables`, `themes` | JSON-Schnittstelle (auch für `crates/fux-bach-web`), Tabellen, Themen |
| `variants` | Fassungen einer Fuge: dieselbe Anfrage mit anderen Werten einzelner Aspekte, jede mit dem Namen dessen, worin sie sich unterscheidet |
| `figures`, `stats`, `chord` | Figuren und Kennzahlen, Akkordschätzung zum Eichen |
| `search`, `layout`, `fugue`, `neusatz`, `study*` | ein erster, einfacherer Satz ohne Gerüst und Neusätze von Fugen aus WTK I, für die Vergleichszahlen von `bach-probe` |
| `moments` | Zählungen an exponierten Stellen: Kadenzen, Einsätze, Enden der Zwischenspiele |
| `motifs` | motivische Arbeit, gemessen: Figuren, Themenbezug, Wiederkehr, Sequenz |
| `b1`, `b2`, `b3`, `b4`, `b5`, `b7` | weitere Zählungen und Messläufe von `bach-probe` |

Ohne das Feature `tools` hat die Bibliothek keine Threads und keine Kommandozeile und übersetzt
nach WebAssembly.

## Tabellen

`data/` enthält, was das Programm zur Laufzeit braucht, als Zahlen: `tables.json` (Rhythmuszellen,
Intervalle und Bänder der 48 Fugen), `harmony.json` (Akkordfolgen, harmonischer Rhythmus und der
Bass der Akkorde), `plans.json` (die Einsatzfolgen von Bachs Fugen in WTK I), `bach.profile` (die
Stufen der Regeln, an Bachs Raten geeicht) und `fuge.profile` (was der Fugenkomponist nach
blinden Hörproben anders setzt). Erzeugt von `bach-probe tables`, `b2-tables` und
`b3-templates`. Keine Tabelle enthält Noten oder Analysen der Korpora, nur Zahlen.

**Herkunft und Lizenz der Tabellen.**

- `tables.json`: gezählt an D. Hurons Kodierung der 48 Fugen des Wohltemperierten Klaviers und an
  C. S. Sapps Kodierungen der Kunst der Fuge und des Musikalischen Opfers.
- `harmony.json`: gezählt an den Stufenanalysen von *When in Rome* (M. Gotham u. a.; die Analysen
  der Choräle und Fugen überwiegend von D. Tymoczko, die der Präludien von M. Gotham u. a.),
  CC BY-SA 4.0, <https://github.com/MarkGotham/When-in-Rome>.
- `plans.json` enthält Informationen aus *algomus-data* (Algomus, `fugues/bach-wtc-i`; M. Giraud
  u. a.: „Computational Fugue Analysis“, *Computer Music Journal* 39 (2), 77–96, 2015,
  doi:10.1162/COMJ_a_00300), die unter der Open Database License 1.0 bereitstehen,
  <https://gitlab.com/algomus.fr/algomus-data>. `plans.json` selbst steht deshalb unter der
  [Open Database License 1.0](https://opendatacommons.org/licenses/odbl/1-0/); Rechte an den
  einzelnen Inhalten unter der [Database Contents License 1.0](https://opendatacommons.org/licenses/dbcl/1-0/).
  Die übrigen Dateien der Crate stehen unter der Lizenz des Repositorys.

Die Tests brauchen die Korpora nicht, bis auf jene, die ohne sie nichts prüfen; diese übergehen
ihre Prüfung dann.
