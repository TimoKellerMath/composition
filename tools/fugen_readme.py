#!/usr/bin/env python3
"""Schreibt examples/fugen/README.md aus den Berichten (.txt) der Fugen dort.

tools/make_fugues.sh ruft es am Ende auf, wenn das Ziel examples/fugen ist.
"""
import pathlib
import os
os.chdir(pathlib.Path(__file__).resolve().parent.parent)
import re
order=["wtk1-02","wtk1-16","bwv578","bwv1080","wtk2-02","wtk1-21","wtk1-01","wtk2-20","wtk1-07","haendel","wtk2-01","wtk2-05","wtk2-12","wtk2-16","wtk2-18","wtk2-11","wtk2-21","bwv1079","wtk1-15","wtk2-09"]
rows=[]
for name in order:
    for v in (3,4):
        t=open(f'examples/fugen/{name}-{v}st.txt').read().splitlines()
        title=t[0].replace('Fuge über ','')
        m=re.match(r'(\S+) (\S+), (\d) Stimmen, (\d+) Takte, (\d+) Töne \((\d+) vorgegeben\), ([\d.]+) s', t[1])
        key, meter, voices, bars, tones, given, sec = m.groups()
        end=[l for l in t if l.startswith('Schluss:')][0]
        ending='Engführung' if 'Engführung' in end else 'Orgelpunkt'
        check=[l for l in t if 'Verbot(e)' in l][0]
        forb=int(check.split()[0]); cost=int(re.search(r'Kosten (\d+)', check).group(1))
        hits=int(re.search(r'Bändern: (\d+) von', [l for l in t if l.startswith('Kennzahlen')][0]).group(1))
        rows.append((name,title,meter,v,int(bars),int(tones),int(given),ending,forb,cost,hits,float(sec)))
table=[]
for (name,title,meter,v,bars,tones,given,ending,forb,cost,hits,sec) in rows:
    table.append(f"| [`{name}-{v}st`]({name}-{v}st.pdf) | {title} | {meter} | {v} | {bars} | {tones} | {ending} | {cost} | {hits} | {str(sec).replace('.',',')} s |")
