\version "2.26.0"

\header {
  title = "J. J. Fux, Gradus ad Parnassum (1725)"
  subtitle = "Alle Notenbeispiele"
  subsubtitle = "Daten: github.com/MarkGotham/species (CC0), erzeugt mit tools/render_fixtures.py"
  tagline = ##f
}

\markup \fill-line { \large \bold "2-stimmig" }

\score {
  \header { piece = "Fig. 5 – dorisch – Gattung 1" opus = "gap_005, S. 47" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | g'1 | a'1 | b'1 | c''1 | c''1 | b'1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 6 – dorisch – Gattung 1 (corrected version)" opus = "gap_006, S. 48" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d1 | d1 | a1 | f1 | e1 | d1 | f1 | c'1 | d'1 | cis'1 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 11 – phrygisch – Gattung 1" opus = "gap_011, S. 51" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" b'1 | c''1 | f'1 | g'1 | a'1 | c''1 | b'1 | e''1 | d''1 | e''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 12 – phrygisch – Gattung 1 (corrected version)" opus = "gap_012, S. 51" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e1 | a1 | d1 | e1 | f1 | f1 | c'1 | c'1 | d'1 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 13 – lydisch – Gattung 1" opus = "gap_013, S. 52" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" f'1 | e'1 | c'1 | f'1 | f'1 | g'1 | a'1 | g'1 | c'1 | f'1 | e'1 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 14 – lydisch – Gattung 1" opus = "gap_014, S. 52" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" f1 | e1 | f1 | a1 | bes1 | g1 | a1 | e1 | f1 | d1 | e1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 15 – mixolydisch – Gattung 1 (corrected version)" opus = "gap_015, S. 53" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" g'1 | e'1 | d'1 | g'1 | g'1 | g'1 | a'1 | b'1 | g'1 | c''1 | a'1 | g'1 | fis'1 | g'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" g1 | c'1 | b1 | g1 | c'1 | e'1 | d'1 | g'1 | e'1 | c'1 | d'1 | b1 | a1 | g1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 21 – mixolydisch – Gattung 1" opus = "gap_021, S. 54" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" g1 | c'1 | b1 | g1 | c'1 | e'1 | d'1 | g'1 | e'1 | c'1 | d'1 | b1 | a1 | g1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" g1 | a1 | g1 | e1 | e1 | c1 | g1 | b1 | c'1 | a1 | fis1 | g1 | fis1 | g1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 22 – äolisch – Gattung 1" opus = "gap_022, S. 55" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | e'1 | g'1 | f'1 | e'1 | c''1 | a'1 | b'1 | b'1 | a'1 | gis'1 | a'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" a1 | c'1 | b1 | d'1 | c'1 | e'1 | f'1 | e'1 | d'1 | c'1 | b1 | a1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 23 – äolisch – Gattung 1" opus = "gap_023, S. 55" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" a1 | c'1 | b1 | d'1 | c'1 | e'1 | f'1 | e'1 | d'1 | c'1 | b1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | a1 | g1 | f1 | e1 | e1 | d1 | c1 | g1 | a1 | gis1 | a1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 33 – dorisch – Gattung 2 (corrected version of 26)" opus = "gap_033, S. 59" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'2 d''2 | a'2 b'2 | c''2 g'2 | a'2 d''2 | b'2 c''2 | d''2 a'2 | c''2 d''2 | e''2 b'2 | d''2 a'2 | b'2 cis''2 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 35 – dorisch – Gattung 2" opus = "gap_035, S. 60" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 d2 | d'2 a2 | c'2 a2 | b2 a2 | g2 b2 | d'2 e'2 | f'2 c'2 | e'2 b2 | d'2 d2 | a2 cis'2 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 36 – phrygisch – Gattung 2" opus = "gap_036, S. 60" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 b'2 | c''2 b'2 | a'2 b'2 | c''2 g'2 | a'2 b'2 | c''2 a'2 | b'2 d''2 | e''2 d''2 | c''2 d''2 | e''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 37 – phrygisch – Gattung 2" opus = "gap_037, S. 60" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 e2 | a2 g2 | f2 d2 | e2 c2 | f2 c'2 | f'2 d'2 | e'2 d'2 | c'2 b2 | a2 d'2 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 38 – lydisch – Gattung 2" opus = "gap_038, S. 61" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" r2 f'2 | e'2 d'2 | c'2 bes2 | a2 g2 | f2 a2 | c'2 bes2 | a2 a'2 | g'2 e'2 | f'2 g'2 | a'2 f'2 | d'2 e'2 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 39 – lydisch – Gattung 2" opus = "gap_039, S. 61" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 f2 | e2 c2 | f2 e2 | d2 c2 | bes,2 bes2 | g2 c'2 | a2 f2 | e2 c2 | f2 f,2 | a,2 d2 | c2 e2 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 40 – mixolydisch – Gattung 2" opus = "gap_040, S. 61" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 g'2 | e'2 f'2 | g'2 a'2 | b'2 a'2 | g'2 c''2 | b'2 c''2 | d''2 c''2 | b'2 a'2 | g'2 f'2 | e'2 c''2 | b'2 a'2 | g'2 d'2 | e'2 fis'2 | g'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" g1 | c'1 | b1 | g1 | c'1 | e'1 | d'1 | g'1 | e'1 | c'1 | d'1 | b1 | a1 | g1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 41 – mixolydisch – Gattung 2" opus = "gap_041, S. 61" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" g1 | c'1 | b1 | g1 | c'1 | e'1 | d'1 | g'1 | e'1 | c'1 | d'1 | b1 | a1 | g1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 g2 | e2 f2 | g2 f2 | e2 d2 | c2 e2 | c2 c'2 | b2 a2 | g2 b2 | c'2 b2 | a2 g2 | fis2 d2 | g2 b,2 | d2 fis2 | g1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 42 – äolisch – Gattung 2" opus = "gap_042, S. 62" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 a'2 | e'2 f'2 | g'2 d'2 | e'2 e''2 | c''2 b'2 | a'2 f'2 | g'2 b'2 | d''2 a'2 | c''2 e'2 | fis'2 gis'2 | a'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" a1 | c'1 | b1 | c'1 | e'1 | f'1 | e'1 | d'1 | c'1 | b1 | a1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 43 – äolisch – Gattung 2" opus = "gap_043, S. 62" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" a1 | c'1 | b1 | d'1 | c'1 | e'1 | f'1 | e'1 | d'1 | c'1 | b1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 a,2 | a2 e2 | g2 e2 | d2 f2 | a2 b2 | c'2 c2 | d2 a,2 | c2 e2 | f2 g2 | a2 a,2 | e2 gis2 | a1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 44 – ionisch – Gattung 2" opus = "gap_044, S. 62" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 g'2 | c''2 b'2 | a'2 d''2 | b'2 a'2 | g'2 b'2 | c''2 d''2 | e''2 d''2 | c''2 b'2 | a'2 b'2 | c''2 g'2 | a'2 b'2 | c''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" c'1 | e'1 | f'1 | g'1 | e'1 | a'1 | g'1 | e'1 | f'1 | e'1 | d'1 | c'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 45 – ionisch – Gattung 2" opus = "gap_045, S. 62" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" c'1 | e'1 | f'1 | g'1 | e'1 | a'1 | g'1 | e'1 | f'1 | e'1 | d'1 | c'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 c2 | c'2 b2 | a2 d'2 | b2 g2 | c'2 b2 | a2 c'2 | e'2 d'2 | c'2 a2 | d'2 a2 | c'2 c2 | g2 b2 | c'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 55 – dorisch – Gattung 3" opus = "gap_055, S. 66" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" d'4 e'4 f'4 g'4 | a'4 b'4 c''4 d''4 | e''4 d''4 b'4 c''4 | d''4 c''4 bes'4 a'4 | bes'4 c''4 d''4 e''4 | f''4 f'4 a'4 b'4 | c''4 a'4 bes'4 c''4 | bes'4 a'4 g'4 bes'4 | a'4 d'4 e'4 f'4 | g'4 a'4 b'4 cis''4 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 56 – dorisch – Gattung 3" opus = "gap_056, S. 67" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d4 e4 f4 g4 | a4 d4 a4 b4 | c'4 b4 g4 a4 | b4 a4 g4 f4 | e4 e'4 b4 c'4 | d'4 a4 d4 e4 | f4 g4 a4 b4 | c'4 d'4 e'4 c'4 | d'4 a4 d4 d'4 | cis'4 a4 b4 cis'4 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 57 – phrygisch – Gattung 3" opus = "gap_057, S. 67" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" b'4 g'4 a'4 b'4 | c''4 b'4 a'4 g'4 | f'4 g'4 a'4 b'4 | c''4 e'4 f'4 g'4 | a'4 c''4 e''4 d''4 | c''4 b'4 a'4 c''4 | b'4 d''4 b'4 a'4 | g'4 b'4 c''4 b'4 | a'4 b'4 c''4 d''4 | e''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 58 – phrygisch – Gattung 3" opus = "gap_058, S. 67" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e4 f4 g4 e4 | a4 g4 f4 e4 | d4 e4 f4 g4 | a4 e4 a4 g4 | f4 e4 d4 e4 | f4 g4 a4 b4 | c'4 d'4 e'4 d'4 | c'4 c4 c'4 b4 | a4 d'4 a4 d'4 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 59 – lydisch – Gattung 3" opus = "gap_059, S. 68" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" f'4 e'4 d'4 c'4 | b4 d'4 g'4 f'4 | e'4 d'4 c'4 bes4 | a4 c'4 d'4 e'4 | f'4 d'4 e'4 f'4 | g'4 e'4 f'4 g'4 | a'4 g'4 f'4 a'4 | g'4 f'4 e'4 d'4 | c'4 e'4 c'4 e'4 | f'4 e'4 d'4 c'4 | bes4 c'4 d'4 e'4 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 60 – lydisch – Gattung 3" opus = "gap_060, S. 68" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" f,4 f4 e4 d4 | e4 c4 d4 e4 | f4 e4 d4 c4 | d4 c4 bes,4 a,4 | bes,4 f4 bes4 a4 | g4 c4 c'4 bes4 | a4 g4 f4 d4 | e4 c4 d4 e4 | f4 e4 d4 c4 | d4 e4 f4 d4 | e4 c4 d4 e4 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 73 – dorisch – Gattung 4" opus = "gap_073, S. 74" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 a'2~ | a'2 d''2~ | d''2 c''2~ | c''2 bes'2~ | bes'2 g'2 | a'2 c''2~ | c''2 f''2~ | f''2 e''2~ | e''2 d''2~ | d''2 cis''2 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 74 – dorisch – Gattung 4" opus = "gap_074, S. 74" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 d2~ | d2 d'2~ | d'2 c'2~ | c'2 b2~ | b2 e'2~ | e'2 d'2~ | d'2 f'2~ | f'2 e'2~ | e'2 d'2~ | d'2 cis'2 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 75 – phrygisch – Gattung 4" opus = "gap_075, S. 75" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 e''2~ | e''2 c''2~ | c''2 b'2 | c''2 e'2~ | e'2 f'2~ | f'2 c''2~ | c''2 b'2~ | b'2 e''2~ | e''2 d''2 | e''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 76 – phrygisch – Gattung 4" opus = "gap_076, S. 75" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 e2~ | e2 a2~ | a2 g2~ | g2 f2~ | f2 d2~ | d2 d'2~ | d'2 c'2~ | c'2 e'2~ | e'2 d'2 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 77 – lydisch – Gattung 4" opus = "gap_077, S. 75" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" r2 f'2~ | f'2 e'2~ | e'2 c'2~ | c'2 f'2~ | f'2 a'2~ | a'2 g'2~ | g'2 f'2~ | f'2 e'2~ | e'2 a'2~ | a'2 f'2~ | f'2 e'2 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 78 – lydisch – Gattung 4" opus = "gap_078, S. 75" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 f2~ | f2 e2~ | e2 d2~ | d2 bes,2~ | bes,2 g,2~ | g,2 g2~ | g2 f2~ | f2 e2~ | e2 d2~ | d2 f2~ | f2 e2 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 82 – dorisch – Gattung 5" opus = "gap_082, S. 77" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 a'2~ | a'4 d'4 e'4 f'4 | g'4 f'4 e'4 g'4 | f'4 d'4 d''2~ | d''4 c''4 bes'4 g'4 | a'4 b'4 c''2~ | c''2 f''2~ | f''4 e''8 d''8 e''2~ | e''4 a'4 d''2~ | d''2 cis''2 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 83 – dorisch – Gattung 5" opus = "gap_083, S. 77" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 d'2~ | d'2 a4 b4 | c'4 g4 c'2~ | c'4 b8 a8 b4 a4 | g4 a4 b4 c'4 | d'4 a4 d'2~ | d'4 e'4 f'2~ | f'4 e'8 d'8 e'2~ | e'4 a4 d'2~ | d'2 cis'2 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 84a – phrygisch – Gattung 5" opus = "gap_084a, S. 78" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 e''2~ | e''4 d''4 c''4 b'4 | a'4 g'4 f'4 d'4 | e'4 g'4 c''2~ | c''4 b'4 a'4 g'4 | f'2 c''2~ | c''4 b'8 a'8 b'4 a'4 | g'4 e'4 e''2~ | e''2 d''2 | e''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 84b – phrygisch – Gattung 5" opus = "gap_084b, S. 78" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 e2 | a4 b4 c'2~ | c'4 b8 a8 b4 g4 | a2 e2 | f4 d4 f2~ | f4 g4 a4 b4 | c'4 b4 c'2~ | c'4 d'4 e'2~ | e'2 d'2 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 85a – lydisch – Gattung 5" opus = "gap_085a, S. 78" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" r2 f'2~ | f'2 e'4 d'4 | c'4 a4 c'2~ | c'4 a4 d'2~ | d'4 c'4 bes4 a4 | g2 c'4 bes4 | a2 a'2~ | a'2 g'2 | a'4 g'4 f'4 e'4 | d'4 c'4 f'2~ | f'2 e'2 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 85b – lydisch – Gattung 5" opus = "gap_085b, S. 78" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 f2~ | f4 e8 d8 c4 bes,4 | a,4 g,4 f,2~ | f,4 a,4 bes,2~ | bes,4 c4 d2~ | d2 c4 bes,4 | a,2 f2 | e2 f2~ | f4 e4 d2~ | d4 e4 f2~ | f2 e2 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 86a – mixolydisch – Gattung 5" opus = "gap_086a, S. 79" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 g'2~ | g'4 f'4 e'4 d'8 c'8 | d'2 g'4 a'4 | b'4 c''4 d''4 b'4 | e''4 d''4 c''2~ | c''4 b'4 a'4 g'4 | a'4 b'8 c''8 d''4 c''4 | b'4 g'4 b'2~ | b'4 a'4 g'2~ | g'4 f'4 e'4 g'4 | fis'4 g'4 a'2~ | a'4 g'4 g'2~ | g'2 fis'2 | g'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" g1 | c'1 | b1 | g1 | c'1 | e'1 | d'1 | g'1 | e'1 | c'1 | d'1 | b1 | a1 | g1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 86b – mixolydisch – Gattung 5" opus = "gap_086b, S. 79" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" g1 | c'1 | b1 | g1 | c'1 | e'1 | d'1 | g'1 | e'1 | c'1 | d'1 | b1 | a1 | g1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 g2 | e2 a2~ | a2 g4 f4 | e4 c4 e2~ | e2 a2~ | a4 b4 c'2~ | c'2 b4 a4 | b2 e'4 d'4 | c'4 b4 a2~ | a4 b4 c'2~ | c'2 b4 a4 | g4 d4 g2~ | g2 fis2 | g1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 87a – äolisch – Gattung 5" opus = "gap_087a, S. 80" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 a'2~ | a'2 g'4 a'4 | b'4 g'4 b'2~ | b'2 a'4 b'4 | c''4 g'4 c''2~ | c''4 b'8 a'8 b'4 c''4 | d''4 a'4 d''2~ | d''4 c''4 c''2~ | c''4 b'8 a'8 b'2~ | b'4 e'4 a'2~ | a'2 gis'2 | a'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" a1 | c'1 | b1 | d'1 | c'1 | e'1 | f'1 | e'1 | d'1 | c'1 | b1 | a1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 87b – äolisch – Gattung 5" opus = "gap_087b, S. 80" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" a1 | c'1 | b1 | d'1 | c'1 | e'1 | f'1 | e'1 | d'1 | c'1 | b1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 a2~ | a4 g4 e4 f4 | g4 d4 g2~ | g4 a4 b2~ | b2 a2~ | a4 b4 c'4 b4 | a2 d'2~ | d'4 g4 c'2~ | c'4 b8 a8 b2~ | b4 e4 a2~ | a2 gis2 | a1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 88a – ionisch – Gattung 5" opus = "gap_088a, S. 80" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 c''2~ | c''2 b'2 | a'4 b'4 c''4 d''4 | e''2 d''4 c''4 | b'4 a'4 g'2 | a'2 d''2~ | d''4 g'4 c''2~ | c''2 b'2 | c''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" c'1 | d'1 | f'1 | e'1 | g'1 | f'1 | e'1 | d'1 | c'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 88b – ionisch – Gattung 5" opus = "gap_088b, S. 80" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" c'1 | d'1 | f'1 | e'1 | g'1 | f'1 | e'1 | d'1 | c'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 c'2~ | c'2 b2 | a2 d'2~ | d'4 c'8 b8 c'2~ | c'4 d'4 e'2~ | e'4 a4 d'2~ | d'4 c'4 c'2~ | c'2 b2 | c'1 \bar "|." }
  >>
  \layout { }
}

\markup \fill-line { \large \bold "3-stimmig" }

\score {
  \header { piece = "Fig. 101 – dorisch – Gattung 1" opus = "gap_101, S. 87" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" f1 | a1 | c'1 | a1 | b1 | d'1 | c'1 | e'1 | d'1 | cis'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | a1 | f1 | e1 | d1 | f1 | c1 | d1 | a1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 104 – dorisch – Gattung 1" opus = "gap_104, S. 88" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" f'1 | a'1 | c''1 | b'1 | b'1 | d''1 | c''1 | e''1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d1 | d1 | e1 | g1 | e1 | d1 | f1 | c1 | d1 | a1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 105 – dorisch – Gattung 1" opus = "gap_105, S. 89" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" f'1 | a'1 | g'1 | f'1 | e'1 | f'1 | e'1 | g'1 | d'1 | cis'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d'1 | c'1 | c'1 | d'1 | bes1 | a1 | c'1 | bes1 | a1 | g1 | a1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 106 – phrygisch – Gattung 1" opus = "gap_106, S. 89" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" g1 | a1 | f1 | e1 | f1 | c'1 | e'1 | c'1 | a1 | gis1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" e1 | f1 | d1 | a,1 | f,1 | f,1 | c1 | c1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 108 – phrygisch – Gattung 1" opus = "gap_108, S. 90" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" g'1 | e'1 | f'1 | a'1 | c''1 | c''1 | e''1 | c''1 | a'1 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e1 | a1 | d1 | f1 | f1 | f1 | c'1 | c'1 | d'1 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 109 – phrygisch – Gattung 1" opus = "gap_109, S. 91" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" g'1 | e'1 | f'1 | e'1 | e'1 | e'1 | g'1 | g'1 | d'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" b1 | c'1 | a1 | a1 | c'1 | c'1 | b1 | b1 | a1 | b1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" e1 | c1 | d1 | c1 | a,1 | a1 | g1 | e1 | f1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 110 – lydisch – Gattung 1" opus = "gap_110, S. 91" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "soprano" f'1 | g'1 | a'1 | f'1 | d'1 | e'1 | f'1 | c''1 | a'1 | f'1 | g'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" c'1 | c'1 | c'1 | d'1 | bes1 | b1 | a1 | a'1 | f'1 | d'1 | e'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" f1 | e1 | f1 | d1 | g1 | g1 | f1 | a,1 | d1 | d1 | c1 | f,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 111 – lydisch – Gattung 1" opus = "gap_111, S. 91" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "treble" a'1 | c''1 | c''1 | d''1 | d''1 | bes'1 | a'1 | g'1 | f'1 | f''1 | e''1 | f''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "soprano" f'1 | g'1 | a'1 | f'1 | d'1 | e'1 | f'1 | c''1 | a'1 | f'1 | g'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" f'1 | e'1 | f'1 | d'1 | bes1 | g1 | f1 | e1 | f1 | d1 | c1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 112 – lydisch – Gattung 1" opus = "gap_112, S. 91" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | bes'1 | c''1 | a'1 | bes'1 | c''1 | a'1 | g'1 | c''1 | a'1 | bes'1 | a'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" f'1 | d'1 | c'1 | c'1 | f'1 | g'1 | c'1 | e'1 | e'1 | f'1 | e'1 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 113 – mixolydisch – Gattung 1" opus = "gap_113, S. 92" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" g1 | c'1 | b1 | g1 | c'1 | e'1 | d'1 | g'1 | e'1 | c'1 | d'1 | b1 | a1 | g1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" b1 | g1 | g1 | b1 | e'1 | c'1 | b1 | g1 | g1 | a1 | fis1 | g1 | fis1 | g1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" g1 | e1 | e1 | e1 | c1 | c1 | g1 | e1 | c1 | a,1 | b,1 | g,1 | d1 | g,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 114 – mixolydisch – Gattung 1" opus = "gap_114, S. 92" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" b'1 | g'1 | g'1 | b'1 | g'1 | a'1 | b'1 | b'1 | c''1 | e'1 | fis'1 | g'1 | fis'1 | g'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" g1 | c'1 | b1 | g1 | c'1 | e'1 | d'1 | g'1 | e'1 | c'1 | d'1 | b1 | a1 | g1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" g1 | e1 | e1 | e1 | e1 | c1 | g1 | e1 | c1 | c1 | b,1 | g,1 | d1 | g,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 115 – äolisch – Gattung 1" opus = "gap_115, S. 92" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "soprano" a'1 | c''1 | b'1 | d''1 | c''1 | e''1 | f''1 | e''1 | d''1 | c''1 | b'1 | a'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" c'1 | e'1 | g'1 | f'1 | e'1 | g'1 | a'1 | g'1 | f'1 | a'1 | gis'1 | a'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | a1 | e'1 | d'1 | a1 | g1 | f1 | c'1 | d'1 | a1 | e'1 | a1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 116 – äolisch – Gattung 1" opus = "gap_116, S. 93" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" e'1 | e'1 | g'1 | f'1 | e'1 | c''1 | a'1 | c''1 | b'1 | a'1 | gis'1 | a'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" a1 | c'1 | b1 | d'1 | c'1 | e'1 | f'1 | e'1 | d'1 | c'1 | b1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" a,1 | a,1 | e1 | d1 | a1 | a1 | d1 | c1 | g1 | a1 | e1 | a,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 117 – äolisch – Gattung 1" opus = "gap_117, S. 93" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" c'1 | e'1 | d'1 | f'1 | e'1 | c'1 | c'1 | c'1 | d'1 | a1 | gis1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | a1 | b1 | b1 | c'1 | g1 | a1 | g1 | f1 | e1 | d1 | e1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" a,1 | c1 | b,1 | d1 | c1 | e1 | f1 | e1 | d1 | c1 | b,1 | a,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 118 – ionisch – Gattung 1" opus = "gap_118, S. 93" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" c'1 | e'1 | f'1 | g'1 | e'1 | a'1 | g'1 | e'1 | f'1 | e'1 | d'1 | c'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" g1 | c'1 | a1 | g1 | c'1 | c'1 | e'1 | c'1 | b1 | c'1 | b1 | c'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" c1 | c1 | d1 | e1 | c1 | f1 | e1 | a1 | d1 | c1 | g1 | c1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 119 – ionisch – Gattung 1" opus = "gap_119, S. 94" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" e'1 | g'1 | a'1 | b'1 | c''1 | c''1 | e''1 | c''1 | a'1 | c''1 | b'1 | c''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" c'1 | e'1 | f'1 | g'1 | e'1 | a'1 | g'1 | e'1 | f'1 | e'1 | d'1 | c'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" c'1 | c'1 | a1 | g1 | a1 | f1 | c'1 | c'1 | d'1 | c'1 | g1 | c'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 121 – dorisch – Gattung 2" opus = "gap_121, S. 95" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 f2 | a2 b2 | c'2 a2 | b2 a2 | g2 b2 | d'2 b2 | a2 c'2 | e'2 c'2 | a2 d'2~ | d'2 cis'2 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 122 – dorisch – Gattung 2" opus = "gap_122, S. 95" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 f'2 | a'2 b'2 | c''2 a'2 | b'2 g'2 | b'2 c''2 | d''2 f''2 | c''2 d''2 | e''2 c''2 | a'2 d''2~ | d''2 cis''2 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c'1 | d'1 | a1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 123 – dorisch – Gattung 2" opus = "gap_123, S. 96" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | c''1 | b'1 | b'1 | d''1 | c''1 | e''1 | a'1 | g'1 | fis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 d'2 | d2 f2 | a2 e2 | g2 f2 | e2 g2 | d2 e2 | f2 a2 | c'2 c2 | d2 d'2~ | d'2 cis'2 | d'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 124 – phrygisch – Gattung 2" opus = "gap_124, S. 97" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 g'2 | a'2 g'2 | f'2 g'2 | a'2 b'2 | c''2 d''2 | c''2 d''2 | e''2 d''2 | c''2 b'2 | a'2 b'2 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e1 | f1 | d1 | f1 | f1 | f'1 | c'1 | c'1 | d'1 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 125 – phrygisch – Gattung 2" opus = "gap_125, S. 97" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" e'1 | e'1 | f'1 | e'1 | f'1 | e'1 | g'1 | g'1 | a'1 | gis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 b2 | c'2 b2 | a2 f2 | g2 e2 | a2 b2 | c'2 a2 | b2 d'2 | e'2 d'2 | c'2 a2 | b1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" e1 | c1 | d1 | c1 | a,1 | a1 | g1 | e1 | f1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 126 – phrygisch – Gattung 2" opus = "gap_126, S. 97" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e1 | a1 | f1 | a1 | f1 | f'1 | e'1 | c'1 | a1 | gis1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 e2 | f2 e2 | d2 e2 | f2 c2 | d2 e2 | f2 a2 | c'2 g2 | a2 e2~ | e2 d2 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 127 – lydisch – Gattung 2" opus = "gap_127, S. 98" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "soprano" f'1 | g'1 | a'1 | f'1 | d'1 | e'1 | f'1 | c''1 | a'1 | f'1 | g'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" r2 f'2 | e'2 d'2 | c'2 a2 | d'2 a2 | bes2 a2 | g2 c'2 | a2 a'2 | g'2 e'2 | c'2 a2 | d'2 f'2~ | f'2 e'2 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" f1 | c1 | f1 | d1 | bes,1 | c1 | d1 | e1 | f1 | d1 | c1 | f,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 128 – lydisch – Gattung 2" opus = "gap_128, S. 98" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "treble" r2 f''2 | e''2 d''2 | c''2 bes'2 | a'2 c''2 | bes'2 a'2 | g'2 c''2 | a'2 a''2 | g''2 e''2 | c''2 a'2 | d''2 f''2~ | f''2 e''2 | f''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "soprano" f'1 | g'1 | a'1 | f'1 | d'1 | e'1 | f'1 | c''1 | a'1 | f'1 | g'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" f1 | c1 | f1 | f1 | g1 | c'1 | d'1 | e'1 | f'1 | d'1 | c'1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 129 – lydisch – Gattung 2" opus = "gap_129, S. 98" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" c'1 | c'1 | c'1 | d'1 | f'1 | g'1 | a'1 | g'1 | c'1 | d'1 | e'1 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 f2 | e2 c2 | f2 e2 | d2 c2 | bes,2 a,2 | g,2 c2 | f,2 f2 | e2 c2 | f2 e2 | d2 bes,2 | g,2 c2 | f,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 130 – dorisch – Gattung 3" opus = "gap_130, S. 100" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'4 d'4 f'4 g'4 | a'4 f'4 a'4 b'4 | c''4 b'4 g'4 a'4 | b'4 c''4 d''4 b'4 | e''4 d''4 b'4 cis''4 | d''4 f''4 e''4 d''4 | c''4 a'4 c''4 d''4 | e''4 d''4 c''4 b'4 | a'4 d'4 a'4 b'4 | cis''4 a'4 b'4 cis''4 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 131 – dorisch – Gattung 3" opus = "gap_131, S. 100" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" d'4 e'4 f'4 g'4 | a'4 a4 c'4 d'4 | e'4 f'4 g'4 e'4 | f'4 a'4 f'4 e'4 | d'4 bes4 d'4 e'4 | f'4 a'4 g'4 f'4 | e'4 c'4 e'4 f'4 | g'4 e'4 f'4 g'4 | a'4 a4 c'4 d'4 | e'4 d'4 b4 cis'4 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | a1 | cis'1 | d'1 | bes1 | a1 | c'1 | bes1 | a1 | g1 | fis1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 132 – dorisch – Gattung 3" opus = "gap_132, S. 101" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | c''1 | b'1 | b'1 | d''1 | c''1 | e''1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r4 d4 f4 e4 | d4 e4 f4 g4 | a4 g4 e4 f4 | g4 d4 g4 f4 | e4 g4 f4 e4 | d4 d'4 d4 e4 | f4 g4 a4 b4 | c'4 e'4 d'4 c'4 | d'4 d4 f4 g4 | a4 g4 a4 a,4 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 133 – dorisch – Gattung 3" opus = "gap_133, S. 101" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" f4 d4 f4 g4 | a4 f4 a4 b4 | c'4 e4 g4 a4 | b4 d'4 b4 a4 | g4 a4 b4 cis'4 | d'4 e'4 f'4 d'4 | c'4 a4 c'4 d'4 | e'4 d'4 c'4 b4 | a4 d'4 a4 b4 | cis'4 a4 b4 cis'4 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 134 – dorisch – Gattung 2+3" opus = "gap_134, S. 102" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" d'4 e'4 f'4 g'4 | a'4 g'4 f'4 a'4 | g'4 f'4 e'4 g'4 | f'4 a'4 f'4 e'4 | d'4 bes4 d'4 e'4 | f'4 a'4 g'4 f'4 | e'4 c'4 e'4 f'4 | g'4 f'4 d'4 e'4 | f'4 e'4 c'4 d'4 | e'4 d'4 b4 cis'4 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a2 d'2 | c'2 a2 | b2 cis'2 | d'2 a2 | bes2 g2 | a2 b2 | c'2 a2 | e'2 b2 | d'2 a2 | g1 | a1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 141 – dorisch – Gattung 4" opus = "gap_141, S. 106" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 a'2~ | a'2 d''2~ | d''2 c''2~ | c''2 b'2~ | b'2 e''2~ | e''2 d''2~ | d''2 f''2~ | f''2 e''2~ | e''2 d''2~ | d''2 cis''2 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | g1 | g1 | e1 | f1 | f1 | c1 | d1 | a1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 143 – dorisch – Gattung 4" opus = "gap_143, S. 108" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" f'1 | a'1 | g'1 | f'1 | bes'1 | a'1 | c''1 | bes'1 | a'1 | g'1 | fis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" r2 a2~ | a2 d'2~ | d'2 c'2~ | c'2 bes2~ | bes2 d'2~ | d'2 c'2~ | c'2 f'2~ | f'2 e'2~ | e'2 d'2~ | d'2 cis'2 | d'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 144 – dorisch – Gattung 4" opus = "gap_144, S. 108" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d1 | d'1 | g1 | g1 | b1 | a1 | f1 | c'1 | a1 | g1 | fis1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r1 | r2 d2~ | d2 c2~ | c2 b,2~ | b,2 e2~ | e2 d2~ | d2 f2~ | f2 e2~ | e2 d2~ | d2 cis2 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 146 – phrygisch – Gattung 4" opus = "gap_146, S. 109" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 e''2~ | e''2 c''2~ | c''2 b'2~ | b'2 a'2~ | a'2 c''2~ | c''2 d''2~ | d''2 c''2~ | c''2 a'2~ | a'2 b'2 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e1 | a1 | g1 | a1 | f1 | f1 | e1 | c1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 147 – phrygisch – Gattung 4" opus = "gap_147, S. 109" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 e'2~ | e'2 c'2~ | c'2 b2~ | b2 a2~ | a2 d'2~ | d'2 c'2~ | c'2 b2~ | b2 a2~ | a2 b2 | gis1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" e1 | a1 | g1 | a1 | f1 | f1 | e1 | c1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 148 – phrygisch – Gattung 4" opus = "gap_148, S. 109" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" g'1 | g'1 | g'1 | e'1 | c''1 | d''1 | e''1 | c''1 | a'1 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 e2~ | e2 c2~ | c2 b,2~ | b,2 a,2~ | a,2 f2~ | f2 d2~ | d2 c2~ | c2 e2~ | e2 d2 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 149 – lydisch – Gattung 4" opus = "gap_149, S. 110" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" r2 f'2~ | f'2 e'2~ | e'2 d'2~ | d'2 f'2~ | f'2 g'2~ | g'2 a'2~ | a'2 f'2~ | f'2 e'2~ | e'2 d'2~ | d'2 f'2~ | f'2 e'2 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" f1 | c1 | f1 | d1 | bes,1 | c1 | d1 | a1 | f1 | d1 | c1 | f,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 150 – lydisch – Gattung 4" opus = "gap_150, S. 110" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | b'1 | c''1 | a'1 | f'1 | g'1 | a'1 | g'1 | c''1 | a'1 | bes'1 | a'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" r2 f'2~ | f'2 e'2~ | e'2 c'2~ | c'2 a2~ | a2 d'2~ | d'2 c'2~ | c'2 f'2~ | f'2 e'2~ | e'2 c'2~ | c'2 f'2~ | f'2 e'2 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 151 – lydisch – Gattung 4" opus = "gap_151, S. 110" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "soprano" f'1 | g'1 | a'1 | f'1 | d'1 | e'1 | f'1 | c''1 | a'1 | f'1 | g'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" f1 | c'1 | f'1 | d'1 | bes1 | g1 | d'1 | e'1 | f'1 | d'1 | bes1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 f2~ | f2 e2~ | e2 d2~ | d2 bes,2~ | bes,2 g,2 | r2 c2~ | c2 bes,2~ | bes,2 a,2~ | a,2 d2~ | d2 f2~ | f2 e2 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 154 – dorisch – Gattung 5" opus = "gap_154, S. 112" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 a'2~ | a'4 f'4 a'4 b'4 | c''4 g'4 c''2~ | c''4 b'8 a'8 b'2~ | b'4 g'4 b'4 cis''4 | d''4 f''4 e''4 d''4 | c''4 f'4 f''2~ | f''4 e''8 d''8 e''2~ | e''4 a'4 d''2~ | d''2 cis''2 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d1 | d'1 | c'1 | g1 | e1 | d1 | f1 | c'1 | d'1 | a1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 155 – dorisch – Gattung 5" opus = "gap_155, S. 112" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 a2~ | a4 f8 g8 a4 b4 | c'4 g4 c'2~ | c'2 b2~ | b4 g8 a8 b4 cis'4 | d'4 a4 d'2~ | d'4 c'4 f'2~ | f'4 e'8 d'8 e'2~ | e'4 a4 d'2~ | d'2 cis'2 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 156 – dorisch – Gattung 5" opus = "gap_156, S. 113" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | c''1 | b'1 | b'1 | d''1 | c''1 | c''1 | a'1 | g'1 | fis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 d2 | d4 e4 f4 g4 | a4 e4 a2~ | a2 g4 f4 | e4 g4 f4 e4 | d4 e4 f2~ | f4 c4 f2~ | f2 e2~ | e4 a,4 d2~ | d2 cis2 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 157 – phrygisch – Gattung 5" opus = "gap_157, S. 113" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 e''2~ | e''4 d''4 c''4 b'4 | a'2 g'4 f'4 | e'2 a'2~ | a'4 b'4 c''2~ | c''2 f''2~ | f''4 e''8 d''8 e''4 d''4 | c''4 g'4 c''2~ | c''2 b'4 a'4 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e1 | a1 | f1 | a1 | f1 | f1 | c'1 | c'1 | d'1 | e'1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 158 – phrygisch – Gattung 5" opus = "gap_158, S. 113" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 gis2 | a2. g4 | f4 d8 e8 f4 g4 | a4 f8 g8 a4 b4 | c'2. b4 | a4 b4 c'4 d'4 | e'2 e4 f4 | g2 c'2~ | c'2 b4 a4 | gis1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" e1 | f1 | d1 | f1 | f1 | f1 | c1 | c1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 159 – phrygisch – Gattung 5" opus = "gap_159, S. 114" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" gis'1 | a'1 | f'1 | e'1 | c''1 | c''1 | e''1 | c''1 | a'1 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 e2 | f2. e4 | d4 e4 f4 g4 | a4 e4 a2~ | a4 g4 f2~ | f4 g4 a4 b4 | c'2 c2~ | c4 d4 e2~ | e2 d2 | e1 \bar "|." }
  >>
  \layout { }
}

\markup \fill-line { \large \bold "4-stimmig" }

\score {
  \header { piece = "Fig. 160 – dorisch – Gattung 1" opus = "gap_160, S. 115" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" f'1 | a'1 | c''1 | b'1 | b'1 | d''1 | c''1 | e''1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | f1 | g1 | g1 | g1 | a1 | a1 | c'1 | a1 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 163 – dorisch – Gattung 1" opus = "gap_163, S. 117" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | a1 | c'1 | b1 | b1 | d'1 | c'1 | e'1 | d'1 | cis'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" f1 | f1 | g1 | g1 | g1 | a1 | a1 | c'1 | a1 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 164 – dorisch – Gattung 1" opus = "gap_164, S. 118" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "alto" f'1 | d'1 | c'1 | d'1 | d'1 | f'1 | e'1 | e'1 | d'1 | cis'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | a1 | a1 | f1 | bes1 | a1 | c'1 | c'1 | a1 | a1 | a1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | a,1 | bes,1 | g,1 | d1 | a,1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 165 – dorisch – Gattung 1" opus = "gap_165, S. 118" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | cis''1 | d''1 | bes'1 | a'1 | a'1 | b'1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" f'1 | f'1 | g'1 | f'1 | d'1 | f'1 | e'1 | g'1 | a'1 | g'1 | fis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d'1 | d'1 | g1 | a1 | bes1 | c'1 | c'1 | d'1 | d1 | g1 | a1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 166 – phrygisch – Gattung 1" opus = "gap_166, S. 119" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" b1 | a1 | a1 | a1 | c'1 | c'1 | e'1 | c'1 | d'1 | b1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" gis1 | a1 | f1 | e1 | e1 | f1 | c'1 | c'1 | a1 | gis1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" e1 | f1 | d1 | a,1 | a,1 | f,1 | e,1 | a,1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 167 – phrygisch – Gattung 1" opus = "gap_167, S. 119" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" gis'1 | a'1 | f'1 | e'1 | e'1 | f'1 | c''1 | c''1 | a'1 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" b1 | a1 | a1 | a1 | c'1 | c'1 | e'1 | c'1 | d'1 | b1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" e1 | f1 | d1 | a,1 | a,1 | f1 | e1 | a1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 168 – phrygisch – Gattung 1" opus = "gap_168, S. 119" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" b'1 | a'1 | a'1 | c''1 | c''1 | c''1 | e''1 | e''1 | d''1 | b'1 \bar "|." }
    \new Staff { \time 2/2 \clef "soprano" gis'1 | a'1 | f'1 | g'1 | a'1 | f'1 | c''1 | c''1 | a'1 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e1 | f1 | d1 | e1 | f1 | f1 | e1 | a1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 169 – lydisch – Gattung 1" opus = "gap_169, S. 120" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | g'1 | f'1 | a'1 | bes'1 | b'1 | a'1 | a'1 | f'1 | f'1 | e'1 | f'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "soprano" f'1 | g'1 | a'1 | f'1 | d'1 | e'1 | f'1 | c''1 | a'1 | f'1 | g'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" c'1 | c'1 | c'1 | d'1 | f'1 | g'1 | f'1 | e'1 | d'1 | a1 | c'1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" f1 | e1 | f1 | d1 | bes,1 | g,1 | d1 | a,1 | d1 | d1 | c1 | f,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 170 – lydisch – Gattung 1" opus = "gap_170, S. 120" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "soprano" f'1 | g'1 | a'1 | f'1 | d'1 | e'1 | f'1 | c''1 | a'1 | f'1 | g'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" c'1 | c'1 | c'1 | c'1 | bes1 | b1 | a1 | a'1 | f'1 | f'1 | e'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | g1 | f1 | a1 | f1 | g1 | f1 | e1 | f1 | a1 | c'1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" f1 | e1 | f1 | f,1 | bes,1 | g,1 | d1 | a,1 | d1 | d1 | c1 | f,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 171 – lydisch – Gattung 1" opus = "gap_171, S. 120" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | g'1 | f'1 | f'1 | f'1 | g'1 | f'1 | e'1 | f'1 | f'1 | e'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" c'1 | c'1 | c'1 | d'1 | d'1 | bes1 | a1 | a1 | f1 | a1 | c'1 | a1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" f1 | e1 | f1 | d1 | bes,1 | g,1 | d1 | a,1 | d1 | d1 | c1 | f,1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 172 – lydisch – Gattung 1" opus = "gap_172, S. 121" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" c''1 | b'1 | c''1 | a'1 | a'1 | c''1 | c''1 | c''1 | c''1 | a'1 | bes'1 | a'1 \bar "|." }
    \new Staff { \time 2/2 \clef "soprano" a'1 | g'1 | e'1 | f'1 | f'1 | g'1 | a'1 | g'1 | f'1 | f'1 | e'1 | f'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" f'1 | d'1 | c'1 | c'1 | d'1 | c'1 | c'1 | e'1 | c'1 | d'1 | bes1 | c'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "tenor" f1 | g1 | a1 | f1 | d1 | e1 | f1 | c'1 | a1 | f1 | g1 | f1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 173 – dorisch – Gattung 2" opus = "gap_173, S. 122" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | d'1 | g1 | g1 | b1 | d'1 | c'1 | e'1 | d'1 | cis'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 f2 | a2 b2 | c'2 g2 | b2 a2 | g2 e2 | a2 f2 | e2 f2 | e2 c'2 | a2 f2 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | a,1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 174 – dorisch – Gattung 2" opus = "gap_174, S. 122" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 f'2 | a'2 b'2 | c''2 g'2 | b'2 d''2 | e''2 b'2 | d''2 f''2 | e''2 c''2 | e''2 c''2 | a'2 d''2 | cis''1 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | f1 | g1 | g1 | g1 | a1 | c'1 | c'1 | f1 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | a,1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 175 – dorisch – Gattung 2" opus = "gap_175, S. 122" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" d''1 | d''1 | c''1 | d''1 | e''1 | f''1 | f''1 | e''1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | a'1 | b'1 | b'1 | d''1 | c''1 | c''1 | a'1 | a'1 | a'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 d'2 | d2 f2 | a2 c'2 | g2 f2 | e2 g2 | d2 e2 | f2 a2 | c'2 c2 | d2 f2 | a2 a,2 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 176 – dorisch – Gattung 2" opus = "gap_176, S. 123" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" f'1 | a'1 | c''1 | d''1 | bes'1 | a'1 | a'1 | b'1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" r2 d'2 | c'2 d'2 | e'2 g'2 | f'2 e'2 | d'2 e'2 | f'2 a'2 | e'2 f'2 | g'2 d'2 | f'2 a'2 | g'2 e'2 | fis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | a1 | g1 | a1 | bes1 | c'1 | c'1 | d'1 | a1 | e'1 | a1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 177 – dorisch – Gattung 3" opus = "gap_177, S. 124" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" d'4 e'4 f'4 g'4 | a'4 f'4 a'4 b'4 | c''4 e''4 d''4 c''4 | b'4 g'4 b'4 d''4 | e''4 d''4 b'4 cis''4 | d''4 f''4 e''4 d''4 | c''4 a'4 c''4 d''4 | e''4 d''4 c''4 b'4 | a'4 b'4 c''4 d''4 | cis''4 a'4 b'4 cis''4 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | f1 | g1 | b1 | g1 | a1 | a1 | c'1 | f1 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 180 – dorisch – Gattung 3" opus = "gap_180, S. 125" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" f'1 | a'1 | g'1 | a'1 | g'1 | a'1 | c''1 | b'1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" d'4 f'4 e'4 d'4 | c'4 a4 c'4 d'4 | e'4 f'4 g'4 e'4 | f'4 g'4 a'4 f'4 | d'4 bes4 d'4 e'4 | f'4 a'4 g'4 f'4 | e'4 c'4 e'4 f'4 | g'4 f'4 d'4 e'4 | f'4 d'4 e'4 f'4 | g'4 f'4 e'4 g'4 | fis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | a1 | c'1 | f1 | bes1 | c'1 | c'1 | d'1 | a1 | g1 | a1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 181 – dorisch – Gattung 3" opus = "gap_181, S. 126" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | d''1 | g'1 | b'1 | b'1 | d''1 | c''1 | e''1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d4 e4 f4 g4 | a4 f4 a4 b4 | c'4 e'4 d'4 c'4 | b4 g4 b4 d'4 | e'4 e4 f4 g4 | a4 d'4 c'4 bes4 | a4 f4 a4 b4 | c'4 g4 c'4 bes4 | a4 d4 f4 g4 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 182 – dorisch – Gattung 3" opus = "gap_182, S. 127" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | c''1 | b'1 | b'1 | d''1 | c''1 | e''1 | d''1 | cis''1 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" f1 | f1 | g1 | b1 | e'1 | a1 | a1 | e1 | a1 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d4 e4 f4 e4 | d4 f4 e4 d4 | c4 d4 e4 f4 | g4 g,4 g4 f4 | e4 g4 f4 e4 | d4 f4 d4 e4 | f4 g4 a4 f4 | c'4 c4 e4 c4 | d4 e4 f4 g4 | a4 g4 a4 a,4 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 183 – phrygisch – Gattung 3" opus = "gap_183, S. 128" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" b'4 e'4 b'4 c''4 | a'4 c''4 a'4 g'4 | f'4 d'4 e'4 f'4 | g'4 e'4 f'4 g'4 | a'4 f'4 a'4 b'4 | c''4 a'4 c''4 d''4 | e''4 b'4 e''4 d''4 | c''4 e''4 c''4 b'4 | a'4 d''4 a'4 b'4 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" gis1 | a1 | a1 | g1 | c'1 | a1 | b1 | c'1 | d'1 | b1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" e1 | f1 | d1 | e1 | f1 | f1 | e1 | a1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 184 – phrygisch – Gattung 3" opus = "gap_184, S. 129" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "treble" e''1 | c''1 | d''1 | c''1 | a'1 | a''1 | g''1 | e''1 | f''1 | e''1 \bar "|." }
    \new Staff { \time 2/2 \clef "soprano" gis'4 b'4 e'4 gis'4 | a'4 c''4 a'4 g'4 | f'4 d'4 e'4 f'4 | g'4 f'4 e'4 d'4 | c'4 c''4 a'4 g'4 | f'4 g'4 a'4 b'4 | c''4 b'4 e''4 b'4 | c''4 a'4 b'4 c''4 | a'4 d''4 a'4 b'4 | gis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" b1 | a1 | a1 | g1 | a1 | c'1 | e'1 | c'1 | d'1 | b1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e1 | f1 | d1 | e1 | f1 | f1 | e1 | a1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 185 – phrygisch – Gattung 3" opus = "gap_185, S. 130" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" b'1 | a'1 | a'1 | c''1 | c''1 | c''1 | e''1 | a'1 | a'1 | gis'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" gis4 b4 e4 gis4 | a4 c'4 a4 g4 | f4 a4 g4 f4 | g4 e4 e'4 d'4 | c'4 b4 a4 g4 | f4 g4 a4 b4 | c'4 d'4 e'4 d'4 | c'4 a4 c'4 b4 | a4 d4 d'4 c'4 | b1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" e1 | f1 | d1 | e1 | f1 | f1 | c1 | c1 | d1 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 186 – phrygisch – Gattung 3" opus = "gap_186, S. 130" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "treble" b'1 | e''1 | f''1 | e''1 | f''1 | f''1 | e''1 | e''1 | a'1 | gis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "soprano" gis'1 | a'1 | a'1 | c''1 | c''1 | c''1 | c''1 | c''1 | d''1 | b'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" e'1 | c'1 | d'1 | c'1 | a1 | a'1 | g'1 | e'1 | f'1 | e'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" e'4 d'4 c'4 b4 | a4 g4 f4 e4 | d4 e4 f4 d4 | a4 c'4 a4 g4 | f4 g4 a4 g4 | f4 g4 a4 b4 | c'4 e4 f4 g4 | a4 g4 f4 e4 | d4 f4 e4 d4 | e1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 193 – dorisch – Gattung 4" opus = "gap_193, S. 133" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 a'2~ | a'2 d''2~ | d''2 c''2~ | c''2 b'2~ | b'2 d''2~ | d''2 a'2~ | a'2 f''2~ | f''2 e''2~ | e''2 d''2~ | d''2 cis''2 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | a1 | a1 | d1 | d'1 | d'1 | c'1 | c'1 | a1 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | a,1 | b,1 | g,1 | d1 | f1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 195 – dorisch – Gattung 4" opus = "gap_195, S. 134" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | g'1 | d''1 | d''1 | f''1 | c''1 | bes'1 | a'1 | g'1 | fis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" f'1 | d'1 | e'1 | f'1 | bes'1 | a'1 | e'2 c'2 | g'1 | c'2 d'2 | e'1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 a2~ | a2 d'2~ | d'2 c'2~ | c'2 bes2~ | bes2 d'2~ | d'2 c'2~ | c'2 f'2~ | f'2 e'2~ | e'2 d'2~ | d'2 cis'2 | d'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 196 – dorisch – Gattung 4" opus = "gap_196, S. 134" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 a2~ | a2 d'2~ | d'2 c'2~ | c'2 b2~ | b2 d'2~ | d'2 a2~ | a2 f'2~ | f'2 e'2~ | e'2 d'2~ | d'2 cis'2 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" f1 | a1 | a1 | d1 | d'1 | d'1 | c'1 | c'1 | a1 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | a,1 | b,1 | g,1 | d1 | f1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 197 – dorisch – Gattung 4" opus = "gap_197, S. 135" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | c''2 b'2 | d''1 | b'2 c''2 | a'1 | d''1 | c''1 | a'1 | a'1 | a'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d1 | d1 | e1 | a1 | g1 | a1 | f1 | c1 | f1 | a1 | fis1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 d2~ | d2 a2~ | a2 g2~ | g2 f2~ | f2 e2~ | e2 d2~ | d2 f2~ | f2 e2~ | e2 d2~ | d2 cis2 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 200 – dorisch – Gattung 5" opus = "gap_200, S. 136" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" r2 a'2~ | a'4 f'4 a'4 b'4 | c''4 g'4 c''2~ | c''2 b'2~ | b'2 cis''2 | d''4 a'4 d''2~ | d''4 c''4 f''2~ | f''2 e''2~ | e''4 a'4 d''2~ | d''2 cis''2 | d''1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | f1 | g1 | g1 | g1 | a1 | a1 | c'1 | a1 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 201 – dorisch – Gattung 5" opus = "gap_201, S. 137" }
  \new StaffGroup <<
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 a2~ | a4 f4 a4 b4 | c'4 g4 c'2~ | c'2 b2~ | b4 g4 b4 cis'4 | d'4 d4 d'2~ | d'4 c'4 f'2~ | f'2 e'2~ | e'4 a4 d'2~ | d'2 cis'2 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" a1 | f1 | g1 | g1 | g1 | a1 | a1 | c'1 | a1 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" d1 | d1 | c1 | g1 | e1 | d1 | f1 | c1 | d1 | a,1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 202 – dorisch – Gattung 5" opus = "gap_202, S. 137" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" d''1 | d''1 | e''1 | a'1 | bes'1 | a'1 | a'1 | bes'1 | a'1 | g'1 | fis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" f'1 | a'1 | g'1 | f'1 | g'1 | c'1 | c'1 | g'1 | c'2 d'2 | e'1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 a2~ | a2 d'2~ | d'2 cis'2 | d'4 e'4 f'2~ | f'2 e'2 | f'2 f2~ | f2 f'2~ | f'4 e'8 d'8 e'2~ | e'2 d'2~ | d'2 cis'2 | d'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 203 – dorisch – Gattung 5" opus = "gap_203, S. 137" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'1 | a'1 | a'1 | b'1~ | b'2 cis''2 | d''1 | c''1 | c''1 | a'1 | a'1 | a'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "alto" d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" d1 | d'1 | c'1 | b1 | g1 | a1 | f'1 | e'1 | cis'2 d'2 | a1 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "bass" r2 d2~ | d4 e4 f4 g4 | a4 a,4 a2~ | a2 g2~ | g4 f4 e2 | d4 e4 f2~ | f4 g4 a4 b4 | c'4 c4 e2~ | e2 d2~ | d2 cis2 | d1 \bar "|." }
  >>
  \layout { }
}

\score {
  \header { piece = "Fig. 204 – dorisch – Gattung 2+3+4" opus = "gap_204, S. 138" }
  \new StaffGroup <<
    \new Staff { \time 2/2 \clef "soprano" a'2 d''2 | c''2 a'2 | b'2 c''2 | d''2 d'2 | bes'2 g'2 | a'2 f'2 | c''2 a'2 | bes'2 g'2 | a'2 f'2 | g'2 e'2 | fis'1 \bar "|." }
    \new Staff { \time 2/2 \clef "alto" d'4 e'4 f'4 g'4 | a'4 g'4 f'4 a'4 | g'4 f'4 e'4 g'4 | f'4 d'4 f'4 e'4 | d'4 c'4 bes4 d'4 | f'4 g'4 a'4 f'4 | e'4 d'4 c'4 a4 | d'4 c'4 bes4 g4 | c'4 bes4 a4 d'4 | e'4 e4 f4 g4 | a1 \bar "|." }
    \new Staff { \time 2/2 \clef "tenor" r2 a2~ | a2 d'2~ | d'2 c'2~ | c'2 bes2~ | bes2 d'2~ | d'2 c'2~ | c'2 f'2~ | f'2 e'2~ | e'2 d'2~ | d'2 cis'2 | d'1 \bar "|." }
    \new Staff \with { instrumentName = "C. f." } { \time 2/2 \clef "bass" d1 | f1 | e1 | d1 | g1 | f1 | a1 | g1 | f1 | e1 | d1 \bar "|." }
  >>
  \layout { }
}
