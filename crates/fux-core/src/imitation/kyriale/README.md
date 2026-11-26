# Choralmelodien des Kyriale (GABC)

Gesänge des Ordinariums, über die Palestrina Messen schrieb. Aus ihren Phrasen gewinnt die
Imitation ihre Soggetti (`--subject chant`). fux-core bettet die Dateien mit
`include_str!` ein (`fux_core::imitation::chant::Kyriale`), daher braucht die Bibliothek sie nicht
als Dateien, auch nicht in der WebAssembly-Version.

## Herkunft und Lizenz

Quelle ist [GregoBase](https://gregobase.selapa.net). Dort heißt es: „All chant transcriptions are
released under CC0 (a.k.a. Public Domain) license.“ Keiner dieser Gesänge ist dort als geschützt
markiert. Vorlage ist jeweils das Graduale Romanum (GR) und der Liber Usualis (LU), beide Solesmes
1961. Die Übertragung stammt jeweils von Andrew Hinkley. Der Schutz wissenschaftlicher Ausgaben
nach § 70 UrhG endet 25 Jahre nach ihrem Erscheinen; für die Bücher von 1961 ist er abgelaufen.

Neu laden mit `python3 tools/fetch_chant.py`. Das Skript überspringt Gesänge, die GregoBase als
geschützt markiert. Die Dateien sind so gespeichert, wie GregoBase sie ausgibt; vereinheitlicht sind
nur die Zeilenenden.

| Datei | Gesang | Modus | GregoBase | GR 1961 | LU 1961 | Übertragung | Bei Palestrina |
|---|---|---|---|---|---|---|---|
| `kyrie-ix.gabc` | Kyrie IX *Cum jubilo* | 1 | [2976](https://gregobase.selapa.net/chant.php?id=2976) | S. 32* | S. 40 | Andrew Hinkley | Missa de beata Virgine (1567 und 1570) |
| `gloria-ix.gabc` | Gloria IX | 7 | [2771](https://gregobase.selapa.net/chant.php?id=2771) | S. 32* | S. 40 | Andrew Hinkley | Missa de beata Virgine (1567) |
| `credo-i.gabc` | Credo I | 4 | [344](https://gregobase.selapa.net/chant.php?id=344) | S. 59* | S. 64 | Andrew Hinkley | Missa de beata Virgine (1567) |
| `sanctus-ix.gabc` | Sanctus IX mit Benedictus | 5 | [587](https://gregobase.selapa.net/chant.php?id=587) | S. 34* | S. 42 | Andrew Hinkley | – |
| `agnus-ix.gabc` | Agnus Dei IX | 5 | [707](https://gregobase.selapa.net/chant.php?id=707) | S. 34* | S. 42 | Andrew Hinkley | Missa in majoribus duplicibus |
| `kyrie-iv.gabc` | Kyrie IV *Cunctipotens genitor Deus* | 1 | [1061](https://gregobase.selapa.net/chant.php?id=1061) | S. 15* | S. 25 | Andrew Hinkley | Missa in minoribus duplicibus |

Die Spalte „Bei Palestrina“ nennt Messen, deren Köpfe im Korpus weit öfter diesem Choral folgen
als die Köpfe freier Messen (gezählt an Palestrinas Messen; Wagner, *Geschichte der Messe* I, 1913,
S. 446 f.). Der Kopf jeder Datei nennt außerdem spätere Bücher mit demselben Gesang (GR 1974,
Liber cantualis 1983, Gregorian Missal 1990).

## Hinweise

- **Text:** Die Gesänge singen die Texte des Ordinariums in `crates/fux-core/src/text/ordinary.txt`
  (ebenfalls Graduale Romanum 1961). Darum finden die Punkte ihre Töne über die Worte. Der Test
  `kyriale_chants_sing_the_ordinary` prüft das für jeden Abschnitt.
- **Intonation:** Gloria und Credo beginnen mit der Intonation des Zelebranten („Gló-ri-a in
  ex-cél-sis De-o“, „Cre-do in u-num De-um“). Die Polyphonie setzt erst mit „Et in terra“ und
  „Patrem“ ein. Die Intonation bleibt in der Datei, als erste Phrase.
- **Phrasen:** `Chant::phrases()` teilt an den Strichen ab dem Viertelstrich (`,`). In den Kyrie
  ist jede Anrufung eine Phrase, bis zum Doppelstrich (`::`). Die Striche innerhalb einer
  Anrufung gliedern nur ihr Melisma, an den Sternchen (`*`, `**`), die den Einsatz des Chors
  zeigen. Kyrie IX schreibt alle neun Anrufungen aus. Kyrie IV schreibt jede Gruppe einmal, mit
  „iij.“ oder „ij.“ für die Wiederholungen.
- **Modus:** Kyrie IV steht im 1. Modus, endet seine letzten Anrufungen aber auf A, der Quinte.
  Als Cantus firmus, dessen letzter Ton die Finalis sein muss, wäre es äolisch (`Chant::mode`).
  Als Quelle von Soggetti gilt der Modus der Nummer, dorisch auf D (`Chant::numbered_mode`).
  Die übrigen Gesänge enden auf ihrer Finalis. Bei Sanctus und Agnus IX (5. Modus mit
  überwiegend B♭) ist das F mit einem ♭, also transponiertes Ionisch.

## Verwendung

- `fux free --subject chant` nimmt den Gesang des Abschnitts (`Kyriale::for_section`): Messe IX
  für Kyrie, Christe, Kyrie II, Gloria, Sanctus mit Pleni, Osanna und Benedictus und für das
  Agnus Dei, Credo I für die Abschnitte des Credo. Jeden Gesang hier nennt auch sein Dateiname
  ohne `.gabc`, etwa `--subject chant:kyrie-iv`. Die GUI bietet „Choral des Kyriale“, die
  Web-Version das Soggetto `kyriale`.
- Ohne `--mode` setzt der Satz im Modus der Nummer, mit `--mode` singt er die Stufen des Gesangs
  von der eigenen Finalis aus und warnt, wenn die Finalis wechselt.
- Ein Punkt nimmt seine Töne dort, wo der Gesang seine Worte singt: das Kyrie I die erste
  Anrufung, das Christe die erste Christe-Anrufung, das Kyrie II die letzte Anrufung. Wie die Töne
  zum Kopf werden, beschreibt der [Regelkatalog](../../../../../docs/regeln.md#choral) (§ 18).
