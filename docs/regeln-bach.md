# Regelkatalog des Profils `bach`

Der Katalog ist die Spezifikation der Regelmaschine in `crates/fux-bach` (`rules.rs`,
`engine.rs`). Er steht neben dem [Regelkatalog](regeln.md) des Fux-Programms und folgt dessen
Form: jede Regel hat eine stabile Kennung, eine Stufe und eine Meldung auf Deutsch und Englisch,
und jede ist einstellbar. Die Zahlen lassen sich mit den Korpora nachrechnen, die
`tools/fetch_bach.sh` lädt:

```sh
cargo run --release -p fux-bach --bin bach-probe -- b1-census      # Zählung je Gelegenheit
cargo run --release -p fux-bach --bin bach-probe -- b1-originals   # Verbote, die Bach bricht
```

## Was anders ist als bei Fux

- **Der Akkord ist eine Eingabe.** Jede Regel liest einen Ton gegen den Akkord seines Augenblicks.
  Woher die Akkorde kommen, entscheidet der Aufrufer: aus einer Analyse (RomanText), aus einer
  Akkordzeile je Takt oder, als Notbehelf, aus der Schätzung in `chord.rs`.
- **Zeit in Ticks.** 96 Ticks sind eine Ganze; Zweiunddreißigstel und Triolen haben ihren Ort. Die
  Maschine geht die Augenblicke durch, in denen eine Stimme anschlägt, verstummt oder der Akkord
  wechselt.
- **Ein Übergang für Suche und Prüfung.** `fux-fuge check` und die Suche rufen dieselben vier
  Funktionen: `hold` (ein liegender Ton unter neuem Akkord), `place` (ein Ton in seiner Stimme),
  `pair` (zwei Stimmen), `sound` (der ganze Klang auf einem Schlag).

## Stufen

| Kürzel | Bedeutung |
|---|---|
| V | Verbot. Ein gesuchter Ton, der es bräche, wird nicht geschrieben. Ein gegebener Ton bricht es mit Meldung. |
| P1 / P2 / P3 / P4 | Präferenz mit Kosten 1 / 2 / 4 / 8 je Verstoß, wie im Fux-Katalog |
| P(*n*) | Präferenz mit Kosten *n* |
| frei | Ausdrücklich erlaubt; die Regel wird gezählt, kostet aber nichts |

