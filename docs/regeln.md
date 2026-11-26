# Regelkatalog

Stand: November 2026. Der Katalog ist die Spezifikation für die Regel-Engine. Jede Regel
hat eine stabile ID und einen **Standardwert**, und jede ist **einstellbar** (siehe
[Einstellbarkeit](#einstellbarkeit)). Die Standardwerte richten sich nach Fux' eigenen Beispielen
(siehe [Leitlinie](#leitlinie-für-standardwerte)). Mit „vorläufig“ markierte Werte ließen sich an
keinem Korpus sicher eichen: bei Fux zu selten belegt, im freien Satz, bei den TXT-Regeln (§ 17) und
bei einigen IMI-Regeln (§ 18) ohne passendes Korpus. Bei den Anteilen des Neubeginns einer Messe
(§ 19) fehlt eine nachgeprüfte Zählung.

## Einstellbarkeit

- **Stufe jeder Regel:** Jede Regel lässt sich auf `aus`, `P1`–`P4`, `P(n)` oder `V` setzen. Das gilt
  global oder getrennt nach Stimmenzahl und Gattung.
- **Parameter:** Regeln mit Parametern (z. B. Grenzwerte, Auswahl wie beim Schlussklang) nennen diese
  mit Standardwert in der Regelbeschreibung.
- **Wo man einstellt:**
  - Profildatei: `profiles/fux.toml` ist das Standardprofil; `--profile jeppesen` lädt das strengere
    [Jeppesen-Profil](#15-jeppesen-profil), eigene Profile `--profile datei.toml`.
  - Einzeln auf der Kommandozeile: `--rule G3-ACCENT-PARALLELS=aus`, `--rule TEX-FINAL-CHORD.terz=bevorzugt`.
  - In der GUI über eine Regeltabelle.
- **Übersicht:** `fux rules` zeigt alle Regeln mit aktueller Einstellung und Quelle.
- **Prüfmodus:** Er meldet Verstöße gegen weiche Regeln mit ihren Kosten und Verstöße gegen Verbote als Fehler.

## Leitlinie für Standardwerte

Grundlage ist, wie oft Fux in seinen 123 Beispielen gegen eine Regel verstößt, gemessen an den
Gelegenheiten dazu.

| Im Korpus | Standard |
|---|---|
| nie, obwohl Gelegenheit bestand | V |
| selten (Einzelfälle, bis etwa 2 %) | P3 |
| gelegentlich (etwa 2–10 %) | P2 |
| häufig (über 10 %) | P1 oder frei |
| nicht auswertbar | Fux' Text, sonst Lehrbuchtradition; „vorläufig“ |

Widersprechen sich Fux' Text und seine Beispiele, entscheidet der Korpus über den Standardwert. Der
Text steht dann in der Quelle.

## Legende

**Stufen**

| Kürzel | Bedeutung |
|---|---|
| V | Verbot. Ist hart und schneidet den Suchbaum ab. |
| P1 / P2 / P3 / P4 | Präferenz mit Kosten 1 / 2 / 4 / 8 je Verstoß (gering / mittel / hoch / Notlösung). Die Skala ist von FuxCP übernommen (Wafflard 2023, S. 16). |
| P(*n*) | Präferenz mit ausdrücklich angegebenen Kosten *n* |
| frei | Ausdrücklich erlaubt. Steht hier, damit keine andere Regel es versehentlich verbietet. |
| aus | Abgeschaltet |

**Gilt für**
- `CF` = Cantus firmus.
- `G1`–`G5` = Gegenstimme in der 1.–5. Gattung.
- `alle` = alle Stimmen.
- `2st` / `3st` / `4st` = Stimmenzahl. Ohne Angabe gilt die Regel für jede Stimmenzahl.

**Quellen**

| Kürzel | Quelle |
|---|---|
| Mann *n* | Fux in der englischen Übersetzung von A. Mann (*The Study of Counterpoint*, rev. Ausg.), Seite *n*, zitiert nach den FuxCP-Arbeiten |
| Ch. *n* | Französische Übersetzung von J.-P. Chevalier (2019), Seite *n*, zitiert nach Wafflard 2023 |
| lat. *n* | Lateinisches Original 1725, Seite *n* |
| Fux S. *n* | In § 18 dasselbe: lateinisches Original 1725, Seite *n*, zitiert wie die übrigen Quellen dort |
| W23, L24, L25, CdP24, B26 | FuxCP-Arbeiten: Wafflard 2023 (zweistimmig), Lamotte 2024 (dreistimmig), Lai 2025 (vierstimmig), Cleenewerk/de Patoul 2024 (vierstimmig), Burignat/de Menten 2026. Regel-IDs wie `1.H5` stammen von dort; ein `*` heißt „implizit, nicht von Fux formuliert“. Siehe [Quellenindex](../references/README.md). |
| Korpus | Auswertung von Fux' 123 Notenbeispielen (`tests/fixtures/fux/`) mit `tools/corpus_stats.py`. Alle Zahlen hier lassen sich damit nachrechnen. |
| Trad. | Lehrbuchtradition (v. a. Jeppesen), nicht Fux |
| Impl. | Festlegung dieses Projekts |

## 0. Grundbegriffe

**Intervalle** werden aus buchstabierten Tönen berechnet (diatonischer Abstand plus Halbtöne).
Ein Intervall hat also eine Qualität (rein, groß, klein, übermäßig, vermindert). Für Konsonanzregeln
zählt das Intervall modulo Oktave, für Abstandsregeln das tatsächliche Intervall.

| Klasse | Intervalle (einschließlich Oktavversetzungen) |
|---|---|
| Vollkommene Konsonanzen | Einklang, reine Quinte, Oktave |
| Unvollkommene Konsonanzen | kleine und große Terz, kleine und große Sexte |
| Dissonanzen | Sekunden, Septimen, alle übermäßigen und verminderten Intervalle (darunter der Tritonus); die reine Quarte im zweistimmigen Satz und gegen den Bass |

**Bewegungsarten** zwischen zwei Stimmen:
- **Gegenbewegung**: Die Stimmen bewegen sich in entgegengesetzter Richtung.
- **Seitenbewegung**: Eine Stimme bleibt liegen.
- **Gerade Bewegung**: Beide Stimmen bewegen sich in dieselbe Richtung. Parallelbewegung ist der Sonderfall mit gleichem Intervall.

**Taktpositionen**: Ein Takt entspricht einer Ganzen des CF.
- **Thesis** ist der Niederschlag (Taktbeginn), **Arsis** der Aufschlag (zweite Halbe).
- In der 3. Gattung zählen die Viertel 1–4.

**Bass** ist ab drei Stimmen in jedem Takt die tiefste klingende Stimme, bei Gleichstand die untere
im Satz. Regeln, die zweistimmig „gegen den CF“ gelten, gelten dann „gegen den Bass“ (Mann 71: „the
upper two being consonant with the lowest“; L24 § 2.2; L25 § 3.1).

**Dissonanz** ist zweistimmig jedes Intervall außer den Konsonanzen. Ab drei Stimmen gilt gegen den
Bass dasselbe (die Quarte eingeschlossen); zwischen Oberstimmen sind Quarten (rein, übermäßig,
vermindert) und die verminderte Quinte frei. Zwei Ergänzungen für mehrere bewegte Stimmen
(`crates/fux-core/src/rules/dissonance.rs`):
- **Wer die Dissonanz trägt**, muss sie behandeln: ein Vorhalt (über den Taktstrich gehaltene Note)
  oder die kürzere von zwei unbetont angeschlagenen Noten (gemessen bis zum Taktstrich). Die andere
  Stimme ist entlastet. Zweistimmig trägt immer die Gegenstimme, weil der CF nie mitten im Takt wechselt.
  Belegt durch Fig. 204 (2., 3. und 4. Gattung zugleich).
- **Verzögerter Leitton im Bass**: Ab drei Stimmen darf der Bass im vorletzten Takt den Leitton der
  Finalis verspätet bringen: Er hält die Finalis über den Taktstrich und steigt nach der Thesis
  einen Halbton ab, auch gegen eine verminderte Quinte in einer Oberstimme (Gallon/Bitsch Regel 54;
  Fux Fig. 123, 144, 151, 156, 197, 203). Die Gattungsregeln nehmen dieses Intervall als konsonant;
  HAR-DISSONANCE meldet seine Verwendung. Ein Leitton, der schon auf der Thesis steht, ist nicht
  verzögert.

## 1. Cantus firmus

Die melodischen Regeln aus § 2 gelten auch für den CF, soweit „Gilt für“ ihn einschließt. Grundlage
des Korpus sind hier die 8 Cantus in `cantus.toml` (6 Standard-Cantus, 2 Varianten).

| ID | Regel | Standard | Gilt für | Quelle |
|---|---|---|---|---|
| CF-START | Der CF beginnt auf der Finalis. | V | CF | Korpus (alle) |
| CF-END | Der CF endet auf der Finalis. | V | CF | Korpus (alle) |
| CF-PENULT | Der vorletzte Ton ist die Sekunde über der Finalis (Schluss 2→1). Nur so sind die Klauselregeln (CAD-PENULT) erfüllbar. | V | CF | Korpus (alle); W23 S. 28 f. |
| CF-LENGTH | Länge des CF. Parameter: erlaubt 6–20, Warnung außerhalb von 8–16. | V | CF | Impl.; Korpus: 9–14 Töne |
| CF-DIATONIC | Nur leitereigene Töne des Modus (einschließlich eines Transpositions-♭), keine Ficta. | V | CF | Korpus (alle) |
| CF-RANGE | Umfang. Parameter: höchstens eine Dezime (V); über eine Oktave kostet P3, unter einer Quinte ebenfalls P3. | V / P3 | CF | Korpus (alle zwischen Quinte und Oktave); Impl. |
| CF-AMBITUS | Töne im Ambitus des Modus mit einer Stufe Toleranz. Authentisch: Finalis bis Oktave darüber. Plagal: Quarte unter bis Quinte über der Finalis. | P2 | CF | Trad.; Korpus: der phrygische Cantus verlässt ihn (§ 13) |
| CF-REPEAT | Keine Tonwiederholung. | V | CF | Korpus (nie) |
| CF-STEPS | Schritte sollen überwiegen. Parameter: höchstens 50 % Sprünge. | P1 | CF | Trad.; Korpus: der mixolydische Cantus hat mehr Sprünge (§ 13) |
| CF-LEAP-COST | Kosten je Sprung im Cantus: Terz frei, Quarte P1, Quinte P2, Sexte P3, Oktave P2. | frei / P1–P3 | CF | Korpus (8 Cantus: 20 Terzen, 5 Quarten, 1 Quinte, 1 Oktave, keine Sexte) |
| CF-CLIMAX | Der höchste Ton kommt genau einmal vor, nicht am Anfang oder Schluss (V). Liegt er außerhalb von 30–75 % der Länge, kostet das P2. | V / P2 | CF | Korpus (alle 8 erfüllen es; Höhepunkte bei 42–64 %) |
| CF-VARIETY | Abwechslung: Pendelfigur x–y–x P1; ein Ton (außer der Finalis) öfter als dreimal P1 je weiteres Vorkommen. | P1 | CF | Korpus (Wechselnote E–F–E vor dem Schluss häufig); Mann 73 |
| CF-MOTIF | Keine Wiederholung von Tonfolgen („monotonia“): wörtlich ab vier Tönen, transponiert (Sequenz) ab fünf Tönen. Überlappende Wiederholungen zählen, wenn sie mindestens zwei Töne versetzt sind; eine Tonleiter ist kein Motiv. | P2 | CF | Mann 54 |
| CF-SOLVABLE | Ein erzeugter CF muss in der 1. Gattung darüber und darunter lösbar sein. | V | CF (nur Generierung) | Impl. |

## 2. Melodik

| ID | Regel | Standard | Gilt für | Quelle |
|---|---|---|---|---|
| MEL-TRITONE | Kein Tritonus als melodisches Intervall. | V | alle | Ch. 59; Mann 35; W23 1.M1; Korpus (nie) |
| MEL-AUG-DIM | Keine übermäßigen oder verminderten Intervalle, also auch keine übermäßige Sekunde und keine verminderte Quinte. Der chromatische Halbton (G→G♯) ist zweistimmig V, drei- und vierstimmig P3. | V / P3 | alle | W23 G6*; L25 (chromatische Linien vierstimmig nur Kosten); Korpus (sonst nie; chromatischer Halbton nur Fig. 110, 169, 170) |
| MEL-MAX-LEAP | Der größte Sprung ist die kleine Sexte. Große Sexte, Septimen und Sprünge über die Oktave sind verboten. Die Oktave regelt MEL-OCTAVE. | V | alle | Mann 37, 79 („The skip of a major sixth is prohibited“); Ch. 61; Korpus (nie große Sexte oder Septime) |
| MEL-OCTAVE | Oktavsprung, je nach Satz und Gattung (Werte unten). | V / P3 / P1 / frei | alle | Mann 45, 115; Ch. 67 f.; W23 1.M2, 2.M1; Korpus |
| MEL-MIN6-DOWN | Kleine Sexte abwärts. | frei (Jeppesen-Profil: V) | alle | Korpus (zweistimmig 4× abwärts, 3× aufwärts); Trad. |
| MEL-LEAP-COST | Schritte werden bevorzugt. Parameter: Kosten je Sprung Terz 1, Quarte 1, Quinte 2, Sexte 2, Oktave 1. | P | alle außer CF | Mann 73 („moving gracefully, stepwise“); W23 G7* |
| MEL-LEAP-RECOVERY | Nach einem Sprung ab der Quinte folgt ein Richtungswechsel, möglichst stufenweise. | P2 vorläufig (Jeppesen-Profil: P3) | alle | Trad. |
| MEL-CONSEC-LEAPS | Höchstens zwei Sprünge in Folge in derselben Richtung. Zusammen höchstens eine Oktave und nur als Dreiklangsbrechung. Parameter `larger_above`: der größere der beiden Sprünge liegt oben (aufwärts Terz, dann Quarte; abwärts Quarte, dann Terz). | P3, über eine Oktave V; vorläufig. Größerer oben P2 (Jeppesen-Profil: V) | alle | Trad.; Korpus (größerer oben 10 von 151 Dreiklangsbrechungen) |
| MEL-TRITONE-OUTLINE | Kein Tritonus zwischen zwei Wendepunkten (lokaler Hoch- und Tiefpunkt) einer Linie. | P2 vorläufig | alle | Trad. |
| MEL-REPEAT | Tonwiederholung, je nach Gattung (Werte unten). Eine Bindung (4./5. Gattung) gilt nicht als Wiederholung. | P1 / V | G1–G3, G5 | W23 1.P2, 2.M2*; Korpus |
| MEL-VARIETY | Prinzip der Abwechslung: möglichst viele verschiedene Töne. Derselbe Ton zwei Viertel später (3. Gattung), ein Viertel, das zum Ton zwei Noten davor zurückkehrt (5. Gattung, x–y–x), oder derselbe Ton innerhalb von drei Takten (1. Gattung) kostet P1. Gleicher Arsis-Ton zwei Takte später (4. Gattung) kostet P3. | P1 / P3 vorläufig | G1–G5 | Mann 73; W23 3.M1*, 4.M2; L24 1.M4; Korpus (x–y–x in Vierteln: 3. Gattung 31 von 228, 5. Gattung 28 von 153) |
| MEL-MOTIF | Keine wörtlich wiederholte Tonfolge in der Gegenstimme („monotonia“). Parameter: ab 4 Tönen; überlappende Wiederholungen zählen ab zwei Tönen Versatz. | P3 (1., 2., 4. Gattung), frei (3., 5. Gattung) | G1–G5 | Mann 54; Korpus (zweistimmig: 1. und 4. Gattung nie, 2. Gattung 3× in Fig. 39–41, 3. Gattung 33×, 5. Gattung 12×) |
| MEL-CLIMAX | Die Gegenstimme hat einen einzigen Höhepunkt. | P1 vorläufig | G1–G5 | Trad. |

Werte von **MEL-OCTAVE**:
- Im CF frei.
- Zweistimmig in der 1. Gattung V (Korpus: nie).
- Zweistimmig in der 2. Gattung:
  - Thesis→Arsis: frei, wenn die Stimmen höchstens eine Terz auseinander liegen; sonst P3 (Korpus: 8× eng, 1× weiter).
  - Arsis→Thesis: frei.
- In der 3.–5. Gattung und im drei- und vierstimmigen Satz: nur MEL-LEAP-COST.

Werte von **MEL-REPEAT**:
- In der 1. Gattung P1 (ergibt Seitenbewegung; im Korpus häufig).
- In der 2.–4. Gattung zwei- und dreistimmig V.
- In der 5. Gattung zwei- und dreistimmig P3 (Korpus: nur Fig. 156, Halbe D und Viertel D im Bass, vom Faksimile bestätigt).
- Ab vier Stimmen in der 2.–5. Gattung P3 (Korpus: nur Fig. 173, 181).
- Vorausnahme in der 5. Gattung, also ein Viertel, dann derselbe Ton als mindestens halbe Note
  geschrieben (ein übergebundenes Viertel zählt als Viertel, denn die Bindung entscheidet sich erst
  danach): P2
  (Korpus: 3 von 53 Gelegenheiten, Fig. 86a, 87a, 88b; frei gestellt nutzte der Generator sie in fast
  jedem Takt).
- Im freien Satz eigene Parameter: `free`, `free_black` und `free_run` (§ 16), in Stimmen mit Text
  `free_texted` (§ 17).

## 3. Zusammenklang

| ID | Regel | Standard | Gilt für | Quelle |
|---|---|---|---|---|
| HAR-CONSONANCE | Auf der Thesis klingt jede Stimme konsonant zum CF (zweistimmig) bzw. zum Bass. In der 1. Gattung gilt das für alle Klänge. Ausnahmen sind Vorhalte (4./5. Gattung) und die Schlussbindung der 2. Gattung im dreistimmigen Satz. | V | alle | Mann 27, 71; Ch. 53, 64; W23 1.H1, 2.H1 |
| HAR-DISSONANCE | Jede Dissonanz zwischen zwei beliebigen Stimmen braucht eine Behandlung nach den Gattungsregeln (§ 8–11). Das gilt auch zwischen Oberstimmen: Dort ist eine auf der Thesis gemeinsam angeschlagene Sekunde oder Septime verboten. Parameter: `upper_fourths` (Quarte oder verminderte Quinte zwischen Oberstimmen, frei), `cadence_tritone` (verzögerter Leitton im Bass, frei). | V | 3st+ | Fux; B26; Gallon/Bitsch Regel 54; Korpus (Sekunden und Septimen zwischen Oberstimmen auf der Thesis nur als Vorhalt oder Schlussbindung) |
| HAR-FOURTH | Die reine Quarte ist zweistimmig und gegen den Bass eine Dissonanz. Zwischen Oberstimmen sind Quarten und die verminderte Quinte frei (siehe § 0). Teil der Dissonanzbestimmung, keine eigene Regel. | V | alle | Fux; W23 1.H1; Korpus |
| HAR-CHROMATIC-CLASH | Kein Ton erklingt gleichzeitig mit seiner Alteration (z. B. F gegen F♯). | V | alle | L25 1.H8; Trad.; Korpus (nie) |
| HAR-UNISON | Einklang im engen Sinn (gleiche Tonhöhe) auf einer inneren Thesis: zwei- und dreistimmig V, vierstimmig P1, ab fünf Stimmen frei (hochgerechnet). Auf der Arsis der 2. Gattung frei. | V / P1 / frei | alle | Ch. 62; Mann 79, 121; W23 1.H5; L25 1.H5; Gallon/Bitsch Regel 53; Korpus (zwei- und dreistimmig nie; vierstimmig in 10 von 100 Klängen) |
| HAR-PERFECT-COST | Unvollkommene Konsonanzen werden bevorzugt. Jede Quinte oder Oktave auf der Thesis kostet, außer im ersten und letzten Takt. Ab drei Stimmen gehören Quinte und Oktave zum vollständigen Klang; dort regelt TEX-TRIAD. | P1 | 2st | Ch. 54; W23 1.H6 |
| HAR-IMPERFECT-RUN | Terzen oder Sexten in Parallelbewegung. Parameter: zweistimmig höchstens 3 in Folge (V darüber), drei- und vierstimmig höchstens 6 (P2 darüber). | V / P2 | alle Stimmpaare | Trad.; Korpus (zweistimmig nie mehr als 3; drei- und vierstimmig bis 6) |

## 4. Bewegung

| ID | Regel | Standard | Gilt für | Quelle |
|---|---|---|---|---|
| MOT-DIRECT-PERFECT | Eine vollkommene Konsonanz darf nicht in gerader Bewegung erreicht werden. Das verbietet offene und verdeckte Quinten und Oktaven. Geprüft wird jeder Klangwechsel, in der 2. und 3. Gattung also auch der von der Arsis bzw. vom 4. Viertel in die Thesis. Für drei und vier Stimmen siehe TEX-PARALLEL und TEX-DIRECT-PERFECT. | V | 2st | Fux' vier Bewegungsregeln, Mann 22; Ch. 51, 57; W23 1.P1, 3.P1; Korpus (nie, in keiner Gattung) |
| MOT-PREFERENCE | Gegenbewegung vor Seitenbewegung vor gerader Bewegung, gezählt in jeden Taktanfang. Die Bewegung kommt vom letzten Ton davor: in der 3. Gattung vom 4. Viertel, in der 2. Gattung von der Thesis, wenn die Arsis höchstens eine Terz von ihr entfernt liegt. Eine übergebundene Note (4. Gattung) bewegt sich nicht und zählt nicht. Ab drei Stimmen zählt jede Stimme gegen den Bass, nicht jedes Stimmpaar. Parameter: Kosten 0 / 1 / 2. | P | alle | Ch. 53; W23 1.P2, 2.P1, 3.P1; L25 1.P2 |
| MOT-BATTUTA | Oktave oder Einklang auf der Thesis, erreicht in Gegenbewegung mit fallender Oberstimme und steigender Unterstimme. Springt die Oberstimme dabei, ist das zu vermeiden; mit Schritt in der Oberstimme ist es frei. Fux nennt die Regel „of little importance“. | P3 vorläufig | 2st | Mann 39; Ch. 61 f.; W23 1.P3, 2.P2* |
| MOT-ANTIPARALLEL | Gleiche vollkommene Konsonanz in Gegenbewegung (z. B. Quinte → Duodezime). | P3 | alle Stimmpaare | Korpus (zwei- und vierstimmig nie, dreistimmig 1×); Fux' Regel 1 erlaubt sie wörtlich |

## 5. Anfang und Schluss (zweistimmig)

Für drei und vier Stimmen siehe TEX-FIRST-CHORD, TEX-PENULT und TEX-FINAL-CHORD.

| ID | Regel | Standard | Gilt für | Quelle |
|---|---|---|---|---|
| CAD-START | Der erste Klang ist eine vollkommene Konsonanz. Liegt die Gegenstimme unten, nur Einklang oder Oktave, weil die Unterstimme die Finalis haben muss. | V | 2st | Ch. 54, 56; W23 1.H2, 1.H4 |
| CAD-END | Der letzte Klang ist Einklang oder Oktave, mit der Finalis in der Unterstimme. | V | 2st | Ch. 54; W23 1.H3, 1.H4 |
| CAD-PENULT | Der vorletzte Klang ist eine große Sexte, wenn der CF unten liegt, oder eine kleine Terz, wenn er oben liegt. Der Schluss erfolgt in Gegenbewegung. So bei Fux, dessen CF stets von oben in die Finalis fällt (Tenorklausel). Steigt ein CF (Choral) von unten in die Finalis, hat er die Diskantklausel, und die Gegenstimme fällt: kleine Terz über ihm, große Sexte unter ihm. Springt der CF, bringt die Gegenstimme den Leitton. | V | 2st | Ch. 54; W23 1.H7, 1.H8 |
| CAD-PENULT-G2 | 2. Gattung, vorletzter Takt: Die Thesis ist eine Quinte, sonst eine Sexte (P4). Die Arsis folgt CAD-PENULT. Wie CAD-PENULT-G3 bis G5 nur, wenn der CF in die Finalis fällt. | V / P4 | G2 | Ch. 64 f., 69; W23 2.H3, 2.H4; Korpus (Quinte 11×, Sexte 1× in Fig. 37, phrygisch unten, wo die Quinte fehlt) |
| CAD-PENULT-G3 | 3. Gattung, vorletzter Takt: Liegt der CF oben, ist das 1. Viertel eine kleine Terz (P2, weil Fux selbst abweicht). Das 4. Viertel folgt CAD-PENULT. | P2 | G3 | Ch. 75; Mann 54; W23 3.H4; Korpus (Fig. 58 weicht ab) |
| CAD-PENULT-G4 | 4. Gattung: Die Synkope im vorletzten Takt ist eine Septime (CF unten) oder Sekunde (CF oben). Sie löst sich in große Sexte bzw. kleine Terz auf. | V | G4 | W23 4.H3, 4.H4 |
| CAD-PENULT-G5 | 5. Gattung: Der vorletzte Takt enthält Synkope und Auflösung wie in der 4. Gattung (Takt `⁀2 2`, Rhythmus über G5-RHYTHM): Septime über bzw. Sekunde unter dem Cantus. | V | G5 | Korpus (alle 12 zweistimmigen Beispiele); W23 5.R6 |

## 5a. Binnenschlüsse

Lange Cantus, etwa Choralmelodien, gliedern sich in Phrasen. Am Ende einer Phrase steht ein
Binnenschluss: Die Stimmen bilden eine Klausel auf dem Phrasenschlusston des CF (dem **Ziel**), wie am
Schluss auf der Finalis. Fux' Beispiele haben keine; die Regeln übertragen deshalb die Schlussregeln
auf das Ziel und sind mit denselben Stufen voreingestellt.

- **Wo:** Binnenschlüsse werden im Satz markiert (Takt und Ziel). Beim Import einer Choralmelodie
  (GABC) entstehen sie an den Gliederungsstrichen, standardmäßig an Halb- und Ganzstrichen
  (`--cadences auto`; `full` nur Ganzstriche, `all` auch Viertelstriche, `none`, oder Takte wie
  `8,15`), sofern dort eine Klausel möglich ist: Der CF erreicht das Ziel stufenweise (dann hat er die
  Tenor- oder die Diskantklausel) oder springt aus der Quinte hinein (Bassklausel), und zwischen zwei
  Binnenschlüssen liegen mindestens drei Takte, ebenso vor dem Schluss. Endet eine Phrase auf
  wiederholten Tönen, liegt das Ziel auf dem ersten.
- **Klausel je Stufe:** Auf jeder Stufe außer der mit verminderter Quinte (H, transponiert E) ist eine
  Klausel möglich. Hat das Ziel einen Ganzton unter sich, wird der Leitton erhöht (vor D C♯, vor G F♯,
  vor A G♯); auf der Stufe mit Halbton darüber (E, transponiert A) ist die Klausel phrygisch: Der
  Halbton liegt in der fallenden Stimme (F→E).
- **Der CF bekommt den Leitton**, wenn er von unten im Ganzton ins Ziel steigt (C–D wird C♯–D), wie es
  Sänger des 16. Jahrhunderts taten; als Ficta über der Note gedruckt. Das gilt auch für den Schluss
  (Veni Creator endet F♯–G).
- **Rhythmus:** Der Takt vor dem Ziel ist für die Taktmuster ein vorletzter Takt, der davor ein
  drittletzter (z. B. 4. Gattung `⁀2 2`, ohne Bindung ins Ziel).

| ID | Regel | Standard | Gilt für | Quelle |
|---|---|---|---|---|
| CAD-INNER-CLAUSULA | Im letzten Klang vor dem Ziel hat genau eine Stimme den Leitton des Ziels (phrygisch: den Halbton darüber) und erreicht das Ziel im Halbtonschritt; der CF zählt mit. Parameter `doubled`: Leitton in mehreren Stimmen. Singt eine Gegenstimme ins Ziel hinein die unerhöhte Stufe (C vor D), ist kein Leitton mehr möglich; das wird gemeldet, sobald der Ton steht. | V / V | alle | wie TEX-CLAUSULA |
| CAD-INNER-PENULT | Ab drei Stimmen: wie TEX-PENULT, bezogen auf das Ziel (`level`, `four`, `bass`). | V / P2 / V | 3st+ | wie TEX-PENULT |
| CAD-INNER-GOAL | Zielklang: zweistimmig Einklang oder Oktave auf dem Ziel; ab drei Stimmen der Bass auf dem Ziel, darüber Oktave, Terz (auch klein) oder Quinte. | V | alle | wie CAD-END und TEX-FINAL-CHORD |

## 6. Musica ficta

| ID | Regel | Standard | Gilt für | Quelle |
|---|---|---|---|---|
| FIC-SUBSEMITONIUM | In der Klausel erreicht die zur Finalis steigende Stimme sie im Halbtonschritt. Dorisch C♯, mixolydisch F♯, äolisch G♯. Lydisch (E) und ionisch (H) sind schon diatonisch. Phrygisch nicht: Der Halbton liegt dort in der fallenden Stimme (F→E). | V | G1–G5 | Ch. 54; W23 S. 28; Korpus (zweistimmig C♯ 11×, F♯ 6×, G♯ 6× im vorletzten Takt) |
| FIC-RAISED-SIXTH | Vor dem Subsemitonium darf auch die Stufe davor erhöht werden, um eine übermäßige Sekunde oder eine verminderte Quinte zu vermeiden. Beispiel äolisch, 2. Gattung: F♯ als Quinte über H, dann G♯–A. | frei | G2–G5 | Korpus (Fig. 42); W23 S. 35 |
| FIC-INNER-CADENCE | Leittöne außerhalb der Schlussklausel. Der Leitton eines markierten Binnenschlusses (§ 5a) im Takt vor dem Ziel ist frei. | P2 je Note | G1–G5 | Korpus (zweistimmig F♯ 3×; drei- und vierstimmig C♯ 13×, G♯ 11×) |
| FIC-BFLAT | B♭ statt H, um Tritonus oder verminderte Quinte gegen F zu vermeiden. Nur in dorisch und lydisch (und deren Transpositionen). | P1 je Note | G1–G5 | Korpus (zweistimmig dorisch 8×, lydisch 19×; in anderen Modi nie) |
| FIC-PICARDY | Große Terz im Schlussklang: dorisch F♯, phrygisch G♯ (ab drei Stimmen, siehe TEX-FINAL-CHORD). | frei | 3st, 4st | Mann 80, 82; Korpus |
| FIC-SCOPE | Erlaubt sind nur: ein erhöhter Ton, der sich spätestens auf der nächsten Thesis stufenweise aufwärts auflöst (Fux auch über Umweg: F♯–D–G, C♯–A–H–C♯–D; eine übergebundene Note bis zur Thesis nach ihrem Ende); B♭ als erniedrigtes mi in dorisch und lydisch; ab drei Stimmen die große Terz der Finalis im ersten oder letzten Klang. Ficta gibt es nur in Gegenstimmen; im CF nur den Leitton vor einer Klausel, in die er von unten steigt (§ 5a). | V | alle | Korpus (Fig. 41, 56, 130, 133, 177; erster Klang Fig. 184, 185); Impl. |
| FIC-CROSS-RELATION | Kein Querstand: ein Ton und seine Alteration in benachbarten Klängen verschiedener Stimmen (z. B. F gegen F♯). | P3 vorläufig | alle Stimmpaare | Trad. |

## 7. Stimmen

| ID | Regel | Standard | Gilt für | Quelle |
|---|---|---|---|---|
| VOI-RANGE | Jede Stimme bleibt im Liniensystem ihres Originalschlüssels (Werte unten). | frei / P3 / P2 / P4 / V | alle | Mann 79 („One should not exceed the limits of the five lines without grave necessity“); Korpus (höchstens 2 Stufen außerhalb, einmal 3) |
| VOI-CROSSING | Stimmkreuzung. Zwei- und dreistimmig P2, vierstimmig P1. | P2 / P1 | alle Stimmpaare | Mann 100; Korpus (Klänge mit Kreuzung: zweistimmig 2,3 %, dreistimmig 1,3 %, vierstimmig 8,9 %) |
| VOI-SPACING | Abstand auf der Thesis (Werte unten). Zum Bass ist der Abstand bis zur Duodezime frei, darüber P1. | frei / P2 / V / P1 | alle | Trad.; Korpus |
| VOI-OVERLAP | Keine Stimmüberschreitung: Eine Stimme geht nicht über den vorigen Ton ihrer Nachbarstimme hinaus. | P2 vorläufig | alle Stimmpaare | Trad. |

Werte von **VOI-RANGE**:
- Parameter je Stimmtyp und Schlüssel:
  - Sopran C1 (System C4–D5)
  - Alt C3 (F3–G4)
  - Tenor C4 (D3–E4)
  - Bass F4 (G2–A3)
  - hoher Sopran G2 (E4–F5)
- Eine Stufe über das System hinaus ist frei, eine Stufe darunter kostet P3 (Parameter
  `one_step_below`), zwei Stufen P2, drei P4, mehr ist verboten.
- Fux verlässt das System eher nach oben: Eine Stufe darüber liegen im Sopran 6 % der Töne, im Alt
  7 %, eine Stufe darunter im Sopran nie, im Alt 0,1 %, im Tenor 1,1 %, im Bass 1,7 % (alle
  Gegenstimmen 0,8 %; `tools/corpus_stats.py`, Abschnitt „Lage“). Ohne Kosten nach unten rutschten die
  Oberstimmen im langen Satz an den unteren Rand ihres Systems.
- Daraus ergeben sich als freie Umfänge: Sopran C4–E5, Alt F3–A4, Tenor D3–F4, Bass G2–H3.

Werte von **VOI-SPACING**:
- Zweistimmig:
  - bis zur Dezime frei
  - Undezime oder Duodezime P2 (Korpus: 2 von 498 Thesen)
  - darüber V (Korpus: nie)
- Benachbarte Oberstimmen: über eine Oktave P2 (Korpus: dreistimmig 3 %, vierstimmig 1 %).

## Zur 2. bis 5. Gattung

- **Rhythmus** als Taktmuster in der Schreibweise der Tabelle in § 11 (`r2 2`, `⁀2 2⁀`, `4 4 4 4`).
  Jede Gattung hat eine eigene Rhythmusregel mit erlaubten Mustern je Stelle im Stück (Anfang,
  Mitte, vorletzter und letzter Takt). Die Rhythmusregeln gelten für jede Stimmenzahl. Dieselbe
  Tabelle (`crates/fux-core/src/rhythm.rs`) steuert den Generator: Er setzt nur Rhythmen, die das
  Profil nicht verbietet.
- **Dissonanzregeln** (G2-PASSING, G3-PASSING, G3-CAMBIATA, G3-THIRD-QUARTER, G3-RUN, G4-ARSIS,
  G4-RESOLUTION, G5-PASSING, G5-ORNAMENTED-RES, G5-EIGHTHS) gelten für jede Stimmenzahl, mit der
  Dissonanzbestimmung aus § 0. Wo Fux ab drei Stimmen freier ist, gibt es eigene Parameter:
  Quartsextklang bei der Auflösung eines Vorhalts (Fig. 141: G4-ARSIS und G4-RESOLUTION
  `fourth_above_bass`, P3), Ketten von Durchgängen im vierstimmigen Bass (Fig. 186: G3-THIRD-QUARTER
  und G3-RUN `four_voices`, P2). Die verminderte Quinte des verzögerten Leittons im Bass (Fig. 123,
  144, 151, 156) und die verminderte Quarte zwischen Oberstimmen (Fig. 183) sind Teil der
  Dissonanzbestimmung. Die Schlussbindung der 2. Gattung ab drei Stimmen ist ein Vorhalt und löst
  sich nach G4-RESOLUTION auf.
- **Ein Verstoß zählt, wenn er entschieden ist**, also beim Ton, der ihn festlegt: ein Durchgang beim
  folgenden Ton, eine Cambiata zwei Töne später. Gemeldet wird er beim Ton, der gegen die Regel
  verstößt. So zählt die Suche jeden Verstoß genau einmal.

## 8. Zweite Gattung

| ID | Regel | Standard | Quelle |
|---|---|---|---|
| G2-RHYTHM | Zwei Halbe pro Takt, im letzten Takt eine Ganze. Der erste Takt beginnt mit halber Pause; ohne Pause kostet P2. Ab drei Stimmen darf die Gegenstimme im Schluss eine Bindung (`2 2⁀ \| ⁀2 2`) oder eine Ganze im vorletzten Takt haben (Parameter `cadence_tie`, frei). Ein späterer Einsatz ist verboten. | V / P2 | Korpus (11 von 12 zweistimmigen Beispielen mit Pause, ohne Pause nur Fig. 33); Mann 87 f., 93; L24 2.M2 |
| G2-THESIS | Die Thesis ist konsonant. Prüft HAR-CONSONANCE. | V | Ch. 64; W23 2.H1 |
| G2-PASSING | Eine Dissonanz auf der Arsis gibt es nur als Durchgang: Die Thesen liegen eine Terz auseinander, und beide Schritte gehen in dieselbe Richtung. | V | Ch. 64; W23 2.H2 |
| G2-DOWNBEAT-MOTION | Zweistimmig: Liegt die Arsis höchstens eine Terz von der Thesis entfernt, zählt zusätzlich die Bewegung von Thesis zu Thesis, und darauf wird MOT-DIRECT-PERFECT angewandt. Ab drei Stimmen zählen nur parallele Quinten oder Oktaven von Thesis zu Thesis: mit Schritt in der Arsis V (`parallel_many`), mit Terzsprung frei (`third_skip_many`, Mann 86). Quinten auf aufeinanderfolgenden Taktanfängen mit Terzsprung dazwischen bleiben also verboten, mit Quart- oder größerem Sprung sind sie erlaubt. Die Bewegung von der Arsis in die Thesis prüft MOT-DIRECT-PERFECT immer. FuxCP ersetzt sie bei kleinem Abstand durch die Thesis-Bewegung und übersieht so Oktavparallelen wie E5–C5 \| D5 über C4 \| D4. Dreistimmig gilt dagegen die Ausnahme in TEX-SUCCESSIVE-PERFECT. | V | Ch. 65–67; W23 2.P1; Korpus (nie verletzt; Quinten mit Quartsprung dazwischen 3×: Fig. 36, 40, 42) |
| G2-REPEAT | Keine Tonwiederholung. Prüft MEL-REPEAT. | V | W23 2.M2*; Korpus |
| G2-UNISON-ARSIS | Einklang auf der Arsis. HAR-UNISON prüft nur Thesen. | frei | Korpus; W23 |
| G2-CADENCE-TIE | Im dreistimmigen Satz darf die Gegenstimme im Schluss eine Bindung oder Ganze haben. Teil von G2-RHYTHM (`cadence_tie`). | frei | Mann 87 f., 93; L24 2.M2 |

## 9. Dritte Gattung

| ID | Regel | Standard | Quelle |
|---|---|---|---|
| G3-RHYTHM | Vier Viertel pro Takt, im letzten Takt eine Ganze. Eine Viertelpause am Anfang ist zweistimmig verboten, ab drei Stimmen kostet sie P2. Ab drei Stimmen kostet eine Ganze im vorletzten Takt P2 (`cadence_whole`). | V / P2 | Fux; Korpus (Viertelpause nur Fig. 132, Ganze im vorletzten Takt nur Fig. 181, je 1 von 14 Stimmen) |
| G3-THESIS | Das 1. Viertel ist konsonant. Prüft HAR-CONSONANCE. | V | Mann 50 |
| G3-RUN | Gehen fünf Viertel vom 1. Viertel bis zur nächsten Thesis stufenweise in eine Richtung, ist das 3. Viertel konsonant. | V | Mann 50; Ch. 73; lat. 63 („continuò gradatim“); W23 3.H1 |
| G3-THIRD-QUARTER | Ein dissonantes 3. Viertel gibt es nur als Durchgang zwischen konsonantem 2. und 4. Viertel. | V | Ch. 73 f.; W23 3.H2 |
| G3-PASSING | Eine Dissonanz auf dem 2., 3. oder 4. Viertel ist ein Durchgang: stufenweise erreicht und in derselben Richtung stufenweise verlassen. Die Ausnahme ist G3-CAMBIATA. Parameter dissonante Wechselnote auf dem 2. oder 4. Viertel: die untere zweistimmig V, ab drei Stimmen P3; die obere V. Auf dem 3. Viertel ist eine Wechselnote eine betonte Dissonanz (V). Erst Jeppesen erlaubt die untere allgemein. | V / P3 (Jeppesen-Profil: untere Wechselnote frei) | Mann 50; Korpus (zweistimmig 59 Durchgänge, 2 Cambiaten, keine Wechselnote; drei- und vierstimmig je 1 untere Wechselnote im Bass der Schlussklausel auf dem 2. Viertel, Fig. 132, 182; nie eine obere) |
| G3-CAMBIATA | Nota cambiata: Das 2. Viertel ist dissonant und stufenweise abwärts erreicht. Danach folgt ein Terzsprung abwärts zur Konsonanz und dann ein Schritt aufwärts. Fux bevorzugt diese Figur mit Dissonanz gegenüber der konsonanten Variante. Als eigene Regel lässt sie sich mit Kosten belegen oder verbieten. | frei | Mann 51; Ch. 74 f.; W23 3.H3; Korpus (Fig. 55, 56) |
| G3-MOTION | Die Bewegungsregeln (§ 4) gelten vom 4. Viertel zur nächsten Thesis. Umgesetzt in MOT-DIRECT-PERFECT und MOT-PREFERENCE. | V | W23 3.P1* |
| G3-ACCENT-PARALLELS | Quinten oder Oktaven auf aufeinanderfolgenden Taktanfängen, in gerader Bewegung erreicht. | P2 | Trad.; Korpus (zweistimmig: 3. Gattung Fig. 55, 57; 5. Gattung Fig. 85b, 86a) |
| G3-ACCENTED-PASSING | Betonter Durchgang aufwärts: ein dissonantes Viertel auf dem 3. Viertel, stufenweise aufwärts erreicht und verlassen. Gilt auch für Viertel der 5. Gattung. | frei (Jeppesen-Profil: V) | Trad. (Jeppesen: betonte Durchgänge nur abwärts); Korpus (24 von 48 betonten Durchgängen aufwärts) |
| G3-ACCENT-LEAP | Sprung aufwärts von einem betonten Viertel (1. oder 3. Viertel). Gilt auch für Viertel der 5. Gattung. | frei (Jeppesen-Profil: V) | Trad. (Jeppesen); Korpus (63 von 179 Sprüngen von betonten Vierteln gehen aufwärts) |
| G3-TRIAD | Ab drei Stimmen: Fehlt auf der Thesis der vollständige Dreiklang, soll er auf dem 2. oder 3. Viertel entstehen. | P1 | Mann 91; L24 3.H6; Korpus (21 von 122 Takten) |

## 10. Vierte Gattung

| ID | Regel | Standard | Quelle |
|---|---|---|---|
| G4-RHYTHM | Erster Takt: halbe Pause und Halbe (ohne Pause V). Danach wird jede Arsis-Halbe an die nächste Thesis gebunden; Takte ohne Bindung regelt G4-BREAK. Im vorletzten Takt löst sich die letzte Synkope auf (`⁀2 2`), im letzten steht eine Ganze. Ab drei Stimmen: halbe Pause statt Synkope P3 (G4-REST), Einsatz erst nach dem ersten Takt P2. | V / P2 / P3 | Korpus (später Einsatz nur Fig. 144, Pause statt Synkope 1 von rund 117 Takten); W23 |
| G4-ARSIS | Die Arsis ist konsonant. | V | Ch. 78; W23 4.H1 |
| G4-RESOLUTION | Eine dissonante Synkope löst sich stufenweise abwärts in die nächste Konsonanz auf der Arsis auf. | V | Ch. 78–81; W23 4.P1 |
| G4-SUSPENSIONS | Daraus folgen diese Vorhalte: oben 7–6 und 4–3, außerdem 9–8 und 2–1 (siehe G4-OCTAVES); unten 2–3 (bzw. 9–10) und 4–5. Liegt der CF oben, ist die Septime (7–8 unten) verboten. | V | Ch. 78–81; W23 4.H2; Korpus (zweistimmig: oben 7–6 9×, 4–3 3×; unten 2–3 9×, 4–5 5×) |
| G4-OCTAVES | Liegt der CF unten, folgt auf einen Einklang oder eine Oktave auf der Arsis keine Sekunde oder None. Das wären versteckte Oktavparallelen, etwa in 9–8-Ketten. | V | Ch. 79 f.; W23 4.P2 |
| G4-FIFTHS | Quintenfolgen auf der Arsis, die durch Synkopen getrennt sind. | frei | Mann 57 („Successions of fifths may therefore be used with syncopations“), 95 |
| G4-BREAK | Bindung lösen (ein Takt ohne Synkope) nur im Notfall, z. B. gegen Monotonie. Der Sprung Arsis→Thesis ist dann höchstens eine kleine Sexte. | P4 vorläufig | Fux; W23 4.M1 |
| G4-REST | Dreistimmig darf statt einer Synkope eine Pause stehen, um verdeckte Quinten zu vermeiden. Teil von G4-RHYTHM (`rest`). | P3 | Mann 98; L24 4.P6; Korpus (Fig. 151) |
| G4-SPLIT | Vierstimmig dürfen die Ganzen der anderen Stimmen geteilt werden. | frei | Mann 129; Korpus (Fig. 195, 197, 202, 203) |

## 11. Fünfte Gattung

Jede Note folgt den Regeln der Gattung, aus der sie stammt: Auf der Thesis angeschlagene Töne sind
konsonant (HAR-CONSONANCE), Synkopen behandeln G4-SUSPENSIONS, G4-OCTAVES und G5-ORNAMENTED-RES,
Viertel und Halbe G5-PASSING, Achtel G5-EIGHTHS.

| ID | Regel | Standard | Quelle |
|---|---|---|---|
| G5-ELEMENTS | Der blühende Kontrapunkt mischt die Elemente der 2.–4. Gattung: ungebundene Halbe, Viertel, Synkopen, dazu Achtelpaare. FuxCP lässt nur 3. und 4. Gattung zu, Fux verwendet aber auch ungebundene Halbe. Umgesetzt durch die Regeln dieses Abschnitts. | V | Fux; Korpus |
| G5-RHYTHM | Taktrhythmen nach Stelle im Stück. Erster Takt: halbe Pause und eine in Takt 2 gebundene Halbe (`r2 2⁀`); ohne Bindung P1. Innere Takte: Kosten nach Häufigkeit bei Fux (Tabelle unten): mindestens 10× belegt frei, 3–9× P1 (`occasional`), 1–2× P2 (`rare`), unbelegt V (als Option P4). Vorletzter Takt `⁀2 2`, ab drei Stimmen auch `⁀2 4 4` (P1). Letzter Takt eine Ganze. Ersetzt die früheren Regeln G5-START und G5-PATTERNS. | V / P1 / P2 | Korpus; W23 5.R6 |
| G5-PASSING | Ein dissonant angeschlagenes Viertel oder eine dissonante Halbe außerhalb der Thesis ist ein Durchgang. Ausnahmen in Vierteln: Nota cambiata (frei) und die dissonante untere Wechselnote auf dem 2. oder 4. Viertel (P3; Jeppesen-Profil: frei); die obere ist V. Eine Synkope beginnt immer konsonant. | V / P3 | Fux (jede Note nach ihrer Gattung); Korpus (zweistimmig: 48 Durchgänge in Vierteln, 1 Cambiata in Fig. 87b, 1 untere Wechselnote in Fig. 86b; Halbe und Synkopen nie dissonant angeschlagen) |
| G5-EIGHTHS | Achtel stehen paarweise auf dem 2. oder 4. Viertel (über G5-RHYTHM). Sie werden stufenweise erreicht und verlassen. Dissonant ist ein Achtel auf dem Viertelschlag nur als Durchgang, unbetont als Durchgang oder untere Wechselnote. Ab drei Stimmen darf das Paar per Terzsprung abwärts erreicht werden, wenn es schrittweise zum Ausgangston zurückführt (A–F–G–A, P1). | V / P1 | Mann 63; Korpus (zweistimmig alle Achtel stufenweise; dissonant 1× betont als Durchgang, unbetont 2× Durchgang, 7× untere Wechselnote, nie obere; dreistimmig 4× Terzsprung in das Paar, Fig. 155, 158) |
| G5-ORNAMENTED-RES | Eine dissonante Synkope löst sich stufenweise abwärts in eine Konsonanz auf: direkt mit dem nächsten Ton, oder nach einer gehaltenen Viertel auf dem 3. Viertel. Dazwischen steht dann ein Viertel oder ein Achtelpaar (verzierte Auflösung), und die Verzierung beginnt abwärts, mit dem Auflösungston oder einem konsonanten Sprung. | V | Ch. 85; W23 Gl. 7.13; Korpus (zweistimmig: gehaltene Halbe 20× direkt; gehaltene Viertel 12× direkt, davon 9× als erstes Achtel eines Paars, und 7× nach Quintsprung abwärts auf dem 3. Viertel) |
| G5-VARIETY | Rhythmische Abwechslung: Mehr als zwei Takte in Folge ohne Bindung kosten P2 je Takt (vorläufig). Ein innerer Takt mit demselben Rhythmus wie der vorige kostet P2. | P2 | W23 (Programmcode); Korpus (Takte ohne Bindung zweistimmig meist 0 oder 1 in Folge, höchstens 3 in Fig. 88a; gleicher Rhythmus 4 von 88 Taktpaaren) |

Taktrhythmen im Korpus (Stimme der 5. Gattung). Die Zahlen sind Notenwerte (2 = Halbe, 4 = Viertel,
8 = Achtel, 2. = punktierte Halbe). Ein ⁀ vorn bedeutet: aus dem Vortakt übergebunden; ⁀ hinten: in
den nächsten Takt gebunden. Kosten nach den zweistimmigen Zahlen.

| Rhythmus | zweistimmig | drei-/vierstimmig | Kosten |
|---|---|---|---|
| `4 4 2⁀` | 22 | 8 / 8 | frei |
| `⁀4 4 2⁀` | 22 | 8 / 4 | frei |
| `⁀4 4 4 4` | 10 | 4 / 5 | frei |
| `⁀2 4 4` | 8 | 1 / – | P1 |
| `2 2⁀` | 7 | 3 / 1 | P1 |
| `4 4 4 4` | 6 | 5 / – | P1 |
| `⁀4 8 8 2⁀` | 5 | 3 / 1 | P1 |
| `⁀4 8 8 4 4` | 5 | 3 / – | P1 |
| `2 4 4` | 4 | 2 / – | P1 |
| `⁀2 2⁀` | 3 | 3 / 9 | P1 |
| `⁀2 2` | 3 | – / 3 | P1 |
| `2 2` | 2 | – / – | P2 |
| `⁀4 4 4 8 8` | 1 | – / – | P2 |
| `4 8 8 4 4` | 1 | 2 / – | P2 |
| `4 4 2` | 1 | – / – | P2 |
| `2. 4` | – | 3 / – | ab drei Stimmen P2, sonst V |
| `⁀4 4 2` | – | – / 1 | ab drei Stimmen P2, sonst V |

Erster Takt: `r2 2⁀` zweistimmig 10×, `r2 2` 2× (Fig. 84b, 86b). Vorletzter Takt: zweistimmig immer
`⁀2 2`, dreistimmig auch `⁀2 4 4` (2 von 6).

## 12. Drei- bis sechsstimmiger Satz

Ab drei Stimmen gelten die Regeln oben weiter. Was zweistimmig „gegen den CF“ heißt, bezieht sich
jetzt auf den Bass (§ 0). Fux' Schema: Eine Stimme steht in Gattung *n*, die übrigen in Ganzen.
Gemischte Gattungen sind selten (Fig. 134; Fig. 204 mit 2., 3. und 4. Gattung). Für fünf und sechs
Stimmen gibt es bei Fux keine Beispiele; die Werte sind aus dem Verlauf von zwei über drei zu vier
Stimmen hochgerechnet und vorläufig (siehe unten).

| ID | Regel | Standard | Gilt für | Quelle |
|---|---|---|---|---|
| TEX-TRIAD | Klangtyp auf der Thesis gegen den Bass, wenn alle Stimmen anschlagen (innere Takte). Fux: Der vollständige Dreiklang „should be employed in every measure if there is no special reason against it“. Kosten je Klangtyp siehe unten; die leere Sexte (Sexte und Oktave ohne Terz) bewertet nur der freie Satz (`free_bare_sixth`, § 16). | frei / P1 / P3 / V | 3st+ | Mann 71 f., 109; L24 1.H8, 1.H9; L25 1.H8; Korpus |
| TEX-DOUBLING | Verdopplung im vollständigen Dreiklang ab vier Stimmen: Grundton frei, Terz P1, Quinte vierstimmig P3, ab fünf Stimmen P1. Eine verdoppelte Terz hat in beiden Stimmen dieselbe Qualität (HAR-CHROMATIC-CLASH). Die „natürliche Lage“ 5–8–10 über dem Bass ist frei, ohne Vorzug; FuxCP bevorzugt sie. | frei / P1 / P3 | 4st+ | Mann 109, 111; L25; Korpus (Grundton 55×, Terz 25×, Quinte 2×; natürliche Lage 18 von 82) |
| TEX-PARALLEL | Keine parallelen Einklänge, Quinten oder Oktaven zwischen beliebigen Stimmpaaren. FuxCP (L24, L25) behandelt sie nur als Kosten. | V | 3st+ | Korpus (in allen 77 Beispielen nie); B26 P8 |
| TEX-DIRECT-PERFECT | Verdeckte Quinten und Oktaven (gerade Bewegung in eine vollkommene Konsonanz). Werte unten. | P3 / P2 / P1 / frei | 3st+ | Mann 77 („if there is no other possibility“), 109, 123; L24 1.P1; L25; Korpus |
| TEX-SUCCESSIVE-PERFECT | Zwei vollkommene Konsonanzen nacheinander im selben Stimmpaar von Thesis zu Thesis, in Gegen- oder Seitenbewegung (z. B. Quinte → Oktave); gerade Bewegung regeln TEX-PARALLEL und TEX-DIRECT-PERFECT, gleiche Konsonanz in Gegenbewegung MOT-ANTIPARALLEL. Quinten von Thesis zu Thesis mit Terzsprung in der 2. Gattung sind frei (G2-DOWNBEAT-MOTION). | P1 vorläufig | 3st+ | Mann 72, 86; L24 1.P4, 2.P3 (Fux hält sich oft nicht daran: Korpus 317 Treffer) |
| TEX-SAME-DIRECTION | Alle Stimmen gleichzeitig in dieselbe Richtung, gemessen vom letzten Ton vor dem Taktanfang. Dreistimmig P2, ab vier Stimmen V. | P2 / V | 3st+ | Mann 76 („can hardly be managed without awkwardness“); L24 1.P6; Korpus (1. Gattung: dreistimmig 10 von 171 Klangwechseln, vierstimmig 0 von 111) |
| TEX-SIXTHS-UP | Aufeinanderfolgende Sexten zum Bass in steigender Bewegung. | P3 | 3st+ | Mann 77 („Ascending sixths on the downbeat sound harsh“); L24 1.P7; Korpus (dreistimmig 1×, vierstimmig nie) |
| TEX-START-DISTANT | Die Oberstimmen beginnen weit vom Bass entfernt, damit Raum für Gegenbewegung bleibt. **Nicht umgesetzt:** Im Korpus liegt die nächste Oberstimme im ersten Klang im Einklang (6×), in der Terz (6×), Quinte (6×) oder Oktave (9×) über dem Bass, also ohne Tendenz; FuxCP setzt sie ebenfalls nicht um (L24 Abschn. 4.1.2: als Heuristik ohne Nutzen verworfen; L25 1.P5). | – | – | Mann 75; L24 1.P5 |
| TEX-FIRST-CHORD | Erster Klang (sobald alle Stimmen eingesetzt haben): Bass auf der Finalis, darüber nur Einklang/Oktave, Terz und Quinte. Eine Terz ist frei; anders als zweistimmig sind nicht nur vollkommene Konsonanzen nötig. | V | 3st+ | L24 S. 26; Korpus (Bass immer auf der Finalis, nie eine Sexte darüber; Terz in 23 von 45 bzw. 24 von 32 ersten Klängen) |
| TEX-PENULT | Vorletzter Klang (der letzte vor dem Schlusstakt). `level`, dreistimmig: Jede Stimme bildet zum Bass Einklang/Oktave, kleine oder große Terz, Quinte oder große Sexte, oder die verminderte Quinte des verzögerten Leittons im Bass (V vorläufig). `four`, ab vier Stimmen: eine Stimme dissonant zum Bass (P2). `bass`: Der Bass führt mit einem Kadenzschritt in die Finalis, von der 5. Stufe, von der 2. Stufe (Tenorklausel, Cantus im Bass) oder als verzögerter Leitton; phrygisch von der 7. Stufe (D) oder der 2. (F) (V). Schon auf der Thesis des vorletzten Takts steht der Bass auf einer dieser Stufen oder hält die Finalis über den Taktstrich; so verwirft die Suche aussichtslose Schlüsse früh. | V / P2 / V | 3st+ | L25 1.H7 (aus Fux' Beispielen abgeleitet, große Terz eingeschlossen); CdP24 S. 30; Korpus (Bass in allen 77 Beispielen: 5. Stufe 38×, 2. Stufe 14×, verzögerter Leitton 6×, phrygisch D 17×, F 2×; vierstimmig eine Dissonanz zum Bass in 32 Beispielen, die Durchgangsseptime in Fig. 185) |
| TEX-CLAUSULA | Klausel: Im vorletzten Klang hat eine Stimme den Leitton der Finalis (phrygisch: den Halbton darüber), und sie erreicht die Finalis im Halbtonschritt. Parameter `doubled`: der Leitton in mehreren Stimmen (V). Singt eine Gegenstimme die unerhöhte Stufe in den Schlussklang hinein, ist kein Leitton mehr möglich; das wird gemeldet, sobald der Ton steht. Zweistimmig regelt das CAD-PENULT. | V | 3st+ | Fux; Korpus (in allen 77 drei- und vierstimmigen Beispielen genau eine Stimme mit dem Leitton, immer in die Finalis aufgelöst) |
| TEX-FINAL-CHORD | Schlussklang: Bass auf der Finalis, nur Dreiklangstöne, keine kleine Terz (alle V). Parameter `third` (große Terz; Einstellung „vermeiden“ wie Fux' Text: P2) und `missing_third` (keine Terz; Einstellung „bevorzugt“: P2), beide frei. Eine große Terz als Dezime oder höher ist frei. Im freien Satz gilt statt `missing_third` der Parameter `free_missing_third`: Ein Schlussklang ohne große Terz kostet dort P2 (§ 16). | V; Terz frei, im freien Satz ohne große Terz P2 | 3st+ | Mann 77, 80 („The minor third is not capable of giving a sense of conclusion“), 82; L25 1.H3, 1.H12; Korpus (Terz dreistimmig in 19 von 45, vierstimmig in 18 von 32 Schlussklängen, immer groß; als Dezime oder höher dreistimmig 8×, vierstimmig 16×); Korpus Palestrina (ohne große Terz 110 von 1156 Schlussklängen) |

Kosten von **TEX-TRIAD** je Klangtyp in den Gattungen (im freien Satz siehe § 16):

| Klangtyp | dreistimmig | ab vier Stimmen |
|---|---|---|
| vollständiger Dreiklang (5/3) | frei | frei |
| Terz mit Oktave, ohne Quinte | P1 (Korpus 33 %) | P3 (Korpus 1 %) |
| Sextklang (6/3) | P1 (Korpus 17 %) | P1 (Korpus 17 %) |
| ohne Terz (Quinte und Oktave) | P3 (Korpus nie, siehe unten) | V (Korpus nie) |
| leere Sexte (Sexte und Oktave, ohne Terz) | frei (Korpus 2 %) | frei (Korpus nie) |

Gezählt sind die inneren Klänge der Beispiele der 1. Gattung, dreistimmig 155, vierstimmig 100
(`tools/corpus_stats.py`). Eine frühere Fassung des Skripts zählte jeden Klang ohne Terz als „ohne
Terz“, auch die leere Sexte, während TEX-TRIAD dort nur Quinte und Oktave bewertet und die leere
Sexte nie: Die drei Klänge, nach denen „ohne Terz“ dreistimmig P3 bekam (2 %), sind leere Sexten
(Fig. 104 T. 3, 115 T. 6, 119 T. 3). Quinte und Oktave ohne Terz schreibt Fux auf keinem inneren
Taktanfang, an dem alle Stimmen anschlagen, auch nicht in der 2. bis 5. Gattung: In seinen 123
Beispielen findet TEX-TRIAD 99-mal Terz und Oktave dreistimmig, 4-mal vierstimmig und 60-mal den
Sextklang, sonst nichts. Seither zählt das Skript beide Klassen getrennt, wie der Code sie
unterscheidet. Die Stufen der Gattungen bleiben, wie sie waren: Quinte und Oktave ohne Terz
dreistimmig P3, wo die Leitlinie nach diesen Zahlen V gäbe, und die leere Sexte frei, wo die
Leitlinie dreistimmig P3 und vierstimmig V gäbe. Sie neu zu eichen änderte Gattungssätze,
Beispiel 10 und das Übungsheft. Im freien Satz kostet die leere Sexte P2 (§ 16).

Werte von **TEX-DIRECT-PERFECT**:

| Stimmpaar | dreistimmig | vierstimmig | ab fünf Stimmen |
|---|---|---|---|
| Außenstimmen, im Satz | P3 (Korpus 6×) | P2 (Korpus 20×) | P1 |
| andere Stimmpaare, im Satz | P2 (Korpus 13×) | P1 (Korpus 44×) | frei |
| in den Schlussklang | P1 (Korpus 22×) | P1 (Korpus 21×) | P1 |

**Hochrechnung für fünf und sechs Stimmen** (vorläufig: Fux hat keine fünf- und sechsstimmigen
Beispiele; den freien Satz eicht § 16 an Palestrinas Messen): Wo die Kosten von zwei über drei zu
vier Stimmen fallen, fallen sie eine Stufe weiter (verdeckte Parallelen, Einklang auf der Thesis
frei wie bei Gallon/Bitsch Regel 53, verdoppelte Quinte P1). Wo sie steigen oder ein Verbot
entsteht, gilt der vierstimmige Wert weiter (alle Stimmen in eine Richtung V, Klang ohne Terz V,
Stimmkreuzung P1).

## 13. Abweichungen in Fux' eigenen Beispielen

Die Standardwerte sind so gewählt, dass diese Stellen höchstens weiche Regeln verletzen. Der
Kalibrierungstest prüft das (§ 14).

- **Phrygischer Cantus**: Er reicht bis A, eine Quinte unter die Finalis E. Deshalb ist CF-AMBITUS weich.
- **Mixolydischer Cantus**: Er hat mehr Sprünge als Schritte. Deshalb ist CF-STEPS nur gering gewichtet.
- **Varianten der Cantus**: Fig. 42 (äolisch, ohne das vierte D) und Fig. 88a/b (ionisch, 9 Töne).
  Sie stehen als eigene Einträge in `tests/fixtures/fux/cantus.toml`.
- **2. Gattung**: Ein Oktavsprung Thesis→Arsis bei weit auseinanderliegenden Stimmen (MEL-OCTAVE).
  Fig. 33 beginnt ohne halbe Pause (G2-RHYTHM). Fig. 37 (phrygisch, Gegenstimme unten) hat auf der
  vorletzten Thesis eine Sexte statt der Quinte, die es dort nicht gibt (CAD-PENULT-G2).
- **Chromatische Halbtonschritte** im drei- und vierstimmigen Satz (Fig. 110, 169, 170).
- **Dissonante Wechselnote** im Bass der Schlussklausel (Fig. 132, 182) und **Tonwiederholungen** in der
  Gattungsstimme (Fig. 173, 181), nur ab drei Stimmen.
- **Tonwiederholung in der 5. Gattung**: Fig. 156 (dreistimmig) beginnt im Bass mit Halbe D, Taktstrich,
  Viertel D, ohne Haltebogen; das Faksimile S. 113 bestätigt die Übertragung.
- **5. Gattung**: Fig. 84b und 86b beginnen ohne Bindung, Fig. 86b hat eine dissonante untere
  Wechselnote in Vierteln, Fig. 88a drei Takte ohne Bindung in Folge (alles Kosten, keine Verbote).
  Dreistimmig springt Fux in Achtelpaare (Fig. 155, 158) und schreibt `2. 4`.
- **3. Gattung**: Der vorletzte Takt folgt nicht immer CAD-PENULT-G3. Mann 54 notiert eine handschriftliche Korrektur.
- **4. Gattung**: Fux löst Bindungen gegen Monotonie und kreuzt Stimmen (W23 S. 64; G4-BREAK 2×).
  Dreistimmig setzt er eine Pause statt einer Synkope (Fig. 151) und lässt die Gegenstimme erst im
  zweiten Takt einsetzen (Fig. 144).
- **Dissonanzen ab drei Stimmen** (2.–5. Gattung): siehe [Zur 2. bis 5. Gattung](#zur-2-bis-5-gattung).
- **Drei- und vierstimmig**:
  - Aufeinanderfolgende vollkommene Konsonanzen (Fig. 108–110; TEX-SUCCESSIVE-PERFECT 317×).
  - Verdeckte Oktaven, vor allem in die Schlussoktave.
  - Einklänge im vierstimmigen Satz (Fig. 166).
  - Verminderte Quinte zum verzögerten Leitton im Bass (Fig. 123, 144, 151, 156), Quartsextklang bei
    einer Vorhaltsauflösung (Fig. 141), Durchgangsketten im vierstimmigen Bass (Fig. 186), mehrere
    bewegte Stimmen zugleich (Fig. 204): siehe § 0 und [Zur 2. bis 5. Gattung](#zur-2-bis-5-gattung).
- **Abweichende Übertragungen**: FuxCP und Gotham unterscheiden sich in Fig. 55 (Takt 8: H oder B) und
  Fig. 56 (Takt 10: C oder C♯). Der Korpus folgt Gotham, also dem Druck von 1725.

## 14. Kalibrierung

Der Test `crates/fux-core/tests/calibration.rs` prüft alle Regeln an Fux' 8 Cantus und an allen 123
Beispielen. Mit dem Standardprofil verstößt keines davon gegen ein Verbot. Die Cantus allein kosten
1–5 Punkte, Fux' zweistimmige Lösungen der 1. Gattung 18–45 Punkte (mit Cantus). Ein zweiter Test
prüft jedes Beispiel Fenster für Fenster, wie die Suche es tut, und verlangt dieselben Verstöße wie
bei der Prüfung des ganzen Satzes, auch an verfälschten Kopien der Beispiele, in denen jede dritte
Note der Gegenstimmen um einen Ganzton verschoben ist. Wie oft jede Regel im Korpus anschlägt,
zeigt `fux rules --corpus` (gezählt, bevor der freie Satz und die Regeln des Textes hinzukamen;
die heutigen Zahlen gibt der Befehl selbst aus):

| Regel | Treffer | Regel | Treffer |
|---|---|---|---|
| MEL-LEAP-COST | 1294 | MEL-CONSEC-LEAPS | 35 |
| MOT-PREFERENCE | 902 | G3-TRIAD | 21 |
| MEL-VARIETY | 518 | MEL-TRITONE-OUTLINE | 18 |
| TEX-SUCCESSIVE-PERFECT | 317 | HAR-UNISON (vierstimmig) | 16 |
| MEL-REPEAT | 261 | TEX-SAME-DIRECTION | 11 |
| TEX-TRIAD | 163 | G5-VARIETY | 10 |
| CF-LEAP-COST | 128 | MEL-MOTIF | 9 |
| TEX-DIRECT-PERFECT | 126 | CF-STEPS | 8 |
| VOI-OVERLAP | 113 | G5-EIGHTHS, G3-THIRD-QUARTER, G3-ACCENT-PARALLELS | je 4 |
| CF-VARIETY | 99 | MEL-AUG-DIM, G3-RUN, G2-RHYTHM, FIC-CROSS-RELATION | je 3 |
| VOI-CROSSING | 96 | G4-RHYTHM, G4-BREAK, G3-RHYTHM, G3-PASSING | je 2 |
| G5-RHYTHM | 93 | TEX-SIXTHS-UP, MOT-ANTIPARALLEL, MEL-OCTAVE | je 1 |
| HAR-PERFECT-COST | 92 | G5-PASSING, G4-RESOLUTION, G4-ARSIS | je 1 |
| MEL-CLIMAX | 82 | CAD-PENULT-G2, CAD-PENULT-G3, TEX-PENULT | je 1 |
| FIC-BFLAT | 81 | | |
| VOI-RANGE | 64 | | |
| TEX-DOUBLING | 58 | | |
| MEL-LEAP-RECOVERY | 44 | | |
| VOI-SPACING | 31 | | |
| FIC-INNER-CADENCE | 29 | | |

Alle übrigen Regeln schlagen bei Fux nie an. Darunter sind HAR-CONSONANCE, HAR-DISSONANCE,
HAR-CHROMATIC-CLASH, MOT-DIRECT-PERFECT, TEX-PARALLEL, G2-DOWNBEAT-MOTION, G5-ORNAMENTED-RES, die
Klang- und Klauselregeln TEX-FIRST-CHORD, TEX-CLAUSULA und TEX-FINAL-CHORD, alle übrigen
CAD-Regeln, FIC-SUBSEMITONIUM und FIC-SCOPE. G5-RHYTHM zählt die Kosten seltener Taktmuster bei Fux
selbst mit.

Ein dritter Test baut jedes Beispiel so auf, wie die Suche es tut: Die Stimmen außer dem Cantus
wachsen Zeitpunkt für Zeitpunkt, und je Schritt zählt nur, was seit dem vorigen entschieden ist. Er
verlangt dieselben Verstöße wie eine Prüfung der ganzen Partitur (ohne die Regeln über das ganze
Stück). So fällt jede Regel auf, die einen Verstoß früher meldet, als er feststeht.

Die als „vorläufig“ markierten Werte passen zu diesen Zahlen, solange man sie gegen die Gelegenheiten
setzt (Tausende Intervalle und Klänge).

Die TXT-Regeln (§ 17) und die IMI-Regeln (§ 18) schweigen auf Fux' Beispielen und auf Palestrinas
Sätzen, denn keine dieser Partituren trägt einen Text des Satzes oder einen Imitationsplan (Tests
`untexted_corpora_have_no_text_cases` für beide Korpora, `txt_rules_are_silent_on_the_corpus` für
Palestrina und `imi_rules_are_silent_on_the_corpus` für beide). Die Zahlen hier und in § 16 bleiben
deshalb gleich.

## 15. Jeppesen-Profil

Das Profil `jeppesen` ([profiles/jeppesen.toml](../profiles/jeppesen.toml)) übernimmt alle Werte von
Fux und ändert nur, worin Knud Jeppesen (*Kontrapunkt. Lehrbuch der klassischen Vokalpolyphonie*,
Leipzig 1935) von Fux abweicht. Jeppesen lehrt den Kontrapunkt nach Palestrinas Praxis. Er ist
strenger in der Melodik und bei betonten Vierteln, erlaubt aber die untere Wechselnote. Das Lehrbuch
liegt nicht im Repo, weil es urheberrechtlich geschützt ist. Die Werte folgen den bekannten Regeln
des Lehrbuchs und sind **vorläufig, am Lehrbuch zu prüfen**. Die Leitlinie für Standardwerte gilt
hier nicht, denn das Profil soll Jeppesens Regeln abbilden, nicht Fux' Praxis.

Geladen wird es mit `--profile jeppesen` oder in der GUI unter „Regeln …“ → „Jeppesen“.

| Regel | Fux | Jeppesen | Begründung | Bei Fux |
|---|---|---|---|---|
| MEL-MIN6-DOWN | frei | V | Die kleine Sexte nur aufwärts. | 8× |
| MEL-LEAP-RECOVERY | P2 | P3 | Große Sprünge werden durch Gegenbewegung ausgeglichen. | 44× |
| MEL-CONSEC-LEAPS `larger_above` | P2 | P2 | Zwei Sprünge in dieselbe Richtung: der größere unten, also aufwärts zuerst, abwärts zuletzt. Früher V; Palestrina zeigt eine Tendenz, kein Verbot (siehe unten). | 10× |
| G3-PASSING `neighbor_two_voices`, `neighbor` | V, P3 | frei | Die untere Wechselnote auf unbetontem Viertel ist bei Palestrina üblich. | 2× |
| G5-PASSING `neighbor` | P3 | frei | Dasselbe in Vierteln der 5. Gattung. | 1× |
| G3-ACCENTED-PASSING | frei | V | Ein dissonantes 3. Viertel nur als Durchgang abwärts. | 24× |
| G3-ACCENT-LEAP | frei | V | Kein Sprung aufwärts von einem betonten Viertel. | 63× |

„Bei Fux“ zählt, wie oft Fux' 123 Beispiele gegen den Wert Jeppesens verstoßen
(`fux rules --corpus --profile jeppesen`). Dass Fux' dritte Gattung so oft von betonten Vierteln
aufwärts springt, ist Jeppesens Hauptkritik an Fux' Viertelbewegung.

In beiden Profilen gleich sind:
- die obere dissonante Wechselnote (V, bei Fux nie),
- eine dissonante Wechselnote auf dem 3. Viertel (V),
- die Nota cambiata (frei),
- höchstens drei parallele Terzen oder Sexten im zweistimmigen Satz,
- das Verbot verdeckter Quinten und Oktaven im zweistimmigen Satz.

Offen und am Lehrbuch zu prüfen sind Jeppesens Regeln zu diesen Punkten:
- Rhythmik der 5. Gattung: Halbe auf unbetonten Zeiten, Zahl der Achtelpaare.
- Vorhalte 9–8 und 4–5.
- Oktavsprung in der 1. Gattung.
- Höchstabstand der Stimmen.
- Tonwiederholung.

Mit dem Profil gelingen alle fünf Gattungen über und unter jedem Cantus von Fux (Test
`every_species_after_jeppesen`), ebenso drei- und vierstimmige Sätze (Stichprobe aller Cantus und
Gattungen über die Kommandozeile).

**Am Korpus Palestrinas geprüft.** Jeppesens Regeln lassen sich an Palestrinas Messen
messen (§ 16; `fux rules --corpus palestrina --profile jeppesen`). In 1156 Sätzen
springt Palestrina nur 29-mal von einem betonten Viertel aufwärts, schreibt 7 betonte Durchgänge
aufwärts und 7 kleine Sexten abwärts: Diese drei Regeln Jeppesens bestätigt der Korpus, sie gelten
deshalb auch im freien Satz als Verbote. Die untere Wechselnote (6921×) ist üblich, die obere selten
(222×). Zwei Sprünge in dieselbe Richtung mit dem größeren oben kommen dagegen in 272 von 2867
Dreiklangsbrechungen vor (9,5 %, bei Fux 6,6 %): Das ist eine Tendenz, kein Verbot; das Profil setzt
sie deshalb von V auf P2.

## 16. Freier Satz (Palestrina)

Im **freien Satz** sind alle Stimmen blühend wie die Gattungsstimme der 5. Gattung, aber in
Palestrinas Rhythmus und ohne Cantus firmus. Er ist die Grundlage der Messsätze (§ 17–19). Die
Standardwerte stammen aus Palestrinas Messen: 1156 Sätze und Abschnitte in 4/2 mit drei bis sechs
Stimmen, zusammen 519 764 Stimmen-Takte (Korpus siehe [Quellen](../references/README.md)).
`fux_core::kern` liest die Humdrum-Dateien. Ein 4/2-Takt zählt als zwei 2/2-Takte, eine Note über
die Taktmitte wird übergebunden; Taktangaben beziehen sich deshalb auf 2/2-Takte (Takt 39 ist die
erste Hälfte von Palestrinas Takt 20). Abschnitte im Dreiertakt, doppelchörige Sätze und sieben
Dateien mit Triolen oder offenem Schluss bleiben außen vor.

**Leitlinie.** Wie bei Fux richtet sich der Standardwert danach, wie oft Palestrina gegen eine Regel
verstößt, gemessen an den Gelegenheiten. Der Korpus ist aus MIDI-Dateien übertragen und enthält die
Ficta der Ausgabe. Fälle unter etwa 0,1 % der Gelegenheiten sind deshalb oft Kodierungs- oder
Ficta-Fehler: Ein Verbot bleibt Verbot, seine Fälle sind unten gezählt. Der Test
`palestrina_breaks_only_the_counted_prohibitions` hält die Zahlen fest; `fux rules --corpus
palestrina` zeigt alle Häufigkeiten.

### Regeln des freien Satzes

| ID | Regel | Standard | Quelle |
|---|---|---|---|
| FREE-RHYTHM | Taktrhythmen nach Palestrinas Häufigkeit (Tabelle unten): ab 1 % der Takte frei, darunter vier Stufen: 0,1–1 % P1 (`uncommon`), 0,01–0,1 % P2 (`occasional`), 0,001–0,01 % P3 (`rare`), seltener P4 (`very_rare`), nicht belegt V. Die Stufe 1–5 % (`frequent`) steht auf aus; mit P1 werden die Sätze ruhiger. Im letzten Takt eine Ganze (gehalten oder angeschlagen) oder eine Pause, selten noch Bewegung. Ein Takt Pause ist überall frei. | V / P1–P4 | Korpus Palestrina |
| FREE-PASSING | Eine Dissonanz außerhalb der Thesis ist ein Durchgang oder eine dieser Figuren: untere Wechselnote (frei, 6921×), obere Wechselnote (P2, 222×), Nota cambiata, auch vom 4. Viertel über den Taktstrich (frei, 875×), **konsonante Quarte** über dem liegenden Bass, schrittweise erreicht und verlassen oder als Vorhalt 4–3 gehalten (frei, 4602×), Vorausnahme (P1, 220×), ein wiederholter Ton, der dissonant wird und schrittweise abwärts geht (P1, 120×). Jede andere Dissonanz ist V. | V / frei / P1 / P2 | Korpus Palestrina; Jeppesen (konsonante Quarte, Portament) |
| FREE-EIGHTHS | Wie G5-EIGHTHS; die dissonante untere Wechselnote darf auch auf das 2. oder 4. Viertel fallen. | V | Korpus Palestrina |
| FREE-RESOLUTION | Wie G5-ORNAMENTED-RES. Die Auflösung darf in die Quarte über dem liegenden Bass gehen, die dann 4–3 weitergeht (`fourth_above_bass`, frei). Geprüft wird jeder Taktanfang, über den eine Note gehalten wird, auch bei einer Note, die auf dem Taktanfang begann; der Bass gilt dabei nur als Vorhalt, wenn eine Oberstimme eine Sekunde, Septime oder einen Tritonus gegen ihn anschlägt (siehe unten). Eine Dissonanz, die über den nächsten Taktanfang gehalten wird, ist kein Vorhalt. | V | Korpus Palestrina |
| FREE-ENTRY | Der erste Ton einer Stimme liegt auf der Finalis oder der Quinte (frei), auf der Quarte (P1, meist Imitation in der Unterquinte) oder einer anderen Stufe (P2); er ist mindestens eine Ganze lang (kürzer `short`, P2). Nach einer Pause setzt eine Stimme mit mindestens einer Halben ein (`reentry_short`, P3). | P1 / P2 / P3 | Korpus Palestrina (erste Töne in Sätzen, die auf der Finalis enden: Finalis 44 %, Quinte 33 %, Quarte 12 %, andere Stufen je 2–7 %; kürzer als eine Ganze 8 %; nach einer Pause kürzer als eine Halbe 0,7 %) |
| FREE-LONG | Eine Note länger als eine Ganze (`over_whole`, P1); länger als eine Brevis zusätzlich je Takt darüber hinaus (P3). Die letzte Note jeder Stimme ist frei. | P1 / P3 je Takt | Korpus Palestrina (länger als eine Ganze 5,3 % der Noten, länger als eine Brevis 0,47 %) |
| FREE-AMBITUS | Sopran und Tenor bewegen sich in der Oktave des Modus (authentisch von der Finalis, plagal von der Unterquarte aus), Alt, Bariton und Bass in der anderen; die Oktave liegt dort, wo ihre Mitte der Mitte der Stimmlage am nächsten ist. Ein Ton mehr als eine Stufe außerhalb kostet je Ton. Dadurch liegt ein plagaler Satz eine Quarte tiefer als ein authentischer. | P3 / P2 je Ton | Zarlino, Glarean; Korpus Palestrina (in Sätzen, die auf der Finalis enden, nach dem Tenor 611 authentisch, 247 plagal: außerhalb Tenor 0,3 %, Cantus 1,4 %, Altus 3,3 %, Bassus 1,9 %, Bariton 0,7 % der Töne; im jeweils anderen Ambitus 8–17 %) |
| FREE-REST | Jede Pause nach dem ersten Einsatz einer Stimme (höchstens ein Takt, denn Noten und Pausen überschreiten den Taktstrich nicht). Palestrinas Stimmen pausieren oft; die Kosten halten die Suche an, es nur dort zu tun, wo es hilft. In einer Stimme mit Text (`texted`, frei, vorläufig) entscheidet der Text, wo sie pausiert (TXT-REST, § 17); mit P1 pausierten Stimmen mit Text kaum (0,09 Pausen je 10 Stimmtakte, ohne Text 0,31). | P1 je Pause, mit Text frei | Korpus Palestrina (zwischen erstem und letztem Ton pausieren die Stimmen dreistimmig 8 %, vierstimmig 14 %, fünfstimmig 15 %, sechsstimmig 19 % der Zeit; die Hälfte der Pausen dauert einen Takt) |
| FREE-TEXTURE | Auf einem Taktanfang nach dem Einsatz aller Stimmen und vor dem letzten Takt klingt weniger als die Hälfte der Stimmen (P3) oder keine (`silence`, P3). Ab drei Stimmen. Eine Stimme, die einen Abschnitt einer Messe nicht singt (keine Note, kein Text), zählt nicht mit (§ 19). | P3 je Taktanfang | Korpus Palestrina (0,4–2,3 % bzw. 0,1 % der Taktanfänge) |

### Was im freien Satz sonst anders gilt

- **Neu angeschlagener Vorhalt.** Palestrina schlägt einen Vorhalt für eine neue Silbe neu an. Ein
  Ton, der den vorigen wiederholt, gilt deshalb als vorbereitet (HAR-CONSONANCE, HAR-DISSONANCE) und
  muss sich auflösen wie ein gehaltener (FREE-RESOLUTION).
- **Vorhalt im Bass.** Schlägt eine Oberstimme gegen einen liegenden Bass eine Sekunde, Septime
  oder einen Tritonus an, ist der Bass der Vorhalt (2–3) und muss stufenweise abwärts gehen
  (Palestrina: 40 Fälle, 10 ohne Auflösung). Eine Quarte über dem liegenden Bass ist dagegen der
  Vorhalt der Oberstimme (4–3, 684 Fälle) oder die konsonante Quarte.
- **Früh entschieden.** Eine Dissonanz außerhalb der Thesis, die eine Stimme mit einem Sprung oder
  nach einer Pause erreicht, kann keine der erlaubten Figuren mehr werden; FREE-PASSING und
  G5-PASSING melden sie deshalb schon an ihrem Beginn. Eine Achtel vor einer Pause meldet
  FREE-EIGHTHS am Beginn der Pause.
- **Verzögerter Leitton im Bass.** Er wird am Bass selbst erkannt: das Ziel über den Taktstrich
  gehalten, dann einen Halbton abwärts. Ein markierter Binnenschluss ist nicht nötig.
- **Schlusskadenz.** Sie wird in den Noten gesucht. Ihr Ziel ist der letzte Ton der tiefsten Stimme:
  Ein Abschnitt darf auf der Finalis, der Quarte oder der Quinte des Modus enden (sonst
  TEX-FINAL-CHORD `free_degree`, P2; bei Palestrina 27 von 1147 Abschnitten). Ihre Ankunft ist der
  letzte Taktanfang, an dem eine Stimme das Ziel mit dem Halbton der Klausel erreicht. Danach darf
  eine Coda über dem Schlusston folgen, etwa plagal. TEX-PENULT prüft nur den letzten Basston vor
  dem Ziel, denn der Bass darf im vorletzten Takt noch gehen (IV–V–I). Der Schlussklang ist der
  letzte Klang (bei Palestrina mit großer Terz 1037-mal, ohne Terz 103-mal, mit kleiner Terz 7-mal;
  ausgelassener Test `endings`). Ohne große Terz kostet er P2 (TEX-FINAL-CHORD `free_missing_third`;
  `missing_third` gilt im freien Satz nicht): So schließen 110 von Palestrinas
  1156 Sätzen (9,5 %; die sieben mit kleiner Terz, V, zählen mit), dreistimmig 19 von 74 (26 %),
  vierstimmig 86 von 448 (19 %), fünf- und sechsstimmig 5 von 634. Nach der Leitlinie gibt der
  Anteil über alle Stimmenzahlen P2, drei- und vierstimmig für sich P1, fünf- und sechsstimmig P3;
  der Parameter gilt für jede Stimmenzahl. Einen ersten Klang gibt es nicht,
  denn die Stimmen setzen nacheinander ein; FIC-SUBSEMITONIUM entfällt, weil TEX-CLAUSULA den
  Halbton prüft.
- **Länge.** Ein freier Satz hat keine vorgegebene Länge. Regeln über den letzten Takt und die
  Schlusskadenz entscheiden deshalb erst bei der Prüfung des ganzen Satzes.
- **Musica ficta.** Das erniedrigte Mi (B♭, transponiert E♭) ist in allen Modi erlaubt und kostet
  nichts (FIC-BFLAT `free` aus; Palestrina erniedrigt 6797 Töne). Ein erhöhter Ton ist auch als
  große Terz über dem Bass (Dur-Klang) oder im Bass unter der kleinen Sexte erlaubt; geht er dann
  nicht aufwärts weiter, kostet er P2 (FIC-SCOPE `free_unresolved`; Palestrina: 1933 von 18 093
  erhöhten Tönen gehen abwärts oder springen). Ein erhöhter Ton außerhalb einer Klausel kostet P2
  (FIC-INNER-CADENCE `free`). Als Klausel gilt neben den markierten Binnenschlüssen und dem
  Schluss jeder Leitton, der im Halbton auf einen Taktanfang steigt, auf dem eine andere Stimme
  denselben Ton von der Stufe darüber (Tenorklausel) oder mit Quart- oder Quintsprung
  (Bassklausel) erreicht: So stehen 67 % von Palestrinas 14 070 Leittönen; insgesamt erhöht er
  in 3,4 % der Stimmen-Takte, außerhalb solcher Klauseln in 1,5 %.
- **Klang und Bewegung** (Parameter `free`): ohne Terz P2 (Quinte und Oktave; dreistimmig 7,4 %, ab
  vier Stimmen 3,2 % der angeschlagenen Klänge innerer Takte), die leere Sexte P2 (Sexte und Oktave
  ohne Terz, TEX-TRIAD `free_bare_sixth`; 9,0 % bzw. 2,6 %, vierstimmig 3,8 %),
  ohne Quinte P1 (38 % bzw. 16 %), alle Stimmen in dieselbe Richtung P2 (4,5 %), Einklang auf der
  Thesis dreistimmig P2, Septime in der Unterstimme eines Vorhalts (7–8) P2 (190×, meist fünf- und
  sechsstimmig). Die Klänge zählt der ausgelassene Test `struck_sonorities`: Von 1439 angeschlagenen
  Klängen dreistimmig und 36 611 ab vier Stimmen sind 36,3 und 63,1 % vollständig, 8,0 und 12,8 %
  Sextklänge und 1,5 und 1,9 % dissonant. Er zählt auch die Taktanfänge, an denen eine Stimme hält,
  deren Klangtyp keine Regel bewertet: Dort sind dreistimmig 6,8 %, vierstimmig 2,7 % der
  konsonanten Klänge leere Sexten.
- **Jeppesens Regeln** (§ 15): Sprung aufwärts von einem betonten Viertel, betonter Durchgang
  aufwärts und kleine Sexte abwärts sind im freien Satz V (Palestrina 29-, 7- und 7-mal).

- **Sprünge** (MEL-LEAP-COST, Parameter `free_*`): Palestrina springt weit mehr als Fux erlaubt,
  der Bass am meisten. Von allen Tonschritten der Oberstimme sind 11,4 % Terzen, 4,0 % Quarten,
  1,8 % Quinten, 0,4 % Oktaven; in den Mittelstimmen 12,6 / 5,3 / 3,5 / 1,0 %, im Bass 11,5 / 9,7 /
  10,0 / 1,3 %. Deshalb sind Terzen frei, Quarten P1, Quinten P2, Sexten P3, Oktaven P2, im Bass
  Quarten und Quinten frei und Oktaven P1. Zwei Sprünge in dieselbe Richtung (auch als Dreiklang)
  kosten P3 (0,5 % der Dreitonfolgen), zwei Sprünge mit Wende P1 (4,3 %; MEL-CONSEC-LEAPS `free`,
  `free_turn`).
- **Tonwiederholungen** (MEL-REPEAT, Parameter `free_*`): Ein Ton, der den vorigen wiederholt,
  kostet P1 (`free`, vorläufig), nach einer schwarzen Note (Viertel, Achtel) ebenso (`free_black`,
  P1, vorläufig; Palestrina wiederholt vierstimmig ein Viertel als Viertel 1,6-mal je 1000 Töne).
  Der dritte und jeder weitere gleiche Ton in Folge kostet mindestens P1 (`free_run`, vorläufig;
  Palestrina: 2,2 Folgen von drei und mehr gleichen Tönen je 100 Stimmtakte, in Abschnitten bis 26
  Takte 1,0). Die Vorausnahme (ein Viertel, dann derselbe Ton als mindestens halbe Note geschrieben)
  kostet wie in der 5. Gattung P2 (`anticipation`); ein Viertel, das als übergebundenes Viertel
  wiederholt wird, kostet wie jede Wiederholung nach einer schwarzen Note (`free_black`), denn am
  wiederholten Ton steht noch nicht fest, ob er übergebunden wird. In einer Stimme mit Text gilt
  nach einer weißen Note `free_texted` (frei), denn dort verlangt der Text eine neue Silbe (§ 17).
  Ohne Text ändern `free_black` und `free_run` nichts, weil schon `free` P1 kostet.

Die übrigen Präferenzen (Stimmkreuzung, Bewegungsarten usw.) gelten wie bei Fux. Palestrina kreuzt
Stimmen ständig (88 950-mal) und überschreitet sie oft (74 791-mal); als Kosten halten sie die
erzeugten Sätze übersichtlich. Den Bass verdoppelt Palestrina im Schlussklang oft im Einklang (389
von 1156 Schlussklängen); das bleibt frei.

### Taktrhythmen (FREE-RHYTHM)

Anteil an allen 519 764 Stimmen-Takten, `⁀` für Bindungen über den Taktstrich:

| Rhythmus | Anteil | Rhythmus | Anteil |
|---|---|---|---|
| `1` | 22,4 % | `⁀2 4 4` | 1,15 % |
| `r1` | 16,1 % | `4 4 2⁀` | 1,14 % |
| `2 2` | 14,0 % | `⁀4 4 2⁀` | 1,13 % |
| `⁀2 2` | 7,6 % | `⁀4 4 2` | 1,07 % |
| `1⁀` | 7,4 % | `2 4 4` | 0,92 % |
| `2 2⁀` | 5,3 % | `⁀2 2⁀` | 0,86 % |
| `⁀1` | 5,2 % | `⁀4 4 4 4` | 0,73 % |
| `2. 4` | 4,5 % | `4 4 2` | 0,65 % |
| `r2 2` | 3,1 % | `2. 8 8` | 0,42 % |
| `r2 2⁀` | 2,3 % | `⁀4 8 8 2` | 0,20 % |
| `⁀1⁀` | 1,9 % | | |
| `4 4 4 4` | 1,7 % | | |

Die Rhythmen mit 1–5 % (`2. 4` bis `⁀4 4 2`) bilden die Stufe `frequent` (aus), die mit 0,1–1 %
(`2 4 4` bis `⁀4 8 8 2`) die Stufe `uncommon` (P1). P2 (`occasional`): `⁀4 4 4 8 8`, `4 4 4 8 8`,
`r2 4 4`, `4 8 8 2`, `4 8 8 2⁀`. P3 (`rare`): `4 2 4`, `2 4 8 8`, `8 8 8 8 2⁀`, `⁀4 8 8 2⁀`,
`⁀2 r2`, `2 r4 4`, `⁀4 8 8 4 4`, `2 r2`, `4 8 8 4 4`, `2 4 4⁀`, `4. 8 2`. P4 (`very_rare`): elf
Rhythmen mit je höchstens vier Belegen. Alle anderen V.

### Ausnahmen bei Palestrina

Verbote, gegen die Palestrinas Sätze trotzdem verstoßen (Standardprofil, 1156 Sätze):

| Regel | Fälle | Art |
|---|---|---|
| FREE-PASSING | 498 | seltene Figuren, je höchstens 14-mal, etwa ein Sprung aus der Dissonanz im Bass |
| FIC-SCOPE | 349 | gehäuft in wenigen Sätzen mit offenbar falsch kodiertem Modus oder Vorzeichen (Credo 13 a/c, Benedictus 69 b, Kyrie 42 b) |
| TEX-PARALLEL | 222 | Quinten, Oktaven und Einklänge, meist fünf- und sechsstimmig (0,04 % der Takte) |
| FREE-RESOLUTION | 298 | etwa ein nach einer Pause neu angeschlagener Vorhalt (je höchstens 15-mal); 67 Dissonanzen gegen eine über mehrere Taktanfänge gehaltene Note; 10 Bassvorhalte ohne Auflösung |
| FREE-EIGHTHS | 96 | Sprung aus dem Achtelpaar |
| TEX-PENULT | 75 | vorletzter Klang einer Schlusskadenz |
| HAR-CONSONANCE | 63 | angeschlagene Dissonanz auf der Thesis |
| MEL-MAX-LEAP | 57 | große Sexte |
| HAR-DISSONANCE | 42 | angeschlagene Dissonanz zwischen Oberstimmen |
| TEX-CLAUSULA | 33 | Abschnitte ohne Halbtonklausel ins Ziel |
| G3-ACCENT-LEAP | 29 | Sprung aufwärts von einem betonten Viertel |
| MEL-CONSEC-LEAPS | 25 | zwei Sprünge über die Oktave |
| TEX-FINAL-CHORD | 15 | kleine Terz oder Sexte im Schlussklang |
| HAR-CHROMATIC-CLASH | 11 | Ton gegen seine Alteration |
| G3-ACCENTED-PASSING | 7 | betonter Durchgang aufwärts |
| MEL-MIN6-DOWN | 7 | kleine Sexte abwärts |
| MEL-AUG-DIM | 2 | verminderte Quarte |
| MEL-TRITONE | 1 | Tritonus |

Die Prüfung entscheidet auch im freien Satz jeden Verstoß, sobald er feststeht: Für alle 1156 Sätze
findet sie Fenster für Fenster und mit Stimmen, die Ton für Ton wachsen, dasselbe wie im ganzen Satz
(Test `window_by_window_on_many`). Das braucht der Generator.

### Freie Sätze erzeugen

`fux free` setzt einen freien Satz ohne Cantus firmus: Modus, Stimmen (2–6, üblich S T, A T B,
S A T B, S A T T B, S S A T T B), Länge in Takten (mindestens 4) und Binnenschlüsse sind vorgegeben,
alles andere findet die Strahlsuche der Gattungen, mit Palestrinas Taktrhythmen (FREE-RHYTHM)
für alle Stimmen. Standardmäßig singen die Stimmen einen Text, der Länge und Binnenschlüsse plant
([§ 17](#text-erzeugen)); `--text none` setzt ohne Text, wie hier beschrieben. In `fux free`
imitieren drei bis fünf Stimmen mit Text einander ([§ 18](#18-imitation)); dann legt der Plan der
Imitation Länge, Binnenschlüsse und Einsätze fest.

- **Kadenzen als feste Formeln.** Jeder Schluss steht vorab fest, nur in der Tonstufe; die Oktave
  wählt die Suche dort, wo die Stimme gerade ist. Die Oberstimme singt die **Diskantklausel**: das
  Ziel über den Taktanfang des vorletzten Takts gehalten (Vorhalt; die Vorbereitung findet die
  Suche), aufgelöst in den Leitton (phrygisch in die Ganztonstufe darunter), dann das Ziel. Im
  zweistimmigen Satz singt die Unterstimme die **Tenorklausel** (Ganze von der Stufe darüber,
  phrygisch vom Halbton darüber). Ab drei Stimmen kommt der Bass von der Quinte (**Bassklausel**),
  ab vier Stimmen singt die Stimme über ihm die Tenorklausel; die phrygische Klausel hat keine
  Bassklausel, dort singt der Bass die Tenorklausel. Die übrigen Stimmen sind frei. Mit denselben
  festen Tönen setzt die Imitation die Köpfe ihrer Einsätze, dort in genauer Oktave
  (§ 18).
- **Binnenschlüsse.** `--cadences auto` setzt etwa alle sechs Takte einen (gleichmäßig verteilt,
  keiner in den letzten vier Takten), reihum auf den Kadenzstufen des Modus: Quinte, Terz, Quarte;
  phrygisch Quarte, Sexte, Terz; eine Stufe mit verminderter Quinte (H, transponiert E) entfällt.
  Dorisch also A, F, G; lydisch C, A; mixolydisch D, C. Eigene Pläne wie `7,12` oder `7:A,12:F`
  nennen den Takt, in dem das Ziel erreicht wird; zwischen zwei Zielen liegen mindestens drei Takte.
- **Pausen.** Jede Stimme setzt im ersten Takt ein. Ab drei Stimmen darf sie danach pausieren, wie
  Palestrinas Stimmen es oft tun (FREE-REST, FREE-TEXTURE wiegen das ab); eine Stimme mit einer
  Klausel singt die drei Takte vor deren Ziel durch, und den letzten Takt singen alle. Zweistimmig
  gibt es nach dem Einsatz keine Pausen. In einer Stimme mit Text bestimmt der Text, wo sie pausiert
  (TXT-REST, § 17). Imitiert der Satz (§ 18), schweigt jede Stimme bis zu ihrem ersten Einsatz,
  auch über den ersten Takt hinaus, und vor jedem weiteren Einsatz eines Punktes mindestens eine
  Halbe; vor einem solchen Einsatz darf sie auch zweistimmig pausieren.
- **Was die Suche vom Ende weiß.** Die Regeln erfahren das Ende eines freien Satzes erst im fertigen
  Satz; die Suche kennt es. Im letzten Takt hält deshalb jede Stimme einen Ton des Schlussklangs
  (Oktave, Quinte oder große Terz der Finalis, der Bass die Finalis), und die Stufe des Halbtons
  einer Klausel (C♯ wie C) singt in der zweiten Hälfte des vorletzten Takts nur die Stimme der
  Klausel. Das Intervall zählt aufwärts vom nächsten Ton der Finalis am oder unter dem Ton (E3 ist
  die Quinte über A2, nicht die Quarte unter A3). Ebenso messen die Vorausschau auf den Schlussklang
  (auch in den Gattungen) und die auf den Zielklang der Binnenschlüsse.
- **Suche.** Feste Töne, die keine Wahl lassen (Bindung, Sekundschritt, ein Ton im Kopf eines
  Einsatzes), werden zu Beginn eines Zeitpunkts gesetzt, damit die tieferen Stimmen, die zuerst
  wählen, gegen sie geprüft werden. Vor einem festen Ton prüft die Suche, ob er sich anschließen
  lässt, und der Takt davor muss mit einer Bindung enden, wenn der feste Ton gehalten ist. Den Rang
  eines Zustands bestimmen neben den Kosten die geringsten Kosten, die der angefangene Takt noch
  erreichen kann (FREE-RHYTHM entscheidet erst am Taktende), und der Abstand zu den nächsten festen
  Tönen. Höchstens ein Achtel des Strahls stammt von einem Zustand des letzten Taktanfangs, damit
  die Suche nicht auf eine Vorgeschichte zusammenschrumpft. Läuft sie dennoch fest, geht sie bis zu
  zwölfmal einen, zwei oder mehr Takte zurück (bis zu vier) und versucht es mit anderem Zufall, ehe
  der Aufrufer den Strahl verbreitert. Vor einer Stimme, deren Note über den Taktstrich gehalten ist
  (ein möglicher Vorhalt), prüft sie, ob die tieferen Stimmen ihr noch einen erlaubten Fortgang
  lassen. Melodische Schritte, die das Profil ohnehin verbietet (MEL-MAX-LEAP, MEL-TRITONE,
  MEL-AUG-DIM), angeschlagene Dissonanzen gegen den Bass auf der Thesis (HAR-CONSONANCE) und
  Parallelen (TEX-PARALLEL) scheiden vor der vollen Prüfung aus; die Regeln würden genau sie
  verbieten.
- **Cantus firmus mit freien Stimmen.** `fux cp --species 0` setzt alle Gegenstimmen frei um einen
  Cantus in Ganzen; die Schlüsse ergeben sich dann aus dem Cantus wie bei den Gattungen. Mit
  `--text` singen die freien Stimmen einen Text ([§ 17](#text-erzeugen)).
- **Laufzeit.** Zwei bis sechs Stimmen gelingen in allen Modi ohne Verbot (Tests
  `every_mode_with_two_to_four_voices`, `five_and_six_voices`, je 12 Takte). Release, 12 Takte:
  zweistimmig 0,4 s, dreistimmig 1,3 s, vierstimmig 2–3 s, fünfstimmig 4–30 s, sechsstimmig
  8–35 s; im Browser ohne Threads etwa fünfmal so lang.
- **Stil.** Ein Vergleich mit Palestrinas vierstimmigen Sätzen (Test `style` in
  `crates/fux-core/tests/palestrina.rs`) diente zum Einstellen der Gewichte; je 12 Sätze ohne Text
  in vier Modi zu 16 Takten, vor und nach zwei Korrekturen der Suche (Schlussklang-Filter,
  Vorausnahme nach dem geschriebenen Wert):

  | | Palestrina | erzeugt, vorher | erzeugt, nachher |
  |---|---|---|---|
  | Sprünge Oberstimme / Mittelstimmen / Bass | 17,0 / 20,6 / 28,8 % | 17,4 / 20,3 / 35,6 % | 21,7 / 23,2 / 37,0 % |
  | Rückkehr a–b–a / Pendeln a–b–a–b | 20,6 / 3,0 % | 20,6 / 4,5 % | 19,7 / 4,2 % |
  | Achtel / Viertel / Halbe / Ganze / länger | 1,6 / 21,1 / 47,8 / 24,1 / 5,3 % | 1,0 / 34,2 / 31,4 / 28,5 / 4,9 % | 0,7 / 33,0 / 33,6 / 27,8 / 4,9 % |

  Vor dem Einstellen kreisten die Linien (Rückkehr 35 %, Pendeln 11 %), sprangen kaum (5 % in den
  Oberstimmen) und bewegten sich zu 16 % in Achteln. Die beiden Korrekturen ändern die meisten Sätze
  ab drei Stimmen, ihre Kosten und Sprunganteile aber nur im Rahmen der Streuung zwischen den Seeds:
  In diesen zwölf Sätzen springen Ober- und Mittelstimmen öfter (Tabelle), in 28 weiteren
  (Seeds 4–10) kaum anders als vorher (Oberstimme 19,3 → 19,1 %, Mittelstimmen 24,0 → 24,3 %,
  Bass 34,3 → 35,3 %; Kosten 1383 → 1351).

## 17. Text und Textunterlegung

Die Stimmen eines freien Satzes singen einen lateinischen Text, standardmäßig das Kyrie I. Die
Regeln der Textunterlegung (TXT-…) legen fest, auf welchem Ton eine Silbe beginnen darf, wo eine
Stimme pausieren und den Text wiederholen darf und wie Wortakzent und Notenwerte zusammengehen. Sie
folgen den Theoretikern des 16. Jahrhunderts (Lanfranco 1533 mit den ersten gedruckten Regeln;
Zarlino und Vicentino aus Willaerts Umkreis; Stoquerus, dessen Regeln „der Neueren“ die
Willaert-Schule beschreiben; Zacconi; Morley, der Zarlino auf Englisch wiedergibt) und Palestrinas
Praxis nach Jeppesen.
Palestrinas Messen liegen ohne Text vor, die [Leitlinie](#leitlinie-für-standardwerte) greift also
nicht: **Alle TXT-Werte sind vorläufig.** Geprüft sind sie daran, ob Palestrinas Stimmen ihren Text
tragen können, und an der Unterlegung, die Jeppesen zu Palestrinas *Missa Dies sanctificatus* druckt
([Prüfung ohne Textkorpus](#prüfung-ohne-textkorpus)).

Die TXT-Regeln prüfen nur Stimmen, die den Text des Satzes singen (`Score.text`), nie einen Cantus
firmus. Fux' Beispiele, Palestrinas Humdrum-Dateien, LilyPond-Eingaben und Choralmelodien (GABC)
tragen keinen solchen Text. Die Zahlen von § 14 und § 16 bleiben deshalb gleich (Tests
`untexted_corpora_have_no_text_cases`, `txt_rules_are_silent_on_the_corpus`), und
`fux rules --corpus palestrina` zählt die TXT-Regeln mit 0; ihre Quelle (`fux rules --id TXT-ONSET`)
nennt „Korpus ohne Text, vorläufig“.

**Quellen** der TXT-Regeln, wie die Tabellen sie nennen:
- Zarlino: *Le istitutioni harmoniche*, Venedig 1558, Buch IV, Kap. 32 (Pausen, Kadenzen,
  Barbarismus) und Kap. 33 mit zehn Regeln („Regel 8c“ ist ein Teil von Regel 8); Zarlino 1588:
  *Sopplimenti musicali*, Buch VIII, Kap. 13 (die drei Arten des Akzents); Zarlino 1589: die
  überarbeitete Fassung in *De tutte l'opere*.
- Lanfranco: *Scintille di musica*, Brescia 1533, S. 68 f. (acht Regeln, hier 1–8). Vicentino:
  *L'antica musica ridotta alla moderna prattica*, Rom 1555, Buch IV, Kap. 29–30.
- Stoquerus: *De musica verbali*, um 1570. N sind seine notwendigen Regeln, O die der Älteren, M die
  der Neueren (Willaert und seine Schule), römische Zahlen Kapitel.
- Zacconi: *Prattica di musica*, 1592. Morley: *A Plaine and Easie Introduction to Practicall
  Musicke*, 1597.
- Jeppesen: *Counterpoint* (englische Ausgabe 1939; Regeln S. 158–160, Palestrinas *Missa Dies
  sanctificatus* mit Text S. 254–258) und *The Style of Palestrina and the Dissonance* (1927,
  „Style“). Über Palestrinas Unterlegung außerdem H. K. Andrews (*An Introduction to the Technique of
  Palestrina*, 1958) und J. Quadflieg (Kirchenmusikalisches Jahrbuch 18, 1903, Regeln I–X).

### Der Text

**Ordinarium.** Die Abschnitte stehen in `crates/fux-core/src/text/ordinary.txt`. Sie folgen dem
Graduale Romanum (Solesmes 1961), Kyriale: Messe VIII für Kyrie, Gloria, Sanctus, Benedictus und
Agnus Dei, Credo I. Text, Schreibung (ae, J), Akzente, Silbentrennung und Zeichensetzung sind die
der Bücher, Wort für Wort an den Scans geprüft; wie dort tragen nur Wörter ab drei Silben einen Akut
(Dó-mi-ne, aber De-us). Die Intonationen (Gloria in excelsis Deo, Credo in unum Deum) fehlen, weil
die Polyphonie mit Et in terra und Patrem beginnt. `fux text --list` zeigt die Abschnitte:

| Abschnitt | Text | Art | Silben | Sinnabschnitte | Takte |
|---|---|---|---|---|---|
| `kyrie` | Kyrie I | kurz | 7 | 1 | 16 |
| `christe` | Christe | kurz | 6 | 1 | 14 |
| `kyrie2` | Kyrie II | kurz | 7 | 1 | 16 |
| `gloria` | Gloria: Et in terra | lang | 98 | 13 | 84 |
| `qui-tollis` | Gloria: Qui tollis | lang | 89 | 13 | 76 |
| `credo` | Credo: Patrem | lang | 128 | 16 | 109 |
| `incarnatus` | Credo: Et incarnatus | lang | 25 | 2 | 23 |
| `crucifixus` | Credo: Crucifixus | lang | 24 | 3 | 22 |
| `resurrexit` | Credo: Et resurrexit | lang | 62 | 7 | 54 |
| `spiritum` | Credo: Et in Spiritum | lang | 120 | 12 | 102 |
| `sanctus` | Sanctus | kurz | 14 | 3 | 25 |
| `pleni` | Sanctus: Pleni | kurz | 13 | 1 | 24 |
| `osanna` | Sanctus: Osanna | kurz | 7 | 1 | 25 |
| `benedictus` | Benedictus | kurz | 14 | 1 | 25 |
| `osanna2` | Benedictus: Osanna | kurz | 7 | 1 | 25 |
| `agnus` | Agnus Dei I und II | kurz | 18 | 3 | 20 |
| `agnus3` | Agnus Dei III | kurz | 18 | 3 | 20 |

„Takte“ ist die automatische Länge eines freien Satzes ([Text erzeugen](#text-erzeugen)). `ID:A-B`
wählt die Sinnabschnitte A bis B, ab 1 gezählt: `gloria:1-5` ist Et in terra … Glorificamus te
(36 Silben, 5 Sinnabschnitte, 32 Takte), `gloria:7` ein einzelner. Die Namen gelten in jeder
Schreibung (`Credo`, `Gloria: 1-5`).

**Verbundene Texte.** Mit `+` verbundene Angaben werden nacheinander als ein Text
gesungen: `sanctus+pleni+osanna` (34 Silben, 5 Sinnabschnitte, 3 Sätze, kurz),
`crucifixus+resurrexit:1-2` (40 Silben, lang). Der Text ist lang, wenn eine seiner Angaben lang ist,
und geht dann im Tempo langer Texte, sonst im Tempo seiner ersten Angabe
([Text erzeugen](#text-erzeugen)), `sanctus+pleni+osanna` also im melismatischen Tempo des Sanctus
(59 Takte). Die gezogenen Werte und Intervalle seiner Eröffnung (§ 18) nimmt er von der ersten
Angabe. So singt die Missa brevis Sanctus, Pleni und Osanna in einem Teil (§ 19). Nennt eine Angabe
keinen Abschnitt des Ordinariums (`Kyrie + pax`), scheitert sie mit dem Hinweis, dass `+` nur
Abschnitte des Ordinariums verbindet und ein eigener Text kein `+` enthalten kann.

**Eigener Text.** Ein Text, der kein Abschnittsname ist, gilt als eigener lateinischer Text, wenn er
ein Leerzeichen, einen Großbuchstaben oder ein Satzzeichen enthält, etwa
`"Ad te le-vá-vi á-ni-mam me-am."` oder `"Amen."`. Bis 20 Silben ist er kurz, darüber lang.
Bindestriche teilen ein Wort, wie sie stehen; ein Akut setzt die Betonung.

**Silben und Betonung.** Ein Wort ohne Bindestriche teilt `fux_core::text::latin` so, wie das
Graduale von 1961 und der Liber Usualis es drucken: zuerst nach einem Lexikon (alle Wörter des
Ordinariums und einige, deren Akzent keine Regel findet, wie María, miserére, Prophétas, procédit),
sonst nach Regeln:
- i und j sind am Wortanfang vor einem Vokal, zwischen Vokalen und nach einer Vorsilbe Konsonanten
  (Je-su, cu-jus), u in qu und nach ng vor einem Vokal (san-guis).
- ae, oe und au sind ein Vokal, jedes andere Vokalpaar zwei (De-us, e-lé-i-son, cú-i); ein Trema
  trennt (po-ë-ta).
- Vorsilben behalten ihren Konsonanten (Ad-o-rá-mus, ex-áu-di, in-i-mí-ci, sús-ci-pe), ebenso ét-i-am
  und o-mní-pot-ens. Sonst geht ein einzelner Konsonant zur nächsten Silbe, doppelte werden geteilt,
  und von einer Gruppe geht das längste Ende, mit dem ein Wort beginnen kann, zur nächsten Silbe:
  Muta cum Liquida (Pa-tris), gn, sc, sp, st und die griechischen ct, pt, mn, ps (San-ctus,
  pro-pter, o-mnis, i-pse); sm und squ nicht (ba-ptís-ma, us-que).
- Ein zweisilbiges Wort ist auf der ersten Silbe betont, ein längeres auf einer schweren vorletzten,
  sonst auf der drittletzten; -que zieht den Akzent auf die Silbe davor (Fi-li-ó-que). Zeigen die
  Buchstaben die Länge der vorletzten Silbe nicht, entscheiden Endungen (-ámus, -átus, -iónem, -ávit
  …); sonst gilt sie als kurz, und das Programm warnt („Akzent unsicher: …; mit Akut angeben“).

Von den 209 mehrsilbigen Wörtern im Text des Ordinariums (149 verschiedene) teilen die Regeln allein
alle wie die Bücher und betonen 203 richtig (nicht mi-se-ré-re dreimal, Pro-phé-tas, pro-cé-dit,
Ma-rí-a); mit dem Lexikon stimmen alle.

**Betonte Silben.** Jedes Wort ab zwei Silben hat eine betonte Silbe, ein Wort ab vier Silben dazu
Nebenakzente zwei und vier Silben davor (Bè-ne-dí-ci-mus). Einsilbige Wörter sind betont außer den
Funktionswörtern et, in, de, ex, e, a, ab, ad, cum, pro, per, sub, qui, quem, quod, sed, ac, nec,
ut; pax, Rex, te, est, sunt und non sind betont. „Unbetont“ heißt in den Regeln: ohne Haupt- und
Nebenakzent. Die unbetonte vorletzte Silbe eines Wortes, das auf der drittletzten betont ist, ist
**kurz** (Dó-mi-nus, Ký-ri-e, e-lé-i-son).

**Sinnabschnitte und Sätze.** Ein Sinnabschnitt reicht bis zu seinem Satzzeichen: `,` `;` `:`
beenden ihn mit einem Zwischenschluss, `.` `?` `!` und das Ende des Textes mit einem Satzende. Ein
Satz des Textes umfasst die Sinnabschnitte bis zum nächsten Satzende.

**Zäsuren und Schlussteile.** Zwei Marken ergänzen den Text der Bücher, nach Zarlinos „alcuna sua
parte“, einem Teil des Satzes, nach dem eine Pause stehen darf:
- `|` zwischen zwei Wörtern ist eine **Zäsur**. Dort darf eine Stimme pausieren, und ein
  Binnenschluss dort gilt wie einer an einem Komma.
- `/` vor einem Wort beginnt einen **Schlussteil**, der für sich wiederholt werden darf
  (Ký-ri-e / e-lé-i-son); davor steht ebenfalls eine Zäsur.

| Abschnitt | Marken (der übrige Text wie im Graduale) |
|---|---|
| `kyrie`, `kyrie2`, `christe` | Ký-ri-e / e-lé-i-son., Chri-ste / e-lé-i-son. |
| `gloria` | pax \| ho-mí-ni-bus, ti-bi \| pro-pter, Fi-li \| u-ni-gé-ni-te \| Je-su |
| `qui-tollis` | sús-ci-pe \| de-pre-ca-ti-ó-nem, se-des \| ad |
| `credo` | Dó-mi-num \| Je-sum, na-tum \| an-te, sa-lú-tem \| de-scén-dit |
| `incarnatus` | est \| de Spí-ri-tu San-cto \| ex Ma-rí-a |
| `crucifixus` | Pi-lá-to \| pas-sus |
| `spiritum` | Fí-li-o \| si-mul, est \| per, san-ctam \| ca-thó-li-cam, ba-ptís-ma \| in, ex-spé-cto \| re-sur-re-cti-ó-nem |
| `sanctus` | San-ctus / Dó-mi-nus De-us Sá-ba-oth. |
| `pleni` | cae-li \| et ter-ra / gló-ri-a tu-a. |
| `osanna`, `osanna2` | Ho-sán-na / in ex-cél-sis. |
| `benedictus` | Be-ne-dí-ctus \| qui ve-nit / in nó-mi-ne Dó-mi-ni. |
| `resurrexit`, `agnus`, `agnus3` | keine |

**Eröffnende Silben und Wiederholungen.** Eine Silbe **eröffnet** einen Teil des Textes, wenn sie
einen Sinnabschnitt oder einen Schlussteil beginnt oder auf eine Zäsur folgt. Nach der letzten Silbe
eines Sinnabschnitts darf eine Stimme weitersingen oder den Text **wiederholen**: Sie beginnt neu am
Anfang eines Sinnabschnitts oder Schlussteils ihres Satzes, höchstens bei diesem Sinnabschnitt
selbst. Nach „mi-se-ré-re no-bis.“ im Agnus Dei also bei „A-gnus“, „qui tol-lis“ oder „mi-se-ré-re“.
Jede Stimme singt den ganzen Text von der ersten bis zur letzten Silbe; Auslassungen sind nicht
umgesetzt (§ 19).

**eleison.** Die Bücher teilen e-lé-i-son, vier Silben mit der kurzen vorletzten „i“. `--eleison 3`
gibt e-léi-son, betont auf „léi“, wie Jeppesen Palestrinas Christe unterlegt. Palestrinas
Kyrie-Stimmen tragen diese Form öfter ([unten](#prüfung-ohne-textkorpus)).

### Regeln der Textunterlegung

Begriffe, jeweils in einer Stimme:
- **Anschlag:** der erste geschriebene Ton eines Klangs; eine Überbindung setzt ihn fort. Ein Anschlag
  ohne Silbe singt die Silbe davor weiter (**Melisma**), eine Pause beendet sie.
- **Schwarz** sind Viertel und Achtel, **weiß** Halbe und längere Noten. Beim Anschlag zählt sein
  geschriebener Wert, beim Klang davor seine Länge mit Überbindungen.
- **Halbe Schläge:** Taktanfang und Taktmitte des 2/2-Takts.
- **Einsatz:** der erste Anschlag einer Stimme und jeder erste nach einer Pause.
- **Daktylus-Viertel:** das Viertel nach einer punktierten Halben, die auf einem halben Schlag
  beginnt (Rhythmus `2. 4`).
- **Erste Kadenzstufe** eines Modus: die erste seiner regelmäßigen Kadenzstufen (§ 16): dorisch A,
  phrygisch A, lydisch C, mixolydisch D, äolisch E, ionisch G.

Erzeugte Sätze setzen Silben nur auf Anschläge; eingelesene können sie auch auf Pausen oder
Überbindungen setzen.

| ID | Regel | Standard | Quelle |
|---|---|---|---|
| TXT-ONSET | Eine neue Silbe steht auf einer Achtel, auf einer weißen Note zwischen den halben Schlägen, auf einem Viertel zwischen ihnen außer dem Daktylus-Viertel oder, in eingelesenen Sätzen, auf einer Pause oder Überbindung (V). Ein textiertes Viertel auf einem halben Schlag beginnt einen Lauf; folgt keine schwarze Note, kostet es `single` (V), ebenso ein textiertes Daktylus-Viertel, dem keine Halbe oder längere Note folgt („qui an-te De-um“). Ein textiertes Viertel auf der Taktmitte, mit dem ein Lauf beginnt, kostet `run_upbeat` (P1, vorläufig). Das Daktylus-Viertel trägt eine unbetonte Silbe im Inneren eines Sinnabschnitts (Dó-mi-ne, `figure`, frei); jede andere Silbe dort, eine betonte, eine eröffnende oder die letzte eines Sinnabschnitts, kostet `figure_accent` (P2, vorläufig). Eine eröffnende Silbe, auch einer Wiederholung, auf einer schwarzen Note auf einem halben Schlag oder nach einer Pause kostet `opening_black` (P1, vorläufig), außer beim ersten Einsatz. Am Einsatz gilt nur `opening_black`: Sonst darf ein Einsatz auf jedem Wert und an jeder Stelle beginnen. | V / P2 / P1 / frei | Zarlino IV.33, Regeln 1b–7 (Einsatz: „qualunque figura“); Lanfranco 4–6; Stoquerus N2, N3, M2, O5, XXIV.1; Jeppesen, Regel 2 und S. 160, 269; Zacconi (Dominus). Läufe beginnen bei Palestrina 11 588-mal auf der Taktmitte, 7 485-mal auf dem Taktanfang. In Jeppesens Unterlegung trägt keines von 13 Daktylus-Vierteln eine Silbe. |
| TXT-AFTER-BLACK | Eine neue Silbe direkt nach Vierteln oder Achteln, ohne Pause dazwischen (V). Nach dem textierten Daktylus-Viertel ist sie frei (`figure`). Die letzte Silbe eines Sinnabschnitts auf einer Halben oder länger kostet `unit_end` (P1), eine Silbe auf einer Halben oder länger nach dem untextierten Daktylus-Viertel `dotted` (P3, vorläufig). | V / P3 / P1 / frei | Stoquerus M3 („wie eine notwendige Regel“); Vicentino 1, 2; Lanfranco 5, 6; Zarlino IV.33, Regeln 4–6 („rare volte“); Jeppesen, Regel 7 („oft geduldet“); Palestrina hält die Regel „bemerkenswert“ ein (Andrews). `dotted`: Jeppesen druckt es zweimal in Palestrinas Christe (Alt „lei-“, Tenor „ste“). |
| TXT-REPEATED-PITCH | Ein Ton wird nach einer Halben oder längeren Note derselben Tonhöhe neu angeschlagen, ohne Bindung und Pause, und trägt keine neue Silbe. Nach Vierteln ist das frei (Antizipation). | V | Stoquerus N4; Jeppesen, Regel 9; Quadflieg IV; Palestrinas Tonwiederholungen tragen neue Silben (Jeppesen S. 94) |
| TXT-ENTRY | Ein Einsatz ohne neue Silbe. | V | Zarlino IV.33, Regel 7 („di necessità“); Stoquerus N5 |
| TXT-ORDER | Eine Silbe ist weder die nächste des Textes noch, direkt nach der letzten Silbe eines Sinnabschnitts, ein Wiederholungsbeginn in dessen Satz (V). Die erste Silbe einer Stimme ist nicht die erste des Textes (`first`, V): Jede Stimme singt den ganzen Text; Auslassungen sind nicht umgesetzt (§ 19). | V | Zarlino IV.33, Regel 8c; Vicentino 4; Zacconi 4; Stoquerus M4 |
| TXT-REPEAT | Eine Wiederholung in einem kurzen Text wie Kyrie, Sanctus, Agnus Dei (`short`, frei); in einem langen Text, außer der seines letzten Sinnabschnitts, wo der Text endet, wie des Amen (`long`, P2; in einem Teil einer Messe, nach dem der Text im nächsten Teil weitergeht, `Text.continued`, zählt auch der letzte Sinnabschnitt, § 19); jede weitere im selben Satz (`again`, P1); eine Wiederholung einzelner Wörter, also von Sinnabschnitten aus je einem Wort, die zusammen keinen Satz bilden (San-ctus, San-ctus; `word`, P2, vorläufig). Ein ganzer Satz wie A-men. zählt nicht als einzelnes Wort. | frei / P2 / P1 | Lanfranco 7; Zarlino IV.33, Regeln 8b–8d („non dico gia di vna sillaba, ne di vna parola“); Vicentino 4; Stoquerus M4 („perraro“); Palestrina wiederholt in Kyrie, Sanctus und Agnus Dei oft, in Gloria und Credo selten (Jeppesen) |
| TXT-REST | Eine Pause nach einer Silbe, deren Wort weitergeht (`word`, V); eine Pause oder das Ende der Stimme direkt nach Vierteln oder Achteln (`short`, V); eine Pause nach einem Wort, das weder einen Sinnabschnitt beendet noch vor einer Zäsur steht (`clause`, P2, vorläufig), oder, wenn danach nur noch ein Wort bis zum Satzzeichen oder zur Zäsur bleibt, das die Stimme dann allein singt, `last_word` (P3, vorläufig; Lau-dá-mus \| te., in nó-mi-ne \| Dó-mi-ni.); eine Pause nach einem Sinnabschnitt ohne Satzende (, ; :) oder an einer Zäsur (`comma`, P1, vorläufig); eine Stille, die länger als eine Halbe wird, obwohl die letzte Silbe keinen Satz beendet hat (`long`, P1, vorläufig). Nach einem Satzende kostet eine Pause nichts, außer nach Vierteln oder Achteln (`short`). | V / P3 / P2 / P1 | Zarlino IV.32: keine Pause, „fino a tanto, che non sia finita … alcuna sua parte“, an den Zwischenpunkten kurze, am Satzende „quanta quantità di pause“; Morley; Zacconi 4; Jeppesen S. 269. Bei Palestrina nie im Wort; in einer Stichprobe von 4 850 Pausen keine nach einer schwarzen Note. Mit `comma` und `clause` pausieren Stimmen mit Text so oft wie Palestrinas in kurzen Abschnitten, frei an jedem Sinnabschnitt doppelt so oft. `last_word`: Zarlino IV.32 („alcuni poco intelligenti“), nach einer Pause beginnt ein Sinnabschnitt (Jeppesen); in Jeppesens Unterlegung keine; als Verbot ließen sich 43 der 632 Stimmen Palestrinas aus Agnus I und Benedictus nicht mehr unterlegen. |
| TXT-COMPLETE | Der letzte Ton einer Stimme trägt nicht die letzte Silbe des Textes (auf ihm oder vorher begonnen). | V | Zarlino IV.33, Regel 10; Stoquerus N5 |
| TXT-MELISMA | Ein Melisma, gezählt ab dem zweiten Anschlag einer Silbe: auf einer unbetonten Silbe, die weder einen Teil des Textes eröffnet noch einen Sinnabschnitt beendet (`unstressed`, P1; das „e-“ von eleison eröffnet den Schlussteil); auf einem Funktionswort außer am Ende eines Sinnabschnitts (`function_word`, P2); auf der letzten Silbe eines Sinnabschnitts (`last_syllable`, P2), außer einem Auftakt aus schwarzen Noten, die innerhalb eines Taktes in ihren letzten Ton führen. Je Ton einer Silbe über ihre Höchstzahl hinaus `long` (P1): auf einer unbetonten Silbe, die weder eröffnet noch einen Sinnabschnitt beendet, 4 Töne (`max_weak`), auf einer unbetonten eröffnenden 5 (`max_opening`), auf der letzten Silbe eines Sinnabschnitts, sobald ihr Melisma kein Auftakt mehr ist, 5 (`max_last`), auf jeder anderen, einer betonten, 10 in kurzen Texten (`max_short`) und 8 in langen (`max_long`); alle vorläufig. | P2 / P1 | Palestrina (Jeppesen, Style S. 38, in einer Messe gezählt); Zarlino 1589 (die Zahl der Töne folgt dem Akzent); Stoquerus XX.3 und O1 (überzählige Töne auf die betonte Silbe des letzten Worts), XXV (lieber den Satz beenden und wiederholen als verweilen), XXIX.4 (Auftakt); die Unterstimmen sind freier (Andrews, Quadflieg VIII). `max_weak`, `max_opening` und `max_last` sind die längsten Melismen ihrer Art in Jeppesens Unterlegung von *Dies sanctificatus*: unbetont 4 („um“, „tus“), unbetont eröffnend 5, am Ende eines Sinnabschnitts 5 (Bass „tem,“). `max_long`: Im Credo singt der Tenor 9 Töne auf „ter-“, der Sopran 6. |
| TXT-ACCENT | Ein Melisma auf der kurzen vorletzten Silbe (Dó-mi-nus, e-lé-i-son; `short_penult`, V): ein Barbarismus. Eine unbetonte Silbe dauert länger als die betonte daneben im selben Wort, außer am Wortende (`longer`, P3); dasselbe für eine eröffnende Silbe wie ein langes „e-“ vor „lé“ (`opening`, P1). Eine betonte Silbe ist kürzer als die unbetonte daneben und beginnt nicht auf dem Taktanfang, während jene auf ihm beginnt (`weak`, P1, einmal je betonte Silbe). Eine Silbe dauert von ihrem ersten Anschlag bis zur nächsten Silbe oder Pause. | V / P3 / P1 | Zarlino IV.32 und IV.33, Regeln 1a, 9b; Zarlino 1588/89; Stoquerus M5; Vicentino 5; Zacconi; Morley; Quadflieg: „hin und wieder“ bei den alten Meistern; Palestrina ist von Barbarismen „bemerkenswert frei“ (Andrews), beim langen „e-“ von eleison nicht (Christe von *Dies sanctificatus*) |
| TXT-LEAP | Ein Silbenwechsel im Wort über einen Sprung: eine Septime oder mehr (`octave`, P2; Septimen kommen nur in eingelesenen Sätzen vor), eine Sexte (`sixth`, P1), eine Quinte oder Quarte (`fifth`, `fourth`, frei). | P2 / P1 / frei | Vicentino 7 (nie über eine Oktave; über eine Quinte „weniger schlecht“, auch Quinten und Quarten meidet man besser); Quadflieg I; die Bassklausel teilt e-lé-i-son an den meisten Kadenzen über eine Quinte |
| TXT-CADENCE | An einem Binnenschluss erreicht eine Stimme auf dem Taktanfang des Ziels den Zielton mit einem Schritt, einem Quart- oder einem Quintsprung und singt dort weder die letzte Silbe eines Sinnabschnitts noch die Silbe vor einer Zäsur (\| oder /) (P3, vorläufig). Endet dort nur ein Sinnabschnitt mit Zwischenschluss (, ; :) oder steht dort eine Zäsur, kostet ein Ziel auf der Finalis oder der ersten Kadenzstufe `principal` (P2), ein anderes `comma` (P1, vorläufig). Am Satzende ist der Binnenschluss frei. Formelstimmen müssen am Ziel keinen Sinnabschnitt beenden. | P3 / P2 / P1 | Zarlino IV.32 (die Kadenz am Punkt; „massimamente l'una delle principali“); Lanfranco 1; Morley („not … till the full sense“) |

**MEL-REPEAT und FREE-REST mit Text.** Zwei Regeln des freien Satzes (§ 16) haben eigene Werte für
Stimmen, die singen:
- **MEL-REPEAT** `free_texted` (frei, statt `free` P1): Ein Ton, der nach einer weißen Note neu
  angeschlagen wird, kostet in einer singenden Stimme nichts. Palestrinas Tonwiederholungen tragen
  neue Silben, und TXT-REPEATED-PITCH verlangt eine. Nach einer schwarzen Note beginnt dagegen keine
  neue Silbe (TXT-AFTER-BLACK), die Wiederholung bleibt im Melisma: Sie kostet wie ohne Text P1
  (`free_black`, vorläufig). Ohne diesen Wert wiederholten Stimmen mit Text ein Viertel als Viertel
  83-mal je 1000 Töne, jetzt 1,7-mal (Palestrina 1,3–1,6-mal). Der dritte und jeder weitere gleiche
  Ton in Folge kostet mindestens P1 (`free_run`, vorläufig), denn der Text rechtfertigt eine
  Wiederholung, kein Rezitieren. Weiße Noten, die den Ton davor neu anschlagen: mit Text 12,7 %,
  Palestrina 6,5–10,3 %; mit P1 statt frei kaum noch eine (0,5–1,2 %).
- **FREE-REST** `texted` (frei, vorläufig, statt P1 je Pause): Wo eine singende Stimme pausieren
  darf, regelt TXT-REST.

### Entscheidungszeitpunkte

Jeder Fall betrifft eine Stimme und wird zum frühesten Zeitpunkt entschieden, an dem ihre Noten ihn
zeigen; markiert wird der Ton, gegen den er spricht. Kein Fall hängt von einer anderen Stimme ab. So
finden die Prüfung Fenster für Fenster, die Prüfung Ton für Ton wachsender Stimmen und die des ganzen
Satzes dieselben Fälle (Tests `texted_settings_check_window_by_window`,
`falsified_underlays_are_reported_at_their_tick`, `cases_of_a_prefix_are_a_prefix`).

| Fall | entschieden | markiert |
|---|---|---|
| TXT-ONSET außer `single`, TXT-AFTER-BLACK, TXT-REPEATED-PITCH, TXT-ENTRY, TXT-ORDER, TXT-REPEAT, TXT-LEAP | am Anschlag | den Ton |
| TXT-ONSET `single` | am nächsten Ereignis der Stimme: Anschlag, Pause oder Ende | das Viertel |
| TXT-MELISMA `unstressed`, `function_word`, TXT-ACCENT `short_penult` | am zweiten Anschlag der Silbe | ihren ersten Ton |
| TXT-MELISMA `last_syllable` | am Anschlag nach einer weißen Note der Silbe oder am ersten, der mehr als einen Takt nach ihrem Beginn kommt | ihren ersten Ton |
| TXT-MELISMA `long` | an jedem Anschlag über der Höchstzahl der Silbe | den Ton |
| TXT-ACCENT `longer`, `opening`, `weak` | sobald die Notenwerte zeigen, dass eine Silbe ihre Nachbarin im Wort überdauert, an einem Anschlag oder einer Überbindung; sonst am Ende der späteren Silbe (nächste Silbe, Pause oder Ende) | den ersten Ton der zu langen bzw. der zu kurzen betonten Silbe |
| TXT-REST `word`, `clause`, `last_word`, `comma` | am Beginn der Pause | die Pause |
| TXT-REST `short` | am Beginn der Pause; am Ende der Stimme nur im ganzen Satz | die schwarze Note |
| TXT-REST `long` | an der Pause, mit der die Stille länger als eine Halbe wird | die Pause |
| TXT-CADENCE | am Taktanfang des Ziels | den Zielton |
| TXT-COMPLETE | nur im ganzen Satz, am Beginn des letzten Klangs | den letzten Ton |

Eine Silbe auf einer Pause oder Überbindung (nur in eingelesenen Sätzen) meldet TXT-ONSET an dieser
Note.

### Prüfung ohne Textkorpus

**Palestrinas Stimmen** (Test `palestrina_voices_admit_an_underlay`). Für jede Stimme der Abschnitte
Kyrie I, Christe, Kyrie II, Agnus I und Benedictus (erster Teil) sucht der Automat der Suche mit den
Stufen des Standardprofils, ohne Kapazitätsschranke, ohne Grenze der Zustände und ohne
Binnenschlüsse, eine Unterlegung des Textes ohne Verbot:

| Abschnitt | Stimmen | eleison viersilbig | eleison dreisilbig |
|---|---|---|---|
| Kyrie I | 151 | 136 (90,1 %) | 149 (98,7 %) |
| Christe | 107 | 104 (97,2 %) | 107 (100 %) |
| Kyrie II | 147 | 124 (84,4 %) | 142 (96,6 %) |
| Agnus I | 407 | 404 (99,3 %) | (kein eleison) |
| Benedictus | 225 | 222 (98,7 %) | (kein eleison) |
| alle | 1037 | 990 (95,5 %) | 1024 (98,7 %) |

Mit der einen oder der anderen Teilung tragen 1026 Stimmen ihren Text (98,9 %). Woran die übrigen
scheitern (vier / drei Silben): TXT-REST 26 / 5, TXT-COMPLETE 15 / 2, TXT-ONSET 5 / 5,
TXT-REPEATED-PITCH 1 / 1. Viersilbig scheitern vor allem Stimmen mit Tonwiederholungen: Jede
verlangt eine neue Silbe, und die Pause danach fällt dann mitten in e-lé-i-son. Der Test verlangt
viersilbig mindestens 90 %.

**Jeppesens Unterlegung** (Test `dies_sanctificatus_underlay`). Jeppesen druckt in *Counterpoint*
(S. 254–258) Ausschnitte aus Palestrinas *Missa Dies sanctificatus* mit Text: das Christe (mit
e-lei-son), die Anfänge von Gloria, Credo und Sanctus und das Et incarnatus, zusammen 362 Anschläge
in 20 Stimmen. Die Testdaten `tests/fixtures/text/dies-sanctificatus.toml` (aus Jeppesens Buch
abgeschrieben, deshalb nicht frei verteilt) halten seine Unterlegung Silbe für Silbe fest, die
Noten kommen aus dem Korpus; der Test läuft, wenn die Testdaten und Palestrinas Messen vorliegen.
Jeder Ausschnitt endet offen. Die Prüfung findet 30 Fälle
mit Kosten 37 und genau drei Verbote (Takte der Humdrum-Dateien, also Brevis-Takte):

| Ausschnitt | Kosten | Fälle |
|---|---|---|
| Christe, T. 20–27 | 10 | TXT-AFTER-BLACK `dotted` zweimal in T. 27: Alt „lei-“ und Tenor „ste“ auf der Halben nach dem untextierten Daktylus-Viertel; TXT-REPEAT `again` (Tenor); TXT-ACCENT `opening` (Bass „e-“) |
| Gloria, T. 1–8 | 4 | TXT-ACCENT `longer`: Alt „lun-“, über den Taktstrich gebunden vor „tá“ |
| Credo, T. 1–9 | 12 | TXT-ORDER V dreimal: Sopran und Alt beginnen mit „fa-ctó-rem“ (Jeppesen weist auf S. 257 darauf hin), der Bass lässt „fa-ctó-rem cae-li et ter-rae,“ aus; TXT-REST `comma` viermal und `long` zweimal; TXT-MELISMA `long` (Tenor, 9 Töne auf „ter-“), `last_syllable` (Bass „tem,“), `unstressed` (Alt „um“); TXT-AFTER-BLACK `unit_end` (Sopran „rae,“); TXT-ACCENT `opening` (Sopran „Fa-“) |
| Sanctus, T. 1–7 | 10 | TXT-MELISMA `long` fünfmal: Der Alt singt 15 Töne auf „San-“; TXT-REST `comma` dreimal und `long` zweimal |
| Et incarnatus, T. 59–63 | 1 | TXT-MELISMA `unstressed` (Tenor „tus“) |

TXT-ONSET `opening_black` und TXT-REST `last_word` schlagen in diesen Ausschnitten nie an. Mehrere
Werte stützen sich auf sie: TXT-AFTER-BLACK `dotted` ist P3 statt V, TXT-MELISMA `max_long` ist 8
statt 5, die Höchstzahlen `max_weak`, `max_opening` und `max_last` (4, 5, 5) sind die längsten
Melismen dieser Arten darin, und TXT-ONSET `figure_accent` kostet auch deshalb, weil Jeppesen keines
der 13 Daktylus-Viertel textiert.

### Text erzeugen

`fux free` setzt einen freien Satz mit Text (Optionen `--text`, `--length`, `--repeat`, `--accents`,
`--eleison`; ebenso die GUI und die Web-Version). Ohne `--text` singen die Stimmen das Kyrie I,
`--text none` setzt ohne Text (§ 16). Die Bibliothek `fux_core` bleibt ohne Text, solange eine
Anfrage keinen nennt. Drei bis fünf Stimmen mit Text imitieren standardmäßig
([§ 18](#18-imitation)): Dann plant der Planer der Imitation Länge,
Binnenschlüsse und Tempo, und was hier folgt, gilt für den freien Kontrapunkt um die Einsätze.
Die TXT-Stufen bleiben dieselben.

- **Plan.** `fux_core::text::plan` plant aus dem Text, wie oft er gesungen wird, die Länge, die
  Binnenschlüsse und das Tempo. Ein kurzer Text wird so oft geplant, dass etwa 14 Silben
  zusammenkommen (Kyrie I zweimal, Agnus Dei einmal), ein langer einmal, jeder Text einmal, wenn
  Wiederholungen verboten sind (`--repeat keine`). Das Tempo in Silbenanfängen je Takt,
  Wiederholungen eingeschlossen, folgt Palestrinas Dichte (vorläufig): 1,0 in Kyrie, Christe, Agnus
  Dei und eigenen kurzen Texten, 0,6 in den melismatischen Abschnitten Sanctus, Pleni, Osanna und
  Benedictus, 1,2 in Gloria, Credo und eigenen langen Texten.
- **Länge** (`--length auto`): max(8, round(m·S / Tempo) + 2) Takte für m Durchgänge von S Silben;
  die zwei Takte geben Raum für den gehaltenen Schlusston und das Melisma der Klausel; die Längen
  der Abschnitte zeigt die Tabelle oben. Die Länge wächst, bis jede Stimme für jede Silbe einen
  halben Schlag hat, ohne die halben Schläge, die ihre Kadenzformeln halten. Eine gegebene Länge,
  die dafür nicht reicht, scheitert vor
  der Suche: „Der Text braucht mindestens N Takte.“ Das Tempo, nach dem die Suche lenkt, verteilt
  die m·S Silben auf alle Takte außer den letzten zwei, auch bei einer gegebenen oder verlängerten
  Länge.
- **Binnenschlüsse** (`--cadences auto`): etwa einer je sechs Takte ((Takte − 4) / 6), gleichmäßig
  verteilt, jeder höchstens drei Takte weit auf das nächste geplante Ende eines Sinnabschnitts oder
  die nächste Zäsur verschoben. Geplant heißt: dort, wo die Silbe läge, wenn die Silben aller
  Durchgänge gleichmäßig über den Satz verteilt wären. Ein Zwischenschluss und eine Zäsur zählen einen
  Takt weiter entfernt als ein Satzende; bei gleicher Entfernung siegt das Ende eines Sinnabschnitts.
  Ein kurzer Text hat höchstens einen Binnenschluss je geplantem Ende eines Sinnabschnitts außer dem
  letzten, oder einen an einer Zäsur, wenn er sonst keinen Platz hat (Sanctus, Pleni, Benedictus).
  Das erste Ziel liegt frühestens im dritten Takt; zwischen zwei Zielen und vom letzten bis zum
  Schlusstakt liegen mindestens drei Takte. An einem Satzende wechseln die Kadenzstufen des Modus
  reihum (dorisch A, F, G), an einem Zwischenschluss oder einer Zäsur nur die Stufen nach der ersten
  (dorisch F, G), denn ein Binnenschluss auf der ersten Kadenzstufe gehört an den Punkt (TXT-CADENCE
  `principal`). Zwei Binnenschlüsse in Folge wechseln die Stufe; in Lydisch und Mixolydisch, deren
  Zwischenschlüsse nur eine Stufe haben (A bzw. C), bleibt sie, statt auf die erste Kadenzstufe zu
  gehen. Beispiele (dorisch):
  Kyrie I in Takt 7 auf A, am Ende des ersten Durchgangs; Christe in Takt 6 auf A; Agnus Dei III
  in Takt 13 auf F nach „mun-di:“; Sanctus in Takt 10 auf F vor „Dó-mi-nus“, Pleni in Takt 13 vor
  „gló-ri-a“, Benedictus in Takt 11 vor „in nó-mi-ne“, jeweils an der Zäsur vor dem Schlussteil;
  `gloria:1-5` in den Takten 13, 22 und 26 auf A, F und G, nach „vo-lun-tá-tis.“, „Be-ne-dí-ci-mus
  te.“ und „Ad-o-rá-mus te.“. Ein eigener Plan (`--cadences 7,12:F`) bleibt, wie er ist.
- **Pausen und Einsätze** wie ohne Text (§ 16): Jede Stimme setzt im ersten Takt ein. Ab drei
  Stimmen darf sie danach pausieren, außer in den drei Takten vor dem Ziel ihrer Klausel und im
  letzten Takt, zweistimmig nie. Wo, entscheidet der Text (TXT-REST), und nach jeder Pause beginnt
  eine Silbe. So pausieren singende Stimmen 0,30-mal je 10 Stimmtakte, im Mittel 1,7 Halbe lang, die
  meisten nach einem Satzende (Palestrina in Abschnitten bis 26 Takten: 0,30-mal, 1,9 Halbe). Mit
  Imitation (§ 18) setzt eine Stimme erst mit ihrem ersten Einsatz ein, mit der ersten Silbe des
  Textes, und schweigt vor jedem weiteren Einsatz mindestens eine Halbe; diese Stille zählt nicht
  als Platz für Silben.
- **Wiederholung** (`--repeat`): `auto` lässt das Profil entscheiden (TXT-REPEAT: in kurzen Texten
  frei, in langen P2, jede weitere im Satz P1, einzelne Wörter P2), `keine` verbietet Wiederholungen
  (`short` und `long` V), `frei` macht `short`, `long` und `again` frei. Den letzten Sinnabschnitt
  eines langen Textes, wo der Text endet, etwa das Amen, zählt TXT-REPEAT `long` nicht; er bleibt
  auch mit `keine` wiederholbar (eine zweite Wiederholung im Satz kostet `again`). Im Teil einer
  Messe, nach dem der Text im nächsten Teil weitergeht (`Text.continued`), zählt er wie jeder
  andere (§ 19); sonst wiederholte jeder Teil seinen letzten Sinnabschnitt so frei wie ein Amen. Der Plan zählt nur die
  geplanten Durchgänge; was eine Stimme wiederholt, entscheidet die Suche. Wiederholungen beginnen
  nach einer Pause oder auf einer weißen Note auf einem halben Schlag (in 98 Sätzen 115 und 93 von
  209), eine auf einer schwarzen Note.
- **Suche.** Die Regeln der Musik lesen keinen Text. Die Suche trägt in jedem Zustand je singender
  Stimme die Menge der Unterlegungen, die deren Noten zulassen: Zustände des Unterlegungsmodells, je
  mit den geringsten TXT-Kosten, höchstens 128. Mit jedem Ton geht das Modell einen Schritt, noch vor
  der Prüfung der Musik. Es bietet nur Silben an, die folgen dürfen, verwirft jeden Zustand, in dem
  ein Verbot entschieden ist, und jeden, der mehr Silben braucht, als die Stimme noch halbe Schläge
  hat (ohne die, die ihre Kadenzformeln halten, und die zweite Hälfte des Schlusstakts). Wird die
  Menge leer, scheidet der Kandidat ohne Regelprüfung aus, und die Diagnose nennt die TXT-Regel. Den
  Rang bestimmen die Kosten der Musik, die geringsten TXT-Kosten jeder Stimme und eine Lenkung: Als
  kostete sie mehr, zählt eine Stimme, die noch mehr Silben braucht als 80 % ihrer übrigen halben
  Schläge, in langen Texten auch eine, die weniger braucht als die Hälfte dessen, was das geplante
  Tempo darauf beginnt, und eine Formelstimme, deren Binnenschluss weniger als zwei Takte vor ihr
  liegt und die bis dahin das Ende ihres Sinnabschnitts oder die nächste Zäsur nicht mehr erreicht
  (Gewicht 2). Weil die Sätze der Suche noch keinen Text tragen, gilt für sie MEL-REPEAT `free` wie
  `free_texted` und FREE-REST `level` wie `texted`; so zahlt sie, was das Ergebnis kostet.
- **Mit Imitation** (§ 18) kommen drei Dinge dazu, im Modell der Suche wie im dynamischen
  Programm:
  - **Festgelegte Silben.** Jeder Anschlag im Kopf eines Einsatzes singt, was der Führer auf
    diesem Ton singt, eine neue Silbe oder das Melisma der vorigen; dort bleibt nur diese Wahl.
    So singen alle Einsätze eines Punktes dieselben Silben auf denselben Tönen (Jeppesen S. 160,
    Regel 8).
  - **Meilensteine.** Die nächste festgelegte Silbe einer Stimme ist zugleich ein Meilenstein:
    Ein Zustand fällt weg, wenn die Stimme bis dorthin mehr Silben beginnen müsste, als sie bis
    zum Einsatz noch halbe Schläge hat; die Diagnose nennt dann TXT-ORDER. Die Lenkung zählt auch
    eine Stimme teurer, deren Einsatz weniger als zwei Takte entfernt ist und die mehr Silben bis
    zu ihm braucht als 80 % der halben Schläge davor.
  - **Tore.** Die eigenen Worte eines späteren Punktes beginnt eine Stimme erst mit ihrem Einsatz
    in diesem Punkt, eine Stimme ohne Einsatz darin erst mit dessen zweitem Einsatz. Bis dahin
    singt sie die Worte davor, auch noch einmal, oder sie pausiert (Jeppesen S. 241: jeder Teil
    des Textes hat sein eigenes Thema, das imitiert wird, bevor der nächste kommt). Ohne die Tore
    sangen 21 von 22 Sätzen Worte späterer Punkte zu früh, 399 von 2252 Silbenanfängen.
  - **Wiederholung vor einem Punkt.** Beginnt ein Punkt mit eigenen Worten mitten in einem
    Sinnabschnitt, an einem Schlussteil (Be-ne-dí-ctus qui ve-nit / in nó-mi-ne Dó-mi-ni.), darf
    eine Stimme nach der Silbe davor wie nach dem Ende eines Sinnabschnitts von vorn beginnen: an
    jedem Anfang eines Sinnabschnitts oder Schlussteils ihres Satzes bis zu dieser Silbe, mit den
    Kosten von TXT-REPEAT. So singt eine Stimme, die auf ihr Tor wartet, „Be-ne-dí-ctus qui
    ve-nit“ noch einmal, statt „nit“ über Takte zu halten: Im vierstimmigen imitierenden
    Benedictus (vier Modi, Seeds 1–3) kamen vorher 8,2 Fälle von TXT-MELISMA `long` auf 100
    Stimmen-Takte, jetzt 0,20 (ohne Imitation 0,33). Die Prüfung kennt diese Stellen aus dem Plan
    der Partitur; ohne Plan (`fux check --imitation aus` oder `erkennen`) gilt ein solcher Neubeginn
    als TXT-ORDER (V).

  Dazu braucht in der Lenkung eine Stimme mit Einsätzen, die ihren Text beendet und danach
  pausiert hat, noch die kürzeste Wiederholung bis zum Schlusstakt, den alle singen; sonst
  füllten breite Strahlen sich mit Stimmen, die nach ihrem letzten Kopf schwiegen und nicht mehr
  enden konnten. Ohne Imitation ändert sich nichts.
- **Nach der Suche** legt ein dynamisches Programm über dieselben Schritte, ohne Schranke und ohne
  Grenze von 128, je Stimme die billigste Unterlegung fest, bei gleichen Kosten die mit den frühesten
  Silben (Stoquerus: „quamprimum“). Dann prüft der Prüfer den ganzen Satz mit allen Regeln; seine
  TXT-Kosten sind die des Programms. Die Silben werden ausgeschrieben, mit den Akzenten der Bücher
  oder ohne (`--accents aus`). `fux free` zeigt unter jeder Stimme ihren Text und zu jedem
  Binnenschluss, was die Oberstimme dort singt (`Takt 7 auf A: …e-lé-i-son.`).
- **Garantie.** Jede Stimme eines Ergebnisses beginnt mit der ersten Silbe, singt den Text in seiner
  Ordnung, beginnt nach jeder Pause eine Silbe und singt auf dem letzten Ton die letzte Silbe. Kein
  Ergebnis verstößt gegen ein Verbot, auch nicht gegen ein TXT-Verbot, das ein eigenes Profil setzt:
  Jeder TXT-Fall betrifft eine Stimme, und das Modell schneidet ihn genau dort ab, wo er entschieden
  ist. Findet die Suche nichts, geht sie wie ohne Text bis zu zwölfmal an frühere Taktanfänge zurück
  und verbreitert dann den Strahl auf das Doppelte. Bei automatischer Länge folgt ein Plan mit einem
  Viertel mehr Takten (Binnenschlüsse neu geplant) mit allen Strahlbreiten, sonst der vierfache Strahl.
  Erst dann nennt die Meldung die Regeln, die im Weg standen, etwa TXT-COMPLETE, mit Hinweisen (mehr
  Takte, `--repeat frei`, ein kürzerer Text, andere Binnenschlüsse). Eine Anfrage mit Text liefert nie
  einen Satz ohne oder mit unvollständigem Text. Mit Imitation gilt dasselbe, und jeder Einsatz singt
  seine festgelegten Silben; wie der Treiber dort neu plant und den Strahl verbreitert, steht in
  § 18.
- **Freie Stimmen um einen Cantus:** `fux cp --species 0 --text kyrie` und die GUI
  (Gattung 0, Gruppe „Text“) lassen die freien Gegenstimmen einen Text singen; der Cantus behält
  seinen eigenen Text oder bleibt ohne. Die Länge ist die des Cantus. Der Plan zählt die halben
  Schläge einer Stimme ohne Kadenzformeln, denn um einen Cantus halten die freien Stimmen keine
  Klauseln; reicht der Cantus nicht, scheitert die Anfrage vor der Suche („Der Text braucht
  mindestens 19 Takte. Der Cantus hat 11 Takte.“, mit Abhilfe). Einen Text singen nur freie
  Gegenstimmen (Gattung 0). Eine MusicXML-Datei mit Cantus und `fux-text` liest `fux check` mit
  ihren übrigen Stimmen als freie Stimmen, so dass ihre Prüfung den Bericht des Erzeugens
  wiederholt. Über den Cantus von Fux gelingen alle 36 Sätze zwei- bis vierstimmig ohne Verbot;
  ein kurzer Text über einem langen Choral kann am Ende scheitern (Kyrie I über Veni Creator, 45
  Takte, dreistimmig).
- **Laufzeit** ohne Imitation (Release, 8 Kerne, dorisch, Mittel der Seeds 1 und 2, Test
  `timings_with_text`; mit Imitation siehe § 18):

  | Text, Takte | zweistimmig | dreistimmig | vierstimmig | fünfstimmig | sechsstimmig |
  |---|---|---|---|---|---|
  | Kyrie I, 16 | 0,9 s | 2,1 s | 3,0 s | 29 s (ohne Text 5,4 s) | 8,9 s |
  | Christe, 14 | 0,7 s | 1,8 s | 2,5 s | 5,7 s | 8,3 s |
  | Agnus Dei III, 20 | 1,3 s | 3,2 s | 4,2 s | 8,5 s | 11 s (ohne Text 26 s) |
  | `gloria:1-5`, 32 | 2,4 s | 6,4 s | 9,0 s | 19 s | 72 s (ohne Text 214 s) |

  Bis vier Stimmen braucht ein Satz mit Text meist 0,8- bis 1,25-mal so lange wie derselbe Plan ohne
  Text. Ausreißer sind ganze Verbreiterungen des Strahls, die mal den Satz mit, mal den ohne Text
  treffen, vor allem ab fünf Stimmen (Kyrie I fünfstimmig, Seed 1: 16 Rücksprünge, dann der doppelte
  Strahl, im Mittel beider Seeds 5,4-mal; `gloria:1-5` sechsstimmig ohne Text dreimal so lang wie
  mit), aber auch darunter (vor den abschließenden Korrekturen `gloria:1-5` vierstimmig, Seed 1:
  2,3-mal). Der Benchmark (`cargo bench -p fux-core --bench search -- free`, vierstimmig, Plan des
  Kyrie I) misst 3,05 s gegen 2,65 s ohne Text (1,15-mal), der Test `texted_search_is_fast_enough`
  1,06-mal (Grenze 1,5). Die
  langen Abschnitte des Gloria und Credo (76–109 Takte) brauchen vierstimmig Minuten. Im Browser ohne
  Threads dauert das Kyrie I zweistimmig etwa 5 s, vierstimmig etwa 15 s.
- **Gemessen** an 98 Sätzen (Kyrie I, Christe, Agnus Dei III, `gloria:1-5`, Sanctus und Benedictus in
  vier Modi, zwei- bis fünfstimmig) gegen Palestrina (Korpus, Deklamation aus Jeppesens Unterlegung):

  | | erzeugt | Palestrina |
  |---|---|---|
  | Töne je Silbe | 1,76 (Kyrie I 1,90, Christe 2,04, `gloria:1-5` 1,41) | Christe 2,00, Gloria 1,07, Credo 1,45 |
  | betonte Silben auf dem Taktanfang oder einem längeren Ton als dem davor | 80 % | 86 % |
  | weiße Noten, die den Ton davor neu anschlagen | 12,7 % | 6,5 % (Abschnitte bis 26 Takte) bis 10,3 % (alle) |
  | Folgen von drei und mehr gleichen Tönen je 100 Stimmtakte | 0,14 | 1,0 (Abschnitte bis 26 Takte) bis 2,2 (alle) |
  | Pausen im Inneren je 10 Stimmtakte, mittlere Länge | 0,30, 1,7 Halbe | 0,30, 1,9 Halbe (Abschnitte bis 26 Takte) |
  | Wiederholungen nach einer Pause / auf einer weißen Note / auf einer schwarzen Note | 115 / 93 / 1 | Wiedereinsätze nach einer Pause zu 97–99 % auf einer Halben oder länger |
  | Melismen ab 6 Tönen auf unbetonten Silben / unbetonten Eröffnungen | 1 / 2 | höchstens 4 / 5 Töne (Jeppesen) |
  | letzte Silbe einer Stimme mit 5 und mehr Tönen über 3 und mehr Takte | 8 von 330 Stimmen | ein Melisma am Ende eines Sinnabschnitts, 5 Töne (Jeppesen) |
  | Pausen, nach denen nur das letzte Wort bis zum Satzzeichen oder zur Zäsur bleibt (`last_word`) | 7 in 5 Sätzen | in Jeppesens Unterlegung keine |

  Der Text trägt 28 % der Kosten: TXT-CADENCE 37 %, TXT-ONSET 23 % (fast ganz `figure_accent`),
  TXT-MELISMA 18 %, TXT-ACCENT 9 %, TXT-REST 7 %.
- **Grenzen.**
  - Sechs Stimmen mit langen Texten brauchen eine bis mehrere Minuten und schwanken stark, ohne Text
    ebenso. In WebAssembly wächst der Strahl höchstens auf das Doppelte, weil fünf und sechs Stimmen
    mit dem Vierfachen mehrere Gigabyte brauchen.
  - Mit `free_run` P1 brauchen das ionische Credo (vierstimmig, Seed 1: 137 statt 109 Takte) und Et
    in Spiritum (mixolydisch: 128 statt 102) den längeren Plan. Weil bei automatischer Länge der
    längere Plan vor dem vierfachen Strahl kommt, kann ein Satz auch länger werden, den der
    vierfache Strahl in seiner Länge gefunden hätte (vor den abschließenden Korrekturen das
    fünfstimmige äolische `gloria:1-5`: 40 statt 32 Takte); die Reihenfolge ist offen.
  - Formelstimmen singen an manchen Binnenschlüssen durch: In den 98 Sätzen kostet das 55-mal
    TXT-CADENCE `level`, 43-mal in `gloria:1-5`, wo die Stimmen vor dem Ziel weder auf einer
    unbetonten Silbe verweilen noch vor dem letzten Wort pausieren sollen. 17-mal endet am Ziel nur
    der Text vor einem Schlussteil, auf der Finalis oder der ersten Kadenzstufe (`principal`: das
    „e“ von Ký-ri-e, das „ste“ von Chri-ste), 87-mal ein Sinnabschnitt mit Zwischenschluss oder der
    Text vor einer Zäsur auf einer anderen Stufe (`comma`).
  - Bei gleichen Kosten legt das dynamische Programm die Silben so früh wie möglich, auch die letzte
    eines Sinnabschnitts, die besser auf den letzten Ton fiele (Zarlino, Regel 10): 8 von 330
    Stimmen halten sie noch fünf und mehr Töne über drei und mehr Takte.
  - Kurze vorletzte Silben (e-lé-I-son, Dó-MI-ni) stehen zu 21 % auf einer Ganzen oder länger, meist
    auf der vorletzten Note der Tenor- und Bassklausel (Zarlino IV.32 nennt genau das einen
    Barbarismus; Jeppesen: keine von 14). Mit `--eleison 3` entfällt der Fall im eleison.
  - Das Daktylus-Viertel trägt 13,6 % aller Silbenanfänge, in Jeppesens Unterlegung keines von 13.
    Die Figur ist zu häufig (21 je 100 Stimmtakte, auch ohne Text; Palestrina etwa 8); `figure` zu
    bepreisen trieb die Formelstimmen in TXT-CADENCE.
  - Lange Texte deklamieren kontrapunktisch: `gloria:1-5` mit 1,41 Tönen je Silbe, Palestrinas
    Gloria in Jeppesens Ausschnitt mit 1,07 (sein Credo mit 1,45). Gleichzeitige Deklamation und
    Homophonie sind nicht umgesetzt (§ 19).
  - Noch nicht umgesetzt: Auslassungen in Gloria und Credo (auch in der Messe nicht, § 19),
    keine Silbe auf einer Dissonanz (Vicentino 9, der sich selbst nicht daran hält), der Höhepunkt
    auf dem Akzent, tote Intervalle über Pausen, ein zweisilbiges Ky-rie und Text im MIDI. Einsätze
    der Imitation mit Text und Text in `fux cp --species 0` beschreiben § 18 und der Abschnitt oben.

### Text prüfen

`fux check --musicxml satz.musicxml` prüft einen Satz mit Liedtext, ebenso die GUI („Prüfen“,
„Datei …“) und die Web-Version („Text der Stimmen“). Der MusicXML-Leser nimmt je Stimme die erste
Strophe; ein Liedtext, der nur eine Melismalinie (`extend`) trägt, ist keine Silbe. Silben auf
Pausen bleiben und werden gemeldet (TXT-ONSET).

- **Zuordnung** (`fux_core::text::align`): Die geschriebenen Silben werden den Stellen des Textes
  zugeordnet, verglichen ohne Groß- und Kleinschreibung, Akzente, Satzzeichen, Bindestriche und h,
  mit j und y wie i, æ, œ und oe wie ae (eleyson, coeli und Osanna passen). Ein kleines dynamisches
  Programm wählt je Stimme die billigste Folge: Die nächste Stelle und ein Neubeginn nach dem Ende
  eines Sinnabschnitts kosten nichts, jeder andere Sprung, eine unbekannte und eine anders
  geschriebene Silbe je 1. Eine anders geschriebene Silbe wird nach ihrer Stelle zugeordnet (Jeppesens
  o-mni-po-ten-tem). Unbekannte und anders geschriebene Silben nennt eine Warnung mit Takt, Sprünge
  meldet TXT-ORDER. Ein Cantus firmus singt nie.
- **Was eine Datei von fux-kontrapunkt mitbringt.** MusicXML hat kein Element für den Text, den die
  Silben singen, für den Kadenzplan und für den plagalen Modus. fux-kontrapunkt schreibt sie deshalb
  als `miscellaneous-field` in die Identifikation der Datei: `fux-text` (`gloria:1-5`, ein eigener
  Text ganz), `fux-cadences` (`13:A,22:F,26:G`, Takt und Ziel) und `fux-mode`
  (`hypophrygian`). Beim Prüfen gelten sie, wenn nichts anderes angegeben ist: `--text`, `--mode`
  und `--cadences` (auch mit Ziel, `--cadences 7:A`) gehen vor, `--cadences none` prüft ohne
  Binnenschlüsse. So ergibt die Prüfung eines exportierten Satzes denselben Bericht wie beim
  Erzeugen, in der GUI („Ergebnis übernehmen“ und „Datei …“) und in der Web-Version ebenso. Ist der
  Satz mit `--repeat keine` oder `frei` entstanden, nennt man das beim Prüfen ebenso
  (`fux check --repeat frei`, in GUI und Web-Version das Feld „Wiederholung“), sonst kosten die
  Wiederholungen nach dem Profil.
- **Welcher Text.** `--text kyrie`, `--text gloria:1-5` oder ein eigener Text nennen den Text, beide
  Teilungen von eleison werden versucht; ohne `--text` gilt der Text der Datei. Nennt die Datei
  keinen, probiert `fux check` jeden Abschnitt des Ordinariums mit beiden Teilungen von eleison und
  nimmt den mit den wenigsten abweichenden Silben (unbekannt, anders geschrieben oder durch einen
  Sprung erreicht). Singen die Stimmen nur einige seiner Sinnabschnitte, gilt der Bereich vom ersten
  bis zum letzten, in dem eine Stimme eine Silbe in der Ordnung des Textes singt (`gloria:1-3`),
  wenn dabei nicht mehr Silben abweichen. Erkannt ist der Text, wenn höchstens 5 % abweichen: „Text
  erkannt: Gloria: Et in terra 1–3; von 50 gesungenen Silben weichen 0 ab“. Sonst bleiben die
  Textregeln ungeprüft („Text nicht erkannt; mit --text angeben“). Hören alle Stimmen eines ganzen
  Abschnitts am selben Sinnabschnitt auf, gilt auch das als Bereich; die Meldung nennt ihn, und
  `--text gloria` prüft den ganzen Abschnitt. `--text none` prüft ohne Textregeln;
  Tonwiederholungen kosten dann wie ohne Text (MEL-REPEAT `free`).
- **Binnenschlüsse** anderer Dateien kennt die Prüfung nur, wenn `--cadences` sie nennt; ohne sie
  prüft TXT-CADENCE nichts. Die GUI hat dafür das Feld „Binnenschlüsse“, das „Ergebnis übernehmen“
  füllt; die Web-Version nimmt sie nur aus Dateien von fux-kontrapunkt.
- **Meldungen** nennen die Silbe am markierten Ton, etwa
  `Takt 5, Viertel 4 („e“), Bass: Melisma an falscher Stelle oder zu lang: …`.
- **Ausgabe.** LilyPond setzt den Text unter jede singende Stimme, mit Silbenstrichen (`--`) und
  Melismalinien (`__`), die Intervallziffern darunter. MusicXML schreibt ihn als `lyric` mit
  `syllabic` und Melismalinien (`extend`), die MuseScore und Verovio zeichnen. Die Linie reicht bis
  unter den letzten Ton der Silbe, auch unter eine Überbindung, und auch eine Silbe am Wortende, die
  nur übergebunden gehalten wird, bekommt eine. MIDI bleibt ohne Text.

## 18. Imitation

Die Stimmen eines freien Satzes mit Text imitieren einander, wie in Palestrinas Messen. Jeder Teil
des Textes ist ein **Punkt**: Seine Stimmen setzen nacheinander mit dem Kopf desselben Soggetto ein,
und jeder Punkt außer dem letzten endet dort, wo seine Worte enden, in einem Binnenschluss. Ein
**Planer** legt vor der Suche Punkte, Soggetti, Einsätze und Binnenschlüsse fest. Die Einsätze
werden feste Töne mit festen Silben, und die Strahlsuche setzt den freien Kontrapunkt um sie herum
(§ 16, § 17). Die IMI-Regeln prüfen, wie ein Satz seinen Plan ausführt und wie der Plan Palestrinas
Praxis folgt.
`fux free`, die GUI und die Web-Version imitieren standardmäßig (`--imitation auto`), und zwar mit
Text, drei bis fünf Stimmen, automatischen Binnenschlüssen und automatischer Länge; mit gegebener
Länge (`--length N`) setzt `auto` ohne Imitation und sagt es. `--imitation an` imitiert auch zwei-
und sechsstimmig und in einer gegebenen Länge; gemessen ist nur der drei- bis fünfstimmige Satz in
der Länge seines Plans. `--imitation aus` setzt ohne Imitation, mit der Suche von § 17; was die
Imitation an der Suche ändert, greift dort nicht. Das halten die Golden-Tests
`untexted_results_are_unchanged`, `untexted_requests_stay_untexted` und
`texted_results_are_unchanged` fest. Imitation braucht einen Text und automatische Binnenschlüsse;
mit `--text none` oder eigenen `--cadences` scheitert `--imitation an` mit einer Meldung auf Deutsch
oder Englisch, die eine Abhilfe nennt. Die Bibliothek `fux_core` imitiert nur, wenn eine Anfrage es
verlangt (`FreeRequest::imitation`, in `FreeRequest::new` leer).

**Quellen**, wie die Tabellen sie nennen (Fux mit den Seiten des lateinischen Originals):
- Zarlino: *Le istitutioni harmoniche*, Venedig 1558, Buch III, Kap. 51–52 (Fuge und Imitation)
  und Buch IV, Kap. 18 (Anfänge auf den Grenztönen des Modus). Vicentino: *L'antica musica ridotta
  alla moderna prattica*, Rom 1555, Buch IV, Kap. 14, 15, 29 und 32.
- Pietro Pontio: *Dialogo … della theorica e prattica di musica*, Parma 1595. Orazio Tigrini: *Il
  compendio della musica*, Venedig 1588, Buch IV, Kap. 2. Morley 1597, Teil III.
- Fux 1725, S. 144 (modale Antwort), 161 (Pausen), 168 f. (der Tenor folgt dem Cantus, der Bass
  dem Alt), 244 (Soggetti), 247 (Pause vor einem neuen Soggetto, Probe der Engführung, Einsatz am
  Binnenschluss).
- Jeppesen: *Counterpoint* (1939), vor allem S. 136–138, 160, 164–170 und 241. R. O. Morris:
  *Contrapuntal Technique in the Sixteenth Century*, Oxford 1922, S. 46 f. P. Wagner: *Geschichte
  der Messe* I, Leipzig 1913, S. 446 (Palestrinas Choralmessen).
- Palestrinas Messen (Korpus von § 16): Eröffnungen und Punkte, Soggetti und Paraphrasen von
  Choral, gezählt mit dem Erkenner unter [Prüfen](#prüfen). Unabhängige Nachzählungen haben die
  Zahlen meist auf einen Prozentpunkt bestätigt. Wo sie abwichen, gelten hier ihre Werte, etwa für
  die Führer späterer Punkte, die Abstände der Einsätze und die Nachbarschaft der Stimmen in der
  Folge der Einsätze, die bei Palestrina nicht häufiger ist als zufällig.

### Begriffe

- **Punkt** (Imitationspunkt): ein Teil des Textes mit seinem Soggetto und seinen Einsätzen. Er
  endet in einem Binnenschluss, der letzte im Schluss.
- **Einsatz:** der erste Ton einer Stimme in einem Punkt, als erster Ton der Stimme überhaupt oder
  nach einer Pause von mindestens einer Halben. Der erste Einsatz eines Punktes ist sein
  **Führer**, die übrigen folgen ihm. Kommt ein Einsatz, bevor der vorige seinen Kopf beendet hat,
  steht er in **Engführung**.
- **Kopf:** die Töne eines Einsatzes, die der Plan festlegt: die ersten Töne des Soggetto,
  mindestens vier, mit Tonhöhen, Werten und Silben. Danach singt die Stimme frei weiter.
- **Soggetto:** das Thema eines Punktes, aus dem Text, aus einer Choralphrase oder aus eigenen
  Noten gebaut: Stufen und Werte seiner Töne und die Silben, die sie singen.
- **Antwort:** wie ein späterer Einsatz das Soggetto bringt. Er beginnt auf derselben Stufe wie der
  Führer (Einklang, Oktave) oder auf der anderen **Hauptstufe**: der Quinte gegen die Finalis und
  umgekehrt, der Finalis gegen die Quarte, im Phrygischen A gegen E. Die Antwort ist **real**, wenn
  sie dieselben diatonischen Intervalle singt, und **modal**, wenn sie in einem der ersten beiden
  Intervalle Quarte und Quinte tauscht, damit Finalis und Quinte einander antworten (Fux S. 144).
- **Ruhefenster:** die Pause, die der Plan vor einem Wiedereinsatz verlangt, mindestens eine
  Halbe; die Suche darf länger pausieren. Vor ihrem ersten Einsatz schweigt eine Stimme vom Anfang
  des Satzes an.
- **Formelstimmen** eines Binnenschlusses sind die Stimmen, die seine Klauseln singen (§ 16): die
  Oberstimme die Diskantklausel, der Bass und ab vier Stimmen die Stimme über ihm die übrigen.
- **Tor:** der Zeitpunkt, ab dem eine Stimme die eigenen Worte eines späteren Punktes beginnen
  darf: ihr Einsatz in diesem Punkt, für eine Stimme ohne Einsatz darin der zweite Einsatz des
  Punktes (§ 17).

Zeitangaben in **Ticks** zählen wie im Programm Achtel: eine Halbe 4, ein 2/2-Takt (eine Ganze) 8,
eine Brevis 16.

### Regeln der Imitation

Alle IMI-Regeln schweigen, solange eine Partitur keinen Imitationsplan trägt (`Score.imitation`),
außer drei Parametern für Sätze ohne Plan: `unplanned_struck`, `unplanned_passing` (IMI-HELD) und
`unplanned_thin` (IMI-SILENCE). Das Standardprofil schaltet sie aus (unten und § 19). Fux'
Beispiele, Palestrinas Humdrum-Dateien und fremde MusicXML-Dateien tragen keinen Plan; die Zahlen
von § 14 und § 16 bleiben deshalb gleich. Die Standardwerte folgen der
[Leitlinie](#leitlinie-für-standardwerte) mit Palestrinas Häufigkeiten. Was der Plan allein
entscheidet (Intervall, Abstand, Antwort, Gestalt, Stufe, `late`), kostet in jedem Zustand der Suche
gleich viel; Planer und Soggetto-Bau bepreisen es vorab.

| ID | Regel | Standard | Quelle |
|---|---|---|---|
| IMI-ENTRY | Ein Einsatz weicht von seinem Kopf ab. Ein Kopfton erklingt nicht wie geplant: mit anderer Tonhöhe, als Pause, als Überbindung oder ohne Anschlag zu seiner Zeit (`level`, V). Ein Kopfton außer dem ersten ist kürzer als geplant (`rhythm`, P1); länger gehalten verdrängt er den nächsten Kopfton (`level`), und der letzte Kopfton darf in den freien Kontrapunkt weiterklingen. In einer singenden Stimme trägt ein Anschlag des Kopfes eine andere Silbe als der Führer auf diesem Ton, eine Silbe, wo der Führer die seine hält, oder keine, wo er eine beginnt (`text`, P2, vorläufig). | V / P2 / P1 | Zarlino III.51 (die Fuge folgt mit denselben Intervallen); Jeppesen S. 160, Regel 8 (die Unterlegung des ersten Einsatzes gilt in allen folgenden); Vicentino IV.29 („la medesima pronuntia“). Palestrina ändert die Werte der Töne 2–4 in 22 % der imitierenden Einsätze. |
| IMI-REST | Ein geplanter Einsatz einer Stimme, die schon gesungen hat, kommt ohne Pause oder nach einer Pause kürzer als eine Halbe (`level`, V). Ein Einsatz liegt zwischen den halben Schlägen (`offbeat`, V). Der Ton vor der Pause ist eine Halbe oder kürzer (`minim_before`, P2, vorläufig). | V / P2 | Pontio 1595, S. 62–65 („almeno d'vna pausa di Minima“); Vicentino IV.15 (keine Minima vor den Pausen in lateinischer Kirchenmusik); Fux S. 247; Jeppesen S. 269. Palestrina: Wiedereinsätze nach weniger als einer Halben 24 von 35 683 (0,07 %, unter der Schwelle von etwa 0,1 %, bis zu der ein Verbot nach § 16 Verbot bleibt); erste Einsätze zwischen den halben Schlägen 0 von 5 377; vor 4,1 % der Pausen eine Halbe. |
| IMI-SILENCE | Eine Stimme, die eingesetzt hat, schweigt auf einem Taktanfang, bevor die letzte Stimme ihren ersten Einsatz hatte (`exposition`, P2 je Taktanfang). Eine Stimme schweigt länger als vier Breven (acht Takte), je Taktanfang darüber hinaus (`long`, P3). Nach dem letzten ersten Einsatz klingt vier- oder sechsstimmig auf einem Taktanfang vor dem letzten Takt genau die Hälfte der Stimmen (`thin`, P1 je Taktanfang; weniger als die Hälfte bepreist FREE-TEXTURE). Ohne Imitationsplan dasselbe vom ersten Ton jeder Stimme an (`unplanned_thin`: aus, im Profil der Messe P1, § 19). Eine Stimme, die einen Abschnitt einer Messe nicht singt, zählt bei `thin` und `unplanned_thin` nicht mit (§ 19). | P2 / P3 / P1 | Fux S. 161 („nunquam pausam ponendam esse, nisi immediate subjectum sequatur“); Vicentino IV.29 (die zuerst endende Stimme beginnt zuerst wieder). Palestrina, drei- bis sechsstimmig: Stimmen, die eingesetzt haben, schweigen vor dem letzten ersten Einsatz an 1,6–3,7 % dieser Taktanfänge (im Mittel etwa 2,4 %); Pausen von vier Breven und mehr sind 0–3,2 % der Pausen ab einer Halben (vierstimmig 1,1 %) und 0–2,5 % der Zeit der Stimmen. Genau die Hälfte klingt nach dem letzten ersten Einsatz eines Abschnitts vierstimmig an 10,7 %, sechsstimmig an 9,9 % der Taktanfänge (Test `held_dissonances_and_half_textures_in_the_corpus`); vom ersten Ton jeder Stimme eines Satzes an, wie `unplanned_thin` zählt, an 11,4 und 10,1 %. |
| IMI-HELD | Ein Ton wird auf dem Taktanfang angeschlagen, dissonant zu einem Ton, der über den Taktstrich gebunden ist, und dieser geht nicht wie ein Vorhalt stufenweise abwärts (direkt oder verziert auf dem dritten Viertel), wo FREE-RESOLUTION ihn nicht als Vorhalt prüft (`struck`, V). Ein Ton nach dem Taktanfang ist dissonant zu einem übergebundenen Ton, der kein Vorhalt ist (auf dem Taktanfang konsonant oder nicht abwärts gehend), und ist weder Durchgang noch eine Figur von FREE-PASSING (`passing`, P3). Dissonant heißt wie in § 16: gegen den Bass auch die Quarte, auch wenn nur zwei Stimmen klingen. Ohne Imitationsplan oder mit einem Plan ohne Punkte prüfen `unplanned_struck` und `unplanned_passing` dieselben Fälle (aus, im Profil der Messe V und P3, § 19). | V / P3 | Wie FREE-PASSING und FREE-RESOLUTION (§ 16), die diese Fälle nicht beurteilen: FREE-PASSING entschuldigt einen Ton gegen einen übergebundenen, als wäre dieser der Vorhalt, und FREE-RESOLUTION prüft je Ton nur den ersten Zusammenstoß und keinen Bass unter einer Quarte. Palestrina, mit einem Plan ohne Einsätze gezählt: `struck` 12 von 40 381 dissonanten Anschlägen gegen einen übergebundenen Ton auf dem Taktanfang (0,03 %, also Verbot), `passing` 63 von 14 650 danach (0,43 %, P3); ohne Plan dieselben Fälle. |
| IMI-INTERVAL | Ein späterer Einsatz beginnt nicht im Einklang, in der Oktave, Quarte oder Quinte zum ersten Ton des Führers: in der Terz oder Sexte, auch der Dezime (`third_sixth`, P2), in der Sekunde oder Septime (`second_seventh`, P3). | P2 / P3 | Zarlino III.51, S. 212, und III.52, S. 217 (die Fuge im Einklang, in der Quarte, Quinte oder Oktave, die Imitation in den übrigen Intervallen); Morley S. 155 („seldome vsed“); Jeppesen S. 167. Palestrina: zweite Einsätze in der Terz oder Sexte etwa 5 % (meist Terzen und Dezimen unter dem Führer, echte Sexten 0,8 %), in der Sekunde oder Septime 0,1 % der imitierenden. |
| IMI-TIME | Der zweite Einsatz eines Punktes kommt mehr als zwei Breven (vier Takte) nach dem ersten (`second`, P3) oder erst nach dem Ende des Kopfes des Führers (`apart`, P2). Ein weiterer Einsatz kommt mehr als vier Breven nach dem vorigen (`later`, P2). | P3 / P2 | Vicentino IV.14 („quattro pause sono quasi troppo“) und IV.32; Pontio S. 48 (nicht mehr als zwei Breven); Morris S. 47 (die Antwort setzt vor dem Ende des Themas ein); Tigrini IV.2 (im Kanon mit mehr Stimmen auch acht oder zehn Pausen). Palestrina: zweiter Einsatz in 735 von 737 Eröffnungen nach höchstens zwei Breven (zweimal nach 40 Ticks: 0,27 %, über 0,1 %, also kein Verbot); erst nach dem Kopf 3,9 %; spätere Abstände über vier Breven 2,8 %. |
| IMI-ANSWER | Ein späterer Einsatz ändert ein Intervall des Führers: Quarte und Quinte in den Intervallen 1–2 getauscht (`modal`, P1), eine andere Änderung um eine Stufe in den Intervallen 1–3 (`adjusted`, P2). Größere Änderungen meldet die Regel nicht. | P1 / P2 | Fux S. 144 (die Quinte wird mit der Quarte beantwortet und umgekehrt); Vicentino IV.32 (in Fugen in Gegenbewegung; Jeppesen S. 22 liest es als erste Regel der tonalen Antwort); Morris S. 47; Jeppesen S. 164 (die reale Antwort ist bei Palestrina die häufigste). Palestrina tauscht Quarte und Quinte in 12,6–20,7 % der Köpfe, wo es möglich ist, zwischen Finalis und Quinte in 32 %; andere Änderungen um eine Stufe etwa 2–5 % (eine obere Grenze, denn stärker veränderte Antworten erkennt die Zählung nicht). |
| IMI-SUBJECT | Gestalt des Soggetto, am Kopf des Führers jedes Punktes. Im ersten Punkt ist der erste Ton kürzer als eine Ganze (`short_first`, P2) oder kürzer als einer der Töne 2–4 (`not_longest`, P1; frei nach einer unbetonten ersten Silbe wie Ho-sán-na). In jedem Punkt: Ein Wert fällt auf ein Viertel des vorigen oder weniger, außer nach einem punktierten (`drop`, P2); der Kopf umfasst eine Septime oder Oktave (`range`, P1) oder mehr als eine Oktave (`beyond_octave`, V, an jedem Kopf, auch an einer Antwort); kein Sprung ab der Terz und kein Wertwechsel in den Intervallen 1–5 (`plain`, P1, vorläufig); ein zweiter Sprung ab der Quarte (`leaps`, P1, vorläufig); der letzte Ton des Kopfes ist schwarz oder liegt nicht auf dem Taktanfang (`end`, P1). | V / P2 / P1 | Vicentino IV.29 (die Anfänge „col moto tardo“); Jeppesen S. 136–138 (vom Langsamen zum Schnellen) und S. 167–168 (nicht zu glatt); Fux S. 244 („subjecta … naturalia, facilia, non tamen sterilia“); Morris S. 46 (im Umfang des Modus). Palestrinas erste Einsätze: erster Ton kürzer als eine Ganze 3,6 %, kürzer als einer der Töne 2–4 11,6 % (im Hosanna ist der zweite Ton in 18 % länger: Ganze, punktierte Ganze, Halbe); Brevis zu Halbe oder Viertel 1,4 %, Ganze zu Viertel 3,6 % der Übergänge; Umfang eine Septime oder Oktave 14 %, mehr nie; letzter Ton schwarz oder nicht auf dem Taktanfang 12–21 %. |
| IMI-DEGREE | Ein späterer Punkt beginnt nicht auf der Finalis oder der Quinte: auf der Quarte (`fourth`, P1) oder auf einer anderen Stufe (`level`, P2). Im Phrygischen gilt A als Quinte und B wie die Quarte. Den ersten Punkt beurteilt FREE-ENTRY (§ 16). | P1 / P2 | Zarlino IV.18, S. 320–321 (die Anfänge auf den Grenztönen von Quinte und Quarte des Modus); Pontio S. 49; Morris S. 46–47; Jeppesen S. 170, Anm. 36 (nennt im Phrygischen B die Dominante; im Korpus ist A häufiger). Palestrina: erste Einsätze auf der Quarte 12,2 %, auf jeder anderen Stufe 3,7–8,9 %. |
| IMI-OVERLAP | Der erste Einsatz eines späteren Punktes kommt mehr als eine Ganze nach dem Ziel des Binnenschlusses davor (`late`, P1, vorläufig), oder keine andere Stimme klingt bei ihm (`bare`, P3). | P1 / P3 | Fux S. 247 („aut sub ipsam modulationem, aut in Clausula, rariús post Clausulam factam“); Vicentino IV.15 (nicht warten, bis eine Stimme geendet hat); Morley S. 158 (ein Einsatz, während alle anderen pausieren, klingt „bare and lame“). Palestrina: neue Punkte beginnen auf dem Ziel einer Klausel oder eine Ganze danach 1,7-mal so oft wie zufällig; an 0,1 % der Taktanfänge klingt keine Stimme (FREE-TEXTURE). |

Geprüft werden die geplanten Einsätze. Andere Wiedereinsätze nach Pausen beurteilen IMI-SILENCE,
FREE-ENTRY `reentry_short` und die TXT-Regeln. IMI-ENTRY `text` vergleicht Silben nur, wo der
Einsatz und sein Führer beide den Text singen; die Sätze der Suche tragen noch keinen Text
(§ 17), dort gilt der Fall erst im fertigen Satz. Erzeugte Sätze halten IMI-ENTRY, IMI-REST
`level` und `offbeat` und IMI-SUBJECT `beyond_octave` durch Konstruktion ein; die übrigen Fälle
zahlen sie wie jede Präferenz.

IMI-HELD und IMI-SILENCE `thin` schließen Lücken der Regeln von § 16, die erst die Imitation häufig
macht. Lange Kopftöne werden über den Taktstrich gebunden, und Eröffnungen beginnen zweistimmig: Das
vierstimmige Kyrie I (Seed 1) schlug in Takt 5 eine unvorbereitete Quarte zweier Stimmen an, und in
erzeugten Sätzen kamen Dissonanzen gegen übergebundene Töne, im Sprung erreicht oder auf dem
Taktanfang ohne Auflösung, 20- bis 300-mal so oft vor wie bei Palestrina, ohne dass eine Regel sie
fand. Vierstimmig klangen nach der Exposition auf 19 % der halben Schläge nur zwei Stimmen (in
Palestrinas Kyrie-Abschnitten 3,8 %), denn FREE-TEXTURE bepreist erst weniger als die Hälfte, und
Pausen nach dem Ende eines Satzes des Textes sind frei. FREE-PASSING, FREE-RESOLUTION und
FREE-TEXTURE selbst bleiben unverändert, damit die Kalibrierung und die Sätze ohne Imitation gleich
bleiben.

**Ohne Plan.** Die Teile einer Messe, die gemeinsam beginnen, haben keinen
Imitationsplan, halten aber Töne über den Taktstrich und pausieren in halber Besetzung wie die
imitierenden (§ 19). Für sie prüfen drei Parameter dieselben Fälle in Sätzen ohne Plan oder mit
einem Plan ohne Punkte: `unplanned_struck` und `unplanned_passing` wie IMI-HELD `struck` und
`passing`, `unplanned_thin` wie IMI-SILENCE `thin`, aber vom ersten Ton jeder Stimme an. Das
Standardprofil schaltet sie aus, so dass Kalibrierungen, Golden-Tests und alle Sätze außerhalb einer
Messe bleiben, wie sie waren; das Profil der Messe gibt ihnen Stufen. Stufen und Zahlen des Korpus
stehen in den Zeilen von IMI-HELD und IMI-SILENCE oben und in § 19
([Regeln der Messe](#regeln-der-messe)). Eine allgemeine Neueichung von FREE-PASSING,
FREE-RESOLUTION und FREE-TEXTURE gibt es nicht.

### Entscheidungszeitpunkte der Imitation

Jeder Fall wird an einem Zeitpunkt des Plans entschieden, an dem die Noten ihn zeigen, und erst
gemeldet, wenn alle betroffenen Stimmen bis dorthin geschrieben sind; markiert wird ein Ton, der
dort klingt. So finden die Prüfung Fenster für Fenster, die Prüfung Ton für Ton wachsender Stimmen
und die des ganzen Satzes dieselben Fälle (Tests `each_parameter_fires_once`,
`falsified_imitations_are_reported_at_their_tick`, `random_imitations_check_window_by_window`).

| Fall | entschieden | markiert |
|---|---|---|
| IMI-ENTRY `level` | am geplanten Anschlag des Kopftons | den Ton oder die Pause, die dort steht |
| IMI-ENTRY `rhythm` | am Ende des zu kurzen Tons, also am nächsten Ereignis der Stimme | den zu kurzen Ton |
| IMI-ENTRY `text` | am späteren der beiden Anschläge, meist dem des folgenden Einsatzes | dessen Ton |
| IMI-REST, IMI-INTERVAL, IMI-TIME, IMI-DEGREE, IMI-OVERLAP `late` | am Anschlag des Einsatzes | den ersten Ton des Einsatzes, bei IMI-REST `minim_before` den Ton vor der Pause |
| IMI-OVERLAP `bare` | am Anschlag des Einsatzes, sobald alle Stimmen dort gesetzt sind | den ersten Ton des Einsatzes |
| IMI-SILENCE `exposition`, `long` | an jedem Taktanfang, an dem die Stimme schweigt | die Pause |
| IMI-SILENCE `thin`, `unplanned_thin` | an jedem Taktanfang, sobald alle Stimmen dort gesetzt sind; ohne Plan erst, wenn jede Stimme gesungen hat | die Pause der höchsten schweigenden Stimme |
| IMI-HELD `struck`, `unplanned_struck` | wo der übergebundene Ton weitergeht: an seinem nächsten Ton oder seiner Pause, am verzierten Ziel auf dem dritten Viertel, am nächsten Taktanfang, wenn er darüber gehalten wird | den angeschlagenen Ton |
| IMI-HELD `passing`, `unplanned_passing` | wie FREE-PASSING die Figur entscheidet, nicht vor dem Weitergehen eines übergebundenen Tons, der auf dem Taktanfang dissonant war | den dissonanten Ton |
| IMI-ANSWER | am Anschlag des Tons, zu dem das geänderte Intervall führt | diesen Ton |
| IMI-SUBJECT `short_first` | am geplanten Ende des ersten Tons | den ersten Ton |
| IMI-SUBJECT `not_longest`, `drop` | am Anschlag des längeren bzw. des kurzen Tons | diesen Ton |
| IMI-SUBJECT `range`, `beyond_octave` | an dem Ton, mit dem der Umfang eine Septime erreicht bzw. die Oktave überschreitet | diesen Ton |
| IMI-SUBJECT `plain` | am sechsten Anschlag des Kopfes | den ersten Ton |
| IMI-SUBJECT `leaps` | am Ziel des zweiten Sprungs | dieses Ziel |
| IMI-SUBJECT `end` | am Anschlag des letzten Kopftons, wenn er nicht auf dem Taktanfang liegt, sonst an seinem geplanten Ende | den letzten Kopfton |

### Soggetto bauen

Für jeden Punkt entstehen vor der Suche höchstens sechs Soggetti, das beste zuerst: aus dem Text,
aus einer Choralphrase oder aus eigenen Noten (`--subject text|chant|chant:DATEI[:PHRASEN]|"Noten"`).
Keines beginnt wie das Soggetto eines früheren Punktes (dieselben ersten drei Intervalle und vier
Werte), außer in einem wiederholten Punkt. Die Soggetti eines Punktes kosten in Release etwa
15–20 ms.

**Kopfworte.** Der Kopf eines Punktes deklamiert dessen Anfang: von der ersten Silbe bis zum Ende
des ersten Wortes ab zwei Silben, das kein Funktionswort ist (§ 17), und die Silbe danach,
höchstens sechs Silben, innerhalb des Punktes und seines ersten Sinnabschnitts. Ist dieser
Sinnabschnitt ein einzelnes Wort, singt der Kopf nur dieses (San-ctus,). Über einsilbige Wörter
reicht der Kopf hinweg und gibt ihnen je einen Ton, so wie Palestrina Ad te levavi bis zum Akzent
syllabisch deklamiert (Jeppesen S. 86):

| Punkt | Kopfworte | Punkt | Kopfworte |
|---|---|---|---|
| Ký-ri-e e-lé-i-son. | Ký-ri-e e- | qui tol-lis pec-cá-ta mun-di: | qui tol-lis pec- |
| e-lé-i-son. (Schlussteil) | e-lé-i-son. | mi-se-ré-re no-bis. | mi-se-ré-re no- |
| Chri-ste e-lé-i-son. | Chri-ste e- | Be-ne-dí-ctus qui ve-nit | Be-ne-dí-ctus qui |
| A-gnus De-i, | A-gnus De- | in nó-mi-ne Dó-mi-ni. | in nó-mi-ne Dó- |
| San-ctus, San-ctus, San-ctus | San-ctus, | Ho-sán-na in ex-cél-sis. | Ho-sán-na in |
| Dó-mi-nus De-us Sá-ba-oth. | Dó-mi-nus De- | Et in ter-ra pax … | Et in ter-ra pax |
| Ad te le-vá-vi á-ni-mam me-am. | Ad te le-vá-vi á- | Tu es Pe-trus. | Tu es Pe-trus. |

**Aus dem Text** entsteht ein Soggetto in zwei Schritten, die beide so bepreisen wie die Suche:
1. **Deklamation.** Ein Strahl von 32 Teilköpfen wählt Ton für Ton, höchstens zehn Töne, einen Wert
   und was der Ton singt (eine neue Silbe oder das Melisma der vorigen). Die Werte folgen
   Palestrinas Übergängen, etwa von der Brevis in eine Ganze oder punktierte Ganze und von der
   punktierten Halben in ihr Viertel; ein Sturz auf ein Viertel des vorigen Werts kostet IMI-SUBJECT
   `drop`. Jeder Takt bleibt der Anfang eines Taktrhythmus, den das Profil zulässt, und ganze Takte
   kosten nach FREE-RHYTHM. Das Modell der Unterlegung (§ 17) bepreist die Silben mit den TXT-Stufen
   des Profils, von dort aus, wo die Stimme vor dem Punkt im Text steht; ein Verbot schneidet ab.
   Dazu kommen Stilkosten aus Palestrinas Köpfen, für einen Anteil im Korpus ab 10 % nichts, für
   5–10 % 1, für 2–5 % 2, für seltenere 4:
   - die ersten drei Werte nach dem Text, etwa im Kyrie I Ganze, Halbe, Halbe (22 %), Brevis, Ganze,
     Ganze (21 %) oder drei Ganze (15 %); ein eigener Text und spätere Punkte nach dem Versfuß
     ihres ersten Wortes;
   - ein Melisma auf der ersten Silbe: syllabisch frei, zwei weiße Töne 2, sonst 4, in einem Punkt
     aus einzelnen Wörtern (San-ctus,) 1 (zehn der zwölf textierten Köpfe Palestrinas bei Jeppesen
     sind syllabisch); mehr als ein Ton auf einem späteren einsilbigen Wort: zwei 1, mehr 4;
   - die erste schwarze Figur, eine Silbe auf dem Viertel nach einer punktierten Halben und die
     Dauer des Kopfes nach Palestrinas Anteilen, getrennt für Eröffnungen (frei bei vier bis sieben
     Ganzen) und spätere Punkte (frei bei drei bis fünf).

   Ein Kopf ist fertig, wenn er jede Silbe seiner Kopfworte begonnen hat, mindestens vier Töne hat,
   auf einem weißen Ton endet und 24 bis 64 Ticks dauert (in langen Texten 16 bis 48). Sechs
   Deklamationen gehen weiter.
2. **Tonhöhen.** Ein Strahl von 24 Melodien je Deklamation beginnt auf der Finalis oder der
   Quinte, ionisch und äolisch auch auf der Quarte, phrygisch auf E oder A, und geht in den
   Schritten 0, ±1, ±2, ±3, ±4, +5 und ±7. Die Töne sind die der Leiter, mit dem erniedrigten Mi
   (B♭, transponiert E♭) gegen einen Tritonus oder ein übermäßiges Intervall und im Dorischen und
   Lydischen auf der oberen Nebennote des La (A B♭ A, „una nota super la semper est canendum fa“;
   Palestrina singt es dorisch zu 63 % so). Der Prüfer bepreist jeden Ton an einem einstimmigen
   freien Satz (Tenor, ohne FREE-AMBITUS und VOI-RANGE, denn die Lage wählt erst der Planer), mit
   dem Kopf als Plan eines Einsatzes, so dass IMI-SUBJECT seine Gestalt beurteilt; das Modell der
   Unterlegung bepreist die Silben auf den wirklichen Tönen. Tonwiederholungen und Pausen kosten
   wie in einer singenden Stimme. Stilkosten: das erste Intervall nach dem Text (im Kyrie I
   Tonwiederholung 39 %, Terz aufwärts 18 %, Sekunde aufwärts 16 %), ein Weitergehen in der Richtung
   eines Sprungs ab der Quarte, ein steigendes Paar mit dem kleineren Intervall zuerst, die
   Bewegung durch ein Viertel nach einem weißen Ton nach ihrem Anteil in Palestrinas Köpfen und ein
   höchster Ton, der nicht einmalig ist oder schwarz. Vier Melodien je Deklamation gehen weiter.

**Gezogen.** Nach Palestrinas Anteilen werden gezogen: die ersten drei Werte in der Eröffnung von
Kyrie, Christe, Kyrie II, Agnus Dei, Sanctus, Benedictus und Osanna (aus dem Choral die ersten vier
Werte in jedem Punkt; die übrigen Punkte und eigene Texte bepreist der Versfuß ihres ersten Wortes,
siehe Deklamation) und für jeden Punkt die Anfangsstufe (dorisch Finalis 45, Quinte 31; mixolydisch
Quinte 49, Finalis 37; ionisch Finalis 38, Quinte 35, Quarte 15; äolisch Finalis 38, Quinte 28,
Quarte 20; phrygisch A 33, E 31; lydisch Finalis und Quinte gleich oft), die Art des ersten
Intervalls, ein Sprung ab der Quarte bis zum vierten Intervall (im ersten Punkt .42, später .25),
ein Umfang von Septime oder Oktave (.22 bzw. .08) und ein Ende auf einer punktierten Halben, deren
Viertel der freie Kontrapunkt singt (.15). Ein Kopf, der das Gezogene nicht nimmt, kostet 2; sonst
gewönnen nie die langen ersten Töne, die Sprünge und die weiten Köpfe, die die Regeln bepreisen, die
Palestrina aber in einem Zehntel bis der Hälfte seiner Köpfe schreibt. Die Ziehungen hängen vom
Seed, vom Punkt und von der Anfrage ab (Text, Modus, Stimmenzahl). Mit dem Seed allein zögen alle
Modi dieselben Kopfformen, und die Stilprobe unten begann zu 45 % mit einer Tonwiederholung
(Palestrina 29 %).

**Antworten und Engführung.** Jeder Kopf wird auf derselben Stufe und auf der anderen Hauptstufe
beantwortet, real, wo das singbar ist, und, wo er zwischen Finalis und Quinte springt, auch modal.
Eine Antwort, die den Kopf über eine Oktave weitet, entfällt. Die **Probe der Engführung** (Fux S.
247 rät, ein Soggetto vor einer engen Führung zu erproben) prüft den Führer gegen einen folgenden
Einsatz, alle anderen Stimmen schweigend, mit den Paarprüfungen unter [Imitation
erzeugen](#imitation-erzeugen): für jede Antwort in jeder Oktavlage, nach einem, zwei, drei und vier
Takten mit dem ersten Wert des Führers und nach ½, 1½, 2½ und 3½ Takten mit verkürztem oder
übergebundenem erstem Ton, jeweils vor dem Ende des Kopfes. Ein Soggetto aus Text oder Choral, dem
keine Antwort innerhalb von zwei Breven folgen kann, entfällt. Was ein Soggetto kostet, richtet sich
nach seiner besten Engführung: auf der anderen Hauptstufe nach einem oder zwei Takten frei, nach
drei oder vier Takten 1, nur auf derselben Stufe oder nur auf der Taktmitte 2. Palestrinas Köpfe
bestehen diese Probe zu 97,4 % (1144 Eröffnungen) und 98,7 % (3493 spätere Punkte). Ein Soggetto aus
Text oder Choral kostet außerdem 2 für jede Stimme, deren Umfang den Kopf in keiner Oktave fasst,
und 4 mehr, wenn er dabei über eine Stufe aus ihrem Liniensystem ragt; so gewinnen im Mixolydischen
und Äolischen engere Köpfe.

**Aus dem Choral** nimmt ein Kopf die Töne einer Choralphrase als Tonhöhen; welche, sagt
[Choral](#choral). Tonwiederholungen verschmelzen, außer der Text verlangt dort eine neue Silbe;
höchstens eine Terz wird mit einem Durchgang gefüllt (ein Viertel auf der unbetonten Hälfte einer
Halben frei, sonst 1) oder eine Quarte mit zweien (2); kein Ton des Chorals fällt aus. Die
Deklamation läuft wie oben über diese Töne, mit den Werten von Palestrinas Choralköpfen (die ersten
vier Werte vier Ganze 11 %, drei Ganze und eine Halbe 7 %, Brevis und drei Ganze 7 %, Ganze und
drei Halbe 7 %), ohne die Stilkosten für Melismen; die Neumen zählen nicht. Ein Choralkopf singt
fünf bis zehn Töne des Chorals (Tonwiederholungen verschmolzen), am liebsten sieben oder acht (sechs
oder neun kosten 1, fünf oder zehn 2), endet am liebsten auf Finalis oder Quinte
und dauert 40 bis 72 Ticks; die Tonhöhen bepreist der Prüfer einmal. Gibt der Choral für einen
Punkt keinen brauchbaren Kopf, nimmt der Punkt ein Soggetto aus dem Text, mit einer Warnung wie
„Punkt 2: kein brauchbarer Kopf aus dem Choral; Soggetto aus dem Text“.

**Eigene Noten** (`--subject "d'1 d'2 a'2 g'4 f'4 e'2 d'1"`, LilyPond absolut, oder Notennamen als
Ganze wie `"D4 F4 E4 D4"`) geben die Köpfe vor,
durch `|` getrennt einen je Punkt; die übrigen Punkte nehmen ihr Soggetto aus dem Text. Der erste
Ton legt die Stufe des Führers fest, die Lage wählt der Planer je Einsatz. Ein Kopf braucht vier
bis zwölf Töne von mindestens einem Viertel, ohne Pausen, im Umfang höchstens einer Oktave, ohne
melodisches Verbot in jeder geplanten Antwort (auf der anderen Hauptstufe muss er sich beantworten
lassen), Takte, die Anfänge erlaubter Taktrhythmen sind, und eine Unterlegung seiner Kopfworte ohne
Verbot: die billigste, die mit der ersten Silbe des Punktes beginnt, notfalls mit weniger Worten bis
zum ersten. Sonst scheitert die Anfrage und nennt Soggetto, Ton und Regel; einen Ersatz gibt es
nicht. Vorzeichen bleiben: Der Führer und Einsätze im Einklang oder in der Oktave singen genau die
gegebenen Töne. Das erniedrigte Mi des Modus gehört zu dessen Leiter, eine Antwort auf der anderen
Hauptstufe singt dort den Ton ihrer Leiter (dorisch A B♭ A G F E D wird D E D C B A G). Andere
erhöhte oder erniedrigte Töne werden um eine Quarte oder Quinte versetzt und bleiben nur, wo der
Modus sie kennt: als Leiterton, als erniedrigtes Mi oder als Leitton, der einen Halbton steigt;
sonst entfällt die Antwort, und ohne Antwort scheitert die Anfrage (FIC-SCOPE). Ein gegebener Kopf
muss die Probe der Engführung nicht bestehen; ohne Engführung kostet er 3.

### Einsätze planen

**Punkte.** Der Text wird an jedem Beginn eines Sinnabschnitts und eines Schlussteils geteilt, nie
an einer Zäsur (`|`). Aufeinanderfolgende einzelne Wörter bilden einen Teil (San-ctus, San-ctus,
San-ctus), und ein Teil unter vier Silben (in langen Texten sechs) geht im nächsten auf, der
letzte im vorigen. Jeder Teil ist ein Punkt mit eigenem Soggetto (Jeppesen S. 241: Jeder
Abschnitt des Textes hat sein eigenes Thema). Ein kurzer Text, den der Plan des Textes zweimal
singt (§ 17), bekommt einen zweiten Punkt: seinen Schlussteil mit neuem Soggetto, oder mit p .25
den ersten Punkt noch einmal, in engerer Engführung (Abstände von einem oder zwei Takten).
`--restate an|aus` legt das fest, `--points N` verschmilzt die kürzesten benachbarten Teile oder
fügt bei kurzen Texten Wiederholungen hinzu.

| Text | Punkte (Standard) |
|---|---|
| `kyrie`, `kyrie2`, `christe` | 1. der ganze Sinnabschnitt · 2. der Schlussteil „e-lé-i-son.“ oder Punkt 1 wiederholt |
| `osanna`, `osanna2` | 1. Ho-sán-na in ex-cél-sis. · 2. der Schlussteil „in ex-cél-sis.“ oder Punkt 1 wiederholt |
| `sanctus` | 1. San-ctus, San-ctus, San-ctus · 2. Dó-mi-nus De-us Sá-ba-oth. |
| `pleni` | 1. Ple-ni sunt cae-li et ter-ra · 2. gló-ri-a tu-a. |
| `benedictus` | 1. Be-ne-dí-ctus qui ve-nit · 2. in nó-mi-ne Dó-mi-ni. |
| `agnus`, `agnus3` | 1. A-gnus De-i, · 2. qui tol-lis pec-cá-ta mun-di: · 3. mi-se-ré-re no-bis. (do-na no-bis pa-cem.) |
| `gloria:1-5` | 1. Et in ter-ra pax … vo-lun-tá-tis. · 2. Lau-dá-mus te. Be-ne-dí-ci-mus te. · 3. Ad-o-rá-mus te. Glo-ri-fi-cá-mus te. |
| eigener Text „Ad te le-vá-vi á-ni-mam me-am.“ | ein Punkt |

**Einsätze.** Was die Anfrage offen lässt, zieht der Planer nach Palestrinas Anteilen
(`--order`, `--intervals`, `--delays`, `--point-voices`, `--pairs`, `--answer` und `--head`
legen es fest):

| Wahl | Standard | Palestrina |
|---|---|---|
| Stimmen | Punkt 1: alle. Spätere Punkte: zwei bis alle, nach den Größen von Palestrinas späteren Punkten; dreistimmig 2 (.67) oder 3, vierstimmig 2 (.605), 3 (.25) oder 4 (.145), fünf- und sechsstimmig meist 2 (.54). Es singen die Stimmen, deren letzter Kopf am frühesten endete (Vicentino IV.29: die zuerst endende Stimme beginnt zuerst wieder), eine Formelstimme des Binnenschlusses davor mit dem Ende seines Zieltakts; bei Gleichstand nach den Anteilen der Führer späterer Punkte. | spätere Punkte vierstimmiger Abschnitte zu 61 % zweistimmig, zu 14,5 % vierstimmig |
| Führer von Punkt 1 | nach Stimmlage: Alt .37, Sopran .26, Tenor .23, Bass .14, auf Stimmen gleicher Lage verteilt | vierstimmige Eröffnungen (alle: A 33,5, S 26,5, T 26,5, B 13 %) |
| Führer späterer Punkte | unter diesen Stimmen nach Stimmlage: Bass und Tenor je .29, Sopran .21, Alt .19; die übrigen in zufälliger Folge | Nachzählung (die erste Zählung hatte den Tenor vor dem Bass) |
| Reihenfolge, vierstimmig | ASTB, SATB, ASBT, TASB, BTAS, ATSB, TBSA wie 28 : 27 : 21 : 15 : 14 : 12 : 10, jede fünfte Planung eine andere. Andere Besetzungen: der Führer nach Stimmlage, die übrigen in zufälliger Folge. | 254 vollständige vierstimmige Eröffnungen; welche Stimme auf welche folgt, ist darüber hinaus zufällig |
| zweiter Einsatz | zwischen verschiedenen Stimmlagen auf der anderen Hauptstufe (.81), zwischen gleichen auf derselben Stufe (.74) | |
| weitere Einsätze | haben die ersten beiden dieselbe Stufe, zuerst mit .5 die andere Hauptstufe; sonst ein Tenor auf der Stufe des Soprans (.79), ein Bass auf der des Alts (.63), wenn diese schon eingesetzt haben (Fux S. 168 f.), andere Stimmen auf der Stufe des Führers (.59) oder des zweiten Einsatzes. Ein Kopf, der nicht auf Finalis, Quinte oder Quarte beginnt (aus dem Choral oder eigenen Noten), antwortet auf der anderen Hauptstufe in der Oberquinte oder der Unterquarte | Tenor auf der Stufe des Soprans 79 %, Bass auf der des Alts 63 %; nach einem zweiten Einsatz auf der Stufe des Führers bleiben beide späteren Einsätze vollständiger vierstimmiger Eröffnungen nur in 14 % dort, alle drei fünfstimmiger in 21 % |
| Abstand des zweiten Einsatzes | ein, zwei, drei oder vier Takte im Verhältnis dreistimmig .27 : .15 : .20 : .33, vierstimmig .38 : .18 : .21 : .16, fünfstimmig .41 : .24 : .08 : .06, sechsstimmig .44 : .18 : .08 : .05, zusammen .95 (fünfstimmig also .49, .29, .10, .07); dazu um eine Halbe verschoben zusammen .05. Immer vor dem Ende des Kopfes des Führers. | 735 von 737 Eröffnungen nach höchstens zwei Breven |
| spätere Abstände | 1–8 Takte: .18, .125, .125, .144, .086, .089, .043, .041; dazu 1½ bis 5½ Takte zusammen .12; in einem wiederholten Punkt ein oder zwei Takte | Abstände in Eröffnungen, Median 3½ Ganze (Nachzählung) |
| erster Ton auf der Taktmitte | um eine Halbe verkürzt (.63) oder über den Taktstrich gebunden | zweite Einsätze auf der Taktmitte beginnen in 22 von 35 Fällen mit einer Halben |
| Antwort | real; modal mit p .25, wo die ersten Intervalle zwischen Finalis und Quinte springen, und wo die reale Antwort in keiner Lage singbar ist | modal in 32 % solcher Köpfe |
| Paare | ab vier Stimmen in jeder zehnten Planung: der dritte und vierte Einsatz wiederholen den ersten und zweiten eine Oktave tiefer oder höher (SA TB und AS BT tiefer, BT AS und TB SA höher), nach einem Abstand, der mindestens einen Takt länger ist | 22 % der vollständigen vierstimmigen Eröffnungen (mit dem Erkenner des Programms 20,4 %); mit den Duos, die Stufen und Abstände von selbst ergeben, misst der Stiltest unten 18,8 %, bei jeder fünften Planung waren es 34 % |
| Lage | die Oktave, in der der Kopf in der Oktave seiner Stimme im Modus (FREE-AMBITUS, eine Stufe Spielraum) und in ihrem Liniensystem liegt; reicht keine, im Liniensystem, dann höchstens eine Stufe außerhalb, dann so weit die Töne der Stimme reichen. Unter gleich guten die mit den geringsten Kosten des Kopfes allein und den wenigsten Kreuzungen mit anderen Köpfen, der zweite Einsatz einer Eröffnung bevorzugt in einer Lage, die die Probe der Engführung besteht. Die Neuplanung wählt nur Lagen der ersten Stufe. | |
| Kopf | alle Töne des Soggetto, gekürzt vor einem Ton, an dem eine Paarprüfung scheitert (fuga sciolta, Vicentino IV.32), oder wie `--head` es verlangt; nie unter vier Töne, mit modaler Antwort nicht unter fünf | im Median 6 imitierte Töne |
| Pausen | Jede Stimme schweigt bis zu ihrem ersten Einsatz; vor jedem weiteren mindestens eine Halbe | |

Der Planer plant nie: einen zweiten Einsatz nach mehr als zwei Breven (außer mit `--delays`),
Einsätze außerhalb der halben Schläge, andere Intervalle als Einklang, Oktave, Quarte und Quinte
(außer mit `--intervals`), einen ersten Punkt ohne eine der Stimmen, Köpfe über eine Oktave und
gleichzeitige Einsätze.

### Choral

**Daten.** fux-core bettet sechs Gesänge des Kyriale ein (`include_str!`, auch in WebAssembly),
über die Palestrina Messen schrieb: Kyrie IX *Cum jubilo* (1. Modus, GregoBase 2976; *Missa de
beata Virgine*), Gloria IX (7. Modus, 2771), Sanctus IX mit Benedictus (5. Modus, 587), Agnus Dei
IX (5. Modus, 707), Credo I (4. Modus, 344) und Kyrie IV *Cunctipotens genitor Deus* (1. Modus,
1061; *Missa in minoribus duplicibus*). Vorlage sind Graduale Romanum und Liber Usualis (Solesmes
1961), die Übertragungen stammen von Andrew Hinkley; GregoBase stellt sie unter CC0 („All chant
transcriptions are released under CC0“). `crates/fux-core/src/imitation/kyriale/README.md` nennt
Seiten und Herkunft, `tools/fetch_chant.py` lädt die Dateien neu und überspringt Gesänge, die
GregoBase als geschützt markiert. Ihre Texte sind die des Ordinariums (`ordinary.txt`), darum finden
die Punkte ihre Töne über die Worte. `--subject chant` nimmt den Gesang des Abschnitts (Messe IX,
für das Credo Credo I), `chant:kyrie-iv` einen mit Namen, `chant:DATEI.gabc` einen beliebigen,
etwa *Iste confessor* aus `tests/fixtures/chant/` zu einem eigenen Text.

**Lesen.** Der GABC-Leser lässt die Töne des *euouae* (`<eu>`) und alles nach „T. P.“ weg.
`Chant::phrases()` teilt an den Gliederungsstrichen ab dem Viertelstrich (`,`); ein Kyrie nur am
Doppelstrich (`::`), so dass jede Anrufung eine Phrase ist.

**Welche Töne.** Ein Punkt nimmt die Töne ab der ersten Stelle nach dem vorigen Punkt, an der der
Gesang seine Kopfworte singt (sonst sein erstes Wort, sonst seine erste Silbe), verglichen ohne
Akzente und h, mit j und y wie i und oe wie ae. So nimmt das Kyrie I die erste Anrufung, das
Christe die erste Christe-Anrufung, sein Schlussteil deren eleison, das Kyrie II die letzte
Anrufung (Wagner S. 446). Ein eigener Text oder Worte, die der Gesang nicht singt, nehmen die
Phrasen der Reihe nach; `chant:DATEI:2,3` nennt sie (ab 1, auch Bereiche wie `2-4`; `chant::2,3`
nimmt Phrasen des Chorals aus dem Kyriale). Die Töne laufen bis zum Ende ihrer Phrase und in die
nächste hinein, solange sie weniger als sechs Töne haben (Tonwiederholungen verschmolzen; ein
Fünftel von Palestrinas Choralköpfen läuft über den Strich), höchstens bis zum zehnten.

**Modus.** Die Töne zählen als Stufen über der Finalis des Modus, den die Nummer des Gesangs nennt
(`Chant::numbered_mode`): Kyrie IV endet auf A, steht aber im 1. Modus. Ohne `--mode` setzt der
Satz im Modus des Gesangs; mit `--mode` singt er dessen Stufen von der eigenen Finalis aus, mit
einer Warnung, wenn die Finalis wechselt („Der Choral hat die Finalis D, der Satz E; seine Stufen
werden von E aus gesungen.“). Die Vorzeichen folgen dem mehrstimmigen Modus: Aus D F G A G F E D
im Kyrie IX wird im transponierten Dorisch G B♭ C D C B♭ A G wie in Palestrinas *Missa de beata
Virgine*. Palestrina beantwortet Choralköpfe im Einklang oder in der Oktave zu 41 %, in der Quinte
zu 36 % und in der Quarte zu 20 %; der Planer behandelt sie wie jedes Soggetto. Ein Kopf, der nicht
auf Finalis, Quinte oder Quarte beginnt (der Schlussteil e-lé-i-son des Kyrie IX auf C, das Pleni
auf A), hat keine andere Hauptstufe: Wo der Planer sie zieht, antwortet er in der Oberquinte oder
der Unterquarte. Vorher sangen alle 273 solchen Punkte einer Stichprobe (zehn Abschnitte, drei- bis
fünfstimmig, Seeds 1–10) jeden Einsatz auf einer Tonstufe, jetzt 47 von 278 (16,9 %), mit drei
und mehr Einsätzen 10 von 156 (6,4 %).

### Imitation erzeugen

**Optionen** von `fux free` (GUI: Gruppe „Imitation“, Web-Version: Feld `imitation` des Befehls
`free`): `--imitation auto|an|aus`, `--subject` (`text`, `chant` oder `choral`, `chant:DATEI` mit
Phrasen wie `chant:DATEI:2,3` oder `2-4`, `chant::2,3` für Phrasen des Chorals aus dem Kyriale,
oder eigene Noten), `--points auto|N`, `--restate auto|an|aus`, `--point-voices auto|alle|N`,
`--order "A S T B; T B"` (je Punkt die Stimmen in der Folge ihrer Einsätze, `;` zwischen den
Punkten; gleiche Stimmlagen von oben als `T1`, `T2` oder mit ihrer Nummer von oben; in Punkt 1
folgen nicht genannte Stimmen in gezogener Folge), `--intervals "d 0 d; 0"` (zum Führer: `0`
dieselbe Stufe, `d` die andere Hauptstufe, `+n`/`-n` Stufen, auch am Anfang wie `"-3 d"`),
`--delays "1 2 1.5; 1"` (Takte, Halbe mit Punkt, durch Leerzeichen getrennt; ein Komma ist
mehrdeutig und ein Fehler), `--pairs auto|an|aus`, `--answer auto|real|modal`, `--head auto|N`
(vier bis zehn Töne, mit eigenen Noten bis zwölf), `--plan` (nur der Plan, ohne Suche) und
`--marks an|aus` (Klammern über den Einsätzen). Angaben, die Chor oder Text nicht erlauben,
scheitern sofort mit einer Meldung und einer Abhilfe, in der Kommandozeile mit den Optionen, in
GUI und Web-Version mit den Feldern: ein Kopf unter vier Tönen oder über zehn (eigene Noten zwölf),
spätere Punkte mit weniger als zwei oder mehr Stimmen, als der Satz hat, Angaben zu einem Punkt,
den der Text nicht hat, eine unbekannte oder doppelte Stimme in einer Reihenfolge, Abstände von
null oder außerhalb der halben Schläge, mehr Abstände oder Intervalle als Einsätze, eine
Wiederholung eines Textes, der nur einmal gesungen wird, Intervalle, in denen kein Soggetto des
Punktes antworten kann, eine Zahl von Punkten, für die der Text keinen Raum hat (`--points`), und
mehr eigene Köpfe oder Choralphrasen, als Punkte ein eigenes Soggetto bringen: Eine Wiederholung
von Punkt 1 nimmt dessen Soggetto; hat der Planer sie selbst gezogen, weicht sie dem Schlussteil,
der den nächsten Kopf singt. Ist die Imitation aus oder lässt `auto` sie weg, zählen ihre Felder in
GUI und Web-Version nicht.

**Zeitleiste.** Punkt 1 beginnt am Anfang, jeder Einsatz folgt dem vorigen um seinen Abstand. Das
Ziel eines Binnenschlusses ist der erste Takt,
- zwei Takte nach den Köpfen seiner Formelstimmen und einen nach denen der übrigen Stimmen,
- drei Takte nach dem vorigen Ziel und nach dem Einsatz des Führers,
- vor dem kein Kopf eine Sekunde oder Septime gegen den Zielton singt, den die Diskantklausel als
  Vorhalt in den vorletzten Takt hält,
- und vor dem jede Formelstimme den Rest der Worte des Punktes auf vier Fünfteln ihrer halben
  Schläge beginnen kann.

Das Ziel liegt auf der Stufe, die der Plan des Textes für das Ende dieser Worte vorsieht (§ 17). Der
Führer des nächsten Punktes setzt auf der Taktmitte nach dem Ziel ein; ist er eine Formelstimme
dieses Schlusses (oder mit p .15 auch sonst), einen Takt später. Der Schluss liegt zwei Takte nach
dem letzten Kopf und drei nach dem letzten Ziel, wo jede Stimme den Rest des Textes auf vier
Fünfteln ihrer halben Schläge beginnen kann. Das Tempo, nach dem die Suche lenkt, verteilt die
Silben aller Punkte auf die Takte außer den letzten zwei.
**Statische Prüfungen.** Jeder gezogene Plan durchläuft vor jeder Suche diese Prüfungen; was sie
verbieten, ist sicher, gleich was der freie Kontrapunkt tut, und was sie bepreisen, zählt in seinen
Kosten:
- **Rhythmus:** Die Takte, die das Ruhefenster und der Kopf berühren, lassen sich noch mit einem
  Taktrhythmus füllen, den das Profil erlaubt (FREE-RHYTHM).
- **Melodie:** jeder Kopf allein in seiner Stimme und Lage geprüft, ohne Verbot.
- **Text:** die Silben jedes Kopfes, von dort aus, wo die Stimme im Text steht, mit seinem ersten
  Ton, wie der Einsatz ihn singt; ein verkürzter erster Ton kann TXT-ACCENT `longer` kosten.
- **Formeln und Enden:** kein Kopfton, wo seine Stimme eine Kadenzformel singt oder den Schlusstakt
  hält, und keiner auf der Stufe des Halbtons einer Klausel, solange dieser klingt.
- **Paare** (a–h, sicher zwischen zwei festen Linien): a ein Ton gegen seine Alteration; b eine
  zugleich angeschlagene Sekunde, Septime, ein Tritonus oder alteriertes Intervall, ab drei Stimmen
  nicht gegen ein Durchgangsviertel; c eine zugleich auf dem Taktanfang angeschlagene Quarte, deren
  unterer Ton der Bass ist (keine Stimme kann darunter klingen); d eine Sekunde oder Septime auf dem
  Taktanfang gegen einen gehaltenen Ton, eine Quarte darunter im Bass oder, im Chor, eine Quarte
  über einem gehaltenen Basston (IMI-HELD), wenn die Linie des gehaltenen Tons nicht stufenweise
  abwärts weitergeht; e ein Einsatz außerhalb des Taktanfangs, dissonant zu einem klingenden Ton,
  und jeder Ton, der unvorbereitet (nach einer Pause oder einem Sprung) außerhalb des Taktanfangs
  eine Dissonanz bildet; f ein Einsatz, dissonant zu einem Formelton in jeder Oktave, die die Suche
  ihm geben kann, oder eine Quarte über einem Formelton der tiefsten Stimme; g ein Einsatz eine
  Quarte über einem gehaltenen Basston;
  h Einklang-, Quint- oder Oktavparallelen zwischen zwei Linien, die beide von festem zu festem Ton
  gehen. Palestrinas Eröffnungen bestehen sie zu 99,84 % (617 von 618).
- **Skelett:** der Prüfer auf einer Partitur aus allen Köpfen mit Pausen dazwischen und dem Plan.
  Ein Verbot, das an einem Zeitpunkt entschieden ist, an dem jede Stimme fest ist oder planmäßig
  schweigt, ist sicher, ebenso jeder Fall der IMI-Regeln, den der Plan allein entscheidet, und eine
  dissonante Figur oder ein dissonanter letzter Kopfton, den keine Fortsetzung mehr auflöst.
- **Textkapazität:** Jede Stimme hat zwischen dem Ende eines Kopfes und ihrem nächsten Einsatz die
  halben Schläge für die Silben dazwischen; jede Silbe über vier Fünftel davon kostet 1.
- **Satzdichte:** Ein Taktanfang nach den Einsätzen, an dem weniger als die Hälfte der Stimmen
  klingen kann, kostet wie FREE-TEXTURE, einer, an dem vier- oder sechsstimmig höchstens die
  Hälfte klingen kann, wie IMI-SILENCE `thin`.
- **Stufen der Einsätze:** Ein Einsatz auf der Tonhöhe zweier früherer Einsätze des Punktes kostet
  1 (drei und mehr Einsätze auf einer Tonhöhe in 11–12 % der vollständigen fünfstimmigen
  Eröffnungen Palestrinas, häufig), ein Punkt, dessen Einsätze alle auf einer Stufe beginnen, mit
  drei Einsätzen 1 (12,1 % der dreistimmigen Eröffnungen), mit vier und mehr 2 (2,7 % der
  vollständigen vierstimmigen, 3,5 % der fünfstimmigen, gelegentlich).

**Auswahl.** Die Gestalt eines Punktes wird einmal gezogen (Abstand des zweiten Einsatzes,
spätere Abstände, Paare, ein später Führer, Stimmenzahl), und 64 Pläne variieren darin Soggetto,
Reihenfolge, Stufen und erste Töne; 32 weitere werden frei gezogen. Zöge man jeden Plan frei und
nähme den billigsten, gewönnen lange Abstände, die seltener anstoßen: Spätere Abstände von einem
Takt kamen dann halb so oft vor wie bei Palestrina. Die Kosten eines Plans sind die seines
Soggetto und die der Prüfungen, gereiht mit Rauschen. Jeder Punkt behält vier gültige Pläne für
die Neuplanung, im Gebrauch den billigsten in der Gestalt. Ist keiner gültig, zieht der Planer noch
einmal mit Köpfen von höchstens fünf Tönen, jedem Abstand einen Takt länger und späteren Punkten
mit einer Stimme weniger; findet auch das nichts, zieht er bis zu zweimal neu, mit anderer
Gestalt (manche Punkte haben unter den Plänen eines Seeds nur einen oder zwei gültige). Findet er
dann keinen, scheitert die Anfrage mit einer Meldung, die Punkt, Worte und Regeln nennt. Eine
Wiederholung von Punkt 1, die der Planer selbst gewählt hat, weicht dann dem Schlussteil.

**Gegebene Länge** (`--length N` mit `--imitation an`; `auto` imitiert nur mit automatischer
Länge): Ist der Plan länger, wird er gestaucht (Abstände höchstens ein Takt, Köpfe von vier Tönen,
spätere Punkte mit zwei Stimmen), ist er kürzer, gestreckt. Jeder Plan, auch jeder neu geplante,
hat die gegebene Länge. Passt keiner, scheitert die Anfrage und nennt die Länge des kürzesten
gefundenen Plans („Kein Plan der Imitation passt in die gegebene Länge; der kürzeste hat 39
Takte.“).

**Probe, Suche, Neuplanung.**
- **Probe:** die Suche mit Text über den Plan mit einem Strahl von 20 und einem Ergebnis, etwa ein
  Zehntel der vollen Suche. Mit Text urteilt sie unter demselben Profil wie die Suche: Von 48 ersten
  Plänen verwarf eine Probe ohne Text 9, die die Suche findet, und ließ 3 durch, die sie nicht
  findet; mit Text 2 und 4.
- **Neuplanung:** Scheitert eine Probe, gehört der Takt einem Punkt: dem mit dem nächsten geplanten
  Einsatz bis zwei Takte danach, sonst dem, der den Takt überspannt, sonst dem letzten. Dieser Punkt
  nimmt seinen nächsten Plan, zuerst den Einsatz nächst dem Fehler in einer anderen Lage, dann
  andere Reihenfolgen, Abstände und Soggetti; die Punkte danach werden neu gelegt. Sind seine Pläne
  aufgebraucht, zieht er höchstens zweimal neu. Nach sechs Proben (in WebAssembly zwei) wird der
  Plan gesucht, dessen Probe am weitesten kam.
- **Suche:** zuerst mit dem Strahl der Anfrage. Scheitert sie, wird der Punkt neu geplant und
  höchstens zweimal geprobt, bevor der Strahl wächst, bis zu zweimal; dann der doppelte Strahl, bei
  automatischer Länge ein Plan mit einem Viertel mehr Takten, zuletzt der vierfache Strahl (nicht
  in WebAssembly). Hat eine Probe schon einen Satz gefunden, wächst der Strahl nicht mehr, denn ein
  Plan, an dem der Strahl der Anfrage scheiterte, gelang mit breiterem kaum; der längere Plan kommt
  dann nur, wenn der Text im letzten Takt keinen Platz fand (TXT-COMPLETE). Scheitert jede Suche,
  ist der billigste Satz einer Probe das Ergebnis.
- **In der Suche** schweigt eine Stimme bis zu ihrem ersten Einsatz und im Ruhefenster vor jedem
  weiteren; ihre einzige Wahl ist dort die Pause bis zum nächsten Taktstrich oder zum Einsatz. Vor
  einem Einsatz darf sie auch zweistimmig pausieren. Die Töne eines Kopfes werden zu Beginn eines
  Zeitpunkts gesetzt, bevor die tieferen Stimmen wählen; ihre genauen Tonhöhen kommen an ihrem
  Anschlag zum Tonvorrat der Stimme. Ein Ton, der in einen kommenden Einsatz hinein dissonant
  klänge, rangiert schlechter, ohne verboten zu sein: Die meisten Einsätze Palestrinas treffen auf
  einen gehaltenen Ton, fast alle konsonant. Die Silben der Köpfe, die Meilensteine, die Tore und
  die Wiederholungen vor einem späteren Punkt beschreibt § 17. Kann eine Stimme einen Kopfton oder
  ihr Ruhefenster nicht nehmen, nennt die Diagnose IMI-ENTRY bzw. IMI-REST.

Ohne Imitation laufen alle diese Wege wie im Satz mit Text (§ 17): Jede Änderung der Suche greift
nur bei Köpfen, Ruhefenstern, festgelegten Silben oder den Punkten eines Plans.

**Garantie.** Jedes Ergebnis einer imitierenden Anfrage
- verstößt gegen kein Verbot des Profils; die Prüfung des ganzen Satzes umfasst die TXT- und
  IMI-Regeln;
- singt in jeder Stimme den ganzen Text wie in § 17, von der ersten Silbe am ersten Einsatz an;
- bringt jeden geplanten Einsatz genau wie geplant: zu seiner Zeit, nach mindestens seiner Pause
  (vor dem ersten Einsatz Schweigen vom Anfang an), mit den geplanten Tonhöhen und Werten und den
  Silben des Führers auf denselben Kopftönen; das bestätigen IMI-ENTRY und im Debug-Build eine
  Zusicherung;
- hält die Grenzen des Plans: alle Stimmen im ersten Punkt, der zweite Einsatz nach höchstens zwei
  Breven (außer mit `--delays`), Einsätze auf halben Schlägen nach mindestens einer Halben Pause,
  Köpfe von mindestens vier Tönen und höchstens einer Oktave, keine Silbe eines späteren Punktes
  vor seinem Tor.

Findet keine Suche und keine Probe einen Satz, scheitert die Anfrage mit dem Punkt, an dem die
letzte Suche scheiterte, seinen Worten, dem Takt und den Regeln, die im Weg standen, und nennt
Abhilfen (ein anderer Seed, andere Abstände, weniger Stimmen späterer Punkte, was die Anfrage an
Kopf, Reihenfolge, Intervallen und Länge vorgibt, wieder automatisch, oder `--imitation aus`); mit
gegebener Länge, die kein Plan hält, nennt sie die kürzeste Länge. Sie liefert nie einen Satz ohne
Imitation.

**Gemessen** wurde in Release auf 8 Kernen, ein Test nach dem anderen.

*Erfolg:* Die Tests in `crates/fux-core/tests/imitation.rs` finden jeden Satz, und jeder
besteht alle Orakel: Der Text ist vollständig und billigst unterlegt, jeder Einsatz steht wie
geplant, der Erkenner findet jeden Punkt, und jeder fünfte Satz wird auch Fenster für Fenster und
Präfix für Präfix geprüft. Die Tests setzen das Kyrie I in 12 Modusvarianten drei- bis fünfstimmig
(36 von 36); Christe, Agnus Dei I und II (Text `agnus`, im Folgenden so genannt), Sanctus,
Benedictus, `gloria:1-5` und ein eigener Text in vier Modi vierstimmig, Agnus Dei I und II und
Benedictus fünfstimmig, Osanna und Pleni dreistimmig (30 von 30); Soggetti aus dem Choral (8 von 8)
und eigene Noten (2 von 2). Die Erfolgsquote (`success_rate`, Kyrie I und Agnus Dei I und II,
dorisch, phrygisch, mixolydisch, äolisch, Seeds 1–10, Strahl 200) verlangt mindestens 97 % drei- und
vierstimmig und 90 % fünfstimmig und kein Verbot:

| Text | Stimmen | gefunden | Takte min / Median / max | Sekunden Mittel / Median / max |
|---|---|---|---|---|
| Kyrie I | 3 | 40 von 40 | 19 / 24 / 35 | 3,1 / 3,0 / 5,8 |
| Kyrie I | 4 | 40 von 40 | 22 / 29 / 44 | 6,7 / 6,3 / 14,1 |
| Kyrie I | 5 | 40 von 40 | 24 / 32 / 54 | 17,1 / 15,8 / 40,7 |
| Agnus Dei I und II | 3 | 40 von 40 | 31 / 38,5 / 49 | 7,0 / 6,2 / 22,7 |
| Agnus Dei I und II | 4 | 40 von 40 | 31 / 39,5 / 50 | 13,2 / 11,7 / 25,4 |
| Agnus Dei I und II | 5 | 40 von 40 | 35 / 45,5 / 62 | 41,3 / 34,0 / 177,3 |

Kein Ergebnis verstößt gegen ein Verbot.

*Länge:* Die ersten Pläne (dorisch, Seed 1) sind 1,0- bis 2,5-mal so lang wie derselbe Text ohne
Imitation, im Median 1,7-mal. Je Satz die Takte und die Binnenschlüsse (Takt und Ziel):

| Text | dreistimmig | vierstimmig | fünfstimmig | ohne Imitation |
|---|---|---|---|---|
| Kyrie I | 23: 14 A | 38: 19 A | 33: 21 A | 16 |
| Christe | 24: 14 A | 35: 19 A | 24: 16 A | 14 |
| Agnus Dei I und II | 37: 15 F, 25 G | 39: 18 F, 29 G | 45: 22 F, 34 G | 20 |
| Sanctus | 25: 11 F | 26: 18 F | 36: 22 F | 25 |
| Benedictus | 34: 21 F | 36: 24 F | 41: 25 F | 25 |
| `gloria:1-5` | 39: 16 A, 26 F | 44: 21 A, 32 F | 65: 29 A, 48 F | 32 |

Neuplanung kann die Länge eines Satzes ändern; die Längen der gefundenen Sätze zeigt die
Erfolgsquote.

*Laufzeit* (Test `imitation_timings`, dorisch, Mittel der Seeds 1 und 2, in Klammern die Takte
beider Sätze; dahinter derselbe Text ohne Imitation):

| Text | dreistimmig | vierstimmig | fünfstimmig |
|---|---|---|---|
| Kyrie I | 3,8 s (23/33) · 2,2 s | 10,7 s (42/30) · 3,2 s | 13,4 s (33/26) · 30,5 s |
| Christe | 2,9 s (24/23) · 1,9 s | 9,7 s (35/34) · 2,6 s | 13,6 s (24/32) · 6,0 s |
| Agnus Dei III | 7,6 s (39/44) · 3,4 s | 9,6 s (36/35) · 4,3 s | 92,8 s (43/50) · 8,8 s |
| `gloria:1-5` | 7,6 s (39/40) · 6,5 s | 18,7 s (44/46) · 9,2 s | 54,4 s (67/56) · 18,8 s |

Zusammen brauchen die imitierenden Sätze 2,51-mal so lange, weil sie länger sind und weil das
fünfstimmige Agnus Dei III mit Seed 1 in Sackgassen des Strahls läuft: Zehn Proben und drei Suchen
scheitern, bevor der doppelte Strahl den Satz findet (145 s). Je Takt ist die Suche mit Imitation
schneller: Der Test `imitative_search_is_fast_enough` vergleicht das vierstimmige Kyrie I (dorisch,
Seeds 1–6) mit der Suche ohne Imitation über dieselbe Zahl von Takten und findet im Median
0,76-mal so viel Zeit (0,62–0,86; Grenze 1,5); gegen die 16 Takte des Textes allein sind es
2,42-mal so viel. Solche Sackgassen hängen vom Plan ab und treffen vor allem fünfstimmige Sätze.
Über 48 Anfragen (Kyrie I, Christe, Agnus Dei I und II in vier Modi, drei- und vierstimmig, Seeds
1–2) brauchen die Proben 38 s (11 von 59 scheiterten) und die vollen Suchen 278 s (keine von 48
scheiterte), zusammen das 1,95-fache derselben Texte ohne Imitation. `fux free` setzt das Kyrie I
dreistimmig in 2,8 s und 4,2 s (Seeds 1 und 2), vierstimmig in 13,1 s und 6,7 s; `--plan` braucht
höchstens 0,035 s. Im Browser ohne Threads (WebAssembly, gemessen mit Deno) dauert es dreistimmig
15 s und 21 s, vierstimmig 62 s und 32 s, ohne Imitation vierstimmig 15 s.

*Stil:* Derselbe Erkenner (Einsätze nach Pausen ab einer Halben) misst Palestrinas 1156 Sätze (6738
Punkte) und 72 erzeugte Sätze (159 Punkte): Kyrie I, Christe, Agnus Dei I und II und Benedictus
vierstimmig in vier Modi mit den Seeds 1–3 und drei- und fünfstimmig in drei Modi mit Seed 1 (Test
`imitation_style`).

| | Palestrina | erzeugt | Ziel |
|---|---|---|---|
| zweiter Einsatz in Quarte oder Quinte | 61,8 % | 79,2 % | 65–85 % |
| zweiter Einsatz in Einklang oder Oktave | 35,9 % | 20,8 % | 10–30 % |
| zweiter Einsatz nach einem Takt, vierstimmig | 29,1 % | 35,4 % | 25–50 % |
| Führer der Eröffnung auf Finalis oder Quinte | 74,2 % | 98,6 % | ab 65 % |
| erster Ton des Führers mindestens eine Ganze | 95,9 % | 100 % | ab 90 % |
| imitierte Töne, Median | 6 | 6 | 5–8 |
| Umfang des Kopfes, Median | Quinte | Quarte | Quarte bis Sexte |
| Engführung | 63,0 % | 71,5 % | 50–80 % |
| reale Antworten | 96,3 % | 100 % | ab 85 % |
| Einsätze späterer Punkte auf der Taktmitte | 78,0 % | 94,0 % | ab 40 % |
| wiederholte Duos in vollständigen vierstimmigen Eröffnungen | 20,4 % | 18,8 % | 10–35 % |
| erstes Intervall eine Tonwiederholung | 28,8 % | 25,0 % | 15–40 % |

Die Spalte „Ziel“ stützt sich auf eine andere Auswahl von 737 Eröffnungen (zweiter Einsatz
in Quarte oder Quinte 75 %, im Einklang oder in der Oktave 20,5 %, nach einem Takt 38 %); mit dem
Erkenner des Programms ergeben Palestrinas Sätze die Werte der Tabelle.

*Satzdichte und Wiedereinsätze* (48 Sätze Kyrie I in vier Modi, drei- bis fünfstimmig, Seeds
1–4; Palestrina dreistimmig mit allen Abschnitten, vier- und fünfstimmig mit den Kyrie-Abschnitten,
die mit 77, 47 und 56 Takten länger sind als die erzeugten mit 26, 30 und 34; in Klammern die
Werte vor den letzten Korrekturen):

| | Palestrina 3 / 4 / 5 Stimmen | erzeugt 3 / 4 / 5 Stimmen |
|---|---|---|
| Pausen ab vier Breven, von allen Pausen ab einer Halben | 0,0 / 0,4 / 1,1 % | 0,0 / 0,0 / 2,1 % (0,0 / 3,6 / 2,2 %) |
| Sätze mit einer solchen Pause | 0,0 / 3,8 / 17,0 % | 0,0 / 0,0 / 18,8 % (0,0 / 18,8 / 12,5 %) |
| eingesetzte Stimmen, die vor dem letzten ersten Einsatz schweigen (halbe Schläge) | 0,8 / 0,5 / 1,9 % | 0,0 / 0,2 / 0,8 % (0,0 / 0,0 / 0,2 %) |
| halbe Schläge nach dem letzten ersten Einsatz, an denen alle Stimmen klingen | 73,6 / 57,3 / 40,7 % | 75,2 / 54,8 / 37,5 % (74,2 / 43,9 / 39,3 %) |
| halbe Schläge danach, an denen vierstimmig nur zwei Stimmen klingen | 3,8 % | 5,4 % (19,1 %) |
| Wiedereinsätze nach einer Halben Pause innerhalb eines erkannten Punktes | 79,0 / 71,5 / 63,6 % | 59,3 / 39,6 / 29,6 % (57,6 / 35,5 / 28,4 %) |
| Wiedereinsätze, die ein Soggetto bringen | 57,9 / 47,6 / 37,7 % | 47,5 / 24,8 / 15,5 % (47,5 / 21,8 / 14,9 %) |

Vierstimmig klingen nach der Exposition im Mittel 3,49 Stimmen (Palestrina 3,53, vorher 3,23), und
keine Stimme schweigt vier Breven lang; fünfstimmig klingen auf 2,7 % der Taktanfänge nach der
Exposition nur zwei Stimmen oder eine (vorher 0,4 %; Palestrina 1,3 %).

In 22 Sätzen mit Punkten auf neuen Worten (Agnus Dei, Benedictus, Sanctus, `gloria:1-5`) beginnt
keine Stimme eine Silbe eines späteren Punktes vor diesem Punkt (0 von 2209), vierstimmig liegen die
Wiedereinsätze zu 63,6 % in einem Punkt und bringen zu 52,5 % ein Soggetto, und 2,0 % der Pausen
dauern vier Breven und mehr (in zwei von 20 Sätzen). In 2040 Plänen von 360 Anfragen (Kyrie I,
Christe, Agnus Dei I und II in vier Modi, drei- bis fünfstimmig, Seeds 1–10, mit Neuplanungen)
ist von 15 245 Köpfen keiner weiter als eine Oktave; 175 liegen unter dem Liniensystem ihrer Stimme
oder mehr als eine Stufe darüber, 1512 mehr als eine Stufe außerhalb der Oktave ihrer Stimme im
Modus, meist mixolydische und äolische Unterstimmen, deren Oktave im Modus unter ihr Liniensystem
reicht (beim Bass D2–D3 gegen ein System ab G2).

*Dissonanzen gegen übergebundene Töne* (dieselben 70 Sätze; eine unabhängige Zählung des Reviews):
Töne nach dem Taktanfang, im Sprung in eine Dissonanz gegen einen übergebundenen Ton, 0,15 je 100
Stimmen-Takte (vorher 0,34), auf dem Taktanfang angeschlagene Dissonanzen gegen einen übergebundenen
Ton, der nicht direkt stufenweise abwärts geht, 0,44 (vorher 0,71); Sätze mit einem solchen Fall 32
von 70 (vorher 45). Was die Zählung noch findet, sind verzierte Auflösungen auf dem dritten Viertel
(F–D–E) und Töne, die sich gegen einen echten Vorhalt bewegen, beides erlaubt; IMI-HELD meldet in
den 70 Sätzen fünf Fälle `passing` (P3). Das vierstimmige Kyrie I (Seed 1) hat keinen Fall mehr.

**Ohne Imitation unverändert.** Die drei Golden-Tests bestehen in Debug und Release, und die
Tests des Textes (`every_section`, `five_and_six_voices_with_text`, `calibration`, `jeppesen`)
ergeben dieselben Sätze, Takte und Kosten wie vor der Einführung der Imitation.

### Prüfen

**Was eine Datei von fux-kontrapunkt mitbringt.** Ein imitierender Satz trägt seinen Plan in der
Partitur (`Score.imitation`); MusicXML schreibt ihn neben `fux-text`, `fux-cadences` und `fux-mode`
(§ 17) als `miscellaneous-field` `fux-imitation`: je Punkt seine Silben und das Ziel seines
Binnenschlusses, je Einsatz Stimme, Tick, Pause und die Töne des Kopfes mit ihren Werten, etwa
`p0-7g9|1@0/0:A3.8,C4.4,…|p3-7|2@84/4:D3.4,…`. `fux check` nimmt den Plan aus der Datei, wenn er
zur Partitur passt (Stimmen, Länge, Silben des Textes, Ziele), und prüft die IMI-Regeln damit, und
die TXT-Regeln mit den Wiederholungen vor seinen Punkten (§ 17); so ergibt die Prüfung eines
exportierten Satzes denselben Bericht wie das Erzeugen (Test
`check_of_an_exported_setting_repeats_its_report`), ebenso in der GUI („Ergebnis übernehmen“,
„Datei …“) und in der Web-Version. Weicht ein Kopf von seinem Plan ab, etwa nach einer Bearbeitung
der Noten in MuseScore, meldet das IMI-ENTRY (eine andere Tonhöhe `level`, V; ein kürzerer Wert
`rhythm`, P1). Passt der Plan nicht zur Partitur, etwa nach gestrichenen Stimmen oder Takten,
meldet `fux check` „Der Imitationsplan der Datei passt nicht zum Satz: …“ mit der Abhilfe
`--imitation aus`; GUI und Web-Version warnen dann („…; geprüft ohne ihn“) und prüfen ohne den
Plan. Die Ausgabe nennt die Punkte wie beim Erzeugen: „Punkt 1 („Ký-ri-e e-lé-i-son.“): Tenor T. 1
A3 · Alt T. 2 A3 (Einklang) · Bass T. 8 D3 (Unterquinte); Kopf 6 Töne, real; Binnenschluss T. 14
auf A“.

**`--imitation datei|erkennen|aus`.** `datei` (Standard) nimmt den Plan der Datei, `aus` prüft ohne
IMI-Regeln. `erkennen` sucht die Punkte in den Noten und listet sie zur Information, mit Stimmen,
Takten, Intervallen zum Führer und der Zahl gleicher Töne; Verstöße gegen IMI-Regeln entstehen
daraus nie.

**Erkennen** (`fux_core::imitation::detect`), wie die Auswertung von Palestrinas Messen:
- **Abschnitte:** Ein gehaltener Klang schließt einen Abschnitt, wenn ihn kein Ton und keine Pause
  überbrückt, mindestens die Hälfte der Stimmen (mindestens zwei) dort einen Ton beenden, jeder
  mindestens eine punktierte Ganze und einer eine Brevis lang, und wenn er mindestens vier Breven
  von beiden Enden entfernt ist; von Schlüssen, die weniger als drei Breven auseinander liegen,
  zählt der letzte.
- **Einsätze:** der erste Ton einer Stimme im Abschnitt und jeder Ton nach einer Pause einer
  gegebenen Mindestlänge (`erkennen`: eine Halbe). Der Kopf reicht bis zur nächsten solchen Pause.
- **Imitation:** Zwei Einsätze verschiedener Stimmen, der zweite 1 bis 96 Ticks (sechs Breven)
  nach dem ersten, imitieren einander, wenn von ihren ersten Tönen an mindestens vier Töne
  übereinstimmen: dieselben diatonischen Intervalle, dieselben Werte außer dem ersten und dem
  letzten übereinstimmenden Ton, Pausen im Kopf an denselben Stellen und gleich lang. Eines der
  ersten beiden Intervalle darf um eine Stufe abweichen (die modale Antwort); dann müssen fünf
  Töne übereinstimmen, oder vier, wenn der Plan der Partitur die beiden Einsätze in einen Punkt
  legt und einen Kopf auf vier Töne kürzt.
- **Punkte:** Jeder Einsatz schließt sich dem Punkt an, dessen Mitglied er am besten imitiert und
  dessen letzter Einsatz höchstens sechs Breven zurückliegt, sonst beginnt er einen neuen. Ein
  Punkt eröffnet seinen Abschnitt, wenn sein Führer mit dem ersten Ton seiner Stimme höchstens
  eine Brevis nach dem ersten Ton des Abschnitts einsetzt.

An Palestrinas Messen ergibt das die Zahlen der Auswertung genau: 91,4 % der Satzanfänge beginnen
mit einem Punkt (mit Einsätzen schon nach einer Halben Pause 90,6 %), und die 1156 Sätze haben 4847
Punkte (6738 mit Einsätzen nach einer Halben), 2480 (3626) davon mit drei und mehr Stimmen (Test
`detector_reproduces_the_corpus_openings`). In jedem Testsatz findet der Erkenner jeden
geplanten Punkt mit seinen Stimmen.

**Ausgabe.** LilyPond setzt über den Kopf jedes Einsatzes eine Klammer mit der Nummer des Punktes
(`Horizontal_bracket_engraver`, über dem System, weg vom Text), MusicXML schreibt sie als
`direction` mit `bracket` und `words`, Anfang und Ende mit dem System der Stimme (`staff`), ohne das
Verovio 6.3.0 die Klammern ab dem dritten System nicht schließt; `--marks aus` lässt sie weg, in der
GUI und in der Web-Version das Kästchen „Einsätze markieren“ über dem Notenbild. `fux render` setzt
die Klammern einer Datei, deren Plan zu ihr passt. MIDI bleibt unverändert. In LilyPond steht die
Nummer klein über dem ersten Ton der Klammer und über der Klammer selbst (`^\markup \small`, mit
`TextScript.outside-staff-priority = #1000` über der Priorität 800, die die Klammer seit LilyPond
2.26 hat; ebenso die Beschriftungen von `fux check --mark` in einem Satz mit Klammern). Läuft eine
Klammer über einen Zeilenwechsel, steht die Nummer nur am Anfang, nicht mehr in Klammern („(2)“) am
Anfang der nächsten Zeile. Jede Text- und Ziffernzeile steht mindestens 7 Linienabstände über der
Mittellinie des Systems darunter ([Notenbild und Ausgabe](#notenbild-und-ausgabe)), so dass Nummern
und Ficta dieses Systems nicht in sie hineinragen.

### Grenzen und Abweichungen von Palestrina

- **Länge.** Ein Satz ist so lang, wie sein Plan es verlangt: das Kyrie I im Median 24, 29 und 32
  Takte drei-, vier- und fünfstimmig, statt 16 ohne Imitation. Palestrinas Kyrie I dauert im
  Mittel etwa 25 Breven, also rund 50 Takte, mit zwei bis drei Punkten; die Imitation nähert die
  Längen nicht an.
- **Der erste Punkt hat alle Stimmen.** Bei Palestrina enthält der Eröffnungspunkt alle Stimmen nur
  in 93, 54, 35 und 17 % der imitierenden Eröffnungen mit drei, vier, fünf und sechs Stimmen; die
  übrigen setzen frei oder mit einem zweiten Soggetto ein.
- **Keine gleichzeitigen Einsätze und kein zweites Soggetto.** Bei Palestrina beginnt je nach
  Satzteil in 11–40 % der Abschnitte ein zweites Soggetto innerhalb einer Brevis nach dem ersten,
  und in 7–23 % setzen zwei oder mehr Stimmen zusammen ein. Der Planer plant weder das eine noch
  das andere.
- **Formelstimmen nach ihrer Stellung, keine umgangenen Kadenzen.** Die Klauseln singen wie in
  § 16 die Oberstimme, die Stimme über dem Bass und der Bass, gleich wer im Punkt einsetzt; eine
  Formelstimme führt den nächsten Punkt erst anderthalb Takte nach dem Ziel und zahlt dafür
  IMI-OVERLAP `late`. Kadenzen, die Palestrina umgeht, gibt es nicht.
- **Wiedereinsätze ohne Soggetto.** Der Planer plant nur die Einsätze der Punkte, keine weiteren mit
  dem Soggetto innerhalb eines Punktes. Vierstimmig bringen so 25 % der Wiedereinsätze ein Soggetto,
  bei Palestrina 48 %.
- **Satzdichte fünfstimmig.** Fünfstimmig dauern 2,1 % der Pausen ab einer Halben vier Breven und
  mehr (Palestrina 1,1 %), und auf 2,7 % der Taktanfänge nach der Exposition klingen höchstens zwei
  Stimmen (1,3 %). Vierstimmig klingen nach dem letzten ersten Einsatz etwa so viele Stimmen wie bei
  Palestrina, seit IMI-SILENCE `thin` die halbe Besetzung bepreist (alle Stimmen auf 54,8 gegen
  57,3 % der halben Schläge, nur zwei auf 5,4 gegen 3,8 %).
- **Dissonanzen gegen übergebundene Töne und halbe Besetzung nur mit Plan.** IMI-HELD und
  IMI-SILENCE `thin` prüfen nur Sätze mit einem Plan der Imitation. In den Teilen einer Messe prüft
  das Profil der Messe dieselben Fälle auch ohne Plan (`unplanned_*`, § 19). In einzelnen Sätzen
  ohne Imitation und in fremden Dateien bleiben die Lücken von FREE-PASSING, FREE-RESOLUTION und
  FREE-TEXTURE (§ 16); eine allgemeine Neueichung dieser Regeln gibt es nicht.
- **Zu regelmäßig:** Der Führer einer Eröffnung beginnt fast immer auf Finalis oder Quinte (98,6
  gegen 74,2 %), spätere Einsätze fast immer auf der Taktmitte (94,0 gegen 78,0 %), die Köpfe sind
  enger (im Median eine Quarte gegen eine Quinte), und modale Antworten sind selten (reale Antworten
  100 gegen 96,3 %). Fünfstimmig setzen in 18 % der ersten Eröffnungen drei Stimmen oder mehr auf
  einer Tonhöhe ein (Palestrina 11–12 % der vollständigen Eröffnungen).
- **Gekürzte Führer.** Stößt der Kopf des Führers mit dem des zweiten Einsatzes zusammen, kürzt der
  Planer den früheren: Der Führer der Eröffnung singt in 51 % der ersten Pläne weniger Töne als
  sein Soggetto, in 35 % nur vier, und 18 % der folgenden Einsätze haben höchstens vier Töne
  (Palestrina imitiert in 13,8 % höchstens vier).
- **Nicht umgesetzt:** Pausen innerhalb eines Kopfes; rhythmische Varianten späterer Einsätze außer
  am ersten Ton; die Terz des Klangs gezielt mit dem dritten Einsatz; der Choral in langen Noten in
  einer Stimme und das Umformen seiner letzten Töne zur Klausel; Homophonie und Auslassungen (auch
  in der Messe nicht, § 19). Ein Kopfmotiv über mehrere Sätze bringt die Messe
  ([§ 19](#kopfmotiv-und-choral)).
- **Tore sind keine Regel.** Die Suche lässt keine Stimme die Worte eines späteren Punktes vor
  ihrem Tor beginnen, aber keine Regel prüft das: `fux check` meldet es in anderen Dateien nicht.

## 19. Messe

`fux mass`, die grafische Oberfläche und die Web-Version setzen eine ganze Messe: Kyrie, Gloria,
Credo, Sanctus, Benedictus und Agnus Dei mit dem Text des Ordinariums (§ 17), in den Satzformen von
Palestrinas Messen, standardmäßig eine vierstimmige Missa brevis im dorischen Modus. Ein **Plan**
teilt die Messe vor jeder Suche in **Teile** und legt für jeden fest, welchen Text er in welchen
Stimmen singt, auf welcher Stufe er schließt, wie er nach dem Teil davor beginnt und wie seine
Binnenschlüsse aufeinander folgen. Jeder Teil ist ein freier Satz mit Text (§ 16, § 17), der
imitiert (§ 18) oder in allen Stimmen gemeinsam auf einem Klang beginnt. Die Teile werden einzeln
gesucht, keiner liest die Noten eines anderen, außer dass die Teile mit dem Kopfmotiv es aus dem
Kyrie I nehmen. So wächst die Rechenzeit mit der Zahl der Teile, nicht mit dem Quadrat der Länge
eines Abschnitts.

Die MASS-Regeln sind Regeln des Plans: Sie gelten durch Konstruktion, keine Prüfung meldet sie, und
man stellt sie mit den Optionen von `fux mass` ein, nicht mit `--rule`. Ihre Werte stammen aus
Zählungen in Palestrinas Messen; vorläufig sind nur die Anteile des Neubeginns an Nähten. Dazu
kommen drei Parameter von IMI-HELD und IMI-SILENCE, mit denen das Profil der Messe die Teile ohne
Imitation so prüft wie die imitierenden ([Regeln der Messe](#regeln-der-messe)).

§ 19 beschreibt, wie die Messe entsteht, ihr Notenbild (ein Buch mit Sätzen und Abschnitten,
wahlweise mit Mensurstrichen, MusicXML und MIDI je Satz), das Prüfen ganzer Sätze, Kopfmotiv und
Choral und die Messe in der grafischen Oberfläche und der Web-Version.

**Stände der Messungen.** Die Zahlen dieses Abschnitts nennen, mit welchen Noten sie gemessen sind:
*vor der Durchsicht* heißt vor der ersten Durchsicht der Messe, deren Korrekturen alle Ziehungen
und Seeds der Teile änderten; *vor der Neueichung* heißt mit den Noten danach, bevor im freien Satz
ab drei Stimmen ein Schlussklang ohne große Terz und die leere Sexte auf einem angeschlagenen
Taktanfang je P2 kosteten (§ 16); *nach der Neueichung* heißt mit den heutigen Noten. Die
schnellere Suche nach dem Plan und das Setzen mehrerer Teile zugleich (*Laufzeit* unten) ändern
keine Note.

**Quellen**, wie die Tabellen sie nennen:
- Palestrinas Messen (Korpus von § 16; für Formen, Besetzungen und Längen alle Sätze, auch die im
  Dreiertakt): gezählt und unabhängig nachgezählt; wo die Nachzählung abweicht, gelten ihre Werte.
  Namen wie CAD-SECTION-END nennen einen Befund dieser Zählung; es sind keine Regeln, und
  `fux rules` kennt sie nicht.
- Palestrinas *Missa brevis* (1570), das Vorbild der Form brevis; im Korpus Kyrie_15, Gloria_12,
  Credo_14, Sanctus_10, Benedictus_11 (a und b), Agnus_I_12 und Agnus_II_11.
- Pietro Pontio: *Ragionamento di musica*, Parma 1588 (gelesen nach R. Murrays Dissertation von
  1989), und *Dialogo*, Parma 1595. Zarlino 1558, Buch III, Kap. 53, Buch IV, Kap. 30 und 32.
  Jeppesen: *Counterpoint* (1939), S. 81 f. und 257, und *The Style of Palestrina and the
  Dissonance* (2. Aufl. 1946), S. 42–44. P. Wagner: *Geschichte der Messe* I, Leipzig 1913, S. 446
  (Palestrinas Choralmessen, wie in § 18).

### Begriffe der Messe

- **Messe:** die sechs Sätze in einer Form (brevis oder normal), einem Modus und einem Chor von zwei
  bis sechs Stimmen.
- **Satz:** Kyrie, Gloria, Credo, Sanctus, Benedictus (mit seinem Osanna) oder Agnus Dei. Ein Satz
  endet auf der Finalis.
- **Abschnitt:** ein Stück eines Satzes zwischen Doppelstrichen, mit eigener Besetzung, etwa
  Kyrie I, Christe und Kyrie II. Er singt einen oder mehrere Texte des Ordinariums (§ 17): Der
  Abschnitt „Et in terra pax“ der Missa brevis singt `gloria` und `qui-tollis`. Die Abschnitte
  heißen wie bei Palestrina und in der Ausgabe von `fux mass`: **Agnus Dei I** singt den Text
  `agnus` (in § 17 und § 18 „Agnus Dei I und II“), **Agnus Dei II** den Text `agnus3` (dort
  „Agnus Dei III“).
- **Teil:** was einmal gesucht und geprüft wird, ein freier Satz für sich. Ein kurzer Abschnitt ist
  ein Teil, ein langer wird an Satzenden in Teile geschnitten (`credo:5-7`).
- **Halteklang:** der Schlussklang eines Teils, den alle Stimmen einen Takt lang halten, bevor ein
  Teil desselben Abschnitts nach einem Takt Pause neu beginnt, an den Stellen, an denen die Missa
  brevis einen hat. Er trägt den Namen der Stelle: in der Form brevis Qui tollis, Et incarnatus est
  und Crucifixus, in der Form normal Et incarnatus est (dort beginnen Qui tollis und Crucifixus
  Abschnitte). Optionen nennen ihn so (`--imitation incarnatus=aus`).
- **Naht:** dasselbe ohne Namen an den übrigen Schnitten eines langen Abschnitts; nach dem
  gehaltenen Klang atmen alle Stimmen nur eine Halbe.
- **Übergang:** wie ein Teil an den Teil davor anschließt: über einen Doppelstrich (Abschnitt),
  einen Halteklang oder eine Naht.
- **Neubeginn:** wie ein Teil beginnt: in **Imitation**, der Führer des ersten Punktes an seinem
  Start (§ 18), oder **gemeinsam**, alle Stimmen auf einem vollen Dreiklang der Finalis (zweistimmig
  ohne Terz, die Oberstimme auf Grundton oder Quinte).
- **Hauptstufe** (P): die zweite Schlussstufe der Messe neben der Finalis, einmal je Messe gezogen.
  Teile, die nicht auf der Finalis schließen, schließen meist auf P ([Kadenzplan](#kadenzplan)). P
  ist nicht die „andere Hauptstufe“ einer Antwort (§ 18), auch wenn beide oft zusammenfallen
  (dorisch A).
- **Kopfmotiv:** die Töne des Soggetto, mit dem das Kyrie I beginnt, wenn Gloria, Credo, Sanctus
  und Agnus Dei I mit ihnen beginnen; **Choral:** der Gesang des Kyriale, den die Soggetti einer
  Messe auf dem Choral paraphrasieren ([Kopfmotiv und Choral](#kopfmotiv-und-choral)).

„Satz“ allein meint hier einen Satz der Messe. Ein Satz des Textes (§ 17) heißt immer so, „Satzende“
meint wie in § 17 sein Ende, und ein Teil ist ein freier Satz wie einer von `fux free`.

Zeitangaben in Ticks zählen wie in § 18 Achtel: eine Halbe 4, ein Takt 8.

### Satzformen und Besetzung

**Formen** (MASS-FORM, `--preset`; ‖ trennt Abschnitte):

| Satz | brevis (Standard) | normal |
|---|---|---|
| Kyrie | Kyrie I ‖ Christe ‖ Kyrie II | ebenso |
| Gloria (Intonation „Glória in excélsis Deo.“) | ein Abschnitt „Et in terra pax“ mit dem Halteklang „Qui tollis“ | Et in terra pax ‖ Qui tollis |
| Credo (Intonation „Credo in unum Deum,“) | ein Abschnitt „Patrem omnipotentem“ mit den Halteklängen „Et incarnatus est“ und „Crucifixus“ | Patrem omnipotentem mit dem Halteklang „Et incarnatus est“ ‖ Crucifixus bis „non erit finis“ (verkleinert) ‖ Et in Spiritum Sanctum bis Amen |
| Sanctus | ein Abschnitt Sanctus, Pleni und Osanna | Sanctus ‖ Pleni sunt caeli (verkleinert) ‖ Osanna |
| Benedictus | Benedictus (verkleinert) ‖ Osanna (neu, gemeinsam beginnend) | Benedictus (verkleinert), dann „Osanna ut supra“: das Osanna des Sanctus, ohne eigene Suche |
| Agnus Dei | Agnus Dei I ‖ Agnus Dei II (vergrößert) | ebenso |

Quellen (Korpus, Zählung und Nachzählung):
- Kyrie I ‖ Christe ‖ Kyrie II in 84 von 86 Messen, Agnus Dei I ‖ Agnus Dei II in 72 von 86.
  Kyrie I, Christe und Kyrie II stehen je für drei Anrufungen, Agnus Dei I für die erste und zweite,
  Agnus Dei II (dona nobis pacem) für die dritte.
- Das Gloria hat in 52 bis 55 von 85 Messen zwei Abschnitte, getrennt bei Qui tollis.
- Ein ungeteiltes Credo gibt es nur in vierstimmigen Messen (8 von 86), und keines der 12 schnellen
  vierstimmigen Credos hat ein eigenes Crucifixus. Die Missa brevis teilt ihr Credo mit Halteklängen
  in 50 | 12 | 84 Breven. Auf ein eigenes Crucifixus bis „non erit finis“ folgt Et in Spiritum in 40
  von 69 Messen.
- Auf ein ungeteiltes Sanctus folgt in 31 von 33 Messen ein neues Osanna II; nach einem dreiteiligen
  steht in 25 von 32 „Osanna ut supra“.

**Besetzung** je Abschnitt (MASS-SCORING, `--scoring`), nach der Mehrheit in Palestrinas Messen der
Stimmenzahl; Chöre von oben, Standard S T, A T B, S A T B, S A T T B, S S A T T B:

| Abschnitt | 3 Stimmen | 4 Stimmen | 5 Stimmen | 6 Stimmen |
|---|---|---|---|---|
| Christe | alle | alle (37 von 37) | alle (19 von 27; 8 ohne Bass) | S S A T (11 von 21 zu vier Stimmen) |
| Benedictus | alle | S A T (dreistimmig in 27 von 38, davon 13 S A T) | S A T T (25 von 27 vierstimmig) | S S A T (18 von 21 vierstimmig) |
| Crucifixus, Pleni (nur normal) | alle | S A T (Crucifixus 19 von 23, Pleni 12 von 14 dreistimmig) | S A T T (Crucifixus 25 von 26) | S S A T (Crucifixus 21 von 21) |
| Agnus Dei II | alle | S A T T B (alle 34 vergrößert, 15 mit einem Tenor, 11 mit einem Alt) | S S A T T B (17 von 24) | alle (15 von 19) |
| alle anderen | alle | alle | alle | alle |

Ein verkleinerter Abschnitt lässt den Bass weg, sechsstimmig auch den zweiten Tenor (S S A T), wie
meist bei Palestrina. Dreistimmige Messen hat Palestrina nicht geschrieben; sie behalten überall
alle Stimmen. Die Stimmen eines Satzes sind die aller seiner Abschnitte, der k-te Tenor eines
Abschnitts ist der k-te des Satzes („Tenor 1“, „Tenor 2“), und wer in einem Abschnitt nicht singt,
pausiert: Im vierstimmigen Agnus Dei pausiert Tenor 2 im Agnus Dei I, im Benedictus der Bass. Die
Missa brevis vergrößert ihr Agnus Dei II um einen zweiten Cantus im Kanon; hier kommt ohne Kanon ein
Tenor dazu (siehe [Grenzen der Messe](#grenzen-der-messe-und-abweichungen-von-palestrina)). Findet
das vergrößerte Agnus Dei II mit keinem seiner drei Seeds einen Satz, wird es im Chor der Messe
gesetzt, mit der Warnung „Abschnitt agnus3: mit der hinzugefügten Stimme nicht zu setzen; gesetzt in
der Besetzung der Messe“.

### Teile langer Abschnitte

**Schnitt** (MASS-PARTS, `--chunk`): Ein langer Abschnitt wird an den Satzenden seines Textes (`.`
`?` `!`) in die wenigsten Teile von höchstens 45 Silben geschnitten. Unter den Schnitten mit so
vielen Teilen gewinnt der mit dem kleinsten größten Teil, dann der mit den gleichmäßigsten Teilen,
dann der früheste. An einem Halteklang wird immer geschnitten; ein Satz des Textes über 45 Silben
wird an seinen Kommas, Semikolons und Doppelpunkten geteilt. Kurze Abschnitte (Kyrie, Sanctus,
Benedictus, Agnus Dei) bleiben ein Teil, auch Sanctus, Pleni und Osanna der Missa brevis, die als
ein Text gesungen werden (`sanctus+pleni+osanna`, § 17). Der Text jedes Teils außer dem letzten
seines Abschnitts geht im nächsten weiter (`Text.continued`, siehe [Regeln](#regeln-der-messe)).

Grund: Jeder Schritt der Suche prüft die ganze Partitur bis dahin, eine Suche kostet also
quadratisch in der Länge, vierstimmig etwa 0,052·N + 0,0029·N² Sekunden für N Takte (24 / 48 / 96 /
144 Takte: 2,9 / 9,4 / 31,2 / 67,2 s). In einer frühen Messung brauchte das ganze Patrem 756 s, in
vier Teilen 61 s. Die Größe von 45 Silben ist gemessen: Mit `--chunk 55` hatten Gloria und Credo der
dorischen Messe in der frühen Messung 11 statt 16 Übergänge, aber zwei Teile hätten im Browser mehr
als vier Minuten gebraucht (mit 45 keiner), und die beiden Sätze dauerten 13 % länger. Nach der
Durchsicht überschreitet auch mit 45 ein Teil des dorischen Credo diese Grenze (`spiritum:8-9`,
siehe *WebAssembly* unter [Messe erzeugen](#messe-erzeugen)). `--chunk 20` bis `200` wählt eine
andere Größe, `--chunk aus` setzt jede Strecke zwischen zwei Halteklängen in einem Teil.

Die Missa brevis vierstimmig (‖ Abschnitt, | Halteklang, · Naht; in Klammern die Silben):

| Satz | Teile |
|---|---|
| Kyrie | kyrie (7) ‖ christe (6) ‖ kyrie2 (7) |
| Gloria | gloria:1-5 (36) · gloria:6-9 (34) · gloria:10-13 (28) \| qui-tollis:1-4 (33) · qui-tollis:5-8 (29) · qui-tollis:9-13 (27) |
| Credo | credo:1-4 (30) · credo:5-7 (35) · credo:8-14 (42) · credo:15-16 (21) \| incarnatus (25) \| crucifixus (24) · resurrexit:1-4 (31) · resurrexit:5-7 (31) · spiritum:1-4 (27) · spiritum:5-7 (30) · spiritum:8-9 (38) · spiritum:10-12 (25) |
| Sanctus | sanctus+pleni+osanna (34) |
| Benedictus | benedictus, S A T (14) ‖ osanna2 (7) |
| Agnus Dei | agnus (18) ‖ agnus3, S A T T B (18) |

Zusammen 10 Abschnitte, 26 Teile, 657 Silben: das Gloria in 3 + 3 Teilen, das Patrem in 4, Et
incarnatus in 1, Crucifixus bis Amen in 7. Mit `--chunk 55` sind es 21 Teile, mit `--chunk aus` 13.
Die Form normal hat 15 Abschnitte, 27 Teile und 650 Silben, denn ihr Osanna ut supra singt nichts
Neues.

**Übergänge und Start.** Wo ein Teil beginnt, schweigt jede Stimme bis zu seinem Start (Tick im
ersten Takt des Teils):

| Vor dem Teil | Start | Der Teil beginnt |
|---|---|---|
| Beginn eines Satzes oder Abschnitts | 0 | auf dem ersten Taktanfang |
| Halteklang: der Schlussklang des Teils davor, ein Takt lang | 8 | nach einem Takt Pause in allen Stimmen auf dem Taktanfang |
| Naht: ebenso | 4 | nach einer Halben Atem auf der Taktmitte |

Das Vorbild beginnt nach einem Halteklang auf dem Taktanfang neu, nach einer Ganzen Pause in allen
Stimmen (Credo_14, Et incarnatus) oder sofort (Gloria_12 bei Qui tollis, Credo_14 beim Crucifixus).
Eine Pause von mindestens einer Halben macht die Übergänge sicher: Kein melodisches Paar, keine
Überbindung und kein geplanter Einsatz reicht über sie (IMI-REST; Pontio 1595, S. 62–65). Die Länge
eines Teils wächst um den Takt seines Starts, und sein erster Binnenschluss kommt frühestens zwei
Takte nach dem Start an, nach einem Anfangsklang auf dem Taktanfang einen Takt später: Die
Diskantklausel bereitet ihren Vorhalt dann mit einem eigenen Ton nach dem Klang vor.

### Kadenzplan

**Schlussstufen** (MASS-ENDING, `--endings`). Ein Teil darf auf der Finalis, der Quinte oder der
Quarte schließen, wo der Modus dort eine Klausel hat: dorisch D A G, phrygisch E A, lydisch F C,
mixolydisch G D C, äolisch A E D, ionisch C G F, transponiert ebenso (dorisch auf G: G D C). Das
sind die Stufen, die TEX-FINAL-CHORD `free_degree` frei lässt (§ 16), außer der Quinte des
Phrygischen und der Quarte des Lydischen, die keine Klausel haben; C im Phrygischen schließt Pontio
aus (1588, S. 106). Der letzte Teil eines Satzes und jeder zweistimmige Teil enden auf der Finalis
(CAD-END ist zweistimmig V); die übrigen ziehen nach diesen Anteilen, beschränkt auf die erlaubten
Stufen und neu auf 1 gerechnet (Befund CAD-SECTION-END der Zählung, mit den Korrekturen der
Nachzählung je Modus; Zarlino IV.30: ein Modus darf einen Teil auf einer mittleren Stufe beenden):

| Teil | Anteile |
|---|---|
| letzter Teil eines Satzes | Finalis (Palestrina: 95,8 % der Schlüsse von 550 Sätzen) |
| Kyrie I | Finalis; phrygisch Finalis .5, A .5 (4 von 8 auf A) |
| Christe | dorisch Quinte .87, Finalis .13; ionisch Quinte .81, Finalis .19; mixolydisch Finalis .38, Quarte .38, Quinte .24; äolisch Quarte .50, Quinte .30, Finalis .20; phrygisch Finalis .5, A .5; lydisch Quinte .8, Finalis .2 |
| Teil vor Qui tollis | dorisch Quinte .71, Finalis .29; ionisch und lydisch Quinte .67, Finalis .33; mixolydisch Finalis .73, P .27; äolisch Finalis .62, Quarte .38; phrygisch Finalis .5, A .5 |
| Agnus Dei I | Finalis .88, P .12; phrygisch Finalis .57, A .43 (3 von 7 auf A) |
| Benedictus vor dem Osanna II | wie jeder andere Teil, aber ionisch Quinte .64, Finalis .36; mixolydisch Finalis .77, P .23 |
| Benedictus vor „Osanna ut supra“ | Finalis .70, Quinte .30; ionisch Quinte .875 (7 von 8), Finalis .125 |
| jeder andere Teil (vor einem Halteklang, einer Naht oder einem Abschnitt) | Finalis: dorisch .53, ionisch und lydisch .46, mixolydisch .70, äolisch .56, phrygisch .50; der Rest auf P |

Phrygisch ist A die Quarte und P. Plagale Modi nehmen die Tabellen ihres authentischen. Das
Lydische, das in den Messen fehlt, schließt wie das Ionische, außer im Christe (eigene Zeile) und im
Benedictus: vor dem Osanna II wie jeder andere Teil (Finalis .46, Quinte .54), vor „Osanna ut supra“
auf der Finalis .70, der Quinte .30.

Innerhalb eines Abschnitts wird eine Schlussstufe, die der des Teils davor gleicht, einmal neu
gezogen, damit die Schlussklänge an den Halteklängen und Nähten eines langen Abschnitts wechseln.
Über einen Doppelstrich hinweg nicht: Das Korpus zählte das Ende jedes Abschnitts nach dem Abschnitt
davor (das Christe nach dem Kyrie I auf der Finalis), eine neue Ziehung zählte den Wechsel doppelt.
Wörtlich für den ganzen Satz genommen hätte die Regel das dorische Christe in 1,6 % der Pläne auf
die Finalis gesetzt statt in 13 %. Weil die neue Ziehung eine Stufe dort wegnimmt, wo der Teil davor
endete, nehmen innerhalb eines Abschnitts die erste und die neue Ziehung erhöhte Anteile
(`tables::redrawn`): d′(x) = d(x) / (1 − e(x) + S), d die Anteile der Tabelle, e die, aus denen der
Teil davor zog, S so, dass d′ eins ergibt. Dann endet ein Teil genau so oft auf jeder Stufe, wie die
Tabelle sagt; ohne die Erhöhung schlössen die mixolydischen Nähte in .645 statt .70 auf der Finalis.
Gemessen an 10 000 Plänen je Form und Modus liegt jede Art von Teil höchstens 1,2 Punkte neben der
Tabelle, und zwei Teile eines Abschnitts enden in Folge auf derselben Stufe in .28 bis .32 der Fälle
(mixolydisch .47) statt .50 bis .60 bei unabhängigen Ziehungen.

`--endings "christe=A; gloria/3=D"` legt Schlussstufen fest; ein Abschnitt nennt seinen letzten
Teil. Eine Stufe ohne Klausel oder außerhalb von Finalis, Quinte und Quarte, ein ♭ nach einem
anderen Buchstaben als B, eine andere Stufe als die Finalis für den letzten Teil eines Satzes oder
für zwei Stimmen scheitern vor der Suche. Ob ein Teil neu zieht, hängt an der gezogenen Stufe des
Teils davor, nicht an einer festgelegten. Eine Angabe ändert deshalb keinen anderen Teil, mit einer
Ausnahme: Schlösse der Teil danach im selben Abschnitt auf der angegebenen Stufe, zieht er noch
einmal (und so weiter, solange eine neue Ziehung eine Stufe ändert). Eine Angabe gleich der
gezogenen Stufe ändert nichts.

**Hauptstufe** P, einmal je Messe gezogen (Befund CAD-PRINCIPAL-SECONDARY):

| Modus | P |
|---|---|
| dorisch, ionisch, lydisch | Quinte |
| phrygisch | A, die Quarte |
| äolisch auf A | Quarte (D) |
| äolisch auf D (transponiert) | Quinte (A) oder Quarte (G), je zur Hälfte |
| mixolydisch | Quarte (C) oder Quinte (D), je zur Hälfte. P nimmt den Rest der Schlussstufen (Tabelle oben); auf der anderen Stufe kann nur das Christe schließen, mit P = C auch das Benedictus vor „Osanna ut supra“. Palestrinas Messen mit der einen kadenzieren auch auf der anderen, deshalb verschiebt P die Anteile der Binnenschlüsse im Korpus nur um ±50 ‰ zu sich hin; die Kette zieht dafür je P mit einer eigenen Zeile von Gewichten (unten). |

Plagale Modi wie ihr authentischer, transponierte wie die untransponierten, außer dem Äolischen.
`fux mass` nennt P in der ersten Zeile des Plans („Hauptstufe A“).

**Binnenschlüsse** (MASS-CADENCES). In einem Teil einer Messe folgen die Stufen der Binnenschlüsse
einer **Kette** über den Satz, statt reihum über die Kadenzstufen des Modus zu wechseln (§ 16,
§ 17). Wo die Ziele liegen und welche Formeln sie singen, bleibt wie in § 17 und § 18; Sätze
außerhalb einer Messe (`fux free`) behalten den Wechsel reihum. Für jedes Ziel:
1. Mit p .30 wiederholt es die Stufe davor; vor dem ersten Ziel eines Teils gilt die Schlussstufe
   des Teils davor. Im Korpus folgt dieselbe Stufe in 28 bis 44 % der aufeinanderfolgenden
   Binnenschlüsse.
2. Sonst zieht es nach den Gewichten unten eine andere Stufe mit Klausel.
3. An einem Zwischenschluss (, ; :), einer Zäsur oder vor einem Schlussteil wiegen die Finalis und
   die erste Kadenzstufe des Modus ×0,3. Ein Binnenschluss auf der Finalis oder der ersten
   Kadenzstufe gehört an ein Satzende des Textes (Zarlino III.53); an Zwischenschlüssen und Zäsuren
   kostet er weiter TXT-CADENCE `principal` (P2, § 17), die Regel bleibt unverändert. Mit ×0,3
   bleibt er dort möglich, nur seltener. Der Plan des Textes außerhalb einer Messe (§ 17) lässt die
   erste Kadenzstufe dort ganz aus und zieht die Finalis nie.
4. Das erste Ziel des Christe gibt der Finalis ×0,4: Im Christe liegen 30 % der artikulierten
   Kadenzen im ersten Fünftel auf der Finalis, im Kyrie I 80 % (geprüfte Zählung).

Ziel i zieht mit einem eigenen Generator (Seed des Teils, i) und der Stufe von Ziel i − 1. Plant der
Planer der Imitation einen späteren Punkt neu, ändert sich kein früherer Binnenschluss.

Gewichte in ‰ je Modus: die Anteile der vollständigen Binnenschlüsse im Korpus (Stufen mit Klausel
und mindestens 2 %; Befund CAD-INTERNAL-DEGREES, alle acht Familien in der Nachzählung auf
2 Punkte bestätigt) und die ausgeglichenen Gewichte, mit denen die Kette zieht:

| Modus | Korpus | Gewichte der Kette |
|---|---|---|
| dorisch | D 448, A 349, G 67, C 63, F 59 | D 692, A 254, G 19, C 19, F 15 |
| dorisch auf G | D 360, G 346, B♭ 95, C 92, F 67, A 40 | D 450, G 433, B♭ 37, C 37, F 29, A 14 |
| phrygisch | A 406, E 180, C 159, G 128, D 114 | A 726, E 120, C 60, G 50, D 43 |
| mixolydisch, P = C | G 396, C 342, D 148, A 70 | G 676, C 162, D 136, A 25 |
| mixolydisch, P = D | G 396, C 242, D 248, A 70 | G 691, C 89, D 198, A 22 |
| äolisch | A 413, D 290, C 119, E 70, F 57, G 51 | A 731, D 121, C 47, E 60, F 22, G 18 |
| äolisch auf D | D 478, G 172, A 167, F 119, C 52 | D 889, G 27, A 56, F 21, C 8 |
| ionisch | C 464, G 314, A 76, D 74, E 46, F 25 | C 754, G 188, A 21, D 20, E 11, F 6 |
| ionisch auf F | F 436, C 361, B♭ 77, A 50, D 41, G 35 | F 620, C 316, B♭ 25, A 17, D 12, G 10 |
| lydisch | F 436, C 361, A 50, D 41, G 35 (das Ionische auf F ohne B♭) | F 695, C 271, A 14, D 12, G 9 |

Warum ausgeglichen: Mit den Anteilen des Korpus als Gewichten kam die Finalis 9 bis 18 Punkte zu
selten heraus (dorisch D .320 statt .454, äolisch auf D .302 statt .484, ionisch C .312 statt .464),
im Phrygischen die erste Kadenzstufe A, die dort P ist (.260 statt .411; die Finalis E .166 statt
.182), und die seltenen Stufen etwa doppelt so oft: Die meisten Ziele der Teile liegen an
Zwischenschlüssen und Zäsuren, wo Finalis und erste Kadenzstufe ×0,3 wiegen, und eine gezogene Stufe
wiederholt sich nur mit p .30. Die Gewichte sind deshalb geeicht (Test `calibrate_chain_weights`):
Die Ketten von 300 Plänen je Form, Chor von drei bis fünf Stimmen und Modus ziehen an den Zielen,
die Text und Punkte ihrer Teile planen, und jedes Gewicht wird 60 Runden lang mit (Anteil im Korpus
/ gezogener Anteil)^0,8 multipliziert. Danach liegt jede Stufe auf 0,3 Punkte bei ihrem Anteil im
Korpus. Die simulierten Ziele sind die der Suchen: In 12 720 von 12 720 Teilen geben Textplan und
Planer der Imitation dieselben Binnenschlüsse (Test `goals_are_those_of_the_settings`). In den vier
Messen unter [Messe erzeugen](#messe-erzeugen) liegen nach der Neueichung (`mass_style`) von 248
Binnenschlüssen .347 auf der Finalis (Korpus .371) und .343 auf P (.346), mit den Noten vor der
Neueichung von 249 .349 und .341; je Messe schwanken die Anteile stärker (61 bis 63 Binnenschlüsse,
etwa mixolydisch G .25 statt .41).

### Neubeginn

Wie ein Teil beginnt (MASS-RESTART, `--imitation auto|an|aus`, auch je Stelle):

| Der Teil beginnt | brevis | normal | Quelle |
|---|---|---|---|
| einen Abschnitt (Kyrie I, Christe, Kyrie II, Et in terra, Patrem, Sanctus, Benedictus, Agnus Dei I, Agnus Dei II; normal auch Qui tollis, Crucifixus, Et in Spiritum, Pleni, Osanna) | Imitation, Start 0 | Imitation, Start 0 | Eröffnungen zu 68–96 % imitierend |
| das Osanna II | gemeinsam, Start 0 | (ut supra) | das Osanna II der Missa brevis beginnt akkordisch |
| nach dem Halteklang Qui tollis oder Crucifixus | gemeinsam, Start 8 | (Abschnitt) | die Missa brevis (Gloria_12, Credo_14) |
| nach dem Halteklang Et incarnatus est | gemeinsam, Start 8 | gemeinsam, Start 8 | Pontio 1595, S. 61; Befund GC-INCARNATUS |
| nach der Naht bei Et resurrexit | gemeinsam, Start 4 | gemeinsam, Start 4 | Eröffnungen zu .80 gemeinsam |
| nach der Naht bei Et in Spiritum | Imitation mit p .35, sonst gemeinsam; Start 4, vorläufig | (Abschnitt) | Eröffnungen zu .52 imitierend, zu .61 homophon (Befund GC-SECTION-OPENING) |
| nach jeder anderen Naht | Imitation mit p .2, sonst gemeinsam; Start 4, vorläufig | Imitation mit p .35, sonst gemeinsam; vorläufig | Imitation in 32–35 % der Eröffnungen in Gloria und Credo |

Ein Abschnitt oder Teil mit zwei oder sechs Stimmen imitiert mit `auto` nicht (wie in § 18) und
beginnt gemeinsam: in einer sechsstimmigen Messe alles außer den vierstimmigen Abschnitten (Christe
und Benedictus, in der Form normal auch Crucifixus und Pleni), in einer fünfstimmigen das
sechsstimmige Agnus Dei II. `--imitation` legt es fest, etwa `"auto; incarnatus=aus; credo/3=an"`;
`an` imitiert auch mit zwei oder sechs Stimmen.

Die Zählung der Eröffnungen nach inneren Halteklängen wurde nicht nachgeprüft. Sie bestimmt den
Neubeginn nur, wo sie zum Vorbild von 1570 und zu Pontio passt; die Anteile an den Nähten bleiben
deshalb vorläufig. Die ersten Messungen der Messe haben sie nach der Form gewählt: Die Missa brevis
deklamiert ihr Gloria und Credo fast syllabisch, 0,82 Takte je Silbe, mit imitierenden Punkten in
15 % (Gloria) und 35 % (Credo) der Takte. Eine imitierende Naht braucht 1,54 Takte je Silbe und
doppelt so viel Zeit wie eine gemeinsame (0,93), gemessen an der dorischen Messe mit jeder Naht in
beiden Weisen. Gloria und Credo kommen mit .35 an den Nähten und .5 bei Et in Spiritum auf 1,17
Takte je Silbe, mit .2 und .35 auf 1,10 und ohne imitierende Nähte (nur Et in terra und Patrem
imitierend) auf 1,02 (gerechnet aus den Längen der frühen Messung). Mit .2 und .35 braucht eine
Messe etwa 4 % weniger Zeit als mit .35 und .5, und in einer Missa brevis eröffnen neben Et in terra
und Patrem im Mittel noch etwa 2,6 Sätze des Textes mit einem Punkt (in den vier Messen 3 bis 4 der
13 Nähte). Die Form normal behält .35, den Anteil imitierender Eröffnungen im Korpus, der zu ihrer
langsameren Deklamation passt.

**Anfangsklang.** Ein Teil, der gemeinsam beginnt, hat keine Imitation. Alle Stimmen schlagen auf
seinem Start den Dreiklang der Finalis in Grundstellung an, mit der Terz des Modus (dorisch D F A,
auf G: G B♭ D), als Ganze auf der ersten Silbe; bei Start 4 über den Taktstrich gebunden. Zwei
Stimmen beginnen ohne Terz auf einer vollkommenen Konsonanz über der Finalis, die Oberstimme auf dem
Grundton oder der Quinte, wie CAD-START es zweistimmig verlangt (§ 5). Die Suche darf den Ton länger
halten. Die Lage ist fest:
- der Bass auf dem Grundton;
- jede andere Stimme auf einem Ton des Klangs in ihrem Liniensystem;
- keine Kreuzung, benachbarte Oberstimmen höchstens eine Oktave auseinander;
- die Terz dabei, bis vier Stimmen einmal, die Quinte ab vier Stimmen; zweistimmig keine Terz;
- unter den Lagen, die das erfüllen, die mit den wenigsten Einklängen, dann die mit den wenigsten
  Tönen mehr als eine Stufe außerhalb der Oktave des Modus, in der die Stimme sich bewegt
  (FREE-AMBITUS, § 16), dann die, deren Töne den Mitten der Systeme am nächsten liegen.

Dorisch für S A T B: F4 D4 A3 D3 von oben, für S T: A4 über D4. Mixolydisch für S A T B: G4 D4 H3
G2; der Bass steht auf G2, nicht auf G3, denn seine Oktave des Modus reicht mit einer Stufe
Spielraum nur bis E3. Unter 240 Kombinationen aus Modus (auch transponiert und plagal) und Chor von
drei bis sechs Stimmen ändert die Oktave des Modus 9 Lagen, alle zu Lagen ohne Ton außerhalb. Als
Ausschluss ginge sie nicht: In 12 Kombinationen aus Modus und Standardchor liegt keine Lage ohne
Einklang ganz in den Oktaven. Die Stimme auf der Terz kostet FREE-ENTRY P2 (§ 16). Ohne festen Klang
begann ein gemeinsamer Neubeginn kahl: Keiner der zehn in einer frühen dorischen Messe hatte eine
Terz, zwei setzten alle Stimmen auf eine Tonklasse, weil TEX-TRIAD den ersten Takt überspringt und
FREE-ENTRY eine Terz bepreist. In den vier Messen unten sind alle 53 Anfangsklänge vollständig, vor
wie nach der Neueichung.

### Längen

Die Längen der kurzen Abschnitte (MASS-LENGTH, `--lengths`):

| `--lengths` | Punkte |
|---|---|
| `palestrina` (Standard) | Benedictus und Agnus Dei I: zwei Punkte mehr, als die Imitation selbst plant (Wiederholungen des kurzen Textes, § 18), also vier und fünf, wo der Text dafür Raum hat und der Teil imitiert |
| `natürlich` | die Punkte, die die Imitation selbst plant (§ 18) |

Grund: Ein erzeugtes Benedictus hat mit seinen eigenen Punkten dreistimmig etwa 25 bis 35 Takte, das
Benedictus von Palestrinas vierstimmigen Messen im Median 67 bis 68; ein Agnus Dei I hat 35 bis 42
Takte gegen 78. In den vier Messen (Seed 1, mit den Noten nach der Neueichung) hat das Benedictus
mit zwei Punkten mehr 46, 51, 45 und 51 Takte (Missa brevis: 76), das Agnus Dei I 59, 64, 67 und 63
(Missa brevis: 94; mit den Noten vor der Neueichung 60, 62, 67 und 65). Eine frühe Messung fand
jeden verlängerten Teil mit dem ersten Seed, für etwa 6 % der Zeit einer Messe. Findet ein
verlängerter Teil mit keinem seiner drei Seeds einen Satz, wird er mit den eigenen Punkten gesetzt,
mit der Warnung „Teil benedictus/1: mit zwei Imitationspunkten mehr nicht zu setzen; gesetzt mit den
Punkten, die seine Imitation selbst plant (wie --lengths natürlich)“. Mit `--repeat keine` behalten
Benedictus und Agnus Dei I ihre eigenen Punkte, denn mehr Punkte wiederholen den Text.

Die Sätze der vier Messen gegen die Missa brevis (Takte zu 2/2; Mittel der vier, nach der
Neueichung; mit den Noten vor der Neueichung Gloria 215 und Agnus Dei 111, die übrigen gleich):

| | Kyrie | Gloria | Credo | Sanctus | Benedictus mit Osanna | Agnus Dei |
|---|---|---|---|---|---|---|
| erzeugt | 92 | 217 | 412 | 67 | 73 | 107 |
| Missa brevis | 118 | 156 | 292 | 96 | 100 | 208 |

### Kopfmotiv und Choral

Zwei Optionen geben den Sätzen einer Messe gemeinsames Material: das **Kopfmotiv** (MASS-MOTIF,
`--motif an`), mit dessen Tönen Gloria, Credo, Sanctus und Agnus Dei I beginnen wie das Kyrie I,
und die **Paraphrase des Chorals** (MASS-CHANT, `--chant ix|iv`), in der die Soggetti den Choral
des Kyriale singen. Beide sind standardmäßig aus, und sie schließen einander aus, denn auf dem
Choral beginnt jeder Satz mit dem Kopf seines eigenen Gesangs. `--motif an --chant ix` scheitert
deshalb vor der Suche („Kopfmotiv und Choralparaphrase schließen einander aus.“), ebenso das
Kopfmotiv ohne das Kyrie („Das Kopfmotiv kommt aus dem Kyrie I; dazu gehört das Kyrie.“). Keine der
beiden ändert eine Ziehung des Plans: Schlussstufen, Neubeginn, Ketten und Seeds bleiben die der
Messe ohne sie (Test `motif_and_chant_change_no_draw`). Mit dem Kopfmotiv singen so bei demselben
Seed das Kyrie I und jeder Teil ohne Motiv dieselben Noten wie ohne. Nur eine Messe auf dem Choral
ohne `--mode` plant anders, in den Modi ihrer Gesänge (unten). In der grafischen Oberfläche und der
Web-Version wählt man beides im Formular der Messe: den Choral, „nach dem Choral“ als Modus und das
Kopfmotiv als Kästchen.

**Das Kopfmotiv.** In den meisten Messen Palestrinas beginnen Gloria, Credo, Sanctus und Agnus Dei I
mit dem Soggetto des Kyrie I, die Töne gleich, der Rhythmus nach den neuen Worten (Befund
MASS-HEAD-MOTIF, nachgezählt): Die Eröffnungen zweier Sätze derselben Messe teilen ein Soggetto in
73,8 % der Paare, die zweier Messen im selben Modus in 10,4 %; alle vier späteren Sätze nehmen das
Kyrie I auf in 57 von 94 Messen (61 %), in Parodiemessen 35 von 47, in Paraphrasemessen 16 von 29,
in freien Messen keiner von 6, und die Missa brevis tut es nicht. Darum ist das Kopfmotiv
standardmäßig aus: Die Form brevis folgt ihr.
- **Woher:** Das Motiv ist das Soggetto des ersten Punktes des Kyrie I, wie es der Plan hatte, mit
  dem der Teil gesetzt wurde, mit allen seinen Tönen, auch wo der Führer weniger davon singt
  (gekürzte Führer, § 18). Ein Ton, der sich sofort wiederholt, zählt einmal. Ein Motiv hat 5 bis 10
  Töne; ein Soggetto mit weniger oder mehr gibt keines. Es wird als Stufen über der Finalis
  gehalten, mit ihren Vorzeichen gegen die Tonleiter des Modus, der erste Ton in der Oktave über der
  Finalis: `0,2,3,4,5b,4` ist dorisch D F G A B♭ A, das B♭ etwa eines „una nota super la“, das das
  Kyrie I sang.
- **Wer es singt:** der erste Teil von Gloria, Credo, Sanctus und Agnus Dei I, wenn er imitierend
  beginnt (mit `--imitation auto` also nicht zwei- und sechsstimmig, und nicht nach
  `--imitation gloria/1=aus`). Sein erster Punkt nimmt das Motiv; alle späteren Punkte und alle
  übrigen Teile nehmen ihre Soggetti aus dem Text, auch Christe und Kyrie II, Benedictus und
  Osanna II, Agnus Dei II und in der Form normal die Abschnitte nach dem ersten eines Satzes.
  `fux mass --plan` markiert die Teile mit dem Motiv: „gloria:1-5 (Imitation, Kopfmotiv) → A“.
- **Wie:** Der Kopf entsteht auf dem Weg eines Chorals (§ 18, [Choral](#choral)): Die Deklamation
  (Schritt 1 unter [Soggetto bauen](#soggetto-bauen)) singt jeden Ton des Motivs in seiner Folge,
  ohne Durchgang dazwischen, und darf für Silben höchstens zwei Töne neu anschlagen
  (`MOTIF_REPEATS`); den Rhythmus geben die neuen Worte. Die Tonhöhen behält der Kopf mit ihren
  Vorzeichen, wie das Kyrie I sie sang, so wie einen eigenen Kopf (§ 18), in der Oktave seiner
  Stimme. Antworten, Probe der Engführung und Einsätze plant der Planer wie für jedes Soggetto.
- **Reihenfolge:** Das Kyrie I wird vor den Teilen mit dem Motiv gesetzt. Setzt `fux mass` einen
  Teil nach dem anderen (`--jobs 1`), ist das die Reihenfolge der Messe. Mit mehreren Teilen
  zugleich (in `fux mass` und der grafischen Oberfläche ohne `--jobs` meist, *Teile zugleich* unter
  [Messe erzeugen](#messe-erzeugen)) und in der Web-Version beginnt das Kyrie I zuerst, und die
  Teile mit dem Motiv warten, bis es gesetzt oder gescheitert ist.
- **Ausweichen**, jedes mit Warnung im Bericht und in `NAME.txt`:
  - Findet sich kein Kopf, der das Motiv auf die neuen Worte singt und die Proben besteht, nimmt der
    erste Punkt sein Soggetto aus dem Text: „Gloria: Mit dem Kopfmotiv fand der erste Punkt keinen
    Plan; sein Soggetto kommt aus dem Text“.
  - Findet kein Versuch einen Satz mit dem Motiv (drei wie geplant, für das verlängerte Agnus Dei I
    drei weitere mit den eigenen Punkten), setzen ihn als letztes Ausweichen drei weitere Versuche
    mit einem Soggetto aus dem Text, mit derselben Warnung: Das Motiv ist das, worum die Anfrage
    bittet, und wird zuletzt aufgegeben.
  - Gibt das Kyrie I kein Motiv, weil es gemeinsam beginnt (`--imitation kyrie=aus`), fehlt oder
    sein erstes Soggetto nicht 5 bis 10 Töne hat, beginnen die Teile mit einem Soggetto aus dem
    Text: „Agnus Dei: Das Kyrie I gab kein Kopfmotiv (es beginnt gemeinsam, fehlt, oder sein erstes
    Soggetto hat nicht 5 bis 10 Töne); der erste Punkt nimmt sein Soggetto aus dem Text“. Die Messe
    wird deshalb nicht abgewiesen.
  - Beginnt der erste Teil von Gloria, Credo, Sanctus oder Agnus Dei I selbst gemeinsam (zwei- und
    sechsstimmig, weil `auto` dort nicht imitiert, oder nach `--imitation gloria/1=aus`), trägt
    der Satz kein Motiv und sagt es: „Gloria: Der Satz beginnt gemeinsam und trägt deshalb kein
    Kopfmotiv; nur ein Anfang in Imitation nimmt es auf“.
- **Dateien und Fortsetzen:** Das Feld `fux-part` des Kyrie I und jedes Teils mit dem Motiv hält es
  (`motif=0,1,-1,0,1,2,0` vor `profile=`), `fux-mass` jeder Messe mit `--motif an` das Kopfmotiv
  (`motif=1`), gleich, welche Sätze ein Lauf setzt und ob einer ihrer Teile das Motiv nimmt; beides
  steht nur da, wo es gilt, so dass die Dateien einer Messe ohne Kopfmotiv bleiben, wie sie waren.
  So nennen die Dateien jedes Laufs dieselbe Messe, und die Datei des Kyrie I hält das Motiv auch
  nach einem Lauf nur mit dem Kyrie: Sätze, die ein späteres `--resume` dazunimmt, beginnen mit ihm.
  Aus der Datei des Kyrie I, die keine Soggetti hält, nehmen `--resume` und `--only` das Motiv
  wieder; die Web-Version gibt es aus der Antwort des Kyrie I an die wartenden Teile weiter. Deshalb
  brauchen `--resume` und `--only` für die Teile einer Messe mit Kopfmotiv das Kyrie in
  `--movements`, dessen Teile aus ihren Dateien kommen; sonst scheitern sie vor der Suche („Die
  Messe in out/messe.teile hat ein Kopfmotiv, und das kommt aus dem Kyrie I; dazu gehört das Kyrie
  in --movements. …“). Ein Teil aus seiner Datei muss das Motiv genommen haben, das das Kyrie I
  gibt. Sonst setzt `--resume` ihn neu, auch in einem Satz, dessen Teile alle vorliegen, statt wie
  bei anderen abweichenden Angaben nichts zu setzen, denn keine Angabe machte ihn passend; mit
  `--only` ohne `--resume` fehlt er. Das gilt für einen Teil, der sonst passt: Beginnt er nach den
  Angaben anders als in seiner Datei (`--imitation agnus/1=aus`), weicht er in der Imitation ab
  („agnus/1 (Imitation)“), und `--resume` setzt nichts wie bei jeder anderen abweichenden Angabe.
  Wird das Kyrie I neu gesetzt, weil seine Datei fehlt oder nicht passt oder `--only` es nennt,
  setzt `fux mass` auch jeden Teil mit dem Motiv neu, selbst wenn das neue Kyrie I dasselbe Motiv
  gibt. Nahm der erste Punkt eines Teils sein Soggetto aus dem Text statt aus dem Motiv, nennt
  `fux-part` ihn (`fell=1` nach `points=`), so dass der Teil aus seiner Datei so warnt wie beim
  Setzen.
- **In der grafischen Oberfläche und der Web-Version:** Im Fenster setzt „Teil neu setzen“ am
  Kyrie I die Teile mit dem Motiv mit, jeden mit seinem Seed, wie `--only kyrie/1 --seed N`, und
  „Fehlende Teile setzen“ ebenso, wenn die Suche des Kyrie I abgebrochen wurde oder noch nicht lief
  (einen gescheiterten Teil setzt es nicht noch einmal mit demselben Seed). Ein Teil mit dem
  Motiv, den ein solcher Lauf nicht neu setzt (abgebrochen oder gescheitert) und der ein anderes
  Motiv singt, als das Kyrie I nun gibt, fehlt danach, statt das alte zu behalten. Findet keiner
  der Versuche ein neues Kyrie I, behält das Fenster das alte wie jeden Teil, und die Teile mit dem
  Motiv kommen mit seinem Motiv unverändert wieder; `fux mass` setzt sie dann ohne Motiv, und das
  nächste `--resume` holt es nach. Die Web-Version setzt keinen Teil mit eigenem Seed; sie setzt
  eine Messe nur aus Teilen zusammen, die das Motiv des Kyrie I genommen haben, und weist einen
  anderen ab („Teil agnus/1: Die Datei wurde mit einem anderen Kopfmotiv gesetzt, als das Kyrie I
  gibt.“). Stürzt dort der Worker des Kyrie I ab, warten die Teile mit dem Motiv weiter, und
  „Fehlende Teile setzen“ setzt sie mit dem Kyrie I, statt ohne Motiv. In beiden markiert
  die Liste der Teile die Teile mit dem Motiv und das Kyrie I, das es gibt (die grafische
  Oberfläche nennt seine Töne, „Kopfmotiv D E F E D H A“), oder sagt „kein Kopfmotiv“, und sie
  zählt die Warnungen eines Teils.
- **Gemessen** (Release, 8 Kerne; Test `a_missa_brevis_with_the_head_motif`, mit den Noten vor der
  Neueichung; im Release-Lauf nach der Neueichung grün, ohne neue Zahlen; seine 861 s dort schließen
  das Warten auf die Messen ein, die die Tests von `tests/mass.rs` nacheinander setzen): Die
  dorische Missa brevis mit Seed 1 für S A T B nimmt das Motiv `0,1,-1,0,1,2,0` (D E C D E F D,
  sieben Töne) aus dem Kyrie I, und alle vier Teile beginnen mit ihm. 26 von 26 Teilen, jeder mit
  dem ersten Versuch, keine Warnung, 941 Takte und Kosten 2631 (ohne Kopfmotiv 934 und 2581), 576 s
  unter Last 5 bis 12, auf eine leere Maschine umgerechnet 439 s wie ohne Kopfmotiv.

**Der Choral.** `--chant ix` paraphrasiert die Messe IX *Cum jubilo* (Kyrie, Gloria, Sanctus mit
Benedictus, Agnus Dei) mit dem Credo I, `--chant iv` das Kyrie IV *Cunctipotens genitor Deus* und in
den übrigen Sätzen dieselben Gesänge wie IX, mit der Warnung „Choral IV: Das Kyriale hat nur ein
Kyrie IV; die übrigen Sätze paraphrasieren die Choräle der Messe IX und das Credo I“. Es sind die
sechs Gesänge des Kyriale, die fux-core einbettet und über die Palestrina Messen schrieb (§ 18,
[Choral](#choral)).
- **Welche Teile:** Jeder Teil, der imitierend beginnt, nimmt die Soggetti seiner Punkte aus dem
  Gesang seines Abschnitts, wie `fux free --subject chant`: Ein Punkt nimmt die Töne ab der Stelle,
  an der der Gesang seine Kopfworte singt (§ 18). Teile, die gemeinsam beginnen, haben keine Punkte
  und singen keinen Choral.
- **Fortlaufend:** Ein Teil, der mitten in einem langen Abschnitt beginnt, nach einer Naht oder
  einem Halteklang in Gloria oder Credo, nimmt den Gesang erst ab der Stelle, an der dieser die
  ersten Worte des Teils singt. So finden seine Punkte ihre Töne dort und nicht an einer früheren
  Stelle mit denselben Worten, und der Choral läuft durch das ganze Credo, statt in jedem Teil von
  vorn zu beginnen. Agnus Dei II und das Osanna II nehmen ihre Worte von der letzten Stelle im
  Gesang, wie das Kyrie II die letzte Anrufung (Wagner S. 446).
- **Modus:** Ohne `--mode` steht jeder Satz im Modus, nach dem sein Gesang nummeriert ist: das
  Kyrie dorisch (Kyrie IX und IV im 1. Modus), das Gloria mixolydisch (7.), das Credo hypophrygisch
  (4.), Sanctus, Benedictus und Agnus Dei im 5. Modus, mit seinem B♭ als ionisch auf F gelesen
  („ionisch, transponiert nach F“). Die erste Zeile des Plans nennt dann „Modus je Satz“ und jede
  Zeile eines Satzes seinen Modus („Credo (hypophrygisch): …“). Jeder Satz zieht Schlussstufen und
  Kette in seinem Modus und seine Hauptstufe mit derselben Zufallszahl der Messe; die erste Zeile
  nennt die des ersten Satzes. Mit `--mode` steht die ganze Messe in einem Modus, und ein Gesang mit
  einer anderen Finalis wird von der Finalis der Messe aus gesungen, wie in § 18, mit einer Warnung
  je Satz: „Gloria: Der Choral hat die Finalis G, die Messe D; seine Stufen werden von D aus
  gesungen“. Grund: Palestrinas Choralmessen wechseln die Finalis mit ihren Gesängen. Von 101 Messen
  enden 82 jeden Satz auf demselben Basston; unter den Ausnahmen sind vier Choralmessen (die beiden
  *De beata Virgine* von 1567 und 1570, *In majoribus* und *In minoribus duplicibus*; Befund
  MASS-MODE-UNITY).
- **Ausweichen:** Gibt die Phrase eines Punktes keinen brauchbaren Kopf, nimmt er sein Soggetto aus
  dem Text, mit der Warnung von § 18: „credo/1: Punkt 4: kein brauchbarer Kopf aus dem Choral;
  Soggetto aus dem Text“.
- **Dateien und Plan:** `fux-mass` hält den Choral (`chant=ix`); `--resume` und `--only` nehmen ihn
  mit Form, Modus, Chor und Seed aus den Dateien. `fux-part` nennt die Punkte, die ihr Soggetto aus
  dem Text nahmen (`fell=4` nach `points=`, nur wo es solche gibt), so dass ein Teil aus seiner
  Datei warnt wie beim Setzen, mit `--resume`, `--only` und in der Web-Version. Die Warnungen des
  Chorals kommen mit den Ergebnissen, im Bericht und in `NAME.txt` je Satz einmal, Choral IV einmal
  je Messe. `--plan` nennt statt ihrer die Modi der Sätze und die Gesänge: „Choral: Kyrie IX,
  Gloria IX, Credo I, Sanctus IX, Agnus Dei IX“.
- **Gemessen** (Test `a_missa_brevis_on_chant_ix`, mit den Noten vor der Neueichung; im Release-Lauf
  nach der Neueichung grün, ohne neue Zahlen; seine 1153 s dort schließen das Warten auf die Messen
  ein, die die Tests von `tests/mass.rs` nacheinander setzen): Die Missa brevis auf Choral IX ohne
  `--mode`, Seed 1, S A T B, steht in den Modi ihrer Gesänge: 26 von 26 Teilen, jeder mit dem ersten
  Versuch, 960 Takte, Kosten 3281. Von den 32 Punkten ihrer zehn imitierenden Teile nehmen 30 ihr
  Soggetto aus dem Choral, zwei aus dem Text (credo/1, Punkt 4; sanctus/1, Punkt 3). 667 s unter
  Last 17 bis 39, auf eine leere Maschine umgerechnet 351 s.

### Regeln der Messe

| ID | Regel | Standard | Entschieden | Quelle |
|---|---|---|---|---|
| MASS-FORM | Satzformen der Messe (Tabelle oben). | brevis (`--preset`) | beim Planen; gilt durch Konstruktion | Korpus (Formen gezählt und nachgezählt); Missa brevis 1570 |
| MASS-SCORING | Besetzung je Abschnitt (Tabelle oben): ein verkleinerter Abschnitt ohne Bass (sechsstimmig auch ohne Tenor 2), Agnus Dei II mit einer Stimme mehr, scheitert es, im Chor der Messe. | Tabelle (`--scoring`) | beim Planen; das Ausweichen beim Setzen | Korpus (Besetzungen gezählt und nachgezählt); Missa brevis: Benedictus S A T |
| MASS-PARTS | Die wenigsten Teile von höchstens 45 Silben, geschnitten an Satzenden, an Halteklängen immer; Übergänge Abschnitt, Halteklang, Naht mit Start 0, 8, 4. | 45 (`--chunk`) | beim Planen | Patrem ganz 756 s, in vier Teilen 61 s; Credo_14: Pause am Halteklang; IMI-REST (Pontio 1595, S. 62–65) |
| MASS-ENDING | Schlussstufe eines Teils (Tabelle oben). | gezogen (`--endings`) | im Plan; in der Suche halten sie die Schlussformel (feste Töne ab dem vorletzten Takt), die Vorausschau auf den Schlussklang einen Ton vor dem letzten Takt und der Schlusston auf dem letzten Taktanfang ein; der Prüfer liest das Ziel am Bass, TEX-FINAL-CHORD `free_degree` schlägt nie an | Befund CAD-SECTION-END mit den Korrekturen je Modus; Zarlino IV.30; Pontio 1588, S. 106 |
| MASS-CADENCES | Binnenschlüsse über den Satz als Kette (oben); Lage und Formeln unverändert. | Kette | wenn der Plan des Teils seine Ziele setzt (Textplan, Planer der Imitation) | Befunde CAD-INTERNAL-DEGREES und CAD-ORDER (nachgezählt auf 2 Punkte); Jeppesen 1939, S. 81 f.; Zarlino III.53, IV.32 |
| MASS-RESTART | Neubeginn nach Abschnitt, Halteklang oder Naht (Tabelle oben); der Anfangsklang ist fest. | Tabelle, an Nähten vorläufig (`--imitation`) | beim Planen; am Tick des Starts | Pontio 1595, S. 61; Jeppesen 1939, S. 257; Befund GC-SECTION-OPENING (Et resurrexit, Et in Spiritum nachgezählt); Missa brevis |
| MASS-LENGTH | Benedictus und Agnus Dei I mit zwei wiederholten Punkten mehr; scheitert das, mit den eigenen. | `palestrina` (`--lengths`) | beim Planen; das Ausweichen beim Setzen | Längen im Korpus (Benedictus vierstimmig 67–68, Agnus Dei I 78 Takte im Median) |
| MASS-MOTIF | Kopfmotiv (oben): Das erste Soggetto des Kyrie I (5 bis 10 Töne) eröffnet, seine Töne neu deklamiert, den ersten Teil von Gloria, Credo, Sanctus und Agnus Dei I, wenn er imitierend beginnt; gelingt das nicht, ein Soggetto aus dem Text, mit Warnung. Nicht mit dem Choral. | aus (`--motif`) | beim Planen, welche Teile es nehmen; das Motiv, sobald das Kyrie I gesetzt ist; der Kopf vor der Suche des Teils, wie jedes Soggetto | Befund MASS-HEAD-MOTIF (nachgezählt: 73,8 % gegen 10,4 %, alle vier Sätze in 57 von 94 Messen; Parodie 35 von 47, Paraphrase 16 von 29, frei 0 von 6, Missa brevis keines); Pontio 1588, S. 155 f. |
| MASS-CHANT | Paraphrase des Kyriale (oben): Jeder imitierende Teil nimmt seine Soggetti aus dem Gesang seines Abschnitts, der Gesang läuft über die Teile fort; ohne `--mode` steht jeder Satz im Modus seines Gesangs. | keiner (`--chant`) | beim Planen (Modi der Sätze); die Köpfe vor der Suche des Teils | Befund MASS-MODE-UNITY (82 von 101 Messen mit einer Finalis, Ausnahmen darunter vier Choralmessen); Wagner 1913, S. 446 |
| MASS-SEED | Je Teil ein Seed, drei Versuche, das tragbare Budget, Seed und Versuch in der Datei des Teils. | aus dem Seed der Messe (`--seed`) | beim Planen und Setzen | Imitation (§ 18): Nativ probt sie sechsmal und verbreitert den Strahl bis aufs Vierfache, in WebAssembly zweimal und aufs Doppelte; derselbe Seed setzt nur mit einem Budget überall denselben Satz |
| IMI-HELD `unplanned_struck`, `unplanned_passing` | Wie `struck` und `passing` (§ 18) in einem Satz ohne Imitationsplan oder mit einem Plan ohne Punkte. | aus; im Profil der Messe V und P3 | wie `struck` und `passing` (§ 18) | Korpus ohne Plan gezählt: 12 von 40 381 und 63 von 14 650 Fällen, dieselben wie mit einem Plan ohne Einsätze |
| IMI-SILENCE `unplanned_thin` | Wie `thin` (§ 18) ohne Imitationsplan: Nach dem ersten Ton jeder Stimme klingt vier- oder sechsstimmig auf einem Taktanfang vor dem letzten Takt genau die Hälfte der Stimmen. | aus; im Profil der Messe P1 | an jedem Taktanfang, sobald jede Stimme gesungen hat und alle dort gesetzt sind | Korpus: 11,4 % (vierstimmig) und 10,1 % (sechsstimmig) der Taktanfänge je Satz |
| TXT-REPEAT (geändert) | Geht der Text eines Teils im nächsten weiter (`Text.continued`), zählt die Wiederholung seines letzten Sinnabschnitts wie jede andere (`long`, P2). | keine neue Stufe | am Anschlag der Wiederholung (§ 17) | Zarlino IV.33, Regel 8; Jeppesen (Gloria und Credo wiederholen selten) |

**Überlagerung der Messe.** Die Teile, die gemeinsam beginnen, haben keinen Imitationsplan, und doch
halten sie Töne über den Taktstrich und pausieren in halber Besetzung wie die imitierenden.
`fux_core::mass::profile` gibt deshalb den Parametern `unplanned_struck`, `unplanned_passing` und
`unplanned_thin` die Stufen von `struck`, `passing` und `thin` des gewählten Profils, bei Fux V, P3
und P1. Im Standardprofil und im Jeppesen-Profil sind sie aus, so dass die Kalibrierungen an Fux und
Palestrina und alle Sätze außerhalb einer Messe gleich bleiben (§ 18). Die Zahlen des Korpus:
- IMI-HELD ohne Plan findet dieselben Fälle wie `struck` und `passing` mit einem Plan, der einen
  Punkt ohne Einsätze hat: 12 von 40 381 dissonanten Anschlägen gegen einen übergebundenen Ton auf
  dem Taktanfang (0,03 %, also Verbot) und 63 von 14 650 danach (0,43 %, P3); Fall für Fall geprüft
  im Test `unplanned_held_and_thin_in_the_corpus`.
- `unplanned_thin` urteilt vom ersten Ton jeder Stimme an, wie die Regel einen Satz ohne Plan sieht:
  in den Sätzen des Korpus vierstimmig an 4968 von 43 488 Taktanfängen (11,4 %), sechsstimmig an
  1907 von 18 789 (10,1 %). Je Abschnitt nach seinem letzten ersten Einsatz gezählt, wie `thin`
  geeicht ist, sind es 10,7 und 9,9 %; ein Satz des Korpus enthält auch die Expositionen seiner
  späteren Abschnitte. Die Anteile, nach denen `unplanned_thin` urteilt (11,4 und 10,1 %), liegen
  beide über 10 %, P1 gilt also nach der [Leitlinie](#leitlinie-für-standardwerte).

Gemessen an den 49 gemeinsam beginnenden Teilen der vier Messen mit den früheren Anteilen des
Neubeginns (.35 und .5), je mit `unplanned_passing` P3 und P2 gesetzt: Alle 49 fanden sich beide
Male mit dem ersten Seed, in 445 und 459 s auf einer leeren Maschine. Mit P3 kamen 2 Fälle auf 217
Gelegenheiten (0,92 %, 0,037 je 100 Stimmen-Takte), mit P2 5 auf 225 (2,22 %, 0,092); Palestrina
ohne Plan: 0,43 % und 0,012. P2 spart also keine Zeit und lässt mehr als doppelt so viele
Dissonanzen zu; P3 bleibt. In den vier Messen mit den Noten nach der Neueichung (`mass_style`)
kommen auf 100 Stimmen-Takte in den Teilen ohne Plan 0,051 Fälle von IMI-HELD (3 in 5852), in den
imitierenden 0,062 (6 in 9639); mit den Noten vor der Neueichung waren es 0,017 (1 in 5820) und
0,072 (7 in 9693).

**`--rule` und die Überlagerung.** Das Profil eines Teils entsteht so: das gewählte Profil, darauf
die Überlagerung, dann `--rule`, dann `--repeat`.
- `--rule IMI-HELD=aus` setzt alle Stufen der Regel auf aus, auch die für Sätze ohne Plan: Die Regel
  schweigt dann auch in den Teilen der Messe.
- `--rule IMI-HELD=P2` (jede Stufe außer aus) setzt `struck` und `passing`, nicht die Parameter ohne
  Plan; diese behalten die Stufe der Überlagerung.
- `--rule IMI-HELD.unplanned_passing=P2` setzt einen Parameter.

Weil die Parameter ohne Plan in jedem mitgelieferten Profil aus sind, wirken `--rule IMI-HELD=aus`
und `--rule IMI-HELD=P2` außerhalb einer Messe wie ohne diese Parameter.
`--rule IMI-HELD.unplanned_passing=P2` schaltet den Parameter dagegen in jedem Befehl ein, auch für
Sätze ohne Imitation und für fremde Dateien (`fux free`, `fux check`).

**Teilenden im zusammengesetzten Abschnitt.** Der Test setzt die Teile eines Abschnitts zu
einer Partitur zusammen (`mass::section_score`): die Teile nacheinander in allen Stimmen des Satzes,
wer im Abschnitt nicht singt, pausierend; die Texte der Teile als ein Text; Marken für jeden
Halteklang und jede Naht (`Score.sections`). Kein solcher Abschnitt darf gegen ein Verbot verstoßen.
Zwei Regeln sehen dort die Enden der Teile:
- Die Schlussstufe jedes Teils außer dem letzten ist im Abschnitt ein **Binnenschluss**. So prüfen
  die Regeln der Binnenschlüsse und TXT-CADENCE auch das Ende eines Teils, und FIC-INNER-CADENCE
  bepreist den Leitton seiner Klausel nicht als erhöhten Ton außerhalb einer Klausel.
- **FIC-SCOPE:** Ab drei Stimmen darf die große Terz der Finalis im ersten und im letzten Klang
  stehen (Fux Fig. 184, 185); im zusammengesetzten Abschnitt gilt das auch für den letzten Klang
  jedes Teils, den alle Stimmen halten, bevor der nächste beginnt. Ohne das verstieß das
  dreistimmige Credo am Ende des Teils credo/11 (`spiritum:8-9`) gegen das Verbot. Nur
  zusammengesetzte Abschnitte haben Marken, so dass sich sonst nichts ändert.

Eine Stimme des Satzes, die im Abschnitt nicht singt (keine Note und kein Text, etwa der Bass im
Benedictus), zählt in FREE-TEXTURE und in der halben Besetzung von IMI-SILENCE (`thin`,
`unplanned_thin`) nicht mit. So zählt der zusammengesetzte Abschnitt seine Stimmen wie seine Teile:
Sonst wäre das dreistimmige Benedictus im vierstimmigen Satz eine halbe Besetzung, sobald eine
seiner drei Stimmen pausiert. Außerhalb einer Messe gibt es keine solche Stimme (auch in keinem der
1156 Sätze des Korpus), so dass sich dort nichts ändert.

### Messe erzeugen

`fux mass` plant die Messe, zeigt den Plan und setzt die Teile, mehrere zugleich (*Teile zugleich*
unten); mit `--jobs 1` einen nach dem anderen in der Reihenfolge der Messe.

- **Optionen** (Standard in Klammern): `--preset brevis|normal` (brevis), `--movements` (alle sechs
  Sätze, kommagetrennt), `--voices 2–6` (4) oder `--parts "S A T B"`, `--mode` (dorisch),
  `--scoring`, `--imitation`, `--endings`, `--chunk` (45), `--lengths` (palestrina), `--motif
  an|aus` (aus) und `--chant keiner|ix|iv` (keiner; [Kopfmotiv und Choral](#kopfmotiv-und-choral)),
  wie bei `fux free` `--repeat`, `--accents`, `--eleison`, `--seed` (sonst zufällig, er wird
  ausgegeben), `--beam` (200), `--noise` (2), `--profile`, `--rule` und `--title` (Missa brevis
  bzw. Missa), dazu die Ausgabe: `-o` (`out/messe`), `-f` (`ly,pdf,midi,musicxml`), `--clefs`,
  `--tempo`, `--program`, `--marks` und für das Notenbild der Messe `--mensurstriche`,
  `--intonation` und `--tacet` ([Notenbild und Ausgabe](#notenbild-und-ausgabe)). Was der Plan nicht
  haben kann, etwa eine unbekannte Stelle, eine Schlussstufe ohne Klausel, ein Teil, den der Satz
  nicht hat, `--chunk 10` oder das Kopfmotiv mit dem Choral, scheitert vor jeder Suche mit einer
  Meldung auf Deutsch oder Englisch.
- **Stellen.** `--imitation`, `--endings` und `--only` nennen einen Teil als `Satz/n`, den n-ten
  Teil eines Satzes von 1 an (`credo/3`), einen Abschnitt mit seinem Namen (`kyrie`, `christe`,
  `kyrie2`, `gloria`, `qui-tollis`, `credo`, `crucifixus`, `spiritum`, `sanctus`, `pleni`, `osanna`,
  `benedictus`, `osanna2`, `agnus`, `agnus3`, soweit die Form ihn hat) oder einen Halteklang, der
  den Teil nennt, der dort beginnt (`incarnatus` oder „Et incarnatus est“, in der Form brevis auch
  `qui-tollis` und `crucifixus`). Ein Abschnitt nennt alle seine Teile, bei `--endings` den letzten;
  bei `--imitation` gilt ein Wert ohne Stelle für alle Teile (`--endings` verlangt eine Stelle), und
  spätere Angaben gehen früheren vor. `--scoring` nennt nur Abschnitte. Das Osanna ut supra der Form
  normal hat keinen eigenen Teil; jede dieser Optionen weist `osanna2` dort mit diesem Hinweis ab.
- **Plan.** `fux mass --plan` zeigt ihn ohne Suche, in Millisekunden: eine Zeile mit Form, Modus,
  Chor, Seed, Zahl der Sätze, Abschnitte, Teile und Silben und der Hauptstufe, eine mit der
  geschätzten Zeit, dann je Satz die Teile mit Neubeginn, Übergang und Schlussstufe, etwa „Credo:
  credo:1-4 (Imitation) → D · credo:5-7 (gemeinsam, Halbe Atem) → D · …“; mit `--details` je Teil
  Stelle, Seed und Schätzung; auf dem Choral nennt er die Modi der Sätze und die Gesänge, mit dem
  Kopfmotiv die Teile, die es nehmen. Die Ziehungen des Plans hängen vom Seed, der Form, dem Modus
  (auf dem Choral ohne `--mode` vom Choral) und dem Chor ab: Die Hauptstufe zieht ein Generator der
  Messe, Schlussstufe, neue Ziehung und Neubeginn je Satz ein eigener Generator in fester Folge über
  seine Teile. So plant ein Satz allein (`--movements credo`) wie in der ganzen Messe, und jeder
  Teil zieht dieselben Zufallszahlen, gleich, was die Optionen anderen Teilen geben.
- **Ein Teil** wird gesucht wie ein freier Satz von `fux free` mit seinem Text, seinen Stimmen,
  seiner Imitation oder seinem Anfangsklang, seiner Schlussstufe, seinem Start und seiner Kette,
  unter dem Profil der Messe und mit dem tragbaren Budget: höchstens sechs Proben und der doppelte
  Strahl, nativ wie im Browser, damit ein Seed überall denselben Teil setzt. Der Seed eines Teils
  hängt vom Seed, der Form, dem Modus und dem Chor der Messe und von der Stelle des Teils ab, nicht
  davon, welche Sätze gesetzt werden. Er ist auch der Startwert der Kette der Binnenschlüsse des
  Teils, so dass Messen mit demselben Seed in anderen Modi oder Chören ihre Binnenschlüsse nicht an
  denselben Zielen wiederholen. Versuch a sucht mit `splitmix64(Seed des Teils + a)`: drei Versuche
  wie geplant, dann für einen verlängerten Teil drei mit den eigenen Punkten, für ein vergrößertes
  Agnus Dei II drei im Chor der Messe und zuletzt für einen Teil mit dem Kopfmotiv drei mit einem
  Soggetto aus dem Text, jedes Ausweichen mit Warnung.
- **Teile zugleich.** `fux mass` setzt mehrere Teile zugleich, `--jobs N`, ohne die Option die
  Hälfte der Threads, die der Rechner bietet (`std::thread::available_parallelism`), mindestens 1
  und höchstens 4; auf acht Kernen also 4. Jeder Teil sucht in einem eigenen Thread, und alle teilen
  sich den Pool der parallelen Suche, denn eine Suche allein nutzt viele Kerne schlecht (mit 2, 4, 8
  und 16 Threads das 1,77-, 2,72-, 2,84- und 3,32-Fache eines Threads). Die Teile beginnen in der
  Reihenfolge des Pools der Web-Version (`MassPlan::pool_order_of`); sobald einer endet, beginnt der
  erste dieser Reihenfolge, der beginnen darf. Zuerst kommt das Kyrie I, wo es das Kopfmotiv gibt
  und unter den Teilen ist, dann bei j Teilen zugleich die j − 1 längsten nach der Schätzung, dann
  die übrigen Satz für Satz in der Reihenfolge der Messe, im Satz den längsten zuerst, und die
  letzten 3 j wieder nach ihrer Länge, von gleichen der frühere in der Messe. Ein Teil mit dem
  Kopfmotiv beginnt erst, wenn das Kyrie I dieses Laufs gesetzt oder gescheitert ist, und nimmt dann
  das Motiv, das die Messe vom Kyrie I hat. `--jobs 1` setzt die Teile wie bisher einen nach dem
  anderen in der Reihenfolge der Messe. Jeder Teil behält seinen Seed, die Teile sind also
  dieselben, Byte für Byte, wie viele auch zugleich laufen (wie im Browser und mit jeder Zahl von
  Threads); ein gescheiterter Teil hält die übrigen nicht auf. Eine vierstimmige Suche braucht etwa
  0,2 GB; die Missa brevis von Beispiel 11 brauchte mit vier Teilen zugleich höchstens 0,62 GB, mit
  dem Programm nach der Neueichung auf leerer Maschine bis 0,92 GB (unten).
- **Dateien.** Jeder fertige Teil wird sofort geschrieben, als MusicXML in `NAME.teile/`
  (`3-02-credo_5-7.musicxml`: Satz, Teil, Text), über eine Zwischendatei, damit ein Abbruch keine
  halbe Datei hinterlässt. Sie trägt neben Text, Binnenschlüssen und Imitationsplan (§ 17, § 18) die
  Felder `fux-mass` (Form, Modus, Chor, Hauptstufe, Seed der Messe, auf dem Choral `chant=`, mit dem
  Kopfmotiv `motif=1`) und `fux-part` (Takte, Stimmen, Übergang, Titel, Schlussstufe, die
  Schlussstufe des Teils davor, von der seine Kette ausgeht (`before=`, leer im ersten Teil eines
  Satzes), Start, Anfangsklang, `cont=1` für einen Text, der weitergeht, Seed, Versuch, Punkte, die
  Punkte, die ihr Soggetto statt aus Choral oder Kopfmotiv aus dem Text nahmen (`fell=`, nur wo es
  solche gibt), für das Kyrie I und die Teile mit dem Kopfmotiv das Motiv, die Nummer des Profils,
  zuletzt die Version des Generators, der den Teil gesetzt hat, `generator=2`; eine Datei ohne die
  Angabe gilt als Version 1) und `fux-lyrics`, die Stelle im Text jeder Silbe: Wiederholt ein Text
  Wörter (San-ctus, San-ctus), ordnet die Prüfung gelesene Silben sonst anders zu als die Suche (das
  Sanctus kostete so 149 statt 139). Die Version steigt mit jeder Änderung des Programms, nach der
  dieselbe Anfrage andere Noten setzt, auch wo Profil und Plan gleich bleiben (Tabellen, Formeln,
  Antrieb der Suche; der Byte-Vergleich `tools/compare_outputs.sh` zeigt es, Konstante
  `fux_core::mass::GENERATOR`).
  Nach dem letzten Teil schreibt `fux mass` die Messe: `NAME.txt` mit allem, was auf der
  Standardausgabe steht, außer den Zeilen „geschrieben:“, je Satz eine MusicXML- und eine
  MIDI-Datei (`NAME-3-credo.musicxml`, `NAME-3-credo.mid`) und zuletzt das Buch, `NAME.ly` und
  `NAME.pdf` ([Notenbild und Ausgabe](#notenbild-und-ausgabe)). MusicXML, MIDI und `NAME.txt`
  entstehen vor LilyPond, so dass ein Fehler von LilyPond nichts verliert; das gilt für jeden
  Befehl.
- **Ausgabe.** Auf der Standardausgabe der Plan, je Teil eine Zeile („[ 1/26] Kyrie · Kyrie I
  (Imitation), S A T B, Schluss D, Seed 7450663617767490811: 34 Takte, Kosten 41, 40,8 s“), mit
  `--details` darunter die Zusammenfassung wie bei `fux free`, je Satz der Kadenzplan mit Takten
  durch den Satz („Kyrie – Kadenzplan: T. 26 D · T. 34 D ‖ T. 60 A · …“; „(Naht)“ nach der
  Schlussstufe vor einer Naht, `|` vor einem Halteklang, `‖` vor einem Abschnitt), die Summe
  („Gesamt: 26 von 26 Teilen, 947 Takte, Kosten 2619, Suchzeit 19:24, keine Warnung“; die Zeiten
  dieser Beispiele unter starker Last; die Suchzeit ist die Summe der Zeiten der Teile, mit
  mehreren Teilen zugleich also länger als die Wanduhr) und die Warnungen. Auf der Fehlerausgabe
  dieselbe Zeile (für einen gescheiterten Teil sein Versuch und warum), sobald der Teil und alle
  Teile vor ihm gesetzt oder gescheitert sind, also in der Reihenfolge der Messe, wie viele Teile
  auch zugleich laufen, die Zeile jedes gesetzten Teils außer dem letzten mit der verbleibenden
  Zeit („– noch etwa 7:36“). Auf einem Terminal steht darunter eine Statuszeile, die sich alle
  0,25 s erneuert: für einen laufenden Teil „Teil 7/26 · Credo · Takt 23/≈40 · noch etwa 3:40“, für
  mehrere jeder mit Stelle und Takt in der Reihenfolge der Messe („kyrie/3 T. 12/≈30 · gloria/2 T.
  4/≈38 · noch etwa 3:40“), höchstens 79 Zeichen breit, so dass sie in ein Terminal von 80 Spalten
  passt; Teile, für die der Platz nicht reicht, zählt sie („+2“). Jeder Teil hat dafür eine
  Kontrolle seiner Suche, so dass sie den Takt jeder Suche für sich nennt. Ein Absturz der Suche
  räumt die Zeile ab. Die verbleibende Zeit kommt aus den Schätzungen des Plans, angepasst an die
  gesetzten Teile, mit den Teilen zugleich wie in der Schätzung des Plans (unten): in der
  Reihenfolge, in der sie beginnen, die Teile mit dem Kopfmotiv erst nach dem Kyrie I.
- **Fortsetzen.** Strg-C beendet das Programm; die gesetzten Teile liegen schon in `NAME.teile/`.
  `fux mass --resume` mit denselben Angaben nimmt jeden Teil wieder, dessen Datei zur Messe passt,
  und setzt die übrigen. Ohne `--seed` nimmt `--resume` Form, Modus, Chor, Seed, Choral und
  Kopfmotiv aus der jüngsten Datei, die eine Messe nennt, alle übrigen Angaben von der Kommandozeile
  ([Notenbild und Ausgabe](#notenbild-und-ausgabe)); planen die Angaben mit `--seed` eine andere
  Messe als die in `NAME.teile/`, setzt `fux mass --resume` ohne `--only` nichts und endet mit
  Rückgabewert 1, denn es ersetzte die Teile dort nach und nach durch die einer anderen Messe; die
  Meldung nennt beide Seeds, wo die beiden Messen sich darin unterscheiden, auch Kopfmotiv und
  Choral („Teile der Messe mit Seed 1 und Kopfmotiv, die --resume mit diesen Angaben (Seed 1, ohne
  Kopfmotiv) durch Teile einer anderen Messe ersetzen würde“), und die Wege weiter. Für die Teile
  einer Messe mit Kopfmotiv brauchen `--resume` und `--only` das Kyrie in `--movements`
  ([Kopfmotiv und Choral](#kopfmotiv-und-choral)). Ebenso setzt `--resume` nichts und endet mit 1,
  wo die übrigen Angaben (`--scoring`, `--imitation`, `--endings`, `--chunk`, `--lengths`,
  `--eleison`, `--profile`, `--rule`, `--repeat`) einen Teil anders planen, als seine Datei ihn
  hält, in einem Satz, dessen Teile alle in Dateien der aufgezeichneten Messe liegen und von denen
  `--only` keinen nennt: Es ersetzte sonst gesetzte Teile durch neue. Das gilt auch für einen Teil,
  den eine andere Version des Generators gesetzt hat; zu ihm passt keine Angabe, denn eine andere
  Version kann aus demselben Plan und Seed andere Noten setzen. Die Meldung nennt jeden solchen
  Teil mit dem, was abweicht (Stimmen, Imitation, Schlussstufe, Regelprofil, Generatorversion,
  Silben, Plan oder Prüfung), und die Wege weiter: die Angaben des Setzens (außer für Teile einer
  anderen Generatorversion), `--only` mit diesen Teilen, `--rewrite`, das die Messe aus den Teilen
  neu schreibt, wie sie sind ([Notenbild und Ausgabe](#notenbild-und-ausgabe)), oder eine neue
  Messe ohne `--resume` oder unter einem anderen `-o`. Fehlt einem Satz ein Teil, setzt
  `--resume` dort jeden Teil neu, dessen Datei nicht passt, wie nach der Abhilfe für einen
  gescheiterten Teil (`--resume --endings …`): Die Teile nach einem neu gesetzten beginnen, wo er
  endet. Ein wiederverwendeter Teil wird mit den Akzenten des Plans gedruckt (`--accents`). Eine
  Datei passt, wenn
  - `fux-mass` gleich ist, Seed, Choral und Kopfmotiv eingeschlossen;
  - `fux-part` zum Plan passt, auch in der Schlussstufe davor;
  - ihr Versuch zum Teil gehört und der Teil die Stimmen und Punkte singt, mit denen dieser Versuch
    ihn setzt (Versuche 0–2 wie geplant, je Ausweichen drei weitere);
  - die Nummer des Profils gleich ist und der Text so geteilt wie im Plan (eleison drei- oder
    viersilbig, `--eleison`);
  - die Version des Generators die des Programms ist (ohne `generator=` Version 1, der Generator,
    bevor die Version festgehalten wurde);
  - ein Teil mit dem Kopfmotiv das Motiv genommen hat, das das Kyrie I gibt; sonst setzt
    `--resume` ihn neu, wenn er im Übrigen passt, auch in einem Satz, dessen Teile alle vorliegen
    ([Kopfmotiv und Choral](#kopfmotiv-und-choral));
  - die Prüfung kein Verbot findet.

  Wird das Kyrie I neu gesetzt, setzt `fux mass` auch die Teile mit dem Kopfmotiv neu. Jede neue
  oder geänderte Stufe im Standardprofil ändert dessen Nummer, so bei der Neueichung TEX-TRIAD
  `free_bare_sixth` und TEX-FINAL-CHORD `free_missing_third` (§ 16), mit denen auch die Version des
  Generators auf 2 stieg: Teile von Messen, die vorher gesetzt wurden, nehmen `--resume` und
  `--only` seither nicht mehr. `--resume` nennt sie mit „Regelprofil, Generatorversion“, denn sie
  passen mit keinen Angaben; `fux mass --rewrite` schreibt eine solche Messe aus ihren Teilen neu.
- **Einzelne Teile.** `--only credo/3,christe` setzt die genannten Teile der aufgezeichneten Messe
  neu, ohne `--seed` mit ihrem eigenen Seed, Byte für Byte wie zuvor, mit `--seed N` mit dem Seed N;
  so gibt `--only` einzelnen Teilen einen anderen Seed. Die übrigen kommen aus ihren Dateien, und wo
  eine fehlt oder nicht passt, fehlt der Teil, außer mit `--resume`. Ohne Dateien einer Messe
  scheitert `--only`. Auch mit `--seed` nimmt `--only` nur Form, Modus, Chor, Seed, Choral und
  Kopfmotiv der Messe aus den Dateien; die übrigen Angaben müssen die des Setzens sein, für
  dieselben Noten auch `--beam` und `--noise`. Nennt `--only` das Kyrie I einer Messe mit
  Kopfmotiv, setzt es auch die Teile mit dem Motiv neu.
- **Rückgabewert:** 0 für eine vollständige Messe, 3, wenn Teile fehlen (dann nennt der Bericht sie
  und wie `--resume` sie nachsetzt), 1 bei einem Fehler.
- **Prüfen eines Teils.** `fux check --musicxml NAME.teile/….musicxml` prüft einen Teil unter dem
  Profil der Messe (mit `--rule` und `--repeat`) und mit dem, was seine Felder sagen:
  `Text.continued` aus `cont=1`, die Silben aus `fux-lyrics`, die Schlussstufe aus `end=` für die
  Zeile des letzten Punktes. Ein Ton, der an einen Ficta-Ton derselben Höhe übergebunden ist, gilt
  wieder als Ficta, wie in der Suche. So wiederholt die Prüfung den Bericht der Suche. Eine Datei
  ohne `fux-part` prüft `fux check` wie bisher; die Datei eines ganzen Satzes Teil für Teil
  ([Messe prüfen](#messe-prüfen)).

`fux free --ending STUFE` setzt einen einzelnen Satz mit der Schlussstufe eines Teils: der Finalis
(Standard), der Quinte oder der Quarte, wo der Modus dort eine Klausel hat. Zweistimmig verbietet
CAD-END jede andere als die Finalis; die Anfrage scheitert dann sofort, statt die Suche in die
Sackgasse laufen zu lassen.

**Garantie.** Jeder fertige Teil
- verstößt gegen kein Verbot des Profils der Messe; die Prüfung des ganzen Teils umfasst die TXT-
  und IMI-Regeln samt Überlagerung;
- singt in jeder Stimme den ganzen Text des Teils, von der ersten Silbe an in seiner Ordnung, mit
  einer Silbe nach jeder Pause und der letzten Silbe auf dem letzten Ton (§ 17);
- bringt jeden geplanten Einsatz und jeden Anfangsklang wie geplant;
- schließt auf seiner Schlussstufe, der Bass auf ihr, jede Stimme im letzten Takt auf ihrer Oktave,
  Quinte oder großen Terz;
- hat vor seinem Start keinen Ton.

Die Messe
- hat die Teile ihres Plans: Der letzte Teil jedes Satzes endet auf der Finalis, jeder andere Teil
  auf der Finalis, der Quinte oder der Quarte mit Klausel;
- hat an jedem Halteklang und jeder Naht in jeder Stimme mindestens eine Halbe Pause, über die kein
  melodisches Paar, keine Überbindung und kein geplanter Einsatz reicht;
- nennt jedes Ausweichen (Längen, Besetzung, Kopfmotiv, Choral) als Warnung im Bericht und in
  `NAME.txt`.

Dass kein zusammengesetzter Abschnitt gegen ein Verbot verstößt, prüft der Test; `fux mass`
prüft es nicht bei jedem Lauf. Findet ein Teil mit keinem Versuch einen Satz, nennt die Meldung den
Teil, seine Worte, den Takt und die Regeln, an denen die letzte Suche scheiterte, und Abhilfen:
einen anderen Seed für diesen Teil (`--only credo/3 --seed N`), den anderen Neubeginn
(`--imitation credo/3=aus`, für einen gemeinsam beginnenden Teil `=an`, auch zwei- und
sechsstimmig), ab drei Stimmen eine andere Schlussstufe, außer für den letzten Teil eines Satzes
(`--endings`), und kleinere Teile, wo sie seinen Text schneiden (`--chunk` mit zwei Dritteln seiner
Silben). Die Messe ist dann unvollständig; die fertigen Teile bleiben und werden geschrieben, nie
durch einen Teil ohne Text oder ohne Prüfung ersetzt.

**Gemessen** wurde in Release auf 8 Kernen.

*Erfolg:* Die Tests in `crates/fux-core/tests/mass.rs` setzen das Kyrie dreistimmig (3 von 3
Teilen), das Credo dreistimmig (12 von 12; zusammengesetzt mit 11 Marken, davon 2 Halteklängen) und
die vierstimmige Missa brevis in vier Modi (dorisch, phrygisch, mixolydisch, ionisch-transponiert,
Seed 1: 4 × 26 von 26 Teilen), zuletzt im Release-Lauf nach der Neueichung (Kyrie, Credo und die
vier Modi grün). Nach der ersten Durchsicht, deren Korrekturen alle Ziehungen und Seeds der Teile
änderten, fand sich jeder Teil mit dem ersten Seed, keiner brauchte ein Ausweichen; nach der
Neueichung ebenso (die Berichte von `fux mass` nennen für jeden der 104 Teile den Seed des Plans).
Jeder besteht alle Orakel: kein Verbot, der Text vollständig und billigst unterlegt, Einsätze und
Anfangsklang wie geplant, der Bass auf der Schlussstufe, nichts vor dem Start, die Prüfung
wiederholt den Bericht, auch über die Felder der Datei. Kein zusammengesetzter Abschnitt (drei- bis
fünfstimmig, mit Nähten und Halteklängen) verstößt gegen ein Verbot. Mit den Noten vor der
Neueichung setzte `fux mass` dieselben vier Messen mit denselben Takten und Kosten (934, 996, 962
und 987 Takte, Kosten 2581, 2500, 3132 und 3057), ohne Warnung, mit Rückgabewert 0, und `fux check`
fand in keiner der 104 Dateien der Teile ein Verbot und für jede die Kosten des Berichts. Nach der
Neueichung setzt `fux mass` die vier Messen mit 947, 991, 949 und 990 Takten, Kosten 2619, 2289,
3192 und 3133, ohne Warnung und mit Rückgabewert 0, und `fux check` gibt aus den 48 Dateien ihrer
Sätze, mit und ohne Mensurstriche, jeden Teil mit den Kosten seines Berichts und ohne Verbot zurück
(`missa_brevis_in_four_modes_written` im Release-Lauf nach der Neueichung). `fux mass --resume`
nimmt alle 26 Teile der dorischen Messe aus ihren Dateien und schreibt dieselben Dateien wie das
Setzen, Byte für Byte (die zwölf Dateien der Sätze und die LilyPond-Quelle des Buchs). Eine frühe
Messung, bevor die Anteile des Neubeginns, die Gewichte der Kette und die Schätzung festgelegt
waren, setzte 187 Teile (die vier Messen, eine dorische mit Seed 2, Gloria und Credo mit `--chunk
55`, die Längen `natürlich`, die gemeinsam beginnenden Teile ohne Überlagerung, alle Nähte einmal
imitierend und einmal gemeinsam), jeden mit dem ersten Seed und ohne Verbot. Mit dem Kopfmotiv
(dorisch) und auf Choral IX (in den Modi der Gesänge) setzen die Tests die Missa brevis mit Seed 1
ebenso vollständig, 26 von 26 Teilen mit dem ersten Versuch ([Kopfmotiv und
Choral](#kopfmotiv-und-choral)).

*Laufzeit:* Eine vierstimmige Missa brevis brauchte vor der schnelleren Suche nach dem Plan (unten)
auf einer leeren Maschine 5½ bis 7½ Minuten, einen Teil nach dem anderen, seitdem etwa die Hälfte.
Beispiel 11 brauchte auf leerer Maschine (nach der Neueichung) einen Teil nach dem anderen 151 bis
155 s, mit vier Teilen zugleich 116 bis 119 s (unten), bei einer Last von 8 bis 16 2:45 bis 3:18 min
und etwa 2¼ min. Die vier Messen des Tests brauchten nach der Neueichung einen Teil nach dem
anderen, auf eine leere Maschine umgerechnet, 185, 191, 189 und 196 s (auf der Wanduhr 193, 203, 204
und 220 s; `mass_timings`, unten). In einem früheren Lauf des Tests, vor der schnelleren Suche und
der Neueichung, liefen die vier Messen neben anderen Arbeiten (Last 10 bis 45) und brauchten auf
eine leere Maschine umgerechnet 439, 391, 337 und 368 s (dorisch, phrygisch, mixolydisch,
ionisch-transponiert), zusammen 25,6 min (umgerechnet über die Rechenzeit eines Vergleichssatzes,
etwa ±15 %; ein zweiter Lauf: 449, 395, 343 und 368 s). Im Mittel der vier: Kyrie 20 s, Gloria 82 s,
Credo 166 s, Sanctus 29 s, Benedictus 16 s, Agnus Dei 71 s. Die vier Messen vor der Durchsicht
brauchten 23,0 min, und `fux mass` setzte sie auf einer fast leeren Maschine in 5:46, 5:18, 5:32 und
5:03 min Suchzeit (mit LilyPond 309 bis 353 s); die Messen seit der Durchsicht imitieren an mehr
Nähten (15 statt 11 der 52), und mit den Noten vor der Neueichung hatten sie zwei schwere Teile
(siehe *WebAssembly* unten). In der frühen Messung, mit den damaligen Anteilen des Neubeginns und
Gewichten der Kette, brauchten die vier Messen 25,4 min. Fünf- und sechsstimmige Messen sind nicht
gemessen; der Plan schätzt eine Missa brevis (vier Modi, Seeds 1 bis 3) fünfstimmig einen Teil nach
dem anderen auf 328 bis 364 s, mit vier Teilen zugleich auf 234 bis 263 s, sechsstimmig in jedem
Modus und mit jedem dieser Seeds auf 449 und 327 s (knapp 7½ und 5½ min).

Die Schätzung des Plans: t = v · (0,037 · N + 0,0033 · N²) Sekunden für N Takte, ×1,2 für einen
Teil, der gemeinsam beginnt; v ist 0,2 für zwei Stimmen, 0,55 für drei, 1 für vier, 2,2 für fünf und
4 für sechs. N ist bei einem gemeinsam beginnenden Teil die Länge, die sein Text plant, und ein Takt
mehr nach einem Start; bei einem imitierenden die automatische Länge seines Textes mal 1,7 (Gloria,
Credo), 1,9 (Kyrie, Christe, Agnus Dei) oder 1,05 (Sanctus, Pleni, Osanna, Benedictus, deren
langsames Tempo schon Raum für die Punkte lässt), dazu 10 Takte je Punkt, den die Längen hinzufügen.
Für den Browser gilt das Fünffache, in Deno bestätigt (*WebAssembly* unten). Auf den 104 Teilen der
vier Messen liegt die Schätzung je Teil im quadratischen Mittel um den Faktor 1,46 daneben und
insgesamt 19 % zu niedrig (1252 s gegen 1536 s), vor allem wegen einiger schwerer Seeds: Der
dorische Teil `spiritum:8-9` brauchte das 3,5-Fache seiner Schätzung, `qui-tollis:9-13` das
4,2-Fache. Vor der Durchsicht waren es ×1,32 und 12 % (1212 s gegen 1380 s); das fünfstimmige Agnus
Dei II brauchte damals im Median das 1,49-Fache seiner Schätzung (jetzt das 1,09-Fache), machte aber
nur 49 der 168 s Fehlbetrag aus, und der größte Einzelfehler war auch damals ein schwerer Seed (der
phrygische Teil `qui-tollis:1-4`, das 3,1-Fache). Die erste Formel schätzte die Teile der frühen
Messung 1,47-mal zu kurz. Diese Zahlen gelten für die frühere Formel v · (0,067 · N + 0,006 · N²).
Die schnellere Suche nach dem Plan (Noten Byte für Byte gleich) setzte die Messe von Beispiel 11 in
165 statt 299 s, im gepaarten Lauf auf derselben Maschine (Faktor 0,551, je Teil 0,47 bis 0,70);
beide Glieder der Formel nahmen den Faktor an. Die Schätzung traf Beispiel 11 vorher mit 293 s gegen
299 s. Die vier Messen von `mass_timings` brauchten danach, mit den Noten vor der Neueichung, auf
eine leere Maschine umgerechnet 12,5 min (Referenz 1,7 statt 3,0 s), geschätzt waren 11,5 min; je
Teil lag die Formel auf den gesetzten Takten ×1,40 im quadratischen Mittel daneben, die beste
Anpassung (a 0,030, b 0,0032) ×1,39. Nach der Neueichung war `mass_timings` im Release-Lauf grün
(1826 s neben den übrigen Tests von `mass.rs`, so lange wie die ganze Datei: Die vier Messen teilt
es mit `missa_brevis_in_four_modes` und `mass_style`, die dorische auch mit `a_missa_brevis`, und
die Datei setzt alle ihre Messen nacheinander, auch Kyrie und Credo a 3, die Messe mit Kopfmotiv und
die über Choral IX; darum enden die drei zusammen). Allein (mit `--nocapture`) brauchten die vier
Messen auf eine leere Maschine umgerechnet 12,7 min (auf der Wanduhr 13,7 min), geschätzt waren
11,5 min (dorisch 185 gegen 173 s, phrygisch 191 gegen 178 s, mixolydisch 189 gegen 165 s,
ionisch-transponiert 196 gegen 173 s); je Teil lag die Formel auf den gesetzten Takten ×1,33 im
quadratischen Mittel daneben, die beste Anpassung (wieder a 0,030, b 0,0032; gemeinsam beginnend
×1,36, dreistimmig ×0,58, fünfstimmig ×2,17) ebenso, die Schätzungen der Pläne je Teil ×1,44. Den
größten Fehler hatte in beiden das mixolydische `gloria:6-9` (gemeinsam, 39 statt 31 Takte): 32 s
gegen 5 s geschätzt, nach der Formel auf seinen 39 Takten 7,8 s.

Mit mehreren Teilen zugleich (*Teile zugleich* oben) schätzt der Plan nativ wie für den Browser die
Zeit des Pools (`MassPlan::pool_seconds`, `fux_cli::mass::native_seconds`): j Teile zugleich als j
Worker, die Teile in der Reihenfolge des Pools, in der `fux mass` sie beginnt, und jeder Teil neben
den anderen 1 + 0,6 · (j − 1)-mal so lang wie allein (`tables::estimate_jobs`, für vier Teile 2,8),
höchstens aber die Summe der Teile, die mit einem Teil zugleich bleibt: Kleine Teile nutzen allein
weniger Threads und gewinnen zugleich mehr (die zweistimmige Messe mit Kopfmotiv aus den Tests
brauchte mit vier Teilen zugleich das 0,65-Fache, 1,8 bis 2,5 statt 2,6 bis 3,8 s, wo der Faktor
allein mehr als die Summe schätzte). Die Schätzung je Teil (`--plan --details`) ist die eines Teils
allein. Geeicht an Beispiel 11, gesetzt mit vier Teilen zugleich und einem nach dem anderen, jeweils
gleich nacheinander auf der Maschine mit acht Kernen und 16 Threads, die andere Arbeit mit ihnen
teilte (nie leer): Bei einer Last von 8 bis 16 brauchten vier zugleich 136 s gegen 165 und 198 s
einen nach dem anderen (das 0,82- und 0,69-Fache), bei einer Last von 30 bis 80 brauchten sie 230
gegen 310 s und 385 gegen 680 s (das 0,74- und 0,57-Fache); jeder Teil brauchte das 2,2- bis
3,3-Fache seiner Zeit allein, und mit diesem Faktor sagt der Pool das Verhältnis auf 0,03 genau
voraus. Mit 2,8 schätzt der Plan 117 statt 161 s, das 0,73-Fache, in der Mitte der Messungen. Auf
leerer Maschine bestätigt (nach der Neueichung, die Teile in der Reihenfolge des Pools): In drei
Paaren, der Reihe nach mit einem, vier, vier, einem, einem und vier Teilen zugleich, jeder Lauf ab
einer Last unter 1, brauchten einer nach dem anderen 151,2, 155,1 und 152,3 s, vier zugleich 118,3,
119,4 und 116,5 s, das 0,78-, 0,77- und 0,76-Fache. Mit S = 161,29 s, der Summe der Schätzungen der
Teile (`MassPlan::estimate`), und P = 41,75 s, dem Pool von vier Workern mit dem Faktor 1
(`pool_seconds(4, 1.0)`), gibt jedes Paar f = ((T4/T1) · S/P − 1)/3: 0,675, 0,658 und 0,652, im
Median 0,658, 9,6 % über 0,6. Der Faktor bleibt also, und mit ihm schätzt der Plan vier zugleich auf
116,9 s, nahe an den gemessenen 116,5 bis 119,4 s, denn S liegt 4 bis 7 % über der Zeit eines Teils
nach dem anderen. Die Rechenzeit war 1228 bis 1244 s mit einem Teil und 1524 bis 1539 s mit vieren
(im Mittel 8 und 13 Threads belegt), der Speicher höchstens 0,65 GB und 0,83 bis 0,92 GB; das Agnus
Dei II brauchte allein 16 bis 17 s, neben den anderen 49 bis 52 s. Die Läufe der ersten Eichung
begannen die Teile noch, wie damals der Pool, mit den längsten (ohne Kopfmotiv); die Schätzung von
Beispiel 11 ist in beiden Reihenfolgen dieselbe (41,8 s mit dem Faktor 1, 117 s mit 2,8). Läufe, in
denen die Last während eines der beiden Läufe stark stieg, zählen nicht (vier zugleich 348 gegen
312 s und 513 gegen 310 s); zwei Teile zugleich (611 s bei Last 50 bis 70) ergaben kein klares Maß,
die Faktoren für zwei und drei sind deshalb nur dazwischen gelegt. Mehr als vier Teile brächten auf
dieser Maschine kaum mehr: Mit vier sind im Mittel 13 der 16 Threads belegt (Rechenzeit 1800 statt
1350 bis 1420 s), einer allein 7 bis 8. In den Läufen der ersten Eichung, mit den Noten vor der
Neueichung, hing die Zeit am fünfstimmigen Agnus Dei II, das allein 37 bis 38 s brauchte (unter
höherer Last 70 bis 186 s; geschätzt 14 s, zwei gescheiterte Suchen) und neben den anderen 128 s
(unter höherer Last 221 bis 382 s): Es beginnt als eines der drei längsten nach der Schätzung
gleich, teilt sich aber die Threads mit den übrigen. Vier Teile zugleich brauchten höchstens 0,62 GB
Speicher, einer nach dem anderen bis 0,51 GB. Alle 39 Dateien des Beispiels (Buch, MusicXML und MIDI
je Satz, die 26 Teile) waren in den Läufen der ersten Eichung mit einem und mit vier Teilen zugleich
Byte für Byte dieselben, in der Reihenfolge der Eichung wie in der des Pools, und dieselben wie vor
dem Setzen mehrerer Teile zugleich, ebenso `NAME.txt` und beide Ausgaben bis auf die Sekunden und
die Zeile der Schätzung; in den sechs Läufen auf leerer Maschine nach der Neueichung ebenso die 26
Teile und die MusicXML-Dateien der sechs Sätze.

*WebAssembly:* Ein Teil rechnet dort ohne Threads etwa 4,6- bis 5,5-mal so lange wie nativ auf 8
Kernen (Deno 2.9.8, unter Last: das Christe dreistimmig 25,5 gegen 4,6 s, der dorische Teil credo/11
970 gegen 210 s, das fünfstimmige Agnus Dei II 277 gegen 54 s; eine frühere Messung am Kyrie I ergab
4,7 bis 5,3). Gegen nativ auf einem Thread braucht WebAssembly nur etwa 1,4-mal so lange (Agnus Dei
II, beide zugleich gestartet: 410 gegen 286 s Rechenzeit); den Rest macht die parallele Suche nativ.
Die Schätzung nimmt weiter das Fünffache (`tables::ESTIMATE_WASM`). Derselbe Seed setzt dort
denselben Teil, Byte für Byte wie nativ (geprüft an vier Teilen, das Agnus Dei II auch nativ mit
einem und mit acht Threads).

Als Grenze für einen Teil im Browser waren vier Minuten gedacht. Vor der schnelleren Suche nach dem
Plan und mit den Noten vor der Neueichung überschritten sie zwei der 104 Teile der vier Messen: der
dorische Teil `spiritum:8-9` (credo/11, 42 Takte, nativ 84 s auf einer leeren Maschine, im Browser
386 bis 462 s, rund sieben Minuten) und das ionische Agnus Dei I (65 Takte, nativ 76 s, 350 bis
418 s). An der Grenze lagen das mixolydische Agnus Dei II (nativ 45 s, 207 bis 248 s) und das
phrygische Sanctus (43 s, 198 bis 237 s). Vor der Durchsicht überschritt nur das damalige
mixolydische Agnus Dei I die Grenze knapp (nativ 47 s). Mit der schnelleren Suche, noch mit den
Noten vor der Neueichung, brauchte credo/11 dort hochgerechnet etwa 200 s. Die Teile der vier Messen
nach der Neueichung sind im Browser nicht gemessen; aus ihren umgerechneten Zeiten hochgerechnet
(`mass_timings`, mit 4,6 bis 5,5 wie oben) bleibt jeder unter der Grenze. Der längste ist der
mixolydische Teil `gloria:6-9` (gloria/2, gemeinsam, 39 Takte, nativ 32 s, im Browser 149 bis 178 s,
nach `mass_timings`, das mit 5,3 rechnet, 172 s), dann das ionische Agnus Dei I (63 Takte, 23 s, 107
bis 128 s) und das phrygische Sanctus (86 Takte, 21 s, 96 bis 115 s); das dorische credo/11 braucht
nativ nur noch 8,5 s. Die Zeit steckt in der Suche mit einem schweren Seed, die den Strahl
verbreitert. Eine Zeitgrenze, nach der der nächste Versuch sucht, machte den Teil von der
Geschwindigkeit des Rechners abhängig, und ein eigenes Budget im Browser kostete die Gleichheit der
Seeds. Darum bleibt es dabei: Die Web-Version setzt ganze Messen mit ihren Workern, ohne Zeitgrenze
je Teil, zeigt je Teil die verstrichene gegen die geschätzte Zeit, und „Abbrechen“ hält jederzeit
an.

**In der grafischen Oberfläche und der Web-Version** (Reiter „Messe“, Bedienung im
[README](../README.md)) plant und setzt dieselbe Bibliothek wie in `fux mass` die Messe, mit
denselben Optionen, Seeds und Teilen. Die grafische Oberfläche setzt die Teile in einem eigenen
Thread, so viele zugleich wie `fux mass` ohne `--jobs` (die Option hat das Formular nicht),
mit Statuszeile (für mehrere Teile wie bei `fux mass`), verbleibender Zeit und Abbruch, der alle
laufenden Suchen anhält, und schreibt beim Export die Dateien der Teile nach `NAME.teile/` und die
Messe wie `fux mass`, so dass `fux mass --resume` eine Messe aus dem Fenster fortsetzt; seine
Schätzung unter dem Formular und im Plan ist die von `fux mass`. Die Web-Version setzt die Teile in
bis zu vier Web Workern nebeneinander: mit dem Kopfmotiv das Kyrie I vor allen, dann bei P Workern
die P − 1 längsten Teile, dann die übrigen Satz für Satz in der Reihenfolge der Messe, im Satz den
längsten zuerst, und die letzten 3 P wieder nach ihrer Länge (`MassPlan::pool_order`, in dessen
Reihenfolge auch `fux mass` und das Fenster mit mehreren Teilen zugleich die Teile beginnen und mit
dem beide Schätzungen rechnen). Jeder Worker plant die Messe selbst (in Millisekunden) und setzt
einen Teil, dessen MusicXML Byte für Byte die Datei ist, die `fux mass` nach `NAME.teile/` schreibt;
auf einen Klick in der Liste der Teile zeigt die Seite diese Datei, sobald der Teil gesetzt ist, mit
seinen Verstößen. Sobald alle Teile eines Satzes gesetzt sind, setzt die Seite sie wie `--resume`
zusammen, geprüft mit den Kosten ihrer Suche, und gibt den Satz als MusicXML und MIDI und die Messe
als LilyPond-Buch aus; einen Teil einer anderen Version des Generators, wie ihn ein Worker mit einem
älteren Modul aus dem Zwischenspeicher des Browsers setzen könnte, weist sie mit beiden Versionen
ab. „Abbrechen“ beendet die laufenden Worker sofort (die Suche im Modul lässt sich nicht
unterbrechen), die gesetzten Teile bleiben. „Fehlende Teile setzen“ setzt in beiden die
abgebrochenen und noch nicht gesetzten Teile mit den Seeds des Plans, in der Web-Version auch einen
Teil, dessen Worker abstürzte; einen gescheiterten nicht, denn mit seinem Seed scheiterte er wieder.
Für ihn nennen Fenster und Seite eine Abhilfe: „Teil neu setzen“ im Fenster, auf der Seite einen
anderen Seed der Messe oder eine andere Wahl in der Tabelle der Abschnitte oder bei der Größe der
Teile.

*Stil:* siehe [Längen](#längen), [Kadenzplan](#kadenzplan) und
[Grenzen der Messe](#grenzen-der-messe-und-abweichungen-von-palestrina).

### Notenbild und Ausgabe

Nach dem letzten Teil schreibt `fux mass` die Messe in den Formaten von `-f` (Standard
`ly,pdf,midi,musicxml`): zuerst `NAME.txt`, dann je Satz eine MusicXML- und eine MIDI-Datei, zuletzt
das Buch, `NAME.ly` und `NAME.pdf`, so dass ein Fehler von LilyPond keine andere Datei verliert. Die
Dateien eines Satzes tragen seine Stelle in der Messe und seinen Namen, `NAME-1-kyrie` bis
`NAME-6-agnus`, auch wenn `--movements` nur einige Sätze setzt. Ein Satz, dem ein Teil fehlt, hat
keine MusicXML- und keine MIDI-Datei; solche Dateien eines Satzes, dem ein Teil fehlt oder den die
Messe nicht setzt, etwa die des Credo einer früheren Messe unter demselben NAME, löscht `fux mass`
in den Formaten, die es schreibt, so dass die Dateien zum Buch passen. Die Teile in `NAME.teile/`
bleiben, ebenso Dateien in anderen Formaten (`NAME-3-credo.mscz`).

**Das Notenbild ändern.** Liegen alle Teile in `NAME.teile/`, schreibt `fux mass --resume` mit
denselben Angaben wie beim Setzen alle Dateien neu, ohne zu suchen, solange die Version des
Generators die beim Setzen ist; das dauert 0,1 s, dazu LilyPond. Anders lauten dürfen die Optionen
des Notenbilds (unten), `--accents`, `--title`, `--lang` und `-f`. So ändert
`fux mass --resume --mensurstriche` mit den Angaben des Setzens nur das Notenbild.
- Form, Modus, Chor, Seed, Choral und Kopfmotiv nimmt `--resume` ohne `--seed` aus den Dateien
  (*Fortsetzen* unter [Messe erzeugen](#messe-erzeugen)), alles andere von der Kommandozeile, auch
  Titel und Akzente: Ohne `--title` trägt die Messe wieder den Titel ihrer Form (Missa brevis bzw.
  Missa).
- Die Dateien halten die Angaben des Plans, des Textes und des Profils nicht fest (`--movements`,
  `--scoring`, `--imitation`, `--endings`, `--chunk`, `--lengths`, `--repeat`, `--eleison`,
  `--profile`, `--rule`). Fehlt eine oder lautet sie anders, so dass ein Teil nicht mehr zu Plan
  und Profil passt, setzt `--resume` nichts und endet mit Rückgabewert 1, wo alle Teile seines
  Satzes in `NAME.teile/` liegen; die Meldung nennt die Teile und was abweicht (*Fortsetzen*).
  Fehlt dem Satz ein Teil, setzt es die Teile neu, die nicht passen, und ersetzt ihre Dateien;
  fehlt `--movements`, setzt es auch die Sätze, die die Messe nicht hatte. Das kann Minuten dauern
  und ändert die Musik.
- Welche Messe die Angaben planen, zeigt vorher `fux mass --resume --plan`; für dieselbe Messe sind
  es die ersten Zeilen von `NAME.txt`.

**Neu schreiben, wie gesetzt (`--rewrite`).** Nach einer Änderung an Regeln, Tabellen, Plan oder
Generator passen die Teile einer fertigen Messe nicht mehr zu Plan, Profil oder Version, und
`--resume` setzt nichts. `fux mass --rewrite` schreibt die Messe aus `NAME.teile/` neu, ohne zu
suchen und ohne Plan und Profil zu vergleichen: Buch, MusicXML und MIDI je Satz und `NAME.txt`. Die
Messe ist die der jüngsten Datei, die eine Messe nennt (wie bei `--resume`), ihre Sätze die, von
denen `NAME.teile/` Teile hat, mit `--movements` eine Auswahl davon (alle sechs gelten wie keine
Angabe). Was ihre Noten bestimmt, kommt aus den Dateien, nicht aus einem Plan des heutigen
Programms, der die Teile anders schneiden, schließen und besetzen oder ihnen andere Seeds geben
kann:
- aus `fux-mass` Form, Modus, Chor, Hauptstufe, Seed, Choral und Kopfmotiv;
- aus der Datei jedes Teils Satz und Nummer (der Name der Datei), Text und Teilung von eleison,
  Übergang mit Abschnitt oder Halteklang, Schlussstufe, Start, Anfangsklang, `cont=`, Stimmen, Seed
  und Versuch der Suche, Punkte, Kopfmotiv, Profil und Version (`fux-part`) und die Noten mit
  Silben, Binnenschlüssen und Imitationsplan; der Seed des Teils, den die Zeilen nennen, folgt aus
  Seed und Versuch der Suche, auch der eigene nach `--only … --seed`;
- je Satz der Modus, die Abschnitte und die Stimmen aus seinen Teilen.

Die Form gibt nur, was keine Ziehung entscheidet: Kennungen und Titel der Abschnitte (über die
Titel in den Dateien), die Namen der Halteklänge, welche Texte ein Abschnitt singt (daran zeigt sich
ein fehlender Teil am Ende eines Satzes), das Osanna ut supra, Intonation und Hinweise, und für ein
Agnus Dei II, dessen Teil im Chor der Messe gesetzt wurde, die Stimmen seines Plans. Einen Teil
von Benedictus oder Agnus Dei I, den ein Ausweichen (Versuch 3 oder später) ohne `points=` setzte,
nimmt es als Ausweichen der Längen (`--lengths palestrina`); gab der Plan einem Agnus Dei I mit
Kopfmotiv keine zusätzlichen Punkte (`--lengths natürlich`, oder `--repeat keine`, unter dem sein
Text keinen Platz für sie hat), wich es in den Versuchen 3–5 mit dem Kopfmotiv aus, und nur seine
Warnung in `NAME.txt` lautet dann anders. Die Dateien der Sätze halten jeden Teil, wie seine
Datei ihn hält, Profil und Version eingeschlossen; Buch, MIDI und Satzdateien einer Messe, die mit
dem heutigen Programm noch passt, sind Byte für Byte die von `--resume` mit den Angaben des
Setzens. Von der Kommandozeile kommen die Optionen des Notenbilds (unten), `-f`, `--title`,
`--lang`, `--accents`, `--details` und das Profil, unter dem die Teile geprüft werden
(`--profile`, `--rule`, `--repeat`): Ihre Kosten in `NAME.txt` sind die, die `fux check` der
Satzdateien mit denselben Angaben meldet. Die Zeile „Text: …“ von `--details` nennt bei jedem Teil
die Wiederholung dieser Prüfung, denn die Dateien halten das `--repeat` des Setzens nicht fest;
mit dem `--repeat` des Setzens sind die Zusammenfassungen die von damals. Hat ein
anderes Regelprofil oder eine andere Version des Generators einen Teil gesetzt, warnt `--rewrite`,
dass diese Kosten von denen beim Setzen abweichen können, und eine zweite Warnung nennt die Teile,
die unter dem heutigen Profil ein Verbot brechen; der Rückgabewert ist trotzdem 0. Die Angaben des
Plans und des Seeds (`--preset`, `--voices`, `--parts`, `--mode`, `--scoring`, `--imitation`,
`--endings`, `--chunk`, `--lengths`, `--motif`, `--chant`, `--eleison`, `--seed`, `--beam`,
`--noise`) und `--plan`, `--only`, `--resume` und `--jobs` weist es ab, wo sie vom Standard
abweichen, denn eine gesetzte Messe ändern sie nicht, und nichts wird gesetzt.

Es scheitert, ohne etwas zu schreiben, wo `NAME.teile/` keine Messe hält, wo einem gewählten Satz
alle Teile oder ein Teil fehlen (eine Nummer oder das Ende des Satzes), wo zwei Dateien der Messe
einen Teil halten (etwa nach einem anderen `--chunk`), wo ein Teil nach einer anderen Schlussstufe
des Teils davor gesetzt wurde, als dessen Datei hält, oder mit einem anderen Kopfmotiv, als das
Kyrie I in den Dateien gibt, wo Übergänge, Titel oder Texte nicht die Abschnitte der Form ergeben,
wo die Teile eines Satzes in Modus, Teilung von eleison oder Stimmen nicht zusammenpassen, wo eine
Datei ihren Teil nicht zurückgibt und wo das Osanna ut supra sein Sanctus nicht hat. Wo ein Weg
weiter führt, nennt ihn die Meldung; passen Übergänge, Titel oder Texte nicht zur Form, passen die
Teile eines Satzes nicht zusammen oder gibt eine Datei ihren Teil nicht zurück, nennt sie nur den
Grund. Halten zwei Dateien einen Teil, gehören die Dateien einer Messe mit anderen Angaben in einen
anderen Ordner; fehlt dem Benedictus ein Sanctus, das in `NAME.teile/` Teile hat, gehört es in
`--movements`. Sonst ist der Weg ein Lauf von `fux mass` mit den übrigen Angaben des Setzens, und
die Meldung nennt seine Sätze (`--movements`), denn die Dateien halten sie nicht fest:
die gewählten, von denen `NAME.teile/` Teile hat, dazu in einer Messe mit Kopfmotiv das Kyrie und
neben dem Benedictus der Form normal das Sanctus, dessen Osanna sein Osanna ut supra wiederholt;
ohne sie setzt `fux mass` die Sätze nicht.
- Wo Version des Generators und Regelprofil jedes Teils dieser Sätze die der Prüfung sind, setzt
  `--resume` die fehlenden Teile und einen Teil mit einem anderen Kopfmotiv; fehlt das Kyrie I
  einer Messe mit Kopfmotiv, setzt es mit ihm jeden Teil mit dem Kopfmotiv neu. Einen Satz ohne
  Teile, auch das Sanctus zu einem Osanna ut supra, setzt `--resume --movements` mit ihm und den
  Sätzen, die `fux mass` neben ihm braucht; deren Teile nimmt es aus ihren Dateien, wo
  `NAME.teile/` sie hat, sonst setzt es sie mit. Setzt der Lauf das Kyrie I neu (es fehlt, oder
  vom Kyrie einer Messe mit Kopfmotiv liegt kein Teil vor), muss das für jeden Teil aller Sätze in
  `NAME.teile/` gelten: Das neue Kyrie I gibt jedem Teil mit dem Kopfmotiv sein Motiv, nach
  einer Änderung womöglich ein anderes als das alte.
- Sonst nähme `--resume` die übrigen Teile nicht aus ihren Dateien, sondern setzte sie neu oder
  setzte nichts, und die Meldung nennt `--only` ohne `--seed` (das den Teilen einen eigenen gäbe)
  mit dem fehlenden Teil oder dem mit einem anderen Kopfmotiv, für einen Satz ohne Teile mit den
  Abschnitten seiner Form. Teile, die nicht passen, lässt `--only`, wie sie sind; es endet dann
  unvollständig mit Rückgabewert 3, und danach schreibt `--rewrite` die Messe. Von Teilen, die am
  Ende eines Satzes fehlen, nennt sie den ersten (`--only credo/7`): Wie viele Teile der Plan von
  heute danach hat, halten die Dateien nicht fest. `fux mass --plan --details` mit denselben
  Angaben nennt sie; ihre Adressen gehören mit in `--only`, sonst nennt das nächste `--rewrite`
  den nächsten fehlenden.
- `--only` plant die Teile, die es setzt, nach den Schlussstufen, die das heutige Programm zieht.
  Damit sie zu den Teilen passen, die es lässt, nennt die Meldung für das Ende von `--endings` die
  Schlussstufen, die deren Dateien halten: die des Teils davor und die, nach der der Teil danach
  gesetzt wurde (etwa `kyrie/1=A;kyrie/2=A`; ein späterer Eintrag gilt vor einem früheren, und
  `--endings` steht nur einmal in der Kommandozeile). Schneidet oder besetzt der Plan von heute
  einen Satz anders, kann `--only` einen Teil nicht so wiederherstellen, dass er zu den übrigen
  passt; dann bleibt, den Satz oder die Messe neu zu setzen.
- Ein Teil mit dem Kopfmotiv nimmt es nur aus der Datei eines Kyrie I, das passt. Nennt die Meldung
  `--only`, wo das Kyrie I fehlt oder nicht passt, nennt sie es mit: für das fehlende Kyrie I und
  ein anderes Kopfmotiv `--only kyrie/1`, für einen fehlenden Teil mit Kopfmotiv oder einen Satz
  ohne Teile, der mit ihm beginnen kann (Gloria, Credo, Sanctus, Agnus Dei), das Kyrie I dazu; wo
  das Kyrie I passt, nimmt `--only` das Motiv aus seiner Datei. Liegt vom Kyrie kein Teil vor, nennt
  sie statt `kyrie/1` seine Abschnitte (`--only kyrie,christe,kyrie2`), auch für das Kyrie selbst.
  Mit dem Kyrie I setzt `--only` jeden Teil mit dem Kopfmotiv in allen Sätzen mit Teilen in
  `NAME.teile/` neu und ersetzt ihre Dateien, jeden mit der Schlussstufe, nach der der Teil danach
  gesetzt wurde; mit weniger Sätzen behielte einer das alte Motiv, und jedes `--rewrite` danach
  scheiterte an einem weiteren.
- Einen Teil nach einer anderen Schlussstufe setzt `--only` mit diesem Teil, den Schlussstufen der
  Dateien und den Angaben neu, mit denen der Teil davor gesetzt wurde, was sich auch geändert hat:
  `--resume` weist ihn mit den Angaben beider Teile ab, denn einer der beiden weicht vom Plan ab,
  und ihr Satz ist vollständig.

`--rewrite` löscht keine Satzdatei: Die Dateien der Sätze, die es nicht schreibt, bleiben, wie sie
sind. Hält `NAME.teile/` keinen Teil eines Satzes, dessen MusicXML- oder MIDI-Datei da ist, schreibt
es die Messe ohne ihn und warnt mit den Namen dieser Dateien, die die letzte Fassung seiner Noten
sein können. Scheitert es, nennt die Meldung sie ebenso, denn `fux mass` ersetzt oder löscht
Satzdateien, wo es sie schreibt (die der Sätze außerhalb seines Plans und der unvollständigen, und
nach einer Änderung ist jeder Satz eines Laufs mit `--only` unvollständig, dessen Teile nicht
passen). Darum nennt jeder Weg mit `fux mass` `-f ly`, sobald eine Satzdatei der Messe da ist: Ohne
MusicXML und MIDI schreibt und löscht `fux mass` keine Satzdatei, und `--rewrite` schreibt danach
die Messe. Eine Satzdatei kann Teile halten, die in `NAME.teile/` fehlen, auch solche, die die
Meldung nicht nennt, denn sie nennt nur den ersten Fehler. Fehlt ein Teil eines Satzes, dessen
Dateien da sind, sagt die Meldung, dass sie ihn, wie er gesetzt wurde, als einzige Fassung halten
können: Ist er neu gesetzt, schreibt `--rewrite` sie neu, und nur eine Kopie an anderem Ort behält
ihn. Das gilt auch, wo `--resume` ihn mit denselben Angaben setzt: Gab ihm `--only … --seed` einen
eigenen Seed, setzt es ihn mit dem des Plans, also anders. Setzt der Weg einen Satz ohne Teile,
schreibt das nächste `--rewrite` seine Dateien aus den neuen Teilen; ohne ihn in `--movements`
bleiben sie. Einen Satz, in dessen erstem Teil das Setzen abbrach, sieht es nicht: Er hat keine
Datei, und die Dateien der Teile halten nicht fest, welche Sätze die Messe hat. Die zweite Zeile von
`NAME.txt` sagt, dass die Messe ohne Suche neu geschrieben ist, statt die Zeit der Suche zu
schätzen, und jede Zeile eines Teils endet mit „aus NAME.teile“, wie bei `--resume`. Beispiel 11
schreibt `--rewrite` aus seinen 26 Teilen in etwa 0,1 s neu (Release-Build, ohne LilyPond). Nur in
`fux mass`, nicht im Fenster und nicht in der Web-Version.

Die Optionen des Notenbilds (Standard in Klammern):
- `--mensurstriche` (aus): Taktstriche nur zwischen den Systemen, im Buch und in MusicXML (unten).
- `--intonation text|aus` (text; auch `off`): die Intonation vor Gloria und Credo, im Buch und in
  MusicXML.
- `--tacet zeigen|verbergen` (zeigen; auch `show`, `hide`): Stimmen, die einen Abschnitt lang
  schweigen, in MusicXML mit ihren Pausen zeigen oder verbergen. Das Buch zeigt ohnehin nur die
  Stimmen, die singen.
- wie bei jedem Befehl `--clefs modern|original` (modern), `--marks an|aus` (an), `--tempo` (72
  Halbe je Minute) und `--program` (53, Choir Aahs).

**Das Buch** (`fux_export::mass_book`):
- Auf der ersten Seite stehen der Titel der Messe (`--title`, sonst Missa brevis bzw. Missa),
  darunter Modus und Chor („dorisch, S A T B“) und rechts „fux-kontrapunkt, Seed N“.
- Jeder Satz beginnt auf einer neuen Seite unter seinem Titel in der Mitte. Darunter steht kursiv,
  was zum Satz gehört: vor Gloria und Credo die Intonation des Priesters nach dem Graduale Romanum
  („Intonatio: Glória in excélsis Deo.“, „Intonatio: Credo in unum Deum,“), die `--intonation aus`
  weglässt, und die Hinweise zur Aufführung: im Kyrie, dass Kyrie I, Christe und Kyrie II je für
  drei Anrufungen stehen (Wiederholung oder Wechsel mit dem Choral nach Ortsbrauch), im Agnus Dei,
  dass Agnus Dei I für die erste und zweite Anrufung gilt und Agnus Dei II (dona nobis pacem) für
  die dritte.
- Jeder Abschnitt ist eine Partitur unter seinem Titel (Kyrie I, Christe, Et in terra pax, …), mit
  den Systemen nur der Stimmen, die in ihm singen: Im vierstimmigen Benedictus stehen drei, im Agnus
  Dei II fünf. Die Stimmen heißen wie im Satz („Tenor 1“, „Tenor 2“), in den folgenden Systemen
  kurz nach ihrer Lage, zwei Stimmen einer Lage nummeriert („T1“, „T2“); den Modus nennt nur die
  erste Seite.
- Die Takte zählen durch den Satz, wie im Kadenzplan und in der [Prüfung](#messe-prüfen): Das
  Christe beginnt mit der Zahl nach dem letzten Takt des Kyrie I.
- Ein Abschnitt, dem ein weiterer folgt, endet mit einem Doppelstrich, der letzte des Satzes mit dem
  Schlussstrich, beide mit Fermaten auf dem Schlussklang.
- Wo ein Teil nach einem Halteklang beginnt, steht dessen Name kursiv über dem ersten Ton, mit dem
  der Teil nach seinem Takt Pause singt (Qui tollis, Et incarnatus est, Crucifixus), so dass er bei
  einem Zeilen- oder Seitenwechsel nach der Pause beim Neubeginn bleibt; eine Naht hat kein
  Zeichen.
- Die Klammern über den Einsätzen (§ 18) zählen die Punkte in jedem Abschnitt von 1 an.
- Ein Abschnitt, dem ein Teil fehlt, steht als ein Takt Pause in den Systemen seiner Stimmen da.
  Darunter steht für jeden fehlenden Teil, warum er fehlt (die Meldung seiner Suche, oder „nicht
  gesetzt, und seine Datei fehlt oder passt nicht zur Messe“), und wie `fux mass --resume` ihn
  nachsetzt. Für die Taktzahlen danach zählt der Abschnitt als ein Takt; seine gesetzten Teile
  liegen in `NAME.teile/`.
- Das Osanna ut supra der Form normal steht mit den Noten des Osanna aus dem Sanctus da, unter
  seinem Titel.
- `--clefs original` setzt Fux' C- und F-Schlüssel, auch zusammen mit Mensurstrichen.
- Das Buch steht in Systemen von 18 pt (`#(set-global-staff-size 18)` vor `\paper`), etwa 6,3 statt
  7,0 mm hoch; einzelne Partituren und das Übungsheft bleiben bei LilyPonds 20 pt, MusicXML bei
  7 mm. Mit den Noten vor der Neueichung hatte Beispiel 11 so 30 statt 35 Seiten, die vier Messen
  unten 32 bis 35 statt 35 bis 39, alle ohne Warnung. Mit den Noten nach der Neueichung hat
  Beispiel 11 32 Seiten, mit Mensurstrichen 30, die vier Messen unten haben 32 bis 35, mit
  Mensurstrichen 30 bis 33 (Release-Lauf), alle ohne Warnung.
- Keine Seite trägt nur ein System (`min-systems-per-page = #2`); sonst stünde das Ende eines
  Satzes mitunter allein auf seiner Seite. Das gilt auch im Übungsheft, in
  jedem Teil mit mehreren Sätzen. Einzelne Partituren setzen es nicht, ebenso wenig ein Teil des
  Übungshefts mit nur einem Satz (eine Gattung und eine Stimmenzahl ab drei): LilyPond hält die
  Zahl auf jeder Seite ein, auch auf der einzigen, und setzte eine Partitur, die in ein System
  passt, in zwei Systeme über die ganze Breite.
- Wie in jeder Partitur von fux-kontrapunkt und im Übungsheft:
  - Zwei Silben eines Worts stehen mindestens einen halben Linienabstand auseinander
    (`LyricHyphen.minimum-distance = #0.5`), so dass der Bindestrich zwischen ihnen Platz hat. Mit
    seinem Standard lässt LilyPond ihn weg, wo zwei Silben enger stehen, im dichten Text von Gloria
    und Credo oft, in einzelnen Partituren etwa vor einer Silbe auf dem Viertel nach einer
    punktierten Note („i son.“ las sich „ison.“).
  - Jede Text- und Ziffernzeile steht mindestens 7 Linienabstände über der Mittellinie des Systems
    darunter (`VerticalAxisGroup.nonstaff-unrelatedstaff-spacing.minimum-distance = #7` im Kontext
    `Lyrics`). Mit LilyPonds Abstand allein standen Nummern der Klammern
    und Ficta dieses Systems in der Textzeile darüber (im Buch von Beispiel 11 auf S. 5, 6 und 34,
    in den Beispielen 03, 04, 06 und 08).
- Die Quelle nennt `\version "2.24.0"`; übersetzt und geprüft ist das Buch nur mit LilyPond 2.26.

**Mensurstriche** (`--mensurstriche`). Wie in Ausgaben der Musik des 16. Jahrhunderts stehen die
Taktstriche nur zwischen den Systemen (LilyPond `measureBarType = "-span|"`); Doppel- und
Schlussstriche gehen durch. Ein Ton, der über einen Taktstrich an denselben Ton gebunden ist, wird
mit ihm als eine Note geschrieben, wo beide zusammen einen Notenwert haben (1, 2, 3, 4, 6, 8, 12, 16
oder 24 Ticks, von der Achtel bis zur punktierten Brevis), Schritt für Schritt eine Kette von
Bindungen entlang: Eine Halbe auf der Taktmitte, an eine Ganze und diese an eine Halbe gebunden,
wird erst zur punktierten Ganzen, dann zur Brevis. Die Silbe der ersten Note und die Klammern der
Einsätze gehen auf die Note, die die zusammengezogenen vertritt; eine Klammer, die so nur noch eine
Note umfasste, entfällt. Eine Bindung in den letzten Ton eines Abschnitts bleibt: Zusammengezogen
stünde seine Fermate am Anfang der Note, einen halben oder ganzen Takt vor denen der übrigen
Stimmen.

Nicht jede Bindung wird so geschrieben. LilyPond bricht eine Zeile nur an einem Taktstrich, den
keine Note überquert, und warnt nicht, wenn ein System keinen solchen hat und über den Rand läuft.
`fux_export::mensur` geht deshalb die Taktstriche jedes Abschnitts der Reihe nach durch und zählt,
an wie vielen in Folge Noten zusammengezogen wurden:
- An einem Taktstrich, den keine Bindung überquert, kann die Zeile brechen; die Folge beginnt neu.
- Nach drei Taktstrichen in Folge mit zusammengezogenen Noten bleiben am nächsten überquerten alle
  Bindungen (K = 4), damit die Zeile dort brechen kann; die Folge beginnt neu.
- An einem überquerten Taktstrich, an dem sich keine Bindung zusammenziehen lässt, bleiben alle; die
  Folge beginnt neu.
- Sonst werden die Bindungen zusammengezogen, die sich zusammenziehen lassen, die übrigen bleiben,
  und die Folge wächst um eins.

Mit K = 4 werden an den erzeugten Beispielen 01, 02, 03, 07 und 08 in
[examples/](../examples/README.md) 106 von 128 Bindungen über Taktstriche zusammengezogen, an allen
acht Beispielen 01–08 157 von 188 (an einer früheren Fassung der Beispiele 104 von 125 und 151 von
177), in den vier Messen mit den Noten vor der Neueichung 82,5 bis 85,0 % (Tabelle unten). Ein
vierstimmiges System fasst im Buch einer Messe mit 18 pt 7 bis 13 Takte, im Median 10; mit 20 pt
waren es 6 bis 12, im Median 9. Ein fünfstimmiges fasst bei beiden Größen 7 bis 13 Takte, im Median
9. Gezählt ist an Beispiel 11 und den vier Messen unten mit den Noten vor der Neueichung, je mit und
ohne Mensurstriche, ohne das letzte System jeder Partitur (vierstimmig 776 Systeme mit 18 pt, 872
mit 20 pt; fünfstimmig 38 und 39); mit Mensurstrichen allein ergeben sich dieselben Spannen und
Mediane. Bei 20 pt stauchte LilyPond im phrygischen Buch mit Mensurstrichen eine Seite, die drei
fünfstimmige Systeme des Agnus Dei II trug, um 3,6 Linienabstände und warnte; mit 18 pt und dem
Mindestabstand der Textzeilen stehen dort zwei Systeme je Seite, und alle zehn Bücher (Beispiel 11
und die vier Messen unten, je mit und ohne Mensurstriche) entstanden mit den Noten vor der
Neueichung ohne Warnung, ebenso mit den Noten nach der Neueichung.

*Vorzeichen:* In LilyPonds Standard gilt ein Vorzeichen bis zum nächsten Taktstrich, auch einem, den
die Mensurstriche im System nicht zeigen. Endet eine zusammengezogene Note mit einer anderen
Alteration als der der Tonleiter in einem späteren Takt, bekommt der nächste Ton derselben Stufe und
Oktave in diesem Takt sein Vorzeichen zur Erinnerung (`?`), wo er die Stufe der Tonleiter singt und
kein Ficta-Ton ist. Das gilt für ein Vorzeichen im System wie für Ficta über ihm; ohne
Mensurstriche zeigt LilyPond dieses Vorzeichen ohnehin, weil die Bindung die Alteration über den
Taktstrich trägt. In den erzeugten Messen sind es Ficta-Töne: In Beispiel 11 mit Mensurstrichen
bekommt so das B♭ auf dem vierten Viertel von T. 104 im Tenor und von T. 200 im Alt des Credo
sein ♭, je nach einem B als Ficta, das über den Taktstrich gehalten ist.

**MusicXML je Satz** (`NAME-3-credo.musicxml`, `fux_export::to_movement_musicxml`). Die Abschnitte
eines Satzes stehen nacheinander in einer Partitur aus allen Stimmen des Satzes, die Takte
durchgezählt.
- **Kopf:** der Titel der Messe als Titel des Werks, Nummer und Titel des Satzes, auf der ersten
  Seite beide und die Zeile „dorisch, S A T B · fux-kontrapunkt, Seed N“; Seite A4, ein System 7 mm
  hoch (40 Zehntel).
- **Stimmen:** eine Klammer um alle Systeme, die Taktstriche durch die Systeme gezogen oder, mit
  `--mensurstriche`, nur zwischen ihnen (`group-barline` `Mensurstrich`). Jede Stimme ist eine
  Chorstimme ihrer Lage (`voice.soprano`, `voice.mezzo-soprano`, `voice.alto`, `voice.tenor`,
  `voice.baritone`, `voice.bass`) mit einem englischen Instrumentnamen, der dazu passt (MuseScore
  nimmt einen bekannten Namen vor dem Klang), auf dem Kanal und mit dem Programm ihrer MIDI-Spur.
  Name und Kurzname der Stimme sind die des Buchs („Tenor 1“, „T1“).
- **Tempo:** das der MIDI-Datei, unsichtbar, als bloßes `<sound tempo="144"/>` (Viertel je Minute,
  zweimal die Halben von `--tempo`) nach den Attributen des ersten Takts der ersten Stimme;
  MuseScore macht daraus ein verborgenes ♩ = 144 und spielt den Satz im Tempo der MIDI-Datei,
  Verovio rechnet seine Zeitkarte damit. In einer `direction` verlangte das Schema eine
  `direction-type`, die das Tempo druckte.
- **Abschnitte:** Jeder Abschnitt nach dem ersten beginnt eine neue Zeile (`new-system`), und jeder,
  auch der einzige eines Satzes wie das Sanctus der Missa brevis, trägt seinen Titel als
  Studierzeichen (`rehearsal`). Am Ende eines Abschnitts stehen ein Doppelstrich und Fermaten auf
  den letzten Tönen, am Ende des Satzes der Schlussstrich. Die Namen der Halteklänge, wie im Buch
  im Takt, in dem der Teil nach seinem Takt Pause singt, und die Intonation stehen kursiv
  (`words`), alles über der obersten Stimme, die im Abschnitt singt. Die Klammern der Einsätze
  zählen je Abschnitt von 1 an, wie im Buch.
- **Pausierende Stimmen:** Eine Stimme, die einen Abschnitt lang schweigt (der Bass im Benedictus,
  Tenor 2 im Agnus Dei I), hat dort in jedem Takt eine Ganztaktpause. Mit `--tacet verbergen` ist
  ihr System in jedem dieser Takte verborgen (`print-object="no"`) und im ersten Takt, in dem sie
  wieder singt, wieder sichtbar. Standard bleibt `zeigen`: Beim Import einer Datei mit verborgenen
  Systemen blendet MuseScore leere Systeme in der ganzen Partitur aus, auch im ersten System. Auch
  Verovio 6.3.0 hilft das nicht: Es liest die Angabe Takt für Takt (das System heißt in MEI
  unsichtbar), zeichnet das System aber doch, mit Linien und Pausen. Die Web-Version zeigt einen
  Satz deshalb Abschnitt für Abschnitt, jeden mit den Stimmen, die in ihm singen, wie das Buch.
- **Felder:** neben `fux-mode` das Feld der Messe, `fux-mass`, und für den N-ten Teil des Satzes
  (von 1 an) `fux-part.N` mit seinen Takten im Satz, `fux-text.N`, `fux-cadences.N` und
  `fux-imitation.N`, im Teil gezählt, und `fux-lyrics.N`, die Stelle im Text jeder seiner Silben.
  Text, Binnenschlüsse und Imitationsplan des ganzen Satzes stehen nicht darin. Aus den Feldern gibt
  `fux check` jeden Teil zurück ([Messe prüfen](#messe-prüfen)).
- **Lesen:** Der Leser von fux-export baut die Abschnitte aus den Feldern der Teile wieder auf (wo
  ein Teil beginnt, einen Abschnitt mit Titel, einen Halteklang mit Namen oder eine Naht) und aus
  dem Notenbild einen Abschnitt, wo kein Teil beginnt (das Osanna ut supra). In einer Datei ohne
  diese Felder beginnt ein Abschnitt, wo im ersten Takt oder nach einem Doppelstrich ein
  Studierzeichen steht, und wenn es einen späteren gibt, auch einer im ersten Takt. fux-kontrapunkt
  behält sie, wenn es zwei oder mehr sind und die Datei kein Feld von fux-kontrapunkt trägt (keines,
  das mit `fux-` beginnt): Ein einzelner Satz, den fux-kontrapunkt geschrieben hat, bleibt eine
  Partitur, welche Studierzeichen und Doppelstriche ein anderes Programm ihm auch hinzufügt.
  `fux render` setzt die Abschnitte einer solchen Datei und der eines Satzes in eine
  LilyPond-Partitur, mit Doppelstrichen, den Titeln der Abschnitte und den Namen der Halteklänge,
  und schließt sie wie einen Satz im Buch, mit Fermaten am Ende jedes Abschnitts, auch des letzten;
  die Datei eines Satzes trägt dort seinen Titel vor dem Modus („Gloria – dorisch“).

MusicXML einer einzelnen Partitur, die von `fux free`, `fux cp` und den übrigen Befehlen und die
Dateien der Teile in `NAME.teile/`, hat dieselben
Stimmen, Seite und dasselbe Tempo wie die Datei eines Satzes, aber keine Abschnitte und einen
eigenen Kopf: den Titel der Partitur als Titel des Werks und auf der ersten Seite, darunter Modus
und Stimmen von oben („dorisch, S A T B“) und, wo der Seed bekannt ist (`fux free`, `fux cp`,
`fux cf`, `fux book`, das Setzen in GUI und Web), die Zeile „fux-kontrapunkt, Seed N“. Die
Kurznamen sind die Abkürzungen der Stimmtypen, zwei Tenöre heißen dort „T“. Eine Partitur aus
einer Stimme, etwa ein Cantus firmus von `fux cf`, hat keine Klammer, wie im PDF: LilyPond druckt
die Klammer um ein einzelnes System nicht. MIDI einer einzelnen Partitur bleibt wie bisher. So
schreibt auch `fux render` MusicXML und MIDI aus der Datei eines Satzes; nur seine
LilyPond-Partitur zeigt die Abschnitte (oben).

**MIDI je Satz** (`NAME-3-credo.mid`, `fux_export::to_movement_midi`), wie jede MIDI-Datei von fux
im Format 1 mit einer Spur je Stimme:
- Die erste Spur heißt nach Messe und Satz („Missa brevis – Gloria“) und trägt an jedem Abschnitt
  eine Marke (FF 06) mit seinem Titel und an jedem Halteklang eine mit seinem Namen, an Nähten
  keine; dazu das Tempo (`--tempo`, 72 Halbe je Minute) und den 2/2-Takt, gleich durch den ganzen
  Satz. Sie endet mit dem letzten Ton des Satzes; eine Marke steht am Anfang ihres Abschnitts oder
  Teils, auch an einem Halteklang, dessen Name im Notenbild erst über dem Neubeginn steht.
- Nach jedem Doppelstrich folgt ein stiller Takt, bevor der nächste Abschnitt beginnt; ein
  Halteklang hat seinen Takt Pause schon. In der dorischen Messe unten beginnen so das Christe bei
  58,33 s und das Kyrie II bei 133,33 s, im Credo Et incarnatus est bei 248,33 s und Crucifixus bei
  288,33 s.
- Jede Stimme klingt auf ihrem Kanal mit dem Programm von `--program` und schweigt, wo sie nicht
  singt. Das Osanna ut supra erklingt als das Osanna des Sanctus.

**MuseScore.** `fux mass` schreibt keine MuseScore-Dateien; MuseScore 4 öffnet die MusicXML-Dateien
der Sätze, mit `mscore -j` alle in einem Lauf. An drei Sätzen der dorischen Messe übernahm es die
Chorstimmen mit ihrem Programm, die Studierzeichen (eingerahmt), Doppelstriche, Fermaten an den
Abschnittsenden, die kursive Intonation, die durchgezählten Takte und die Klammer; Mensurstriche
zieht es durch die Systeme. Verovio 6.3.0, das Notenbild der Web-Version, zeichnet sie richtig: Es
liest `Mensurstrich` als `bar.method="mensur"` und zieht die Taktstriche nur zwischen den Systemen
(geprüft an einem dreistimmigen Agnus Dei). Beispiel 11 in
[examples/](../examples/README.md) ist eine so geschriebene Missa brevis.

**Gemessen** an den vier Messen unter [Messe erzeugen](#messe-erzeugen), aus den Dateien ihrer Teile
geschrieben, wie gesetzt und noch einmal mit `--mensurstriche --tacet verbergen` (LilyPond 2.26; die
ersten fünf Zeilen mit den Noten vor der Neueichung, die Seiten mit 18 pt und dem Mindestabstand der
Textzeilen, „vorher“ mit 20 pt und ohne ihn; die letzten zwei mit den Noten nach der Neueichung, aus
einem Release-Lauf von `missa_brevis_in_four_modes_written`):

| | dorisch | phrygisch | mixolydisch | ionisch-transponiert |
|---|---|---|---|---|
| Takte | 934 | 996 | 962 | 987 |
| Seiten des Buchs | 32 | 35 | 34 | 35 |
| mit Mensurstrichen | 31 | 32 | 32 | 33 |
| vorher, ohne / mit Mensurstrichen | 35 / 34 | 38 / 35 | 37 / 35 | 39 / 37 |
| zusammengezogene Bindungen | 734 von 875 (83,9 %) | 777 von 942 (82,5 %) | 771 von 908 (84,9 %) | 794 von 934 (85,0 %) |
| nach der Neueichung: Takte | 947 | 991 | 949 | 990 |
| Seiten ohne / mit Mensurstrichen | 33 / 31 | 35 / 33 | 32 / 30 | 35 / 32 |

- LilyPond braucht für ein Buch etwa 15 s, mit Mensurstrichen 11 bis 13 s (unter Last; auf ruhiger
  Maschine für Beispiel 11, mit den Noten nach der Neueichung, 6,3 und 5,2 s); alle übrigen Dateien
  schreibt `fux mass` in 0,1 s.
- Alle 48 Dateien der Sätze waren mit den Noten vor der Neueichung gültig nach dem Schema von
  MusicXML 4.0 (W3C, geprüft mit `xmllint`; nach der Neueichung nicht neu geprüft). Die der
  dorischen Messe sind nach der Neueichung zusammen 1,9 MB groß (das Credo 773 kB), ihre
  MIDI-Dateien 50 kB, das Buch 0,5 MB (mit Mensurstrichen 0,4 MB); mit den Noten vor der Neueichung
  1,8 MB, 765 kB und 49 kB.
- `fux check` gibt aus jeder der 48 Dateien jeden ihrer Teile mit den Kosten seiner Suche zurück,
  104 von 104 je Fassung, ohne Verbot ([Messe prüfen](#messe-prüfen)), mit den Noten vor der
  Neueichung wie nach der Neueichung.

### Messe prüfen

**Die Datei eines Satzes.** `fux check --musicxml NAME-3-credo.musicxml` teilt den Satz nach den
Feldern `fux-part.N` in seine Teile (`fux_core::mass::split_movement`) und prüft jeden für sich, wie
die Datei des Teils in `NAME.teile/` (*Prüfen eines Teils* unter [Messe erzeugen](#messe-erzeugen)):
- unter dem Profil der Messe, mit `--rule` und `--repeat`;
- mit seinem Text aus `fux-text.N` (oder dem von `--text`), der nach ihm weitergeht, wo `cont=1` es
  sagt, und den Stellen seiner Silben aus `fux-lyrics.N`;
- mit seinen Binnenschlüssen, seinem Imitationsplan (`--imitation datei|erkennen|aus`, § 18) und
  seiner Schlussstufe;
- mit seinen Ficta-Tönen, auch denen, die an einen Ficta-Ton derselben Höhe übergebunden sind (die
  Datei markiert nur den ersten).

So wiederholt die Prüfung den Bericht der Suche jedes Teils; nur die Takte zählt sie durch den Satz,
wie Buch und Kadenzplan. Die Überschrift eines Teils nennt ihn wie `fux mass` (`Satz/n`), dazu einen
Abschnitt aus einem Teil mit seinem Titel, einen Teil nach einem Halteklang mit dessen Namen und
jeden anderen mit seinem Text, und seine Takte im Satz: „Teil kyrie/2: Christe, Takte 35–78“,
„Teil credo/2: credo:5-7, Takte 59–90“. Eingerückt folgen die Zeilen zu Text und Imitation
(„Punkt 1 („Chri-ste e-lé-i-son.“): Sopran T. 35 A4 · Alt T. 39 D4 (Unterquinte) · …“), die
Verstöße („Takt 39, Viertel 3 („e-“), Sopran: …“) und die Zeile mit den Verboten und Kosten des
Teils, zuletzt die Summe: „Gesamt: 0 Verbot(e), Kosten 178 aus 130 Präferenz(en) in 3 Teilen.“
Der Rückgabewert ist 2, wenn ein Teil gegen ein Verbot verstößt. `--mark` schreibt jeden Teil als
eigene Partitur mit markierten Verstößen, die Takte gezählt wie in der Datei. Die Takte des Osanna
ut supra gehören keinem Teil: Es sind die Noten des Osanna, das im Sanctus geprüft wird.

Passt die Datei nicht mehr zu ihren Feldern, etwa weil in MuseScore Takte eingefügt oder
gestrichen wurden oder eine pausierende Stimme jetzt singt, scheitert die Prüfung mit einer Meldung,
die Feld oder Teil nennt („Teil credo/3: Die Takte 91–128 liegen nicht in den 120 Takten der Datei
nach denen des Teils davor.“). Die Teile müssen die Datei von Takt 1 an lückenlos decken; nach dem
letzten darf nur das Osanna ut supra folgen, wo die Form des Satzes mit ihm endet und sein
Studierzeichen gleich nach den Teilen steht. Sonst nennt die Meldung den letzten Teil: „Teil
kyrie/3: Die Takte der Teile enden mit Takt 82, die der Datei mit Takt 83.“, „Teil benedictus/1:
Die Takte der Teile enden mit Takt 40, und nach ihm beginnt kein Osanna ut supra.“

**Ohne Felder.** Eine Datei ohne die Felder von fux-kontrapunkt, etwa ein Satz aus einem anderen
Programm, prüft `fux check` Abschnitt für Abschnitt, wenn ihr Notenbild zwei oder mehr Abschnitte
zeigt (nach Studierzeichen und Doppelstrichen, wie unter
[Notenbild und Ausgabe](#notenbild-und-ausgabe) beschrieben). Jeder Abschnitt wird für sich geprüft,
mit den Stimmen, die in ihm singen, unter dem gewählten Profil (ohne die Überlagerung der Messe),
mit dem Text, den `fux check` in ihm erkennt oder `--text` nennt, und auf der Finalis schließend;
`--imitation erkennen` sucht in jedem die Punkte. Eine Bindung über den Doppelstrich endet mit dem
Abschnitt, und ein Binnenschluss aus `--cadences`, für den ein Abschnitt keinen Raum hat, entfällt.
Der Bericht zählt die Takte der Datei: „Abschnitt 2: Christe, Takte 35–78“, zuletzt „Gesamt: … in 3
Abschnitten.“ Eine Datei mit höchstens einem Abschnitt prüft `fux check` wie bisher im Ganzen,
ebenso jede Datei mit Feldern von fux-kontrapunkt außer der eines Satzes: die einzelne Partitur von
`fux free` oder `fux cp`, auch wenn ein anderes Programm ihr Studierzeichen nach Doppelstrichen
hinzugefügt hat.

**In der grafischen Oberfläche und der Web-Version** (Reiter „Prüfen“) wird ebenso
geprüft wie mit `fux check`: die Datei eines Satzes Teil für Teil, die Takte durch den Satz gezählt,
mit der Summe der Teile (die grafische Oberfläche zeigt jeden Teil als eigenes Ergebnis, die
Web-Version die Datei als eines); die Datei eines Teils mit dem Profil der Messe und dem, was ihre
Felder sagen (*Prüfen eines Teils* unter [Messe erzeugen](#messe-erzeugen)). In der grafischen
Oberfläche übergibt „Ergebnis übernehmen“ einen vollständigen Satz einer Messe mit den Feldern
seiner Teile an den Reiter „Prüfen“.

### Grenzen der Messe und Abweichungen von Palestrina

Gemessen an den vier Messen mit den Noten nach der Neueichung (Test `mass_style`), gegen Palestrinas
Missa brevis; aus derselben Zählung stammen die Zahlen der erzeugten Messen in den Punkten zur
Deklamation, zur Homophonie, zu den umgangenen Kadenzen und zu IMI-HELD:

| | erzeugt | Missa brevis |
|---|---|---|
| Takte je Silbe in Gloria und Credo | 1,15 (je Messe 1,13–1,19) | 0,82 |
| Töne je Silbe und Stimme in Gloria und Credo | 1,66 (1,61–1,70) | 1,04 (Gloria) und 0,98 (Credo); Palestrinas vierstimmige Messen im Median 1,65 und 1,38 |
| Silbenanfänge in Gloria und Credo nach Jeppesens Klassen A / B / C | 7,7 / 51,0 / 41,3 % | 21,9 / 33,5 / 44,6 % (Gloria, nach Jeppesen) |
| gehaltene Klänge an Halteklängen und Nähten in Gloria und Credo | 5 und 11 | 3 |

Jeppesens Klassen (Style, S. 42–44) teilen die Zeitpunkte, an denen eine Silbe beginnt: A, wo sie in
einer Stimme allein oder in allen zugleich beginnt; B, wo eine Silbe beginnt, während eine andere
klingende Stimme eine andere hält; C, wo verschiedene Silben zugleich beginnen.

- **Nähte statt durchkomponierter Abschnitte.** Gloria und Credo bestehen aus 6 und 12 Teilen; an
  jeder Naht halten alle Stimmen einen Klang und atmen danach. Die Missa brevis hat dort nur ihre
  drei Halteklänge. Einen langen Abschnitt durchzukomponieren, verlangte, die Suche fortzusetzen,
  und dazu zuerst eine Prüfung, die nicht bei jedem Schritt die ganze Partitur liest (siehe
  [Teile langer Abschnitte](#teile-langer-abschnitte)).
- **Langsame, dichte Deklamation.** Gloria und Credo brauchen 1,15 Takte je Silbe gegen 0,82, und
  jede Stimme singt 1,66 Töne je Silbe gegen etwa einen. Diese Dichte haben Palestrinas gewöhnliche
  vierstimmige Messen (im Median 1,65 im Gloria, 1,38 im Credo), nicht seine Missa brevis.
- **Keine Homophonie und keine Auslassungen.** Jede Stimme singt jedes Wort (TXT-ORDER `first` V,
  § 17), und gemeinsame Deklamation plant nur der gemeinsame Anfang eines Teils. Darum beginnen zu
  wenige Silben in einer Stimme allein oder in allen zugleich (A 7,7 gegen 21,9 %). In der Missa
  brevis singen manche Stimmen nicht den ganzen Text (0,98 Töne je Silbe und Stimme im Credo).
- **Keine umgangenen Kadenzen.** Jeder Binnenschluss singt seine Formel vollständig. Palestrina
  umgeht 34 % seiner Binnenschlüsse, 53 %, wo die Stimmen ohne Pause weitergehen. Im Bass liegen 34
  von 180 Binnenschlüssen in Gloria und Credo mitten in einem Wort (19 %).
- **Formelstimmen nach ihrer Stellung** (§ 16, § 18): Die Oberstimme, die Stimme über dem Bass und
  der Bass singen die Klauseln. Palestrinas Diskantklausel liegt vierstimmig nur in 32 % der
  Kadenzen im Cantus.
- **Kein Kanon.** Agnus Dei II bekommt vierstimmig einen freien Tenor (fünfstimmig einen Sopran);
  die Missa brevis fügt einen zweiten Cantus im Kanon hinzu, und 16 von 30 vierstimmigen
  Agnus Dei II mit einer Stimme mehr haben einen strengen Kanon.
- **Kein Dreiertakt.** Alle Teile stehen in 2/2 (Palestrinas 4/2). Im Korpus stehen 113 Abschnitte
  in 3/2, und 53 von 86 Messen haben einen.
- **Phrygischer Schluss.** Ein Teil auf E schließt mit der phrygischen Klausel, der Bass singt die
  Tenorklausel F–E (§ 16). Palestrinas phrygische Abschnitte auf E erreichen den Schlussklang zu
  86 % mit dem Bass A–E, oft nach einer Klausel auf A. Allgemein fehlt die plagale Coda (der Bass
  kommt bei 26 bis 36 % der Schlüsse auf der Finalis von der Quarte, äolisch 48 %), und der
  Schlussklang dauert einen Takt statt meist einer Brevis (93 %).
- **Längen.** Gloria und Credo sind zu lang, Kyrie, Sanctus, Benedictus und Agnus Dei zu kurz
  ([Längen](#längen)).
- **Lange Folgen auf der Finalis im Mixolydischen.** Mixolydisch zieht der Plan für die Teile im
  Inneren eines Abschnitts zu .70 die Finalis ([Kadenzplan](#kadenzplan)), und eine gleiche
  Schlussstufe wird nur einmal neu gezogen: Das mixolydische Credo mit Seed 1 endet fünf Teile in
  Folge auf G (Crucifixus bis `spiritum:5-7`).
- **IMI-HELD selten und schwankend.** In den Teilen ohne Plan kommen 0,051 Dissonanzen gegen
  übergebundene Töne auf 100 Stimmen-Takte (3 Fälle), in den imitierenden 0,062 (6 Fälle), bei
  Palestrina ohne Plan 0,012. Bei so wenigen Fällen schwankt der Anteil von Messe zu Messe: Mit den
  Noten vor der Neueichung waren es 0,017 (1 Fall) gegen 0,072 (7 Fälle), vor der Durchsicht 0,063
  gegen 0,069.
- **Fünf und sechs Stimmen** sind nicht gemessen. Mit sechs Stimmen beginnt fast jeder Teil
  gemeinsam, weil der sechsstimmige Satz mit `auto` nicht imitiert (§ 18); ein Kopfmotiv nimmt dann
  kein Teil, und der Choral klingt nur in den vierstimmigen Abschnitten.
- **Kopfmotiv nur am Anfang.** Das Motiv eröffnet nur den ersten Punkt von Gloria, Credo, Sanctus
  und Agnus Dei I, und nur dort, wo dieser Teil imitierend beginnt. Eine Parodie, die mehr übernimmt
  als den Kopf des Kyrie I, etwa spätere Punkte oder die Musik einer Vorlage, gibt es nicht.
- **Choral nur als Kopf.** Der Choral liefert nur die Köpfe der Punkte imitierender Teile; Teile,
  die gemeinsam beginnen, singen ihn nicht, und keine Stimme trägt ihn in langen Noten (§ 18). Zur
  Wahl stehen nur die Messe IX mit dem Credo I und das Kyrie IV; in der Missa brevis auf Choral IX
  nahmen 2 von 32 Punkten ihr Soggetto aus dem Text.
- **Notenbild.** Die Takte sind halb so lang wie in Palestrinas Gesamtausgabe (2/2 statt 4/2, § 16).
  Die Intonation steht als Text da, nicht in Choralnoten, und kein Abschnitt hat ein Incipit mit den
  alten Schlüsseln und Werten. Das PDF hat keine Lesezeichen.
- **Nicht umgesetzt:** Homophonie und Auslassungen, das
  Fortsetzen der Suche über lange Abschnitte, umgangene Kadenzen und Formelstimmen nach dem Text,
  der Kanon, der Dreiertakt, zwei Soggetti zugleich, die plagale Coda und der phrygische Schluss
  Palestrinas; im Notenbild Choralnoten für die Intonation, Incipits, Takte zu Breven und
  Lesezeichen im PDF.
