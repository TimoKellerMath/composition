# fux-fuge im Browser

Die Seite setzt zu einem Thema eine Fuge im Stil Bachs, für zwei bis vier Stimmen. Sie rechnet im
Browser: Das Programm aus `crates/fux-bach` läuft als WebAssembly-Modul in einem Web Worker, ohne
Server. Sie steht neben der Seite von fux-kontrapunkt (`web/`, `crates/fux-web`,
`tools/build_web.sh`) und folgt deren Aufbau, ändert aber nichts an ihr. Was das Programm kann und
wo seine Grenzen liegen, steht in [`docs/fux-fuge.md`](../docs/fux-fuge.md).

## Was die Seite tut

- **Thema.** Man wählt eines der Themen, die das Modul mitbringt, oder gibt eigene Töne ein, dazu
  Tonart, Takt, Einsatzabstand und Titel. Ein gewähltes Thema zeigt seine Töne in denselben
  Feldern. Wer dort etwas ändert, setzt ein eigenes Thema. „Eigene Töne …“ in der Auswahl holt die
  eigene Eingabe zurück. Die Schreibweise der Töne erklärt die Seite unter dem Formular. Die Felder
  der Schreibweise schalten Großschreibung und Korrektur einer Telefontastatur ab, und
  typografische Apostrophe (’) liest die Seite als Oktavzeichen.
- **Eigene Antwort.** Sie ersetzt die Antwort nach der Regel: so viele Töne wie das Thema, in
  dessen Rhythmus. Sie gehört zum gewählten Thema und wird geleert, wenn man ein anderes wählt.
- **Fuge.** „Stimmen“ wählt zwei, drei oder vier. „Schluss“ wählt Orgelpunkt oder Engführung
  oder überlässt es dem Programm (eine Engführung, wenn das Thema eine hergibt, sonst ein
  Orgelpunkt). „Freie Stimmen“ wählt, wie die Stimmen neben Thema und Kontrasubjekt arbeiten:
  mit Figuren und mit Sequenzen in allen Stimmen der Zwischenspiele (die Fuge wird dafür
  mehrmals gesetzt) oder schlicht aus Rhythmustabellen (ein Satz, schneller). „Klang“
  wählt die Preise der Regeln: nach dem Ohr (Profil `fuge`, die Vorgabe: zusammen
  angeschlagene Dissonanz kostet mehr, drei Stimmen bilden öfter den vollen Dreiklang) oder nach
  der Zählung (Profil `bach`, an Bachs Raten geeicht). „Bass“ wählt, ob die Suche findet, was
  zuunterst liegt, oder das Gerüst je Akkord einen Basston nennt (in einer blinden Hörprobe
  klang beides gleich). „Plan der Einsätze“ nimmt die Einsätze nach der Exposition aus einer
  Fuge von WTK I oder überlässt sie dem Programm. Die Namen der Pläne nennt das Modul.
  „Qualität“ wählt die Breite der Suche (`beam` 200, 400 oder 800). Mehr ist langsamer und
  selten besser.
- **Fassungen.** Unter „Fuge setzen“ steht ein zweiter Weg: mehrere Fassungen derselben Fuge, die
  sich nur in den angekreuzten Aspekten unterscheiden (Freie Stimmen, Klang, Bass, dazu weitere
  Seeds), höchstens acht auf einmal. „Fassungen setzen“ setzt sie nacheinander. Über der Fuge
  steht dann, worin sie sich unterscheiden („Die Fassungen unterscheiden sich in: Klang, Bass.
  Alles andere ist gleich.“), und je Fassung ein Knopf mit ihrem Buchstaben und ihren Werten
  („b“ und „Klang: nach dem Ohr · Bass: geplant“). Ein Klick zeigt die Fassung mit Noten, Bericht,
  Klang und Dateien; die Zeile unter dem Titel und die Dateien nennen sie beim Namen, und die
  Dateinamen tragen den Buchstaben. Mit „blind“ sind die Buchstaben gelost, und die Seite sagt
  erst nach „Auflösen“, worin sich die Fassungen unterscheiden (die Dateien einer blind
  gesetzten Fassung nennen auch danach nur ihren Buchstaben). „Diese Fassung übernehmen“ stellt
  das ganze Formular auf die gezeigte Fassung, mit ihrem Seed, was immer inzwischen darin
  geändert wurde.