Die Einheit ist halb so groß wie im Profil `ausgang`, mit dem die Eichung begann: ein um eine
Klasse verfehlter Intervallwunsch kostet 4 (siehe [Preise des Plans](#preise-des-plans)). So gibt
es zwischen „frei“ und dem P1 von `ausgang` eine Stufe.

## Wer einen Verstoß trägt

Die Maschine unterscheidet gesuchte und gegebene Töne (Themeneinsätze, Kontrasubjekte, alles in
einem zu prüfenden Satz):

- Ein **gesuchter** Ton bricht kein Verbot; die Suche verwirft ihn.
- Ein **gegebener** Ton wird gelesen, nie verworfen. Bricht er ein Verbot nach einem gegebenen Ton
  oder einer Pause, ist das ein Verstoß der gegebenen Linie selbst (`own`). Bricht er es nach einem
  gesuchten Ton, hat die gesuchte Stimme nicht in den Einsatz geführt: eine **Naht** (`junction`).
  Ein Satz mit Nahtfehler gilt nicht als sauber; die Suche zieht dann einen neuen Rhythmus.
- Löst ein gesuchter Ton den fremden gegebenen Ton vor ihm nicht auf, ist das **entschuldigt**
  (`excused`): im Neusatz war der gegebene Ton schon gegen seine Akkorde nicht erklärt. Im
  Komponisten gilt die Entschuldigung nur, wo kein Ton auflösen kann; die Prüfung der fertigen
  Fuge meldet sie als Verbot.

In `fux-fuge check` ist jeder Ton gegeben; der Bericht nennt jeden Verstoß mit Takt, Schlag,
Stimmen, Kennung, Stufe und Meldung und endet wie `fux check` mit „*n* Verbot(e), Kosten *k* aus
*m* Präferenz(en)“ (Rückgabewert 2 bei Verboten).

## Leitlinie und Eichung

Die Spalte **Leitlinie** gibt die Stufe, die der [Leitlinie des Fux-Katalogs](regeln.md#leitlinie-für-standardwerte)
nach Bachs Häufigkeit entspräche: nie V, bis etwa 2 % P3, 2 bis 10 % P2, darüber P1 oder frei. Die
Spalte **Stufe** ist die geeichte: an elf Entwicklungsfugen so gesetzt, dass die Suche eine
Regel etwa so oft bricht wie Bach in denselben Fugen. Wo beide auseinandergehen, steht der Grund unter der Tabelle.

Gezählt ist an den 48 Fugen des WTK gegen geschätzte Akkorde (Gelegenheiten, Verstöße, Anteil),
an den sieben Fugen mit Tymoczkos Analyse gegen dessen Akkorde, und an 19 Stücken der Kunst der
Fuge gegen geschätzte Akkorde. Die Kunst der Fuge ist das zweite Korpus: an ihr wurde nichts
eingestellt. Die Schätzung erklärt betonte Dissonanzen weg und erfindet dafür andere Verstöße;
wo die Spalte „Tymoczko“ stark abweicht, ist sie die verlässlichere.

Ein **Verbot** steht nur, wo die Suche es braucht. Seine Ausnahmen bei Bach sind gezählt, und ein
Test (`b1::tests::bach_breaks_the_counted_prohibitions`) hält die Zahlen fest: MEL-LEAP-BIG 87,
FIG-UNRESOLVED 333, FIG-TOO-LONG 4, FIG-BEFORE-REST 14, MOT-PARALLEL 34, HAR-CLASH 68.

## Melodik

Das Intervall von einem Ton einer Stimme zum nächsten, ohne Pause dazwischen. Die Größe des Intervalls ist kein Gegenstand einer Regel: sie wird als Wunsch aus den Tabellen gezogen (siehe Preise des Plans).

| Kennung | Stufe | Regel | Gelegenheit | WTK: Gel. | Verst. | Anteil | 7 Fugen, Tymoczko | KdF | Leitlinie |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| `MEL-REPEAT` | P(3) | Ein Ton wird wiederholt. | jedes Intervall einer Stimme | 48495 | 1790 | 3,69 % | 4,43 % | 3,09 % | P2 |
| `MEL-CHROMATIC` | P4 | Chromatischer Halbtonschritt (gleicher Stammton). | jedes Intervall | 48495 | 314 | 0,65 % | 0,39 % | 1,17 % | P3 |
| `MEL-LEAP-BIG` | V | Sprung über die Oktave hinaus. | jedes Intervall | 48495 | 87 | 0,18 % | 0,11 % | 0,08 % | P3 |
| `MEL-ALTERED-OUT` | P4 | Übermäßiger oder verminderter Sprung, dessen Töne nicht beide zu einem Dominantklang (Dominantsept, vermindert, halbvermindert) gehören. | jedes Intervall | 48495 | 424 | 0,87 % | 0,50 % | 0,77 % | P3 |
| `MEL-ALTERED-IN` | frei | Übermäßiger oder verminderter Sprung zwischen zwei Tönen eines Dominantklangs. | jedes Intervall | 48495 | 392 | 0,81 % | 0,84 % | 0,56 % | P3 |
| `MEL-SEVENTH-OUT` | P2 | Septsprung, dessen Töne nicht beide Akkordtöne sind. | jedes Intervall | 48495 | 291 | 0,60 % | 0,67 % | 0,25 % | P3 |
| `MEL-SEVENTH-IN` | frei | Septsprung zwischen zwei Akkordtönen. | jedes Intervall | 48495 | 160 | 0,33 % | 0,61 % | 0,10 % | P3 |
| `MEL-LEAPS-SAME` | P4 | Zwei Sprünge nacheinander in gleicher Richtung, deren Töne nicht alle zum Akkord gehören. | jeder Sprung nach einem Sprung | 5887 | 289 | 4,91 % | 3,38 % | 5,40 % | P2 |
| `MEL-ARPEGGIO` | frei | Zwei Sprünge nacheinander in gleicher Richtung im Akkord. | jeder Sprung nach einem Sprung | 5887 | 1140 | 19,36 % | 14,27 % | 23,05 % | frei |
| `MEL-RECOVERY` | frei | Nach einem Sprung ab der Quarte folgt kein Schritt in die Gegenrichtung. | jeder Ton nach einem solchen Sprung | 7021 | 4439 | 63,22 % | 72,10 % | 59,58 % | frei |
| `MEL-BACK` | frei | Die Stimme kehrt zum vorletzten Ton zurück. | jeder Ton mit zwei Vorgängern | 44781 | 8107 | 18,10 % | 19,70 % | 19,05 % | frei |
| `MEL-TRILL` | P1 | Die Stimme pendelt weiter zwischen denselben zwei Tönen. | jeder Ton mit zwei Vorgängern | 44781 | 1230 | 2,75 % | 4,80 % | 3,21 % | P2 |
| `MEL-ENTRY-FAR` | P2 | Nach einer Pause setzt die Stimme mehr als eine Oktave vom letzten Ton wieder ein. | jeder Einsatz nach einer Pause | 2050 | 98 | 4,78 % | 9,32 % | 4,52 % | P2 |
| `MEL-ENTRY-WIDE` | frei | Nach einer Pause setzt die Stimme mehr als eine Quinte vom letzten Ton wieder ein. | jeder Einsatz nach einer Pause | 2050 | 260 | 12,68 % | 8,47 % | 18,26 % | frei |
| `MEL-LEADING` | P4 | Der Leitton einer Dominante steigt nicht um einen Schritt, wenn der Akkord wechselt. | jeder Leitton als Akkordton vor einem Wechsel zu einem Nicht-Dominantklang | 1736 | 657 | 37,85 % | 27,80 % | 22,65 % | frei |

- `MEL-REPEAT`: An der Rate geeicht: frei wiederholt die Suche 10 % der Töne, bei P(3) 2,8 % (Bach in denselben Fugen 2,9 %).
- `MEL-CHROMATIC`: Rate der Suche unter Bachs.
- `MEL-LEAP-BIG`: Verbot als Festlegung: die Suche bietet keinen Ton jenseits der Oktave an. Bachs 87 Fälle sind gezählt, nicht einzeln durchgesehen.
- `MEL-ALTERED-OUT`: Im Profil `ausgang` ein Verbot; Bach tut es in 0,9 % der Intervalle (0,5 % gegen Tymoczkos Akkorde).
- `MEL-ALTERED-IN`: Frei: die Suche schreibt ihn halb so oft wie Bach.
- `MEL-SEVENTH-OUT`: Im Profil `ausgang` ein Verbot. An der Rate geeicht (0,8 % gegen Bachs 0,9 %).
- `MEL-SEVENTH-IN`: Frei: die Suche schreibt ihn seltener als Bach.
- `MEL-LEAPS-SAME`: Rate geeicht (3,3 % gegen 3,2 %).
- `MEL-ARPEGGIO`: Bach bricht den Akkord in jedem fünften Fall so.
- `MEL-RECOVERY`: Die Schulregel gilt bei Bach in zwei von drei Fällen nicht.
- `MEL-BACK`: 18 % bei Bach; frei trifft die Suche dieselbe Rate.
- `MEL-TRILL`: Rate geeicht (1,4 % gegen 1,8 %).
- `MEL-ENTRY-FAR`: Im Profil `ausgang` ein Verbot.
- `MEL-ENTRY-WIDE`: Bach tut es in jedem achten Einsatz.
- `MEL-LEADING`: An der Rate nicht zu eichen: der Anteil der Suche (44 %) bewegt sich mit dem Preis kaum und liegt über Bachs in denselben Fugen (34 %). Der Preis ist der des Profils `ausgang`.

## Akkordfremde Töne

Ein Ton, der nicht zum Akkord seines Augenblicks gehört, braucht eine Figur. Die Figuren ohne eigene Regel sind frei: Durchgang (auch zwei nacheinander), Wechselnote, vorbereiteter Vorhalt mit Auflösung abwärts, liegender Ton, unter dem der Akkord wiederkehrt, Orgelpunkt.

| Kennung | Stufe | Regel | Gelegenheit | WTK: Gel. | Verst. | Anteil | 7 Fugen, Tymoczko | KdF | Leitlinie |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| `FIG-UNRESOLVED` | V | Ein akkordfremder Ton, den keine Figur erklärt: weder Durchgang (auch zwei nacheinander), Wechselnote, Wechselnotengruppe, Vorhalt, Vorschlag, abspringende Nebennote, Cambiata, Antizipation noch verspäteter Wechsel. | jeder akkordfremde Ton, dem ein Ton folgt | 9241 | 333 | 3,60 % | 4,62 % | 4,77 % | P2 |
| `FIG-ACCENTED` | P(6) | Ein schrittweise erreichter akkordfremder Ton fällt auf einen Schlag; auf schwerem Schlag kostet er das Anderthalbfache. | jeder schrittweise erreichte Ton auf einem Schlag | 12828 | 186 | 1,45 % | 6,42 % | 4,43 % | P3 |
| `FIG-APPOGGIATURA` | P3 | Vorschlag: ein akkordfremder Ton im Sprung oder nach einer Pause, auf Schlag oder Schlaghälfte. | jeder so erreichte Ton auf Schlag oder Schlaghälfte | 11996 | 83 | 0,69 % | 3,02 % | 1,83 % | P3 |
| `FIG-APPOGGIATURA-WEAK` | P4 | Vorschlag abseits von Schlag und Schlaghälfte. | jeder im Sprung oder nach einer Pause erreichte Ton auf leichtester Zeit | 3586 | 285 | 7,95 % | 12,04 % | 8,05 % | P2 |
| `FIG-RESTRUCK` | P2 | Ein akkordfremder Ton wird angeschlagen, der schon klang (wiederholter Vorhalt). | jede Tonwiederholung | 1790 | 71 | 3,97 % | 14,78 % | 8,93 % | P2 |
| `FIG-SUSPENSION-UP` | P(16) | Ein Vorhalt löst sich aufwärts auf. | jeder stufenweise aufgelöste Vorhalt | 545 | 69 | 12,66 % | 0,50 % | 22,39 % | P3 |
| `FIG-GROUP` | P4 | Wechselnotengruppe: obere und untere Nebennote nacheinander. | jeder akkordfremde Ton, dem ein Ton folgt | 9241 | 35 | 0,38 % | 1,22 % | 0,36 % | P3 |
| `FIG-ESCAPE` | P(10) | Abspringende Nebennote oder Cambiata. | jeder akkordfremde Ton, dem ein Ton folgt | 9241 | 245 | 2,65 % | 1,63 % | 1,51 % | P2 |
| `FIG-ANTICIPATION` | P2 | Antizipation: der fremde Ton wird als Ton des nächsten Akkords wiederholt. | jeder akkordfremde Ton, dem ein Ton folgt | 9241 | 103 | 1,11 % | 1,07 % | 0,28 % | P3 |
| `FIG-DELAYED` | P2 | Verspäteter Wechsel: ein Ton des alten Akkords liegt höchstens einen Schlag in den neuen hinein und springt dann in einen Akkordton. | jeder akkordfremde Ton, dem ein Ton folgt | 9241 | 126 | 1,36 % | 1,42 % | 1,27 % | P3 |
| `FIG-LONG` | P4 | Ein akkordfremder Ton dauert länger als einen Schlag. | jeder akkordfremd angeschlagene Ton | 8612 | 13 | 0,15 % | 0,11 % | 0,86 % | P3 |
| `FIG-TOO-LONG` | V | Ein akkordfremder Ton dauert länger als zwei Schläge. | jeder akkordfremd angeschlagene Ton | 8612 | 4 | 0,05 % | 0,06 % | 0,29 % | P3 |
| `FIG-BEFORE-REST` | V | Ein Ton ist an seinem Ende akkordfremd, und danach schweigt die Stimme. | jeder Ton vor einer Pause oder am Ende | 2185 | 14 | 0,64 % | 0,80 % | 1,99 % | P3 |
| `FIG-LYING` | P3 | Ein liegender akkordfremder Ton bleibt über einen weiteren Akkordwechsel fremd. | jeder liegende fremde Ton bei einem Akkordwechsel | 160 | 14 | 8,75 % | 18,18 % | 20,57 % | P2 |

- `FIG-UNRESOLVED`: Verbot: die Figuren sind das, was einen fremden Ton zu einem Ton des Stils macht. Bachs 333 Ausnahmen (3,6 %) sind gezählt, nicht einzeln durchgesehen; gegen Tymoczkos Akkorde sind es nicht weniger (4,6 %), sie liegen also nicht nur an der Akkordschätzung.
- `FIG-ACCENTED`: Gegen geschätzte Akkorde 1,5 %, gegen Tymoczkos 6,4 %: die Schätzung erklärt betonte Durchgänge weg.
- `FIG-APPOGGIATURA`: Rate geeicht (1,5 % gegen 1,4 %).
- `FIG-APPOGGIATURA-WEAK`: Im Profil `ausgang` ein Verbot; Bach tut es in 8 % der Fälle.
- `FIG-SUSPENSION-UP`: Gegen Tymoczkos Akkorde 0,5 %, gegen geschätzte 13 %: Leitlinie und Preis folgen der Analyse. Die Suche bleibt auch bei P(16) darüber (11 % gegen geschätzte Akkorde).
- `FIG-GROUP`: Rate geeicht (0,11 % gegen 0,11 %).
- `FIG-ESCAPE`: Rate geeicht (2,5 % gegen 2,6 %).
- `FIG-ANTICIPATION`: Im Profil `ausgang` nicht vorgesehen (Verbot). Rate geeicht (0,62 % gegen 0,64 %).
- `FIG-DELAYED`: Im Profil `ausgang` nicht vorgesehen (Verbot). Rate geeicht (0,9 % gegen 1,4 %).
- `FIG-TOO-LONG`: Verbot mit vier gezählten Ausnahmen.
- `FIG-BEFORE-REST`: Verbot mit 14 gezählten Ausnahmen. In der Kunst der Fuge 2,0 % statt 0,6 %: eine benannte Abweichung des zweiten Korpus.

## Bewegung zweier Stimmen

| Kennung | Stufe | Regel | Gelegenheit | WTK: Gel. | Verst. | Anteil | 7 Fugen, Tymoczko | KdF | Leitlinie |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| `MOT-PARALLEL` | V | Zwei Stimmen gehen in gleicher Richtung von einer reinen Quinte oder Oktave (auch Einklang) in dieselbe. | jedes Stimmenpaar, das zugleich in gleicher Richtung anschlägt | 10831 | 34 | 0,31 % | 0,06 % | 0,02 % | P3 |
| `MOT-DIRECT` | P2 | Zwei Stimmen erreichen in gleicher Richtung eine reine Quinte oder Oktave, die obere im Sprung. | wie MOT-PARALLEL | 10831 | 316 | 2,92 % | 3,23 % | 2,10 % | P2 |
| `MOT-CROSSING` | P2 | Die oben notierte Stimme klingt unter der unteren. Der Preis wächst mit der Tiefe: einmal mehr je drei Halbtöne. | jedes Stimmenpaar in jedem Augenblick, in dem eine der beiden anschlägt | 70141 | 488 | 0,70 % | 0,71 % | 1,22 % | P3 |
| `MOT-CROSSING-DEEP` | P2 | Die Kreuzung ist tiefer als eine große Terz. Gilt zwischen zwei gesuchten oder zwei gegebenen Stimmen; gegen eine gegebene Stimme zahlt eine gesuchte nur MOT-CROSSING. | wie MOT-CROSSING | 70141 | 223 | 0,32 % | 0,37 % | 0,46 % | P3 |
| `MOT-UNISON` | P2 | Zwei Stimmen schlagen denselben Ton zugleich an. | jedes zugleich anschlagende Stimmenpaar | 27308 | 338 | 1,24 % | 1,41 % | 1,27 % | P3 |
| `MOT-OUTER-SIMILAR` | frei | Die Außenstimmen schlagen zugleich an und gehen in gleicher Richtung. | jedes Mal, wenn beide Außenstimmen zugleich weitergehen | 6304 | 3060 | 48,54 % | 45,06 % | 40,64 % | frei |

- `MOT-PARALLEL`: Verbot mit 34 gezählten Ausnahmen (0,31 %; in den sieben analysierten Fugen 0,06 %, in der Kunst der Fuge 0,02 %).
- `MOT-DIRECT`: Rate geeicht (3,4 % gegen 3,0 %).
- `MOT-CROSSING-DEEP`: Im Profil `ausgang` ein Verbot; Bach tut es in 0,3 % der Fälle.
- `MOT-OUTER-SIMILAR`: Frei: ohne Preis gehen die Außenstimmen der Suche etwas seltener so als Bachs (43 % gegen 52 %); ein Preis triebe sie weiter davon weg.

## Zusammenklang

HAR-SEVENTH-RESOLVE steht hier, obwohl sie eine Stimme betrifft: sie hängt am Akkord.

| Kennung | Stufe | Regel | Gelegenheit | WTK: Gel. | Verst. | Anteil | 7 Fugen, Tymoczko | KdF | Leitlinie |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| `HAR-SEVENTH-RESOLVE` | P4 | Die Septime eines Akkords geht nicht stufenweise abwärts weiter. | jede verlassene Akkordseptime | 5450 | 2297 | 42,15 % | 29,66 % | 47,07 % | frei |
| `HAR-CLASH` | V | Zwei akkordfremde Töne werden zugleich angeschlagen und bilden eine Dissonanz (die Quarte gilt hier als Konsonanz). | jedes Paar zugleich angeschlagener fremder Töne | 1186 | 68 | 5,73 % | 7,69 % | 7,00 % | P2 |
| `HAR-STRUCK` | P2 | Zwei Töne werden auf einem Schlag zugleich angeschlagen und bilden eine Dissonanz; die Quarte ist eine gegen den Bass. | jedes auf einem Schlag zugleich angeschlagene Paar | 14224 | 1603 | 11,27 % | 10,70 % | 10,99 % | P1 |
| `HAR-DOUBLED-LEADING` | P4 | Zwei Stimmen haben den Leitton über einem Dominantklang. | jedes Paar mit gleicher Tonklasse | 6942 | 104 | 1,50 % | 1,12 % | 0,41 % | P3 |
| `HAR-DOUBLED-SEVENTH` | P4 | Zwei Stimmen haben die Septime des Akkords. | jedes Paar mit gleicher Tonklasse | 6942 | 592 | 8,53 % | 1,68 % | 6,89 % | P2 |
| `HAR-DOUBLED-FOREIGN` | frei | Zwei Stimmen haben denselben akkordfremden Ton. | jedes Paar mit gleicher Tonklasse | 6942 | 252 | 3,63 % | 5,41 % | 4,26 % | P2 |
| `HAR-HARSH` | P1 | Ein angeschlagener akkordfremder Ton liegt einen Halbton (kleine None, große Septime) neben einem klingenden Ton. | jedes Paar mit einem angeschlagenen fremden Ton | 15771 | 1684 | 10,68 % | 9,10 % | 9,86 % | P1 |
| `HAR-NO-THIRD` | P3 | Auf einem Schlag mit drei oder mehr klingenden Stimmen fehlt die Terz des Akkords. | jeder Schlag mit drei oder mehr klingenden Stimmen, auf dem eine anschlägt | 5906 | 804 | 13,61 % | 17,01 % | 17,48 % | P1 |
| `HAR-SIX-FOUR` | P(6) | Auf einem solchen Schlag liegt die Quinte eines Dreiklangs im Bass. | wie HAR-NO-THIRD | 5906 | 508 | 8,60 % | 5,01 % | 10,54 % | P2 |
| `HAR-WIDE` | frei | Zwischen benachbarten Oberstimmen liegen mehr als 19 Halbtöne. | wie HAR-NO-THIRD | 5906 | 207 | 3,50 % | 2,30 % | 3,17 % | P2 |
| `HAR-INCOMPLETE` | P1 | Auf einem Schlag mit genau drei klingenden Stimmen ergeben die Töne keinen vollständigen Dur-, Moll- oder verminderten Dreiklang. | jeder Schlag mit genau drei klingenden Stimmen, auf dem eine anschlägt | 4740 | 2863 | 60,40 % | 61,02 % | 55,75 % | frei |
| `HAR-BASS` | frei | Wo ein Akkord beginnt, der seinen Bass nennt, ist der tiefste klingende Ton nicht dieser Basston. | jeder Anfang eines Akkords mit genanntem Bass, an dem mindestens zwei Stimmen klingen | 0 | 0 |  | 13,89 % |  | frei |

- `HAR-SEVENTH-RESOLVE`: Wie MEL-LEADING: der Anteil der Suche (59 % gegen Bachs 46 %) bewegt sich mit dem Preis kaum. Der Preis ist der des Profils `ausgang`.
- `HAR-CLASH`: Verbot als Festlegung, mit 68 gezählten Ausnahmen (5,7 %): zwei fremde Töne, die zugleich angeschlagen dissonieren, erklären einander nicht. An der Rate nicht zu eichen: gegen die neu geschätzten Akkorde eines erzeugten Satzes zeigt die Zählung auch beim Verbot 6 bis 11 %.
- `HAR-STRUCK`: Der Preis hält „Schläge mit angeschlagener Dissonanz“ im Band: bei P1 liegen 6 von 33 Entwicklungsläufen darüber, bei P2 keiner.
- `HAR-DOUBLED-SEVENTH`: Gegen Tymoczkos Akkorde 1,7 %, das wäre P3; die Suche trifft bei P4 Bachs Rate gegen geschätzte Akkorde (10,6 %).
- `HAR-DOUBLED-FOREIGN`: Frei: die Suche tut es halb so oft wie Bach.
- `HAR-HARSH`: Rate geeicht (8,8 % gegen 11,2 %; frei wären es 17,7 %).
- `HAR-NO-THIRD`: Rate geeicht (14,0 % gegen 14,9 %).
- `HAR-SIX-FOUR`: Rate geeicht (8,0 % gegen 9,2 %).
- `HAR-WIDE`: Frei: die Suche trifft Bachs Rate ohne Preis (8,4 % gegen 7,2 %).
- `HAR-INCOMPLETE`: Neu gegenüber dem Profil `ausgang`, für die Kennzahl „Klang: vollständiger Dreiklang“: ohne Preis liegen dreistimmige Sätze knapp unter dem Band, mit P1 auf allen Schlägen 28 von 33 Entwicklungsläufen darin (zwei Stimmen können die Kennzahl nicht erfüllen).
- `HAR-BASS`: Geschätzte Akkorde und Gerüste ohne `--bass planned` nennen keinen Bass, dort hat die Regel keine Gelegenheit; an den sieben Fugen mit Tymoczkos Bezifferung bricht Bach sie in 13,9 %. Im Profil `fuge` hat sie einen Preis, damit die Suche dem geplanten Bass folgt.

## Doppelter Kontrapunkt

Gilt nur, wo der Plan zwei Stimmen als vertauschbar markiert. In Bachs Fugen ist kein Abschnitt markiert, darum keine Zählung.

| Kennung | Stufe | Regel | Gelegenheit | WTK: Gel. | Verst. | Anteil | 7 Fugen, Tymoczko | KdF | Leitlinie |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| `DC-FIFTH` | V | Im doppelten Kontrapunkt der Oktave: Quinte oder Quarte zwischen den beiden Stimmen auf einem Schlag. | nur im markierten Abschnitt (Kontrasubjekt gegen Antwort) | 0 | 0 |  |  |  | Festlegung |
| `DC-WIDE` | V | Im doppelten Kontrapunkt der Oktave: die Stimmen liegen mehr als eine Oktave auseinander (bei einem Thema mit mehr als einer None Umfang: mehr als zwei) oder kreuzen sich. | nur im markierten Abschnitt | 0 | 0 |  |  |  | Festlegung |

- `DC-FIFTH`: Festlegung: die Umkehrung der Quinte ist die Quarte gegen den Bass.
- `DC-WIDE`: Festlegung: sonst kreuzen sie sich nach der Umkehrung.

## Parameter

| Name | Wert | Bedeutung |
|---|---:|---|
| `accented_strong_halves` | 3 | FIG-ACCENTED auf schwerem Schlag: so viele Hälften des Preises |
| `crossing_per` | 3 | MOT-CROSSING kostet einmal mehr je so viele Halbtöne Tiefe |
| `crossing_deep` | 4 | Halbtöne, ab denen eine Kreuzung tief ist |
| `wide` | 19 | Halbtöne zwischen benachbarten Oberstimmen, ab denen HAR-WIDE greift |
| `incomplete_weight` | 2 | HAR-INCOMPLETE ab diesem metrischen Gewicht (2 jeder Schlag, 3 die schweren) |
| `incomplete_voices` | 3 | HAR-INCOMPLETE nur bei genau so vielen klingenden Stimmen (0: ab drei) |
| `reach` | 12 | Halbtöne, die ein gesuchter Ton vom Ton davor entfernt sein darf |
| `reach_after_rest` | 14 | dasselbe nach einer Pause, vom letzten Ton |
| `close` | 16 | so oft zählen im Schlussakkord eine fehlende Terz (HAR-NO-THIRD), ab vier Stimmen ein unvollständiger Dreiklang (HAR-INCOMPLETE), eine Stimmkreuzung (MOT-CROSSING) und der Wunsch der Oberstimme, der dort ein Schritt ist |

Wo der Plan einen Schlussakkord nennt,
nehmen die Außenstimmen den Grundton, und `close` gewichtet, was in ihm fehlt. Gezählt an den
Schlüssen der 48 Fugen (`tools/bach_schlussakkorde.py`; eine Datei bricht vorher ab): keine endet
auf einer Quinte ohne Terz, die vierstimmigen haben den vollen Dreiklang, die Oberstimme schließt
in 43 von 47 auf dem Grundton und erreicht ihren Schlusston, vom letzten anderen Ton her, in 46
schrittweise. `fux-fuge check` kennt den Plan nicht und rechnet am Schlussakkord wie überall.
Mit `close` = 8 reichte das in einer von zwanzig vierstimmigen Beispielfugen nicht für die
Quinte, weil die freien Stimmen bis zum Schlussakkord in Figuren laufen; mit 16 schließen alle
zwanzig mit dem Dreiklang.

## Preise des Plans

Was die Suche zahlt, ohne dass es eine Regel des Stils wäre. `fux-fuge check` kennt den Plan nicht
und rechnet diese Preise nicht.

| Name | Wert | Bedeutung |
|---|---:|---|
| `wish_class` | 4 | je Klasse, die ein Intervall vom gezogenen Wunsch abweicht (Wiederholung, Schritt, Terz, Quarte oder Quinte, größer), höchstens zweimal |
| `wish_turn` | 2 | die Linie wendet, wo sie weitergehen wollte, oder umgekehrt |
| `edge` | 2 | ein gesuchter Ton am Rand des Umfangs seiner Stimme |
| `into_given` | 6, 0, 2, 4, 8 | das Intervall von einem gesuchten Ton in einen gegebenen, nach Klasse |
| `figure` | 6 | ein Ton einer Figur weicht vom Schritt ab, den die Figur dort hat: je Schritt Abstand, höchstens zweimal |
| `given_fault` | 40 | ein gegebener Ton bricht ein Verbot |
| `excused` | 12 | ein gesuchter Ton löst den fremden gegebenen Ton vor ihm nicht auf |
| `far` | 2 | je Halbton, den eine gesuchte Stimme noch zu weit von ihrem nächsten gegebenen Ton entfernt ist; ordnet nur den Strahl |

Rhythmus und Intervallwünsche der gesuchten Stimmen kommen aus den Tabellen der acht Fugen, deren
Themen dem gegebenen in den Notenwerten am nächsten sind (`cells.rs`, `melody.rs`). Das gilt
nur, wo eine Stimme keine Figur hat: Sonst steht der Rhythmus der Figur fest, und
ihre Schritte sind die Wünsche.

## Profil fuge

Der Fugenkomponist setzt mit einem eigenen Profil: `bach` mit vier anderen Stufen
(`crates/fux-bach/data/fuge.profile`).

| Kennung | `bach` | `fuge` | Warum |
|---|---|---|---|
| `HAR-STRUCK` | P2 | P4 | In einer blinden Hörprobe innerhalb eines Themas klang die Harmonik schlechter, je mehr Dissonanz zusammen angeschlagen wurde. |
| `HAR-HARSH` | P1 | P3 | wie `HAR-STRUCK` |
| `HAR-INCOMPLETE` | P1 | P3 | In der zweiten Probe klang der dreistimmige Satz mit volleren Klängen am besten. Die Regel gilt, wo genau drei Stimmen klingen. |
| `HAR-BASS` | frei | P4 | Wirksam nur mit `--bass planned`: Dann nennt das Gerüst je Akkord einen Basston, und mit diesem Preis liegt er an 86 % der Akkordanfänge zuunterst, wie bei Bach unter den Akkorden von Tymoczkos Analysen. |

Die Stufen von `bach` sind an Bachs Raten geeicht, die von `fuge` am Ohr, und beides widerspricht
sich: Mit `fuge` liegt „Schläge mit Dissonanz“ in 36 von 40 Fugen unter Bachs Band (mit `bach` in
14). Bach hat mehr Dissonanz auf dem Schlag als das Programm, aber sie liegt; die des Programms
wird angeschlagen, und das Ohr zieht ihr die Konsonanz vor. Was Bach an ihrer Stelle hat, fehlt
dem Programm weiter.

## Einstellen

- `fux-fuge rules` zeigt alle Regeln mit Stufe, `fux-fuge rules --file` das Profil als Datei.
- `--profile bach` (Standard von `check` und `rules`), `--profile fuge` (Standard von `fugue`),
  `--profile ausgang` (die Preise, mit denen die Eichung begann) oder `--profile datei`:
  eine Datei mit Zeilen `KENNUNG = Stufe` und `parameter = Wert`, die das Profil `bach`
  überschreiben.
- Einzeln: `--rule FIG-ANTICIPATION=V`, `--rule wish_class=6`.

## Grenzen

- Die Stufen sind an geschätzten Akkorden geeicht. Mit einem erzeugten Gerüst stehen
  die Akkorde fest, und die Zählung an Bach bleibt, was sie ist: eine obere Schranke.
- Vorhalte und liegende Dissonanzen schreibt die Suche seltener als Bach („Schläge mit Dissonanz“
  liegt in etwa der Hälfte der Läufe unter dem Band). Kein Preis hebt sie, ohne die angeschlagene
  Dissonanz über ihr Band zu treiben; sie hängen am Rhythmus (übergebundene Töne) und an den
  Akkorden.
- Querstände und die Behandlung der None haben keine eigene Regel.
