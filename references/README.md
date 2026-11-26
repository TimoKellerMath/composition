# Quellen

Literatur, Korpora und Ausgaben, auf die sich Regelkatalog und Testdaten stützen. Was davon fremden
Rechten unterliegt, gibt dieses Repository nicht weiter: Arbeiten und Ausgaben sind verlinkt, die
Korpora laden Skripte von ihren Quellen. Mitgeliefert werden nur Daten unter freien Lizenzen und
Zahlen, die an den Korpora gezählt sind.

## Literatur

Die Kürzel benutzt der [Regelkatalog](../docs/regeln.md).

| Kürzel | Arbeit | Link |
|---|---|---|
| W23 | T. Wafflard: FuxCP, zweistimmig. Masterarbeit UCLouvain 2023, 165 S. Regeln `1.H1` usw. mit Seitenangaben zu Mann und Chevalier. Hauptquelle für die zweistimmigen Regeln. | [webperso.info.ucl.ac.be](https://webperso.info.ucl.ac.be/~pvr/WAFFLARD_46581700_2023.pdf) |
| | D. Sprockeels, T. Wafflard, P. Van Roy, K. Haddad: *A Constraint Formalization of Fux's Counterpoint*, JIM 2023, 10 S. | [webperso.info.ucl.ac.be](https://webperso.info.ucl.ac.be/~pvr/SPROCKEELS-JIM2023.pdf) |
| L24 | Lamotte: FuxCP, dreistimmig. Masterarbeit UCLouvain 2024, 210 S. Regeln in Kap. 3, Kosten in § 4.2. | [webperso.info.ucl.ac.be](https://webperso.info.ucl.ac.be/~pvr/Lamotte_43031800_2024.pdf) |
| CdP24 | Cleenewerk, de Patoul: FuxCP, vierstimmig. Masterarbeit UCLouvain 2024, 238 S. | [dial.uclouvain.be](https://thesis.dial.uclouvain.be/server/api/core/bitstreams/1df8fbb1-1176-427c-939f-a81fbed2b7f5/content) |
| L25 | T. Lai: FuxCP, vierstimmig, harte und weiche Regeln getrennt. Masterarbeit UCLouvain 2025, 368 S. | [webperso.info.ucl.ac.be](https://webperso.info.ucl.ac.be/~pvr/LAI_08721900_2025.pdf) |
| B26 | Burignat, de Menten de Horne: neueste FuxCP-Arbeit. Masterarbeit UCLouvain 2026, 360 S. | [webperso.info.ucl.ac.be](https://webperso.info.ucl.ac.be/~pvr/Burignat_35171700_deMentendeHorne_62682000_2026.pdf) |
| | B. Schottstaedt: *Automatic Species Counterpoint*, CCRMA STAN-M-19, 1984. 51 S., Scan. Enthält ein Strafpunktsystem für die Regeln. | [ccrma.stanford.edu](https://ccrma.stanford.edu/files/papers/stanm19.pdf) |

Als Gegenprobe nützlich ist der Code von FuxCP5 (L25), mit Fux' Beispielen als MIDI-Zahlen in
`figureTests.cpp`: [github.com/TomLaiUCL/FuxCP5](https://github.com/TomLaiUCL/FuxCP5), Branch
`main-fuxcp`. Er hat keine Lizenzdatei.

Nicht frei verfügbar sind Manns englische Übersetzung (*The Study of Counterpoint*) und Jeppesens
*Kontrapunkt*. Jeppesens Unterlegung von Palestrinas *Missa Dies sanctificatus* (*Counterpoint*,
1939, S. 254–258), die ein Test prüft, ist als Testdatei
`tests/fixtures/text/dies-sanctificatus.toml` aus dem Buch abgeschrieben und wird deshalb nicht
weitergegeben; ohne sie übergeht der Test seine Prüfung.

## Datensatz Gotham (`datasets/gotham-species/`)

M. Gotham: Fux' sämtliche 123 Notenbeispiele (46 zwei-, 45 drei-, 32 vierstimmig), übertragen aus
dem Druck von 1725, im Humdrum-Format `**kern` mit Metadaten als JSON. Quelle ist
[github.com/MarkGotham/species](https://github.com/MarkGotham/species); Commit und Umfang der Kopie
stehen in `SOURCE.txt`. Übernommen sind die `.krn`- und `.json`-Dateien und die Übersichten
`data.tsv` und `corpus.json`. Lizenz: Die Kodierungen sind CC0 1.0 (so der Kopf jeder `.krn`-Datei:
„Mark Gotham and FourScoreAndMore.org waive all rights to this document“), der Code des
Repositorys steht unter der MIT-Lizenz (`datasets/gotham-species/LICENSE`). Die Datei
`III/sp5/gap_201.json` enthält ein überzähliges Komma; das Importskript toleriert es.

Daraus erzeugt `tools/import_gotham.py` die Testdaten in `tests/fixtures/fux/`. Am 2026-10-18 wurden
sie gegen eine unabhängige Extraktion derselben Daten geprüft: Alle 123 Beispiele stimmen Note für
Note überein, ebenso die Lage des Cantus.

## Korpora, die Skripte laden

| Verzeichnis | Inhalt | Laden | Rechte |
|---|---|---|---|
| `datasets/palestrina-masses/kern/` | Palestrinas Messen in Humdrum `**kern`, 1318 Dateien aus 101 Messen, kodiert von John Miller, übertragen von Bret Aarden, aus dem Korpus von music21 | `python3 tools/fetch_palestrina.py` | Urheberrecht John Miller, Lizenztext nicht mehr abrufbar ([README](datasets/palestrina-masses/README.md)) |
| `datasets/bach-fugen/` | Die 48 Fugen des Wohltemperierten Klaviers (D. Huron), die Kunst der Fuge und das Musikalische Opfer (C. S. Sapp), Stufenanalysen von *When in Rome*, die Algomus-Etiketten zu WTK I | `tools/fetch_bach.sh` | je Quelle verschieden ([README](datasets/bach-fugen/README.md)) |

Die Programme brauchen diese Korpora nicht. Sie dienen den Zählungen, aus denen Stufen und Tabellen
stammen (`fux rules --corpus palestrina`, `bach-probe`), und den Tests, die die Regeln an ihnen
prüfen; ohne die Korpora übergehen diese Tests ihre Prüfung.

## Fux-Ausgaben (`fux/`)

| Datei | Inhalt | Herkunft | Rechte |
|---|---|---|---|
| `gradus-beispiele.ly`, `gradus-beispiele.pdf` | Alle 123 Beispiele aus den Testdaten, mit Originalschlüsseln gesetzt (22 S.) | erzeugt mit `tools/render_fixtures.py` | CC0, wie die Daten von M. Gotham |

Neusätze aller Beispiele, die nur verlinkt sind:

| Inhalt | Link | Rechte |
|---|---|---|
| R. Würz: alle zweistimmigen Beispiele, Neusatz mit Intervallziffern (5 S.) | [IMSLP #803315](https://imslp.org/wiki/Special:ImagefromIndex/803315) | CC BY-NC-SA 4.0 |
| R. Würz: alle dreistimmigen Beispiele (9 S.) | [IMSLP #803316](https://imslp.org/wiki/Special:ImagefromIndex/803316) | CC BY-NC-SA 4.0 |
| A. Papillon: *Contrepoints du Gradus ad Parnassum*, Neusatz aller Beispiele (51 S.) | [IMSLP #479841](https://imslp.org/wiki/Special:ImagefromIndex/479841) | CC BY-NC-SA 4.0 |

Die beiden vollständigen historischen Ausgaben sind groß. Sie werden bei Bedarf im Browser von IMSLP
geladen, das automatische Downloads blockiert, und unter `fux/scans/` abgelegt. Dieser Ordner ist per
`.gitignore` vom Commit ausgenommen.

| Ablegen als | Inhalt | Link | Rechte |
|---|---|---|---|
| `fux/scans/fux-1725-gradus-latein.pdf` | Originalausgabe Wien 1725, Faksimile-Nachdruck Broude 1966 (23 MB) | [IMSLP #91138](https://imslp.org/wiki/Special:ImagefromIndex/91138) | gemeinfrei |
| `fux/scans/mizler-1742-gradus-deutsch.pdf` | Deutsche Übersetzung von L. C. Mizler, Leipzig 1742 (46 MB) | [IMSLP #273867](https://imslp.org/wiki/Special:ImagefromIndex/273867) | gemeinfrei |

Die Werkseite mit allen Dateien ist
[Gradus ad Parnassum auf IMSLP](https://imslp.org/wiki/Gradus_ad_Parnassum_(Fux,_Johann_Joseph)).

## Skripte (`../tools/`)

Alle Python-Skripte brauchen nur Python ≥ 3.11 ohne Zusatzpakete, `fetch_bach.sh` dazu `curl`. Sie
werden aus dem Wurzelverzeichnis des Repos aufgerufen.

| Skript | Zweck |
|---|---|
| `import_gotham.py` | erzeugt `tests/fixtures/fux/gradus-{2,3,4}st.toml` aus dem Gotham-Datensatz und ordnet jedem Beispiel seinen Cantus aus `cantus.toml` zu |
| `render_fixtures.py` | setzt alle Beispiele mit LilyPond (`fux/gradus-beispiele.ly` und `.pdf`); `--modern-clefs` für moderne Schlüssel |
| `corpus_stats.py` | Auswertungen, auf die sich der Regelkatalog mit „Korpus“ beruft |
| `fetch_palestrina.py`, `fetch_bach.sh` | laden die Korpora (oben) |
| `fetch_chant.py` | lädt die Choralmelodien von GregoBase neu (`tests/fixtures/chant/`, `crates/fux-core/src/imitation/kyriale/`) |
