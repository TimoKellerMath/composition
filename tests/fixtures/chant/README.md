# Choralmelodien (GABC)

Hymnen als lange Cantus für den mehrstimmigen Satz, jeweils die erste Strophe. *Iste
confessor* liegt Palestrinas gleichnamiger Messe zugrunde; seine Phrasen geben der Imitation
Soggetti für einen eigenen Text. Quelle ist [GregoBase](https://gregobase.selapa.net): „All chant
transcriptions are released under CC0 (a.k.a. Public Domain) license.“ Neu laden mit
`python3 tools/fetch_chant.py`; das Skript lädt auch die Gesänge des Kyriale, die fux-core
einbettet (`crates/fux-core/src/imitation/kyriale/`).

| Datei | Hymnus | Modus | Töne | GregoBase | Vorlage |
|---|---|---|---|---|---|
| `ave-maris-stella.gabc` | Ave maris stella | 1 | 27 | [9733](https://gregobase.selapa.net/chant.php?id=9733) | Antiphonale Monasticum 1934, S. 712 (Transkription rcarey) |
| `veni-creator.gabc` | Veni Creator Spiritus | 8 | 46 | [3241](https://gregobase.selapa.net/chant.php?id=3241) | Antiphonale Monasticum 1934, S. 518 (Transkription Albert Bloomfield) |
| `pange-lingua.gabc` | Pange lingua … Corporis | 3 | 53 | [3855](https://gregobase.selapa.net/chant.php?id=3855) | Graduale Romanum 1908, S. 123* (Transkription Andrew Hinkley) |
| `iste-confessor.gabc` | Iste confessor | 8 | 48 | [2334](https://gregobase.selapa.net/chant.php?id=2334) | Liber Usualis 1961, S. 1177 (Transkription Andrew Hinkley) |

GABC ist das Format von [Gregorio](https://gregorio-project.github.io/gabc/). `fux cp --cf DATEI.gabc`
liest es als langen Cantus ein.
