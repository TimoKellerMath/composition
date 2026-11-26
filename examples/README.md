# Beispiele

Elf Beispiele, die `fux` erzeugt hat: acht freie Sätze im Stil Palestrinas mit lateinischem Text,
sieben davon mit Imitation, einer ohne zum Vergleich, zwei Sätze in Fux' Gattungen und eine ganze
Messe, eine vierstimmige Missa brevis. **Die Sätze sind vom
Programm erzeugt, nicht von Palestrina oder Fux komponiert.** Aus Fux' *Gradus ad Parnassum* (1725)
und aus Palestrinas Messen stammen die Regeln, nach denen das Programm sucht und bewertet; die Noten
selbst sind das Ergebnis der Suche. Alle elf sind am 5.11.2026 erzeugt; was den Sätzen noch
fehlt, steht unter [Grenzen](#grenzen).

Die Fugen im Stil Bachs, die `fux-fuge` erzeugt, liegen mit eigener Übersicht in
[fugen/](fugen/README.md).

## Übersicht

| Nr. | Satz | Modus | Stimmen | Takte | Kosten | Seed | Zeit |
|---|---|---|---|---|---|---|---|
| [01](#01-kyrie-i-dorisch-vierstimmig-mit-imitation) | Kyrie I, Imitation | dorisch | S A T B | 25 | 47 | 5 | 3,7 s |
| [02](#02-christe-phrygisch-dreistimmig-mit-imitation) | Christe, Imitation | phrygisch | A T B | 27 | 25 | 10 | 2,8 s |
| [03](#03-agnus-dei-i-und-ii-hypomixolydisch-fünfstimmig-mit-imitation) | Agnus Dei I und II, Imitation | hypomixolydisch | S A T T B | 43 | 126 | 5 | 12,8 s |
| [04](#04-kyrie-i-über-dem-choral-kyrie-ix-cum-jubilo-vierstimmig) | Kyrie I über Kyrie IX *Cum jubilo* | dorisch (1. Modus des Chorals) | S A T B | 35 | 48 | 14 | 5,9 s |
| [05](#05-benedictus-hypoäolisch-vierstimmig-mit-imitation) | Benedictus, Imitation | hypoäolisch | S A T B | 31 | 31 | 5 | 4,7 s |
| [06](#06-eigener-text-ad-te-levavi-dorisch-dreistimmig-mit-imitation) | „Ad te levavi animam meam“, Imitation | dorisch | A T B | 14 | 7 | 6 | 1,7 s |
| [07](#07-gloria-et-in-terra--glorificamus-te-ionisch-vierstimmig-mit-imitation) | Gloria „Et in terra … Glorificamus te“, Imitation | ionisch | S A T B | 43 | 52 | 6 | 8,5 s |
| [08](#08-kyrie-i-ohne-imitation-dorisch-vierstimmig-zum-vergleich-mit-01) | Kyrie I ohne Imitation | dorisch | S A T B | 16 | 33 | 4 | 2,8 s |
| [09](#09-fux-fünfte-gattung-zweistimmig-über-dem-dorischen-cantus) | Fux: 5. Gattung zweistimmig | dorisch | S, Cantus im Alt | 11 | 4 | 4 | 1,0 s |
| [10](#10-fux-fünfte-gattung-vierstimmig-cantus-im-tenor) | Fux: 5. Gattung vierstimmig, Cantus im Tenor | äolisch | S A T B | 12 | 36 | 4 | 1,2 s |
| [11](#11-missa-brevis-ionisch-transponiert-vierstimmig) | Missa brevis: Kyrie, Gloria, Credo, Sanctus, Benedictus, Agnus Dei | ionisch, nach F transponiert | S A T B; Benedictus S A T, Agnus Dei II S A T T B | 907 | 3149 | 2 | 2,1 min |

Takte zählen im 2/2-Takt, eine Ganze je Takt. Die **Kosten** sind die Summe der Präferenzen, gegen
die ein Satz verstößt (Stufen P1–P4 kosten 1, 2, 4, 8; Verbote kommen in keinem Satz vor), und nur
zwischen Sätzen gleicher Art vergleichbar: Mehr Stimmen und mehr Takte kosten mehr; bei der Messe
sind es die Kosten ihrer 26 Teile zusammen. Die **Zeit** ist die des Skripts für ein Beispiel, mit
PDF und MuseScore-Datei, gemessen am 5.11.2026 in einem Lauf des ganzen Skripts auf einer sonst
ruhigen Maschine mit acht Kernen, die Messe mit vier Teilen zugleich (siehe
[11](#11-missa-brevis-ionisch-transponiert-vierstimmig)).

## Dateien

Jedes der Beispiele 01–10 besteht aus sechs Dateien gleichen Namens, etwa
`01-kyrie-imitation-dorisch-4stimmig.pdf`:

| Endung | Inhalt | Öffnen mit |
|---|---|---|
| `.pdf` | Partitur (LilyPond, A4) mit dem Text unter jeder Stimme. Über dem Kopf jedes Einsatzes steht eine Klammer mit der Nummer seines Punktes, über einzelnen Noten Musica ficta (♭, ♯). | jedem PDF-Betrachter |
| `.mid` | MIDI, eine Spur je Stimme, Klang „Choir Aahs“ (General MIDI 53), 72 Halbe pro Minute | jedem MIDI-Player, etwa VLC, TiMidity++ oder fluidsynth mit einer SoundFont, oder MuseScore |
| `.mscz` | MuseScore-Partitur mit Text und Klammern, aus der MusicXML-Datei umgewandelt; MuseScore legt die Stimmen als Chor an und spielt im Tempo der MIDI-Datei. Zum Anhören, Bearbeiten und Weiterexportieren | MuseScore 4 |
| `.musicxml` | MusicXML 4.0 mit Text, die Stimmen als Singstimmen in einer Klammer (`voice.soprano` …, Klang „Choir Aahs“), mit dem Tempo der MIDI-Datei (♩ = 144, nicht angezeigt) und einem Kopf: der Titel des PDF, darunter Modus und Stimmen, rechts „fux-kontrapunkt, Seed N“ mit dem Seed des Beispiels; dazu trägt die Datei Modus, Binnenschlüsse und den Plan der Imitation | jedem Notenprogramm (MuseScore, Dorico, Finale, Sibelius …); `fux check --musicxml DATEI` prüft den Satz wieder und listet seine Kosten Takt für Takt: bei den freien Sätzen dieselben wie beim Erzeugen, bei 09 und 10 je einen Punkt mehr, weil die Prüfung auch Fux' Cantus selbst bewertet |
| `.ly` | LilyPond-Quelltext des PDF | `lilypond DATEI.ly` |
| `.txt` | Ausgabe von `fux`: Kopfzeile, Text, Binnenschlüsse, Punkte mit ihren Einsätzen, Kosten und jede Stimme in Notennamen (`r` Pause, `~` Haltebogen) mit ihren Silben | jedem Texteditor |

Die Messe (11) hat statt einer Partitur ein Buch und besteht aus 21 Dateien:
`11-missa-brevis-ionisch-transponiert-4stimmig.pdf`, `.ly` und `.txt` für die ganze Messe und je
Satz eine `.musicxml`-, `.mid`- und `.mscz`-Datei mit der Nummer und dem Namen des Satzes,
`11-…-1-kyrie.musicxml` bis `11-…-6-agnus.musicxml`. Sie öffnen sich mit denselben Programmen.

| Datei | Inhalt |
|---|---|
| `.pdf`, `.ly` | Das Buch (LilyPond, A4, Systeme in 18 pt, 32 Seiten). Auf der ersten Seite Titel, Modus und Chor, rechts „fux-kontrapunkt, Seed 2“. Jeder Satz beginnt auf einer neuen Seite mit seinem Namen; darunter stehen bei Gloria und Credo die Intonation des Priesters („Intonatio: Glória in excélsis Deo.“), bei Kyrie und Agnus Dei ein Hinweis zur Aufführung. Jeder Abschnitt (Kyrie I, Christe, Kyrie II, Et in terra pax, …) ist eine eigene Partitur mit nur den Stimmen, die in ihm singen. Die Takte sind durch den Satz gezählt, ein Abschnitt endet mit Doppelstrich und Fermaten, der Satz mit dem Schlussstrich. Wo die Stimmen nach dem Takt Pause eines Halteklangs wieder singen, steht der Name des Teils, der dort beginnt (*Qui tollis*, *Et incarnatus est*, *Crucifixus*). Ab dem zweiten System heißen die Stimmen kurz, die zwei Tenöre des Agnus Dei II „T1“ und „T2“. Klammern über den Einsätzen und Ficta wie bei den anderen Beispielen. |
| `-1-kyrie.musicxml` … `-6-agnus.musicxml` | MusicXML 4.0 je Satz mit allen Stimmen des Satzes, als Singstimmen und mit dem Tempo der MIDI-Datei wie bei den anderen Beispielen; wer einen Abschnitt lang schweigt, hat dort Ganztaktpausen. Jeder Abschnitt beginnt auf einem neuen System mit seinem Namen als Studierzeichen und endet mit Doppelstrich und Fermaten. Dazu die Felder jedes Teils, aus denen `fux check --musicxml DATEI` den Bericht jedes Teils wiederholt, mit den Takten des Satzes. |
| `-1-kyrie.mid` … `-6-agnus.mid` | MIDI je Satz wie bei den anderen Beispielen, mit einer Marke je Abschnitt und Halteklang und einem stillen Takt nach jedem Doppelstrich |
| `-1-kyrie.mscz` … `-6-agnus.mscz` | MuseScore je Satz, aus der MusicXML-Datei umgewandelt |
| `.txt` | Ausgabe von `fux mass --details` ohne die Suchzeiten: der Plan, je Teil eine Zeile mit Neubeginn, Stimmen, Schlussstufe, Seed, Takten und Kosten und darunter die Zusammenfassung wie bei `fux free` (Text, Binnenschlüsse, Punkte mit ihren Einsätzen), je Satz der Kadenzplan, die Summe. Tonnamen deutsch: B ist B♭. |

## Neu erzeugen

```sh
tools/make_examples.sh                     # alle Beispiele: 01–10 etwa 45 s auf acht Kernen, die Messe etwa 2 min
tools/make_examples.sh 03 7                # nur die Beispiele 03 und 07
tools/make_examples.sh 11                  # nur die Messe
tools/make_examples.sh --nur-notenbild 11  # nur die Messe, aus ihren Teilen ohne Suche, etwa 10 s
```

Das Skript baut `fux` bei Bedarf (`cargo build --release -p fux-cli`) und ruft für jedes Beispiel
den unten genannten Befehl mit `-f ly,midi,musicxml -o examples/NAME` auf. Dann gibt es der
`.ly`- und der `.musicxml`-Datei den Titel des Beispiels: `fux` nennt einen Satz in LilyPond nach
Modus und Text, was 01, 04 und 08 gleich als „dorisch – Kyrie I“ betitelt, und in MusicXML „Freier
Satz 1“ bzw. „Kontrapunkt 1“, was MuseScore als Titel zeigt. Sonst bleiben die Dateien, wie `fux`
sie schreibt. Das PDF setzt das Skript wie `fux` mit LilyPond (`lilypond -dno-point-and-click
--pdf`), die MusicXML-Datei wandelt es mit MuseScore um (`QT_QPA_PLATFORM=offscreen mscore -o
NAME.mscz NAME.musicxml`). Es braucht LilyPond ab 2.24 (ein anderer Pfad mit `FUX_LILYPOND`) und
MuseScore 4 (ein anderer Befehl mit `MSCORE`). Die Seeds stehen fest im Skript, und jeder Lauf
schreibt dieselben Sätze: `.ly`, `.mid`, `.musicxml` und `.txt` sind Byte für Byte gleich. PDF und
`.mscz` enthalten bei gleichen Noten Zeitstempel und zufällige Kennungen (von Ghostscript bzw.
MuseScore) und unterscheiden sich darin von Lauf zu Lauf. Ändert ein späterer Stand des Programms
seine Regeln oder seine Suche, können dieselben Seeds andere Sätze ergeben; dann gelten die Angaben
hier nicht mehr.

Die Messe setzt das Skript mit `fux mass … -f ly,midi,musicxml` in den Ordner `target/beispiele/`,
neben den Ordner `NAME.teile/`, in den `fux mass` jeden Teil legt, sobald er gesetzt ist, und kopiert
von dort das Buch, die Sätze und `NAME.txt` nach `examples/`. Was davon ein früherer Lauf dort
ließ, löscht es vorher, damit es nur kopiert, was `fux` jetzt schreibt. Die Titel bleiben, wie
`fux` sie schreibt: „Missa brevis“ mit Modus und Chor im Buch, dazu der Name des Satzes in jeder
MusicXML-Datei. Aus der `.txt`-Datei nimmt das Skript die Suchzeiten der Teile und der Messe
heraus, die von Lauf zu Lauf schwanken; so ist auch sie Byte für Byte gleich, denn die Messe setzt
es auf jedem Rechner mit `--jobs 4`, vier Teilen zugleich: Zeile 2 nennt die native Schätzung für so
viele Teile, die sonst von den Threads des Rechners abhinge, und die Noten sind mit jedem
`--jobs` dieselben. Das Buch setzt es wie die anderen PDF mit LilyPond, die sechs Sätze wandelt
MuseScore in einem Auftrag um (`mscore -j` mit einer Liste der Dateien). Bleibt die Messe
unvollständig, bricht das Skript ab, bevor es etwas in `examples/` ändert. Die Teile bleiben in
`target/beispiele/NAME.teile/`; wie alles unter `target/` stehen sie nicht im Repository, und
`cargo clean` löscht sie mit `target/` (außer wenn `CARGO_TARGET_DIR` woandershin zeigt).

**Nur das Notenbild.** Nach einer Änderung allein am Notenbild (LilyPond, MusicXML, MIDI) setzt
`tools/make_examples.sh --nur-notenbild 11` die Messe ohne Suche neu: `fux mass … --resume` nimmt
jeden Teil aus seiner Datei in `target/beispiele/NAME.teile/` und schreibt Buch, Sätze und MIDI in
weniger als einer Sekunde neu; mit LilyPond und MuseScore braucht das Skript dann 10 s statt etwa
2 min. Der Schalter darf an beliebiger Stelle unter den Nummern stehen und betrifft nur die Messe;
01–10 haben keine Teile und entstehen wie ohne ihn. Ohne aufgehobene Teile bricht das Skript mit
einer Meldung ab, bevor es `fux` baut oder ein Beispiel setzt: Der erste Lauf ist einer ohne
Schalter. `--resume` weist Teile ab, die mit einem anderen Plan, Seed, Regelprofil oder von einer
anderen Version des Generators gesetzt sind (`generator=` in `fux-part`, Konstante
`fux_core::mass::GENERATOR`), und seine Meldung beendet das Skript. Die Version erhöht von Hand jede
Änderung, nach der dieselbe Anfrage andere Noten setzt; hat eine das versäumt, nimmt `--resume` die
alten Teile, und das Skript merkt es nicht. Nach einer Änderung am Generator (Regeln, Tabellen,
Suche), die andere Noten setzen kann, ist die Messe deshalb ohne den Schalter neu zu setzen. Ein
Lauf mit `--resume` nennt bei jedem Teil statt der Suchzeit den Ordner, aus dem er ihn nahm, und in
der Summe, wie viele Teile von dort kamen. Das Skript nimmt beides wie die Suchzeiten aus dem
Bericht und macht nur weiter, wenn er dann Byte für Byte mit `examples/NAME.txt` übereinstimmt;
diese Datei bleibt, wie sie ist. Sonst sind es nicht die Teile des Beispiels, oder `fux` berichtet
eine Messe inzwischen anders, und das Skript bricht ab, bevor es eine Datei der Messe in `examples/`
ändert. Fehlen im Ordner Teile, etwa nach einem abgebrochenen Lauf, sucht `fux` sie, wie `--resume`
es tut.

**Auswahl.** Für jedes Beispiel entstanden mehrere Kandidaten, meist mit den Seeds 1–10 (das Agnus
Dei mixolydisch 1–16 und hypomixolydisch 1–10, das Kyrie über dem Choral 1–20, das Gloria 1–6, das
Benedictus äolisch 1–8 und hypoäolisch 1–6, Beispiel 10 mit jedem der sechs Cantus von Fux und den
Seeds 1–5, der äolische auch 6–12, die Messe mit den Seeds 1–5). Gewählt wurde nach den Maßstäben
des Programms (keine Warnung, kein Verbot, niedrige Kosten, alle geplanten Einsätze klar) und nach
dem Notenbild: ein deutliches Soggetto, Einsätze in Quinte oder Oktave, die einander überlappen,
eine natürliche Unterlegung, klare Schlüsse am Ende von Sinnabschnitten und die Lagen der Stimmen;
bei der Messe auch nach ihrem Kadenzplan, dessen Binnenschlüsse und Schlussstufen wechseln sollen.
Ein billigerer Kandidat musste weichen, wo ihm eines davon deutlich fehlte. Die Kommentare im
Skript nennen die Kandidaten mit ihren Kosten.

Die Befehle unten sind die des Skripts; `fux` steht für `target/release/fux`. Die Angaben zu
Punkten und Einsätzen sind die Zusammenfassung, die `fux free` ausgibt („T. 18½“: auf der Mitte von
Takt 18; die Intervalle zum ersten Ton des Führers). Mit `--details` nennt `fux free` außerdem
jeden einzelnen Posten der Kosten, `fux mass` die Zusammenfassung jedes Teils.

## 01 Kyrie I, dorisch, vierstimmig, mit Imitation

Freier Satz mit Text und Imitation. Text: Kyrie I („Ký-ri-e e-lé-i-son.“). Modus:
dorisch. Stimmen: Sopran, Alt, Tenor, Bass. Seed 5, 25 Takte, Kosten 47.

```sh
fux free --mode dorisch --voices 4 --text kyrie --seed 5
```

- Punkt 1 („Ký-ri-e e-lé-i-son.“): Alt T. 1 D4 · Bass T. 3 D3 (Unteroktave) · Sopran T. 5 A4
  (Quinte) · Tenor T. 10 A3 (Unterquarte); Kopf 4–7 Töne, real; Binnenschluss T. 17 auf A
- Punkt 2 („e-lé-i-son.“): Tenor T. 18½ D3 · Alt T. 19½ A3 (Quinte); Kopf 5 Töne, real; Schluss
  T. 25 auf D

Das Soggetto steigt von der Finalis zur Quinte, D–F–G–A (Ganze, zwei Halbe, Ganze). Der Bass bringt
es zwei Takte später eine Oktave tiefer, Sopran und Tenor antworten auf der Quinte mit
A–C–D–E–D–C–D: Finalis und Quinte antworten einander wie bei Palestrina. Bass und Sopran setzen ein,
bevor die Stimme vor ihnen ihren Kopf beendet hat (Engführung). Nach dem Binnenschluss auf A singen
Tenor und Alt den Schlussteil „e-lé-i-son.“ als zweiten Punkt mit eigenem Soggetto, zweistimmig wie
die meisten späteren Punkte Palestrinas. Der Schluss auf D verbindet die Diskantklausel mit Vorhalt
im Sopran (D–C♯–D), die Tenorklausel E–D und die Bassklausel A–D; der Alt singt die große Terz F♯.
Denselben Text ohne Imitation zeigt Beispiel 08.

## 02 Christe, phrygisch, dreistimmig, mit Imitation

Mit Imitation. Text: Christe („Chri-ste e-lé-i-son.“). Modus: phrygisch. Stimmen: Alt, Tenor, Bass.
Seed 10, 27 Takte, Kosten 25.

```sh
fux free --mode phrygisch --voices 3 --text christe --seed 10
```

- Punkt 1 („Chri-ste e-lé-i-son.“): Alt T. 1 E4 · Tenor T. 5 A3 (Unterquinte) · Bass T. 9 E3
  (Unteroktave); Kopf 6–7 Töne, real; Binnenschluss T. 16 auf A
- Punkt 2 („e-lé-i-son.“): Tenor T. 16½ E4 · Bass T. 20½ E3 (Unteroktave); Kopf 7 Töne, real;
  Schluss T. 27 auf E

Der Alt beginnt mit E–G–F–E–F–E–C (Ganze, vier Halbe, zwei Ganze), einem Soggetto, das die Finalis
E umkreist, mit dem Halbton E–F über ihr, dem Kennzeichen des Phrygischen. Der Tenor antwortet auf
A, das im Phrygischen der Finalis E als zweite Hauptstufe antwortet, der Bass eine Oktave unter dem
Alt. Auch der Kopf von Punkt 2 beginnt mit dem Halbton, E–F–E im Tenor, und der Bass antwortet
eine Oktave tiefer. Binnenschluss auf A, Schluss auf E mit der phrygischen Klausel: Der Bass fällt
im Halbton F–E, der Alt steigt im Ganzton D–E, und der Tenor endet auf der großen Terz G♯.

## 03 Agnus Dei I und II, hypomixolydisch, fünfstimmig, mit Imitation

Mit Imitation, fünfstimmig und plagal. Text: `agnus`, Agnus Dei I und II („A-gnus De-i,
qui tol-lis pec-cá-ta mun-di: mi-se-ré-re no-bis.“). Modus: hypomixolydisch, der plagale Modus zur
Finalis G mit dem Ambitus D–d. Stimmen: Sopran, Alt, Tenor 1, Tenor 2, Bass. Seed 5, 43 Takte,
Kosten 126.

```sh
fux free --mode hypomixolydisch --voices 5 --text agnus --seed 5
```

- Punkt 1 („A-gnus De-i,“): Sopran T. 1 D5 · Tenor 1 T. 3½ G3 (Unterduodezime) · Bass T. 7½ D3
  (Unterdoppeloktave) · Tenor 2 T. 14½ D4 (Unteroktave) · Alt T. 18 G4 (Unterquinte); Kopf 5–9
  Töne, real; Binnenschluss T. 26 auf C
- Punkt 2 („qui tol-lis pec-cá-ta mun-di:“): Tenor 1 T. 26½ G3 · Alt T. 27½ D4 (Quinte); Kopf 5
  Töne, real; Binnenschluss T. 33 auf C
- Punkt 3 („mi-se-ré-re no-bis.“): Tenor 1 T. 34½ D4 · Alt T. 36½ G4 (Quarte); Kopf 7 Töne, real;
  Schluss T. 43 auf G

Jeder Sinnabschnitt des Textes ist ein Punkt mit eigenem Soggetto. Der Sopran eröffnet hoch auf der
Quinte D mit D–D–E–D–B (drei Ganze, zwei Halbe); Tenor 1 und Bass bringen das Soggetto auf G und D
in Synkopen, um eine Halbe gegen den Takt verschoben, dann folgen Tenor 2 auf D und der Alt auf G.
Punkt 2 und Punkt 3 sind Duos von Tenor 1 und Alt. Die Binnenschlüsse liegen auf C, der Schluss auf
G mit dem Leitton F♯ im Sopran und vollem Klang (G2, G3, B3, D4, G4).

Warum hypomixolydisch: Im Mixolydischen soll sich der Sopran in der Oktave G4–G5 bewegen
(FREE-AMBITUS), sein System reicht aber nur bis D5 (VOI-RANGE). In allen sechzehn mixolydischen
Kandidaten sank er bis B3, stieg in elf nicht über A4 und geriet in dreizehn zeitweise unter eine
tiefere Stimme, in acht unter den Alt; sie kosteten 182 bis 379 gegen 114 bis 199 im
Hypomixolydischen. Im plagalen Ambitus D–d, der in sein System passt, singt er C4–E5. Die Kosten
sind die höchsten der Beispiele 01–10 und verteilen sich auf 90 Posten, fast alle der Stufen P1 und
P2 (62 und 24, dazu vier der Stufe P3). Am meisten kosten die Bewegungsart in den Taktanfang
(bevorzugt ist ab drei Stimmen Gegenbewegung zum Bass; 24 Punkte) und der Binnenschluss in T. 33, an
dem Sopran, Tenor 2 und Bass noch mitten in „qui tollis peccata mundi“ stehen (12 Punkte; daher das
„…tol-lis“ in der Zeile „Binnenschlüsse“ der `.txt`-Datei). Seed 7 kostet weniger (114), aber sein
Soggetto hält das G drei Takte lang und geht nur zum F und zurück, und seine Stimmen setzen bis zu
acht Takte auseinander ein. Stimmkreuzungen kosten 3 Punkte, nur zwischen den beiden Tenören. Unter
den Beispielen 01–10 hat 03 die meisten Stimmen und die längste Rechenzeit.

## 04 Kyrie I über dem Choral Kyrie IX *Cum jubilo*, vierstimmig

Imitation mit Soggetti aus dem Choral (`--subject chant`). Text: Kyrie I. Modus: ohne `--mode` der
des Chorals, der 1. Modus (dorisch). Stimmen: Sopran, Alt, Tenor, Bass. Seed 14, 35 Takte,
Kosten 48.

```sh
fux free --voices 4 --text kyrie --subject chant --seed 14
```

- Punkt 1 („Ký-ri-e e-lé-i-son.“): Alt T. 1 D4 · Sopran T. 4½ A4 (Quinte) · Tenor T. 7½ A3
  (Unterquarte) · Bass T. 16½ D3 (Unteroktave); Kopf 4–5 Töne, real; Binnenschluss T. 23 auf A
- Punkt 2 („e-lé-i-son.“): Alt T. 23½ C4 · Tenor T. 26½ G3 (Unterquarte); Kopf 7 Töne, real;
  Schluss T. 35 auf D

Die Köpfe nehmen ihre Töne aus der ersten Anrufung des Kyrie IX *Cum jubilo* (Kyriale, Graduale
Romanum 1961), über das Palestrina seine *Missa de beata Virgine* schrieb. Punkt 1 singt den Anfang
des Chorals, D–F–G–A, im Alt in Ganzen; Sopran und Tenor antworten auf der Quinte mit A–C–D–E,
beide auf der Taktmitte, zuletzt bringt ihn der Bass auf D. Punkt 2 nimmt die Töne des
„e-lé-i-son“ derselben Anrufung, C–D–F–G–F, im Alt, als Duo mit dem Tenor, der sie eine Quarte
tiefer auf G beantwortet. Der Schluss auf D ist ein heller Klang ohne Quinte (D3, D4, F♯4, D5): Der
Sopran endet oben auf D5 nach dem Vorhalt D–C♯–D, der Alt auf der großen Terz F♯. Die Seeds 2 und 5
kosten wenig mehr, beginnen aber mit zwei Einsätzen auf demselben D, 13 wiederholt im zweiten Punkt
den ersten statt der Phrase des „e-lé-i-son“. Soggetti aus den Worten statt aus dem Choral zeigt
Beispiel 01.

## 05 Benedictus, hypoäolisch, vierstimmig, mit Imitation

Imitation in einem plagalen Modus: Im Hypoäolischen liegen die Stimmen eine Quarte
tiefer als im Äolischen; der Ambitus reicht von der Quarte unter der Finalis A bis zur Quinte
darüber (E–e). Text: Benedictus („Be-ne-dí-ctus qui ve-nit in nó-mi-ne Dó-mi-ni.“). Stimmen:
Sopran, Alt, Tenor, Bass. Seed 5, 31 Takte, Kosten 31.

```sh
fux free --mode hypoäolisch --voices 4 --text benedictus --seed 5
```

- Punkt 1 („Be-ne-dí-ctus qui ve-nit“): Sopran T. 1 A4 · Alt T. 5 E4 (Unterquarte) · Tenor T. 8 A3
  (Unteroktave) · Bass T. 15 E3 (Unterundezime); Kopf 6 Töne, real; Binnenschluss T. 22 auf C
- Punkt 2 („in nó-mi-ne Dó-mi-ni.“): Bass T. 23½ D3 · Alt T. 24½ A3 (Quinte); Kopf 4–7 Töne,
  real; Schluss T. 31 auf A

Der Sopran beginnt mit A–G–F–A–B–C (drei Ganze, zwei Halbe, Ganze): Er sinkt von der Finalis A und
steigt über sie zur Terz C. Alt und Bass antworten auf E, der anderen Hauptstufe, der Tenor auf A.
Der Text teilt sich in zwei Punkte mit eigenen Soggetti; der zweite beginnt als Duo von Bass und
Alt. Schluss auf A mit dem Leitton G♯ im Sopran und der großen Terz C♯ im Alt; Tenor und Bass
enden im Einklang auf A3. Unter den äolischen und hypoäolischen Kandidaten hatte dieser Satz die
geringsten Kosten.

## 06 Eigener Text „Ad te levavi“, dorisch, dreistimmig, mit Imitation

Text und Imitation: ein eigener lateinischer Text, der Anfang des Introitus vom ersten
Adventssonntag. Die Akzente in le-vá-vi und á-ni-mam legen die betonten Silben fest; Silben und
Wörter findet das Programm selbst (`fux text "Ad te levavi animam meam."`). Modus: dorisch.
Stimmen: Alt, Tenor, Bass. Seed 6, 14 Takte, Kosten 7.

```sh
fux free --mode dorisch --voices 3 --text "Ad te le-vá-vi á-ni-mam me-am." --seed 6
```

- Punkt 1 („Ad te le-vá-vi á-ni-mam me-am.“): Tenor T. 1 D3 · Bass T. 3 A2 (Unterquarte) · Alt
  T. 6 D4 (Oktave); Kopf 5–7 Töne, real; Schluss T. 14 auf D

Der kurze Text ist ein einziger Punkt, und jede Stimme singt ihn einmal. Das Soggetto deklamiert
„Ad te le-vá-“ Silbe für Silbe bis zum Akzent, wie Palestrina diese Worte nach Jeppesen setzt:
D–F–E–A–G (Ganze, Halbe, Ganze, Halbe, Halbe), dann folgt ein Melisma auf der betonten Silbe. Bass
und Alt antworten auf A und D; im Schlussklang singt der Tenor die große Terz F♯. Der kürzeste und
billigste der freien Sätze; vier Kandidaten kosteten einen Punkt weniger, beginnen aber in
Synkopen mit zwei Stimmen eine Halbe nacheinander, wiederholen im Soggetto den Ton F oder bringen
die dritte Stimme erst sechs Takte später.

## 07 Gloria „Et in terra … Glorificamus te“, ionisch, vierstimmig, mit Imitation

Imitation mit einem langen Text: die Sinnabschnitte 1–5 des Gloria (`gloria:1-5`, „Et in ter-ra pax
ho-mí-ni-bus bo-nae vo-lun-tá-tis. Lau-dá-mus te. Be-ne-dí-ci-mus te. Ad-o-rá-mus te.
Glo-ri-fi-cá-mus te.“). Modus: ionisch. Stimmen: Sopran, Alt, Tenor, Bass. Seed 6, 43 Takte,
Kosten 52.

```sh
fux free --mode ionisch --voices 4 --text gloria:1-5 --seed 6
```

- Punkt 1 („Et in ter-ra pax ho-mí-ni-bus bo-nae vo-lun-tá-tis.“): Tenor T. 1 F3 · Bass T. 2 C3
  (Unterquarte) · Sopran T. 7 F4 (Oktave) · Alt T. 8 C4 (Quinte); Kopf 4–5 Töne, real;
  Binnenschluss T. 17 auf G
- Punkt 2 („Lau-dá-mus te. Be-ne-dí-ci-mus te.“): Tenor T. 18½ G3 · Alt T. 21½ C4 (Quarte) ·
  Sopran T. 24½ G4 (Oktave); Kopf 4–5 Töne, real; Binnenschluss T. 33 auf E
- Punkt 3 („Ad-o-rá-mus te. Glo-ri-fi-cá-mus te.“): Alt T. 33½ C4 · Tenor T. 34½ G3 (Unterquarte);
  Kopf 6 Töne, real; Schluss T. 43 auf C

Die fünf Sätze des Textes werden zu drei Punkten. Punkt 1 beginnt im Tenor auf der Quarte F mit
F–A–G–G (zwei Ganze, zwei Halbe; „Et in ter-ra“), der Bass antwortet einen Takt später auf der
Finalis mit C–E–D–D, Sopran und Alt folgen eine Oktave höher. Gloria und Credo wiederholen ihren
Text selten (`--repeat auto`): Jede Stimme singt die fünf Sätze einmal. Binnenschlüsse auf G und
E, Schluss auf C mit der großen Terz E im Alt. Den ersten Binnenschluss (T. 17) singen Tenor und
Bass am Ende von „vo-lun-tá-tis.“, der Sopran aber schon auf „vo-“; dieser Satz kostet trotzdem
weit weniger als die übrigen Kandidaten (52 gegen 75 bis 114).

## 08 Kyrie I ohne Imitation, dorisch, vierstimmig (zum Vergleich mit 01)

Freier Satz mit Text, ohne Imitation. Derselbe Text im selben Modus wie 01,
aber mit `--imitation aus`: Die Suche setzt alle Stimmen frei, ohne geplante Einsätze.
Stimmen: Sopran, Alt, Tenor, Bass. Seed 4, wie bei 01 aus den Seeds 1–10 gewählt;
16 Takte, Kosten 33.

```sh
fux free --mode dorisch --voices 4 --text kyrie --imitation aus --seed 4
```

- Keine Punkte: Sopran und Bass beginnen in Takt 1, Alt und Tenor eine Halbe später, jede Stimme
  mit eigener Melodie.
- Binnenschlüsse: Takt 7 auf A („…e-lé-i-son.“); Schluss T. 16 auf D, mit der großen Terz F♯ im Alt.

Der Satz hat die automatische Länge des Textes, 16 Takte (`fux text kyrie`); jede Stimme singt den
Text zweimal (Tenor und Bass beim zweiten Mal nur „e-lé-i-son.“), mit dem Binnenschluss
dazwischen. Neben Beispiel 01 (25 Takte) hört man, was der Plan der Imitation ändert: ein
gemeinsames Soggetto statt freier Anfänge, gestaffelte Einsätze und ein längerer Satz. Seed 8
kostet weniger (29), aber sein Sopran beginnt das „Ký-“ mit Vierteln im Einklang mit dem Alt.

## 09 Fux: fünfte Gattung, zweistimmig, über dem dorischen Cantus

Gattungskontrapunkt nach Fux: blühender Kontrapunkt über Fux' dorischem Cantus firmus
(`fux:dorian`: D F E D G F A G F E D, im Alt), die Gegenstimme im Sopran. Seed 4, 11 Takte,
Kosten 4.

```sh
fux cp --cf fux:dorian --species 5 --seed 4
```

Gegenstimme: `r D5~ | D5 C5 B4 A4 | C5 B4 C5~ | C5 B4 A4 B4~ | B4 C5 D5~ | D5 C5 B4 C5~ | C5 D5 E5~ | E5 D5~ | D5 C5 D5~ | D5 C#5 | D5`

Sie beginnt nach einer halben Pause mit einer Synkope und mischt Viertel wie in der 3. Gattung
(T. 2), Synkopen wie in der 4. und Achtelpaare (T. 4 und 6), in Takt 4 als verzierte Auflösung des
Septimvorhalts C–B über dem D des Cantus. Der Schluss hat den Vorhalt D–C♯ über dem E des Cantus
und endet in der Oktave. Alle zehn Seeds kosteten 4; dieser mischt die Elemente am meisten.

## 10 Fux: fünfte Gattung, vierstimmig, Cantus im Tenor

Fux' Satz für drei und mehr Stimmen: Eine Stimme steht in der Gattung, die übrigen in
Ganzen wie in der 1. Gattung. Fux' äolischer Cantus (`fux:aeolian`: A C B D C E F E D C B A) liegt
im Tenor, der Sopran singt die 5. Gattung, Alt und Bass Ganze. Seed 4, 12 Takte, Kosten 36.

```sh
fux cp --cf fux:aeolian --voices 4 --cf-voice T --species 5 --voice S --seed 4
```

- Sopran (5. Gattung): `r A4~ | A4 G4 F4~ | F4 G4 A4 B4~ | B4 C5 D5~ | D5 C5 B4 C5~ | C5 D5 E5~ | E5 D5~ | D5 C5 B4~ | B4 A4 G4 F4~ | F4 E4 E5~ | E5 B4 | C#5`
- Alt: `E4 | F4 | D4 | G4 | E4 | A4 | A4 | G4 | F4 | E4 | G#4 | A4`
- Tenor (Cantus): `A3 C4 B3 D4 C4 E4 F4 E4 D4 C4 B3 A3`
- Bass: `A3 | A2 | D3 | B2 | C3 | C3 | D3 | E3 | F3 | A3 | E3 | A2`

Im Schluss singt der Alt den Leitton G♯–A, der Cantus im Tenor B–A und der Bass E–A; der Sopran
endet auf der großen Terz C♯, denn eine kleine Terz im Schlussklang lässt Fux nicht zu. Die Kosten
(36 gegen 4 im zweistimmigen Satz) kommen aus vielen kleinen Posten der Stufen P1 und P2: Sprünge,
Bewegung in den Taktanfang, verdeckte Quinten und Oktaven, Rhythmen der 5. Gattung, die bei Fux
seltener sind. Von Fux' sechs Cantus liegen der äolische, der lydische und der mixolydische
unverändert im Tenor; der dorische, ionische und phrygische klingen dort eine Oktave tiefer, der
phrygische bis hinab zum A2. Unter jenen dreien kostete dieser Satz am wenigsten.

## 11 Missa brevis, ionisch-transponiert, vierstimmig

Eine ganze Messe, gesetzt mit `fux mass` in der Form brevis nach Palestrinas *Missa
brevis* (1570). Wie diese steht sie im transponierten Ionischen auf F, mit B♭ als Vorzeichen;
Hauptstufe ist C. Stimmen: Sopran, Alt, Tenor, Bass; das Benedictus singen Sopran, Alt und Tenor,
das Agnus Dei II fünf Stimmen mit einem zweiten Tenor. Seed 2, 26 Teile, 907 Takte, Kosten 3149,
keine Warnung. Die Formen, Besetzungen und Regeln der Messe beschreibt
[§ 19 des Regelkatalogs](../docs/regeln.md#19-messe).

```sh
fux mass --mode ionisch-transponiert --seed 2 --details --jobs 4
```

Der Plan (→ Schlussstufe des Teils; ‖ neuer Abschnitt, \| Halteklang, · Naht):

| Satz | Takte | Teile |
|---|---|---|
| Kyrie | 90 | Kyrie I → F ‖ Christe → C ‖ Kyrie II → F, alle in Imitation |
| Gloria | 216 | gloria:1-5 (Imitation) → F · gloria:6-9 (gemeinsam) → C · gloria:10-13 (Imitation) → F \| Qui tollis (gemeinsam) → C · qui-tollis:5-8 (Imitation) → C · qui-tollis:9-13 (gemeinsam) → F |
| Credo | 353 | credo:1-4 (Imitation) → F · credo:5-7 → F · credo:8-14 → C · credo:15-16 → F \| Et incarnatus est → C \| Crucifixus → F · resurrexit:1-4 → C · resurrexit:5-7 → F · spiritum:1-4 → C · spiritum:5-7 → F · spiritum:8-9 → C · spiritum:10-12 → F, alle nach dem ersten gemeinsam |
| Sanctus | 65 | Sanctus, Pleni und Osanna in einem Teil (Imitation) → F |
| Benedictus | 68 | Benedictus, S A T (Imitation) → C ‖ Osanna (gemeinsam) → F |
| Agnus Dei | 115 | Agnus Dei I (Imitation) → F ‖ Agnus Dei II, S A T T B (Imitation) → F |

Jeder Teil ist ein freier Satz mit Text wie die Beispiele 01–08, für sich gesucht und geprüft: 11
beginnen in Imitation, 15 gemeinsam. Ein gemeinsamer Neubeginn setzt alle Stimmen als Ganze auf
der ersten Silbe auf den vollen Klang der Finalis (Sopran F4, Alt C4, Tenor A3, Bass F3).

**Kyrie.** Das Soggetto des Kyrie I steigt von der Finalis zur Terz und umspielt sie, F–F–A–A–G–B♭–A
(Ganze, zwei Halbe, dann Ganze). Der Alt beginnt es auf F3, der Sopran antwortet in der Oktave,
Bass und Tenor auf der Quinte C, beide auf der Taktmitte. Das Christe beginnt im Tenor auf C und
schließt auf C, das Kyrie II beginnt im Bass auf B♭ mit der Antwort auf F.

**Gloria und Credo** beginnen nach der Intonation des Priesters, die im Buch unter dem Namen des
Satzes steht, mit „Et in terra pax“ und „Patrem omnipotentem“ in Imitation. Sie haben 6 und 12
Teile. An den Halteklängen *Qui tollis*, *Et incarnatus est* und *Crucifixus* halten alle Stimmen
den Schlussklang einen Takt lang, schweigen einen Takt und beginnen gemeinsam neu, wie in der
Missa brevis; an den übrigen Schnitten, den Nähten, atmen sie nach dem gehaltenen Klang nur eine
Halbe. Von den 16 Teilen nach einem Halteklang oder einer Naht beginnen 2 in Imitation
(gloria:10-13 und qui-tollis:5-8).

**Sanctus bis Agnus Dei.** Sanctus, Pleni und Osanna sind ein Teil mit fünf Punkten: „San-ctus,
San-ctus, San-ctus“, „Dó-mi-nus De-us Sá-ba-oth.“, „Ple-ni sunt cae-li et ter-ra“, „gló-ri-a
tu-a.“ und „Ho-sán-na in ex-cél-sis.“. Das Benedictus singt seinen Text in vier Punkten zweimal
(`--lengths palestrina`: zwei Punkte mehr, als die Imitation selbst plant) und schließt auf C;
das Osanna danach beginnt gemeinsam in allen vier Stimmen, wie das Osanna II der Missa brevis.
Agnus Dei I hat fünf Punkte, auch hier zwei mehr, als die Imitation selbst plant. Im Agnus Dei II
setzen Sopran, Alt, Tenor 1, Bass und Tenor 2 nacheinander auf C und F ein; „do-na no-bis
pa-cem.“ singen Bass, Tenor 1 und Alt als letzten Punkt.

**Kadenzplan.** Die Teile schließen auf F oder C, im Credo vom zweiten Teil an von Teil zu Teil
abwechselnd. Die 63 Binnenschlüsse liegen auf F (27), C (21), B♭ (8), D (6) und A (1), nahe bei
Palestrinas Messen im Ionischen auf F (F 44, C 36, B♭ 8, A 5, D 4, G 4 %). Höchstens drei
Schlüsse nacheinander liegen auf derselben Stufe, außer am Ende des Sanctus (viermal F). In der
`.txt`-Datei steht der Kadenzplan jedes Satzes mit Takten durch den Satz.

**Prüfen.** `fux check --musicxml examples/11-…-3-credo.musicxml` prüft das Credo Teil für Teil,
mit Text, Binnenschlüssen und Plan jedes Teils aus den Feldern der Datei, und nennt jeden Verstoß
mit dem Takt des Satzes. Für die sechs Sätze ergibt die Prüfung die Kosten des Berichts (Kyrie
220, Gloria 755, Credo 1390, Sanctus 154, Benedictus 202, Agnus Dei 428) und kein Verbot.

**Auswahl.** Die Seeds 1–5 setzten jeder alle 26 Teile, ohne Warnung (5.11.2026):

| Seed | Takte | Kosten | Kyrie I beginnt auf | längste Folge von Schlüssen auf einer Stufe | Zeit von `fux` |
|---|---|---|---|---|---|
| 1 | 990 | 3133 | B♭ (Alt) | 4 (Credo, F) | 2:38 |
| 2 | 907 | 3149 | F (Alt) | 4 (Ende des Sanctus, F) | 1:58 |
| 3 | 878 | 3139 | B♭ (Alt) | 5 (Credo, C) | 2:15 |
| 4 | 902 | 3249 | C (Tenor) | 4 (Credo, C) | 3:25 |
| 5 | 882 | 2876 | C (Tenor) | 5 (Credo, C) | 2:07 |

Seed 2 hat den abwechslungsreichsten Kadenzplan: Seine Teile schließen im Credo vom zweiten an
abwechselnd auf F und C, und nur am Ende des Sanctus folgen mehr als drei Schlüsse auf derselben
Stufe. Seed 5 kostet am wenigsten und beginnt fast jeden Abschnitt auf F oder C, schließt aber in
der Mitte des Credo achtmal in neun Schlüssen auf C (T. 110–163) und im Agnus Dei I viermal
nacheinander. Die Seeds 1 und 3 beginnen die Messe auf B♭, der Quarte; 4 kostet am meisten. Die
Kandidaten liefen einer nach dem anderen auf einer sonst ruhigen Maschine mit acht Kernen, jeder
mit vier Teilen zugleich (`--jobs 4`); die Zeit ist die Wanduhr von `fux` allein, für Seed 2 der
Median dreier gleicher Läufe. Die Suchzeit
im Bericht ist mit vier Teilen zugleich die Summe der Teile, hier das 3,4- bis 4,0-Fache. Das Skript
brauchte für die Messe 128 s, davon etwa 10 s für Buch und Sätze mit LilyPond und MuseScore (so
lange braucht `--nur-notenbild 11`). Einen Teil nach dem anderen braucht Beispiel 11 151 bis 155 s
([regeln.md § 19](../docs/regeln.md#messe-erzeugen)).

## Grenzen

Die Beispiele zeigen das Programm vom 5.11.2026 mit seinen Schwächen. Diese Stellen
sind keine Druckfehler:

- **Musica ficta.** Im freien Satz ist das erniedrigte Mi (B♭) in allen Modi erlaubt und kostet
  nichts, wie bei Palestrina; das Programm setzt es gegen Tritonus und übermäßige Intervalle und
  auf der oberen Nebennote des A („una nota super la semper est canendum fa“). So wechseln B und B♭
  mitunter innerhalb von ein, zwei Takten, in einer Stimme und zwischen den Stimmen, etwa im Alt
  von 01 (T. 21–23), im Bass von 06 (T. 6–8), in Sopran und Tenor von 04 (T. 12–16) und im Tenor
  von 08 (T. 3–8). Die Regel gegen den Querstand (FIC-CROSS-RELATION) vergleicht nur unmittelbar
  aufeinanderfolgende Klänge und erfasst diese Fälle nicht. In 01 klingt in T. 16 auf dem dritten
  Viertel sogar der Durchgang C4 des Alts gegen das G♯4 des Soprans (mi contra fa), was das Programm
  nicht beanstandet.

Die Messe (11) setzt Palestrinas Formen um, aber nicht seine Art, lange Texte zu setzen. Ihre
Grenzen beschreibt der Regelkatalog in
[§ 19](../docs/regeln.md#grenzen-der-messe-und-abweichungen-von-palestrina); an diesem Beispiel,
verglichen mit Palestrinas Missa brevis von 1570:

- **Nähte statt durchkomponierter Abschnitte.** Gloria und Credo bestehen aus 6 und 12 Teilen, die
  einzeln gesucht werden. An jedem der 16 Übergänge halten alle Stimmen einen Klang und schweigen
  danach, an den 13 Nähten eine Halbe, an den drei Halteklängen einen Takt; die Missa brevis hält
  nur an diesen drei. Einen langen Abschnitt durchzukomponieren, verlangte, die Suche fortzusetzen,
  und dafür zuerst eine Prüfung, die nicht bei jedem Schritt die ganze Partitur liest.
- **Langsame, dichte Deklamation.** Gloria und Credo brauchen 1,04 Takte je Silbe (Gloria 1,16,
  Credo 0,98) gegen 0,82 in der Missa brevis, und jede Stimme singt 1,51 Töne je Silbe gegen etwa
  einen (Missa brevis: 1,04 im Gloria, 0,98 im Credo).
- **Keine Homophonie und keine Auslassungen.** Jede Stimme singt jedes Wort, und gemeinsam
  deklamieren die Stimmen nur am Anfang eines gemeinsam beginnenden Teils. In der Missa brevis
  singen manche Stimmen nicht den ganzen Text, und viele Silben beginnen in allen Stimmen zugleich.
- **Kadenzen.** Jeder Binnenschluss singt seine Formel vollständig; Palestrina umgeht etwa ein
  Drittel seiner Binnenschlüsse. Die Klauseln singen immer die Oberstimme, die Stimme über dem Bass
  und der Bass; Palestrinas Diskantklausel liegt vierstimmig nur in einem Drittel der Kadenzen im
  Cantus.
- **Kein Kanon, kein Dreiertakt.** Das Agnus Dei II bekommt einen freien zweiten Tenor; die Missa
  brevis fügt dort einen zweiten Cantus im Kanon hinzu. Alle Teile stehen in 2/2.
- **Längen.** Gloria und Credo sind länger als in der Missa brevis, die übrigen Sätze kürzer, in
  Takten zu 2/2 (Missa brevis in Klammern): Kyrie 90 (118), Gloria 216 (156), Credo 353 (292),
  Sanctus 65 (96), Benedictus mit Osanna 68 (100), Agnus Dei 115 (208).
- **Notenbild der Messe.** Die Takte sind halb so lang wie in Palestrinas Gesamtausgabe (2/2 statt
  4/2). Die Intonation steht als Text da, nicht in Choralnoten, kein Abschnitt hat ein Incipit mit
  den alten Schlüsseln und Werten, und das PDF hat keine Lesezeichen.

## Lizenz

Die Beispiele in diesem Verzeichnis und in [fugen/](fugen/README.md) sind vom Programm erzeugt.
Soweit an ihnen Rechte bestehen, sind sie der Allgemeinheit gewidmet:
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/deed.de). Die Cantus firmi von Fux, die
Choralmelodien, die lateinischen Texte und die Themen von Bach und Händel sind gemeinfrei.