- **Seed.** Mit „neuer Seed“ bekommt jede Fuge einen neuen Seed, sonst gilt der im Feld. Derselbe
  Seed setzt dieselbe Fuge. Den Seed der gezeigten Fuge nennt die Zeile unter ihrem Titel.
- **Setzen.** „Fuge setzen“ schickt die Anfrage an das Modul. Mit Figuren braucht eine
  dreistimmige Fuge im Browser etwa 3 bis 15 Sekunden, eine vierstimmige 10 bis 70; schlicht sind
  es 1 bis 4 und 2 bis 9 (siehe [Größen und Zeiten](#größen-und-zeiten)). Solange gerechnet
  wird, läuft die Zeit unter dem
  Knopf, die gezeigten Noten sind blass, vor der ersten Fuge steht an ihrer Stelle „Setze die
  Fuge …“, und „Abbrechen“ beendet den Worker sofort und startet einen neuen. Die Seite bleibt
  bedienbar. Die Zeit misst sie selbst, denn das Modul hat keine Uhr.
- **Ergebnis.** Unter dem Titel stehen Tonart, Takt, Stimmen, Takte, Töne (davon vorgegeben) und
  Seed. Das Notenbild setzt [Verovio](https://www.verovio.org) aus dem MusicXML der Antwort.
  Abgespielt wird mit WebAudio im Orgelklang der Seite von fux-kontrapunkt, im Tempo der Fuge oder
  in einem eigenen. Ein Klick auf einen Abschnitt spielt ab seinem Takt. Herunterladen kann man
  MusicXML, MIDI, LilyPond und die Akkorde des Gerüsts (`.akkorde.txt`, die Datei, die
  `fux-fuge check --chords` liest). Die vorgegebenen Töne sind nur in der LilyPond-Datei markiert.
- **Bericht.** Daneben stehen:
  - die Antwort (real oder tonal) und das Kontrasubjekt,
  - der Schluss (Orgelpunkt, oder Engführung mit dem Abstand ihrer Einsätze in Takten),
  - wie viele der geplanten Einsätze ein Themenfinder in den Noten wiederfindet,
  - die Abschnitte mit ihrem Takt, Einsätze mit ihrer Tonart,
  - die Prüfung in den Worten des Moduls (Verbote, Kosten, Präferenzen), dazu je eine Zeile für
    Verbote, die das vorgegebene Material selbst bricht, und für Verbote an Nahtstellen, aber nur,
    wenn es welche gibt,
  - die Tabelle der Kennzahlen mit Bachs Band. Eine Kennzahl außerhalb ihres Bandes ist markiert,
    ▲ über und ▼ unter dem Band. Darüber steht, wie viele Kennzahlen in den Bändern liegen und ob
    das den Maßstab erreicht.
- **Sprache.** Die Seite ist deutsch, in einem Browser mit anderer Sprache englisch, und lässt
  sich oben umschalten. Beim Umschalten setzt sie die gezeigte Fuge mit demselben Seed noch
  einmal, damit auch die Wörter des Moduls (Titel, Abschnitte, Prüfung, die Wörter in den Dateien)
  in der neuen Sprache stehen.
- **Fehler.** Eine Fehlermeldung des Moduls erscheint als Text in der Statuszeile, die gezeigte
  Fuge bleibt stehen. Ein Beispiel: Wer zur Kunst der Fuge eine Engführung verlangt, liest
  „bwv1080: the subject gives no stretto“. Die Meldungen des Moduls sind englisch. Bricht das
  Modul ab (ein Trap), lädt der Worker ein neues, und die Seite sagt das dazu.

## Bauen

```sh
tools/build_web_fuge.sh                       # baut nach target/web-fuge
python3 -m http.server -d target/web-fuge     # dann http://localhost:8000 öffnen
```

Das Skript arbeitet wie `tools/build_web.sh` und braucht dasselbe: das Ziel
`wasm32-unknown-unknown` (unter Arch Linux das Paket `rust-wasm`, sonst
`rustup target add wasm32-unknown-unknown`), Python 3, gzip und beim ersten Mal curl. Seine
Schritte:

1. Es prüft, dass die Content-Security-Policy in `index.html` den Hash des Inline-Skripts nennt.
2. Es baut `crates/fux-bach-web` mit dem Cargo-Profil `wasm` in das eigene Zielverzeichnis
   `target/web-fuge-build`.
3. Es legt das Modul als `fuge_web.wasm` und seine Kopie `fuge_web.wasm.gz` (`gzip -9 -n`) nach
   `target/web-fuge`.
4. Es kopiert die Seite (`index.html`, `style.css`, `app.js`, `worker.js`, `fuge.js`) dazu.
5. Es legt Verovio 6.3.0 mit beiden Lizenztexten und einer `README.txt` nach
   `target/web-fuge/verovio`. Die Dateien kommen aus `target/verovio-6.3.0`, dem Zwischenspeicher,
   den auch `tools/build_web.sh` benutzt. Fehlen sie dort, lädt das Skript sie von denselben
   Adressen und prüft dieselben SHA-256-Summen.

Die Seite braucht einen Webserver. Von `file://` aus starten Browser weder den Worker noch laden
sie das Modul.

## Testen

```sh
cargo test -p fux-bach-web
tools/build_web_fuge.sh
DENO_DIR=target/deno deno run --allow-read web-fuge/smoke-test.mjs target/web-fuge/fuge_web.wasm
DENO_DIR=target/deno deno run --allow-read web-fuge/page-test.mjs target/web-fuge/fuge_web.wasm
```

Mit Node.js statt Deno:

```sh
node web-fuge/smoke-test.mjs target/web-fuge/fuge_web.wasm
node web-fuge/page-test.mjs target/web-fuge/fuge_web.wasm
```

Beide Tests sind ES-Module ohne Abhängigkeiten. Sie schreiben je Prüfung eine Zeile (`ok` oder
`FAIL`) und enden mit Status 1, wenn eine Prüfung scheitert. In Deno sind es 58 und 89 Prüfungen.
In Node.js entfallen im ersten Test zwei, dort steht dann `skip`. Der erste Test setzt einundzwanzig
Fugen und braucht etwa 70 s, der zweite etwa 45 s. `DENO_DIR` hält den Zwischenspeicher von Deno im Projekt und darf fehlen.

Der Test der Seite nimmt, was er vom Modul erwartet, aus dessen Antworten (Titel, Abschnitte,
Schluss, Einsätze, Prüfung, Bänder, Maßstab, Tempo, Wortlaut der Fehler). Er scheitert also nicht,
wenn das Modul andere Zahlen liefert, sondern wenn die Seite etwas anderes zeigt, als das Modul
antwortet. An Wörtern des Moduls setzt er nur voraus, dass die Prüfung von Verboten spricht, dass
ein Einsatz seine Tonart in Klammern nennt und dass der Fehler zu einem Schluss, den das Thema
nicht hergibt, die Engführung nennt. Der Test des Moduls hält fest, worauf die Seite baut: die
Felder der Antworten, die Abschnitte auf Deutsch und Englisch, derselbe Seed für dieselbe Fuge,
keine Uhr und keine Importe.

**Festgehaltene Fugen.** Der Test des Moduls nennt drei Fugen zum Thema der Kunst der Fuge
(`bwv1080`, Seed 1, `beam` 400) mit ihren Zahlen:

| Anfrage | Stimmen | Takte | Töne | Schluss | Einsätze |
|---|---|---|---|---|---|
| ohne Weiteres | 3 | 40 | 452 | Orgelpunkt | 6 von 6 |
| `voices: 4` | 4 | 44 | 690 | Orgelpunkt | 7 von 7 |
| `plan: "wtk1-02"` | 3 | 60 | 728 | Orgelpunkt | 9 von 9 |

Ändert sich der Komponist, scheitern diese drei Prüfungen und nennen in ihrer Zeile die neuen
Zahlen. Dann sind die Zahlen in `smoke-test.mjs` (dreimal `size(…) === "…"`) und in dieser Tabelle
zu ersetzen.

- **`cargo test -p fux-bach-web`** schickt Anfragen durch die drei Exporte `fuge_input`,
  `fuge_run` und `fuge_output`, wie die Seite es tut, und prüft, dass die Antworten die Felder
  haben, die die Seite zeigt.
- **`smoke-test.mjs`** prüft das gebaute Modul: keine Importe, kein Abschnitt `name`, die
  gzip-Kopie enthält genau dieses Modul, und `Fuge.fetch` lädt es auf jedem Weg (Kopie, vom Server
  entpackte Kopie, ohne Kopie, kaputte Kopie, ohne `DecompressionStream`). Dann `version` und
  `themes` mit Themen und Plänen. Dann Fugen: die drei festgehaltenen, eine mit schlichten
  freien Stimmen (`free_voices: "plain"`), eine mit dem Profil `bach`, eine mit geplantem Bass
  (`bass: "planned"`), eine mit dem Namen einer Fassung (`variant`), eine mit eigener
  Antwort,
  beide Schlüsse zu einem Thema, das eine Engführung hergibt, eine zweistimmige und eine zu
  eigenen Tönen mit Auftakt, auf Englisch. Jede Antwort wird als Ganzes geprüft: Stimmen, Töne zum
  Abspielen, Abschnitte, Schluss, Einsätze, Prüfung, Kennzahlen mit Bändern und Treffern, die vier
  Dateien.
  Dazu derselbe Seed (dieselbe Fuge), ein anderer Seed (eine andere) und neunzehn fehlerhafte
  Anfragen, deren Meldung jeweils nennt, was falsch ist. Das echte Verovio aus
  `target/web-fuge/verovio` setzt das MusicXML von drei Fugen mit den Optionen der Seite, darunter
  die vierstimmige. Der Test zählt im SVG die Takte, die Noten und die Wörter über den
  Abschnitten. `worker.js` läuft im Test mit einem Modul, dessen `fuge_run` abbricht: Die Antwort
  trägt `broken`, und die nächste Anfrage beantwortet ein neu geladenes Modul. Wo die Laufzeit Web
  Worker hat und ihr `fetch` Dateien liest (Deno, nicht Node.js), antwortet zuletzt `worker.js`
  aus dem Build als echter Worker.
- **`page-test.mjs`** prüft die Seite selbst: `index.html` und `app.js`, wie sie sind, auf dem
  kleinen DOM aus `web/page-dom.mjs` (der Test liest die Datei der anderen Seite, ändert sie
  nicht). Die Anfragen beantwortet das echte Modul oder der Test, Verovio und der Klang sind
  Attrappen. Geprüft wird: das Formular aus der Liste der Felder, aus `themes` und aus den Plänen
  des Moduls, die Anfrage für ein Thema des Moduls und für eigene Töne, leere und fehlende Felder,
  vier Stimmen nach einem Plan, ein Schluss, den das Thema nicht hergibt, eine eigene Antwort,
  schlichte freie Stimmen, der Klang nach der Zählung, Fassungen (zwei, die sich im Bass
  unterscheiden; blind mit „Auflösen“; weitere Seeds; zu viele; eine übernommen; in beiden
  Sprachen), die
  gezeigte Fuge mit Bericht, Schluss, Einsätzen, Prüfung, Tabelle und Markierungen, eine Fuge mit
  fehlenden Einsätzen und Verboten, die vier Dateien zum Herunterladen, die Zeiten der
  eingeplanten Töne beim Abspielen (ab einem Abschnitt, in anderem Tempo, nach einer verspäteten
  Runde, bis zum Ende) und „Stopp“, Fehler des Moduls, ein abgebrochenes Modul, ein gescheiterter
  Worker, ein Browser, der keinen Worker startet, „Abbrechen“, beide Sprachen mit den Links im
  Fuß, eine Antwort mit unbekannten und ohne bekannte Felder, MusicXML, das Verovio ablehnt, und
  eine Seite ohne Verovio auf schmalem Schirm.

In einem echten Browser prüfen die Tests nichts. Layout, Klang, Notenbild in der Seite und
Herunterladen muss man dort ansehen.

## Hochladen

Hochzuladen ist immer der ganze Ordner `target/web-fuge`, mit `fuge_web.wasm.gz` und `verovio/`.
Die Seite holt die Kopie `fuge_web.wasm.gz` und entpackt sie selbst, weil manche Server, auch der
von timo-keller.de, `.wasm`-Dateien unkomprimiert ausliefern. Läge neben einem neuen Modul eine
alte Kopie, rechnete die Seite mit dem alten Modul. Ohne `DecompressionStream`, ohne die Kopie oder
mit einer kaputten Kopie holt die Seite `fuge_web.wasm` selbst. Jeder statische Webserver genügt.
Die Dateien brauchen nur Leserechte, das Skript setzt sie auf 644 und die Ordner auf 755.

Die Seite bringt ihre Content-Security-Policy als `<meta>` mit. Eine Content-Security-Policy des
Servers gilt zusätzlich und muss dann mindestens `script-src 'self' 'wasm-unsafe-eval'` mit dem
Hash des Inline-Skripts aus `index.html`, `worker-src 'self'`, `connect-src 'self'` und
`style-src 'self' 'unsafe-inline'` erlauben. Das Inline-Skript ist Zeichen für Zeichen das der
Seite von fux-kontrapunkt, der Hash also derselbe.

## Was die Seite lädt und speichert

Für die Datenschutzerklärung der Website:

- **Die Seite lädt nur Dateien vom eigenen Server:** `index.html`, `style.css`, `app.js`,
  `worker.js`, `fuge.js`, das Modul `fuge_web.wasm.gz` (ersatzweise `fuge_web.wasm`) und
  `verovio/verovio-toolkit-wasm.js`. Schriften, Bilder, Zähler oder Skripte fremder Server gibt
  es nicht. Die Content-Security-Policy der Seite (`default-src 'none'`, alles Weitere nur
  `'self'`) verbietet dem Browser Anfragen an andere Server.
- **Gerechnet wird im Browser.** JavaScript und WebAssembly laufen lokal. Themen, Eingaben und
  Fugen verlassen das Gerät nicht, die Seite schickt nach dem Laden keine Daten an den Server.
  Der Server erfährt nur die Abrufe der Dateien, wie bei jeder Seite.
- **Die Seite speichert nichts:** kein `localStorage`, kein `sessionStorage`, keine IndexedDB,
  keine Cookies. Nach dem Neuladen sind Eingaben und Ergebnis weg. Der Browser hält die geladenen
  Dateien wie bei jeder Seite in seinem Cache.
- **Heruntergeladene Dateien** (MusicXML, MIDI, LilyPond, Akkorde) entstehen im Browser aus der
  Antwort des Moduls und werden nicht vom Server geholt.
- **Links nach außen** stehen im Fuß: Startseite, Kontakt, Impressum und Datenschutzerklärung von
  timo-keller.de und die Seite von Verovio. Erst ein Klick ruft sie auf.

## Aufbau

| Datei | Inhalt |
|---|---|
| `crates/fux-bach-web/` | Das Modul: `fux-bach` ohne die Programme (`default-features = false`, also ohne Threads) und die drei Exporte `fuge_input`, `fuge_run`, `fuge_output` mit dem Pufferprotokoll von `crates/fux-web/src/ffi.rs`. `unsafe_code` ist nur in `ffi.rs` erlaubt. |
| `web-fuge/fuge.js` | Lädt das Modul und stellt eine Anfrage (`Fuge.fetch`, `Fuge.load`, `ask`), wie `web/fux.js`. |
| `web-fuge/worker.js` | Der Worker: `{id, request}` hinein, `{id, answer}` heraus. Nach einem Trap lädt er das Modul neu und antwortet mit `broken`. |
| `web-fuge/app.js` | Die Seite: Texte in beiden Sprachen, Formular, Statuszeile, Bericht, Notenbild, Abspielen, Herunterladen. |
| `web-fuge/index.html`, `style.css` | Gerüst und Aussehen, nach `web/`. |
| `web-fuge/smoke-test.mjs`, `page-test.mjs` | Die Tests, siehe oben. |
| `web-fuge/package.json` | Sagt Node.js, dass die Skripte ES-Module sind. Es gibt keine Abhängigkeiten. |
| `tools/build_web_fuge.sh` | Der Bau. |

Die Schnittstelle des Moduls ist `fux_bach::api::answer_json`: eine Anfrage als JSON hinein, eine
Antwort als JSON heraus (`crates/fux-bach/src/api.rs`). Die Seite ist so gebaut, dass sie mit dem
Modul wachsen kann:

- **Ein neues Feld der Anfrage** ist ein Eintrag in der Liste `FIELDS` in `app.js`, mit Namen in
  der Anfrage (`key`), Art (`select`, `number`, `text` für Prosa, `textarea` und `code` für die
  Schreibweise in mehreren Zeilen und in einer), Gruppe im Formular (`group`), Beschriftung und
  Hinweis in beiden Sprachen und bei einer Auswahl den `options`. Aus der Liste entstehen Formular
  und Anfrage. So stehen dort `voices`, `ending`, `free_voices`, `profile`, `bass`, `plan`,
  `answer` und `beam`. Ein Feld mit `vary: true` ist außerdem ein Aspekt, in dem sich
  Fassungen unterscheiden können: Es bekommt ein Kästchen im Formular der Fassungen, und seine
  `options` sind die Werte. Ein leeres Feld
  und die leere Wahl einer Auswahl fehlen in der Anfrage. Ist das Feld `required`, fragt die Seite
  stattdessen nach.
- **Felder des eigenen Themas** tragen `own`: Die Seite schickt sie nur statt `theme`, und ein
  Thema des Moduls zeigt darin, was `themes` unter demselben Namen nennt. Ein Feld mit `bound`
  (die eigene Antwort) gehört zum gewählten Thema und wird mit einem anderen Thema geleert.
- **Auswahlen aus dem Modul.** Die Themen und die Pläne kommen aus der Antwort auf `themes`
  (`themes` und `plans`) und füllen die Auswahlen `THEME` und `PLAN`.
- **Eine neue Datei der Antwort** ist ein Eintrag in `FILES`, mit ihrem Namen in `files`, der
  Beschriftung des Knopfs, Endung und Typ. So kam `chords` als vierte dazu.
- **Themen, Pläne, Abschnitte und Kennzahlen** zeigt die Seite, wie sie kommen, in beliebiger
  Zahl. Unbekannte Felder einer Antwort übergeht sie, fehlende lässt sie aus: Ohne `playback`
  bleibt „Abspielen“ aus, ohne `tick` ist ein Abschnitt nur Text, ohne `check` fehlt die Prüfung,
  ohne Band hat eine Kennzahl nur ihren Wert. Ein unbekannter Schluss erscheint mit seinem Namen.
- **Kennzahlen** nennt das Modul ohne Einheit. Als Anteil in Prozent zeigt die Seite eine
  Kennzahl, deren Zahlen alle höchstens 1 sind und deren Name nicht „je“ (oder „per“) enthält,
  sonst als Rate mit zwei Nachkommastellen. Bekäme die Antwort später eine Einheit, wäre `isShare`
  in `app.js` die Stelle dafür.

## Größen und Zeiten

Stand 2026-11-25, Ryzen 7 PRO 6850U, Deno 2.9.8, auf der sonst ruhenden Maschine. Die Größen
gelten für diesen Stand; die Zeiten der Tests und der Fugen sind an etwas älteren Ständen
gemessen, die der Fugen mit dem Profil `bach` (auf der Seite „Klang: nach der Zählung“). Eine
Fuge wird zwei- bis viermal gesetzt (Sequenzen und Figuren der freien Stimmen); schlicht ist sie
ein Satz und braucht ein Drittel bis ein Fünftel der Zeit. Mit dem Profil `fuge` braucht die Fuge zur
Kunst der Fuge im Modul dreistimmig 3,6 s und vierstimmig 11,4 s (je der Median aus fünf Läufen);
alle zwanzig Themen sind damit nicht neu gemessen.

- **Größen.** Das Modul hat 1 073 194 Bytes, seine gzip-Kopie 330 844. Die fünf Dateien der Seite
  haben zusammen 75 kB, Verovio 7,31 MB (mit gzip 2,34 MB, wenn der Server JavaScript komprimiert).
  Ein erster Besuch lädt also etwa 7,7 MB, mit komprimiertem JavaScript etwa 2,7 MB.
- **Kunst der Fuge.** Die Fuge zu `bwv1080` (Seed 1, `beam` 400) braucht dreistimmig im Modul
  unter Deno 3,12 bis 3,14 s und mit `target/release/fux-fuge fugue --theme bwv1080` 2,05 bis
  2,08 s, vierstimmig (`--voices 4`) 14,47 bis 14,70 s und 9,72 bis 9,74 s. Das sind zwei
  Durchgänge im Wechsel, je der Median aus fünf Läufen, nativ einschließlich Programmstart und
  Dateien. Der Faktor ist etwa 1,5. Die Suche läuft auch nativ in einem Thread, deshalb ist der
  Abstand kleiner als bei fux-kontrapunkt. Schlicht (`free_voices: "plain"`, nativ `--plain`):
  dreistimmig 0,80 s im Modul und 0,54 s nativ, vierstimmig 1,67 s und 1,12 s.
- **Alle zwanzig Themen** (Seed 1, `beam` 400, im Modul unter Deno, je eine Messung): dreistimmig
  3,1 bis 13,7 s (Median 6,0 s), vierstimmig 10,3 bis 70,9 s (Median 18,9 s). Vier Stimmen
  brauchen je Thema das 1,8- bis 11,5-Fache von dreien. Schlicht sind es 0,8 bis 3,9 s und 1,7
  bis 9,1 s: Die Figuren kosten in der Summe das 3,2-Fache bei drei Stimmen und das 4,8-Fache bei
  vier, je Thema das 1,7- bis 11,5-Fache. Am längsten braucht WTK I/7 vierstimmig (70,9 s): Dort
  scheitert der erste Satz mit laufendem Rhythmus in allen Versuchen, und der Komponist nimmt
  die Figuren zurück. Es folgen Händel (50,2 s) und WTK II/12 (42,8 s), beide vierstimmig. Der
  Speicher des Moduls wuchs dabei auf 32 MB.
- **Notensatz.** Verovio setzte die Fugen aller zwanzig Themen, drei- und vierstimmig, in 0,1 bis
  0,4 s auf eine bis vier Seiten (einmal von Hand geprüft, mit den Optionen der Seite, an einem
  älteren Stand; seither haben die Fugen mehr Töne, die dreistimmige zur Kunst der Fuge 453 statt
  321, und neu gemessen ist das nicht).

## Nicht geprüft

- **Kein echter Browser.** Layout, Klang, das Notenbild in der Seite, das Herunterladen, die
  Content-Security-Policy im Browser und die Seite auf einem Telefon sind in keinem Browser
  angesehen.
  Geprüft ist: die Logik der Seite auf einem kleinen DOM (`page-test.mjs`), das echte Verovio in
  Deno (`smoke-test.mjs`) und einmal von Hand der Worker des Builds über
  `python3 -m http.server`, der dabei `worker.js`, `fuge.js` und `fuge_web.wasm.gz` holte und die
  Fuge setzte.
- **Node.js.** Die Tests liefen in Deno 2.9.8 und in der Node-Laufzeit 24.22.1 von VS Code, nicht
  in einem eigens installierten Node.js.
- **Wörter des Moduls.** Die Namen der Kennzahlen, der Pläne und die Titel der Themen kommen, wie
  das Modul sie nennt, die Kennzahlen und Titel deutsch, die Fehlermeldungen englisch, gleich in
  welcher Sprache die Seite fragt. Übersetzen müsste sie `crates/fux-bach/src/api.rs`, wie es die
  Namen der Abschnitte und die Prüfung schon tut.