total_forbidden=sum(r[8] for r in rows)
text=f'''# Fugen

Vierzig Fugen, die `fux-fuge` erzeugt hat: zu jedem der zwanzig Themen, die mit dem Programm
kommen, eine dreistimmige und eine vierstimmige, alle mit Seed 1. **Die Fugen sind vom Programm
erzeugt, nicht von Bach.** Von Bach und Händel stammen die Themen; Antwort, Kontrasubjekt, Plan,
Gerüst und jede weitere Note sind Ergebnis von Regeln und Suche. Was das Programm tut und wo
seine Grenzen liegen, steht in [docs/fux-fuge.md](../../docs/fux-fuge.md).

## Übersicht

| Datei | Thema | Takt | Stimmen | Takte | Töne | Schluss | Kosten | Kennzahlen | Zeit |
|---|---|---|---:|---:|---:|---|---:|---:|---:|
{chr(10).join(table)}

**Kosten** sind die Summe der Präferenzen des Regelkatalogs
([docs/regeln-bach.md](../../docs/regeln-bach.md), Profil `fuge`), gegen die eine Fuge verstößt; in keiner bricht
ein Verbot{"" if total_forbidden == 0 else f" (ausgenommen {total_forbidden} Stellen, siehe die Berichte)"}. **Kennzahlen** zählt, wie viele von 29 Zählungen (Notenwerte, Intervalle,
Dissonanz, Figuren, Satz) im Band von Bachs 48 Fugen liegen; Bachs unauffälligste Fugen erreichen
18. Die **Zeit** ist die der Suche auf einem Kern, ohne Notensatz.

## Dateien

Je Fuge vier Dateien gleichen Namens: `.pdf` (Klaviersystem, von LilyPond gesetzt), `.mid`,
`.mscz` (MuseScore, aus dem MusicXML des Programms) und `.txt` (der Bericht: Antwort, Abschnitte
mit Takt und Tonart, Schluss, Prüfung, Kennzahlen). Im PDF und in der MuseScore-Datei stehen die
Abschnitte als Worte über den Noten (Thema, Antwort, Zwischenspiel, Einsatz mit Tonart).

## Neu erzeugen

```sh
tools/make_fugues.sh                        # alle vierzig, etwa acht Minuten
tools/make_fugues.sh examples/fugen bwv578  # ein Thema
ALLES=1 tools/make_fugues.sh out/fugen      # dazu MusicXML und LilyPond
```

Dieselbe Eingabe gibt dieselbe Fuge. Eine andere Fuge zum selben Thema gibt ein anderer Seed:

```sh
target/release/fux-fuge fugue --theme bwv578 --voices 4 --seed 2 --out out/g-moll --pdf --mscz
```

Mehrere Fassungen zum Vergleichen, die sich nur in den genannten Aspekten unterscheiden (jede
trägt in ihren Dateien, worin):

```sh
target/release/fux-fuge variants --theme bwv578 --voices 4 --vary bass,sound --out out/g-moll --pdf
target/release/fux-fuge choose out/g-moll b     # Fassung b als out/g-moll.*
```

## Was man hört und was fehlt

- **Thema, Antwort, Kontrasubjekt** stehen, wie eine Fuge sie setzt: Exposition mit allen Stimmen,
  Einsätze in verwandten Tonarten, am Schluss eine Engführung oder ein Orgelpunkt.
- **Zwischenspiele** tragen den Kopf oder den Schwanz des Themas durch zwei Stimmen, stufenweise
  abwärts (bei nur zwei Aussagen bleibt das Motiv in einer Stimme). Sie sind Sequenzen in
  allen Stimmen: Was die Stimmen neben der ersten Aussage des Motivs haben, kehrt
  mit jeder weiteren eine Stufe tiefer wieder.
- **Kadenzen** schließen Gruppen von Einsätzen, nicht mehr jedes Zwischenspiel: Der Bass geht
  von der Quinte in den Grundton, die Oberstimme, wo sie frei ist, über Quartvorhalt und Leitton
  in den Grundton. In jede zweite Gruppe läuft das Zwischenspiel ohne Pause hinein.
- **Der Schluss** steht über dem Orgelpunkt der Tonika: die Oberstimme geht mit einem Schritt in
  den Grundton (in einer der vierzig Fugen hält sie ihn schon), der Schlussakkord hat die Terz
  (dreistimmig Grundton, Terz, Grundton, vierstimmig den Dreiklang), in Moll die Durterz.
- **Die übrigen Stimmen** arbeiten mit Figuren: mit Zellen aus Thema und Kontrasubjekt und mit
  eigenen, die sie eine Stufe tiefer wieder aufnehmen. Gezählt ist das mehr Zusammenhang (die
  freien Töne kehren etwa so oft wieder wie bei Bach) und mehr Gleichlauf: Die Stimmen schlagen
  öfter gemeinsam an als bei Bach, und der Satz ist dichter. `fux-fuge fugue --plain` setzt zum
  Vergleich ohne Sequenzen und Figuren, in einem einzigen Satz.
- **Die Harmonik** ist beim Hören die größte Schwäche: schwächer als Bachs, bei getragenen
  Themen deutlicher als bei laufenden. Der Komponist setzt mit Preisen, die zwei blinde Hörproben bestimmt haben (zusammen angeschlagene Dissonanz kostet
  mehr, drei Stimmen bilden öfter den vollen Dreiklang); `--profile bach` setzt mit den Preisen
  davor. Der Bass ist weiter ungeplant, und liegende Dissonanz ist selten.
- **Vorhalte** außerhalb der Kadenzen sind seltener als bei Bach; der Satz ist konsonanter.
'''
open('examples/fugen/README.md','w').write(text)
print(len(rows), 'fugues; forbidden in total', total_forbidden, '; bars', min(r[4] for r in rows), max(r[4] for r in rows), '; hits', min(r[10] for r in rows), max(r[10] for r in rows), '; seconds', min(r[11] for r in rows), max(r[11] for r in rows))
print('3 voices sec', min(r[11] for r in rows if r[3]==3), max(r[11] for r in rows if r[3]==3), '4 voices', min(r[11] for r in rows if r[3]==4), max(r[11] for r in rows if r[3]==4))
