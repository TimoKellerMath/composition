# fux-fuge: Fugen zu einem gegebenen Thema

`fux-fuge` setzt zu einem Thema eine Fuge für zwei bis vier Stimmen und prüft Sätze gegen
gegebene Akkorde. Es ist ein zweites Programm neben `fux` und teilt mit ihm die Methode (Regeln
mit Kennung und Stufe, an einem Korpus geeicht; eine Suche mit festen Tönen), nicht den Code:
Bachs Stil braucht Tonarten, Akkorde und ein feineres Zeitraster. Die Regeln stehen in
[`regeln-bach.md`](regeln-bach.md).

**Die Fugen sind vom Programm erzeugt, nicht von Bach.** Von Bach (und Händel) stammen die
Themen der Liste; alles andere ist Ergebnis von Planung und Suche. Sie sind vollständig, brechen
kein Verbot des Regelkatalogs und liegen in den Bändern von Bachs Fugen; meisterhaft sind sie
nicht (siehe [Grenzen](#grenzen)).

## Bauen

```sh
cargo build --release -p fux-bach
target/release/fux-fuge --help
```

Für PDF braucht es LilyPond, für MuseScore-Dateien `mscore`.

## Eine Fuge

```sh
fux-fuge themes                                   # die zwanzig Themen, die mitkommen, und die Pläne
fux-fuge fugue --theme bwv1080 --voices 4 --out out/kdf --pdf --mscz
fux-fuge fugue --subject "e'4 a' c''8 b' a' gis' | a'4 e' f'8 e' d' c' | b4" \
    --key a --meter 4/4 --out out/eigen --pdf
```

| Option | Bedeutung | Standard |
|---|---|---|
| `--theme NAME` | ein Thema der Liste | |
| `--subject "…"` | eigenes Thema, siehe [Notation](#notation) | |
| `--subject-file DATEI` | Thema aus kern oder MusicXML: die Stimme, die zuerst einsetzt, bis `--period-bars` (sonst bis zur ersten Pause von einem Schlag); Tonart und Takt aus der Datei | |
| `--key` | Tonart: Buchstabe mit `#` oder `b`, groß für Dur (`C`, `f#`, `Bb`) | nötig bei `--subject` |
| `--meter` | Taktart | `4/4` |
| `--period-bars` | Takte von einem Einsatz der Exposition zum nächsten (das Themenende) | Länge des Themas |
| `--voices` | 2, 3 oder 4 | 3 |
| `--ending` | `auto` (Engführung, wenn das Thema eine hergibt, sonst Orgelpunkt), `pedal`, `stretto` | `auto` |
| `--plan NAME` | Einsätze nach der Exposition wie in einer Fuge von WTK I (`wtk1-02`) | eigener Planer |
| `--plain` | ohne Sequenzen in allen Stimmen und ohne Figuren der freien Stimmen: ein Satz statt zweier bis vier, im Mittel ein Drittel bis ein Fünftel der Zeit | mit beidem |
| `--answer "…"` | eigene Antwort statt der Regel, im Rhythmus des Themas | Regel |
| `--bass planned` | das Gerüst nennt je Akkord einen Basston, gezogen aus dem, was die Analysen von Bachs Fugen über den Bass sagen; `free`: die Suche findet, was zuunterst liegt | `free` |
| `--profile NAME` | `fuge` (die Preise nach blinden Hörproben), `bach` (die an Bachs Raten geeichten) oder eine Profildatei | `fuge` |
| `--rule K=W` | einzelne Regel oder Stellgröße (`MEL-REPEAT=P2`), mehrfach | |
| `--seed` | andere Zahl, andere Fuge | 1 |
| `--beam` | Breite der Suche; mehr ist langsamer und selten besser | 400 |
| `--out PFAD` | schreibt `PFAD.musicxml`, `.ly`, `.mid` und `.akkorde.txt` (das Gerüst) | `out/fuge` |
| `--pdf`, `--mscz` | dazu PDF (LilyPond) und MuseScore | |
| `--lang en` | Bericht und Worte in der Partitur auf Englisch | `de` |

Dieselbe Eingabe gibt dieselbe Fuge. An den zwanzig Themen der Liste braucht eine dreistimmige
Fuge 2 bis 9 Sekunden, eine vierstimmige 7 bis 47 (ein Kern, Ryzen 7 PRO 6850U); mit `--plain`
sind es etwa 0,5 bis 3 und 1 bis 6.

### Fassungen vergleichen

`variants` setzt mehrere Fassungen derselben Fuge, die sich in den genannten Aspekten
unterscheiden und sonst in nichts, und `choose` nimmt eine davon:

```sh
fux-fuge variants --theme bwv1080 --voices 4 --vary bass,sound --out out/kdf --pdf
fux-fuge choose out/kdf b
```

| Option | Bedeutung |
|---|---|
| `--vary A,B` | die Aspekte, in denen sich die Fassungen unterscheiden: `bass` (frei, geplant), `sound` (das Profil: nach dem Ohr, nach der Zählung), `free-voices` (mit Figuren, schlicht), `seed`. Jede Kombination ihrer Werte ist eine Fassung, höchstens acht; die Werte des Aufrufs selbst geben Fassung a |
| `--seeds N` | mit `seed` unter den Aspekten: so viele Seeds, von `--seed` an (Vorgabe 3) |
| `--blind` | für eine blinde Hörprobe: die Buchstaben werden gelost, die Dateien einer Fassung nennen nur ihren Buchstaben, und worin sich die Fassungen unterscheiden, steht allein in `OUT-schluessel.md` |

Alle Optionen von `fugue` gelten auch hier. Geschrieben wird je Fassung `OUT-a.musicxml`, `.ly`,
`.mid`, `.akkorde.txt` (mit `--pdf` und `--mscz` auch diese), dazu `OUT-fassungen.md`: worin sich
die Fassungen unterscheiden, eine Tabelle mit dem Wert jedes Aspekts je Fassung und ihren Zahlen
(Takte, Töne, Kosten, Kennzahlen im Band, Zeit) und der Aufruf, der jede einzeln setzt. Jede
Fassung trägt ihren Namen in ihren Dateien, etwa „Fassung b: Klang: nach dem Ohr · Bass: geplant“
(im PDF als Untertitel, in MusicXML und MuseScore hinter dem Titel).

`choose OUT b` kopiert die Dateien der Fassung b nach `OUT.*` und nennt, was sie ist und wie
man sie setzt.

### Notation

Tonnamen wie in LilyPond mit niederländischen Vorzeichen (`cis`, `bes`, `es`), Oktavzeichen `'`
und `,` (`c'` ist das eingestrichene c), Dauern als Zahl hinter dem Ton (`4` Viertel, `8.`
punktierte Achtel); ein Ton ohne Zahl ist so lang wie der vorige. `r` ist eine Pause, `~` bindet
an den nächsten Ton, `|` ist nur für das Auge. Das Thema beginnt am Taktstrich: ein Auftakt wird
mit Pausen davor geschrieben (`r8 c''16 b' c''8 g' …`).

### Was der Bericht sagt

```text
Fuge über WTK I/2, BWV 847, c-Moll
c 4/4, 3 Stimmen, 36 Takte, 891 Töne (410 vorgegeben), 4.5 s
Antwort: r8 g''16 fis''16 g''8 c''8 es''8 … (tonal)
Takte: 1 Thema, 3 Antwort, 5 Codetta, 6 Thema, 8 Zwischenspiel, 10 Einsatz (Eb), …, 34 Kadenz
Schluss: Orgelpunkt; 10 von 10 Einsätzen wiedergefunden
0 Verbot(e), Kosten 874 aus 270 Präferenz(en).
Kennzahlen in Bachs Bändern: 26 von 29 (Maßstab 18.0)
Außerhalb: Schläge mit Dissonanz; Außenstimmen: Gegenbewegung; Klang: vollständiger Dreiklang
```

- **vorgegeben** sind die Töne, die nicht gesucht wurden: Thema und Antwort in jedem Einsatz, das
  Kontrasubjekt, die Motive der Zwischenspiele, die Orgelpunkte und von den Kadenzen der Bass
  (Quinte, Grundton) und der Vorhalt der Oberstimme mit seiner Auflösung. Die Glieder der
  Sequenzen und die Figuren der freien Stimmen zählen nicht dazu: Sie sind gesucht und dann
  wiederholt.
- **Verbote und Kosten** kommen aus der Prüfung der fertigen Fuge gegen ihr eigenes Gerüst, wie
  `fux-fuge check` sie rechnet. Ein Verbot kann nur gegebenes Material brechen, etwa ein Thema,
  das sich gegen kein Gerüst erklären lässt.
- **Kennzahlen:** 29 Zählungen (Notenwerte, Intervalle, Dissonanz, Figuren, Satz), jede gegen das
  Band, in dem 80 % von Bachs 48 Fugen liegen. Der Maßstab ist, was Bachs unauffälligste Fugen
  erreichen.
- **wiedergefunden:** ein Themenfinder, der nur die Noten kennt, findet die geplanten Einsätze.

Die Abschnitte der Zeile „Takte“ stehen auch in den Noten, als Worte über dem oberen System:
im PDF (LilyPond), in MusicXML und in der MuseScore-Datei.

## Einen Satz prüfen

```sh
fux-fuge check satz.musicxml --chords akkorde.txt
fux-fuge check out/kdf.musicxml --chords out/kdf.akkorde.txt --profile fuge   # eine eigene Fuge gegen ihr Gerüst
fux-fuge check fuge.krn                    # ohne Akkorde: aus den Noten geschätzt, ein Notbehelf
fux-fuge rules                             # alle Regeln mit Stufe
fux-fuge rules --file > mein.profil        # als Datei zum Ändern; dann --profile mein.profil
```

Gelesen werden kern und MusicXML (jede Stimme jedes Systems eine Stimme). Die Akkorde kommen als
RomanText oder als Zeilen je Takt:

```text
key d
1: Dm | 2: A7/C# Dm | 3: Gm . A7 Dm
5: key F  C7 F
```

`N:` beginnt Takt N, die Akkorde teilen den Takt in gleiche Teile, ein Punkt hält den vorigen
(nach `key` in der neuen Tonart). Ein Akkord mit Schrägstrich nennt seinen Bass (`A7/C#`, auch
`Dm/D` für die Grundstellung); dann prüft `HAR-BASS`, ob dieser Ton zuunterst klingt, wo der
Akkord beginnt. Ohne Schrägstrich ist kein Bass genannt.
Der Bericht nennt je Befund Takt, Schlag, Stimmen, Kennung, Stufe und Meldung und endet mit
„*n* Verbot(e), Kosten *k* aus *m* Präferenz(en)“; der Rückgabewert ist 2, wenn ein Verbot bricht.

`check` rechnet mit dem Profil `bach`, solange kein anderes genannt ist; der Komponist setzt
mit `fuge`, darum prüft man seine Fugen mit `--profile fuge`. Eine eigene Fuge, aus ihren Dateien
zurückgelesen, bricht kein Verbot (alle vierzig Beispiele, `tools/bach_fugen_dateien_pruefen.sh`).
Die Kosten können um wenige Punkte von denen im Bericht des Komponisten abweichen: die Prüfung
einer Datei weiß nicht, welche Töne vorgegeben waren, und schätzt die Orgelpunkte aus den Noten.

## Im Browser

Die Seite in [`web-fuge/`](../web-fuge/) setzt dieselben Fugen im Browser, mit Notenbild, Klang
und Dateien zum Herunterladen; `tools/build_web_fuge.sh` baut sie nach `target/web-fuge`. Auch
dort lassen sich Fassungen setzen und vergleichen: Die Seite nennt, worin sie sich unterscheiden,
und eine Fassung lässt sich ins Formular übernehmen.

## Grenzen

- **Die Harmonik** ist die größte bekannte Schwäche: Sie klingt nicht so gut wie bei Bach. Zwei
  blinde Hörproben haben die Preise des Komponisten verschoben (Profil
  `fuge`: zusammen angeschlagene Dissonanz kostet mehr, drei Stimmen bilden öfter den vollen
  Dreiklang). Das Gerüst kann je Akkord einen Basston nennen (`--bass planned`):
  gezählt liegt dann der Grundton so oft zuunterst wie bei Bach (50 statt 33 % der
  Akkordanfänge), aber in einer blinden Hörprobe klang es harmonisch gleich; der freie Bass
  bleibt die Vorgabe. Was weiter fehlt: aufgelöste Septimen und Leittöne, und die liegende Dissonanz,
  die Bach hat, wo das Programm Konsonanz setzt. Getragene Themen (Kunst der Fuge, Händel) legen
  das offener als laufende.
- **Die Stimmen neben Thema und Kontrasubjekt** arbeiten mit Figuren: mit Zellen aus Thema und
  Kontrasubjekt und mit eigenen, die sie eine Stufe tiefer wieder aufnehmen. Gezählt kehren die
  freien Töne etwa so oft wieder wie bei Bach (das Programm regelt darauf), aber die Stimmen
  schlagen öfter gemeinsam an als seine, weil die
  Zellen aus demselben Material kommen. Umkehrung und Vergrößerung als Arbeit gibt es nicht,
  auch keine Imitation zwischen den freien Stimmen.
- **Zwischenspiele** sind Sequenzen in allen Stimmen: das erste Glied als Modell, die folgenden
  eine Stufe tiefer. Glieder, die gegen das Gerüst nicht stehen, fallen weg und werden gesucht.
- **Rechenzeit.** Die Figuren brauchen zwei Sätze, bei zu viel Gleichförmigkeit einen dritten
  und vierten: im Mittel das Dreifache der Zeit von `--plain` bei drei Stimmen und knapp das
  Fünffache bei vier, bei einzelnen Themen das Elffache (dort führt ein fester Rhythmus in
  Sackgassen, und der Komponist nimmt die Figuren erst nach allen Versuchen zurück).
- **Eine Bauart.** Exposition, Gruppen von Einsätzen in verwandten Tonarten, Schluss mit
  Engführung oder Orgelpunkt. Vorlagen (`--plan`) ändern die Folge der Einsätze, nicht die Bauart.
  Auch der Schluss ist immer derselbe: ein Takt Kadenz, zwei Takte über dem Orgelpunkt der
  Tonika, im letzten der Schlussakkord mit dem Grundton in beiden Außenstimmen (dreistimmig
  Grundton, Terz, Grundton, vierstimmig der Dreiklang), in Moll mit der Durterz.
- **Kadenzen** gibt es nur in Grundstellung, am Ende von Gruppen von Einsätzen und vor dem
  Schluss; Halbschluss und Trugschluss plant das Programm nicht.
- **Themen, die nicht mit dem Programm kommen,** tragen weniger weit: An 17 ungeprüften Themen
  aus WTK I entstehen 33 von 34 Fugen, und in 8 von ihnen bleibt ein Ton des Themas gegen das
  Gerüst unerklärt (der Bericht nennt dann Verbote).
- **Vorhalte** schreibt die Suche seltener als Bach; der Satz ist konsonanter. Gesetzt sind nur
  die Quartvorhalte vor den Kadenzen.
- **Der Engführungsfinder** nimmt jede Distanz, die kein Verbot bricht. Nicht jede klingt.
- **Triolen** im Thema werden gesetzt und geschrieben, sind aber wenig erprobt.
