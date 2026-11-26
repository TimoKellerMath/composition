\version "2.24.0"
\language "nederlands"

\header {
  title = "hypomixolydisch – Agnus Dei I und II"
  tagline = ##f
}

\paper {
  #(set-paper-size "a4")
}

\layout {
  \context {
    \Lyrics
    \override LyricHyphen.minimum-distance = #0.5
    \override VerticalAxisGroup.nonstaff-unrelatedstaff-spacing.minimum-distance = #7
  }
}

\score {
  \header { piece = "Freier Satz 1 – hypomixolydisch" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key g \mixolydian \time 2/2 \clef "treble"
      d''1\startGroup^\markup \small "1" | d''1 | e''1 | d''2 b'2\stopGroup | b'4 g'4 f'4 e'4 | d'1 | e'2 d'2 | f'4 g'4 f'2~ | f'4 d'4 c'2 | c'2 d'2~ | d'2 e'4 c'4 | d'2. f'4 | e'2 f'2 | e'2 f'2~ | f'1 | g'2 a'2~ | a'2 f'2 | g'1~ | g'2 g'2 | r2 c''2~ | c''2 b'4 a'4 | f'2. g'4 | a'1 | a'4 g'4 c''2~ | c''2 b'2 | c''1 | b'2 b'2~ | b'2 a'2 | g'2 b'2~ | b'2 a'4 g'4 | a'2 c''2~ | c''2 b'2 | c''1 | b'2 b'2~ | b'2 a'4 f'4 | g'2. g'4 | c''2 c''2~ | c''2 b'2 | d''1 | d''2 c''2 | c''2 g'2~ | g'2 \once \set suggestAccidentals = ##t fis'2 | g'1
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "A" -- "gnus" "De" -- _ _ "i," __ _ _ _ _ "A" -- "gnus" "De" -- _ _ _ _ "i," "A" -- _ _ _ "gnus" "De" -- _ _ _ _ _ _ _ "i," "A" -- _ _ _ "gnus" "De" -- "i," __ _ _ _ _ _ "qui" __ "tol" -- _ _ _ _ _ _ _ "lis" "pec" -- "cá" -- _ _ _ "ta" "mun" -- "di:" __ "mi" -- "se" -- "ré" -- _ "re" "no" -- _ "bis." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key g \mixolydian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | g'1\startGroup^\markup \small "1" | g'1 | a'1 | g'2 e'2 | f'1 | e'2 f'2 | g'2\stopGroup e'2~ | e'2 d'2 | e'1 | r2 d'2\startGroup^\markup \small "2" | f'1 | g'1 | f'1 | a'2.\stopGroup g'8 \once \set suggestAccidentals = ##t fis'8 | g'2 g'2~ | g'4 f'4 e'2~ | e'4 d'4 f'2~ | f'2 f'2 | r2 g'2\startGroup^\markup \small "3" | g'2 a'2 | e'2 f'2~ | f'2 d'2 | e'2\stopGroup g'2~ | g'2 g'2 | f'4 e'4 d'2~ | d'1
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "A" -- "gnus" "De" -- _ _ _ _ _ _ _ _ "i," "qui" "tol" -- _ "lis" "pec" -- _ _ _ "cá" -- _ _ "ta" "mun" -- "di:" "mi" -- "se" -- "ré" -- _ "re" __ "no" -- _ _ "bis." __ _ _ _ }
    \new Staff = "staff2" \with { instrumentName = "Tenor 1" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key g \mixolydian \time 2/2 \clef "treble_8"
      r1 | r1 | r2 g2~\startGroup^\markup \small "1" | g2 g2~ | g2 a2~ | a2 g2 | e2 f2~ | f2\stopGroup f2 | a2 g2 | a2. f4 | g2. e4 | f4 g4 b2~ | b4 a4 b2~ | b4 c'4 d'2~ | d'1~ | d'2 c'2 | d'1 | e'1 | e'1 | r2 e'2 | d'2 b2 | b2 a2~ | a4 c'4 d'2 | c'2 c'2 | r1 | r2 g2\startGroup^\markup \small "2" | b1 | c'1 | b1 | d'2.\stopGroup e'4 | d'2 e'2~ | e'2 d'2 | e'1 | r2 d'2~\startGroup^\markup \small "3" | d'2 d'2 | e'2 b2 | c'1 | a2 b2\stopGroup | a1 | b2 g2 | g2. b4 | a1 | b1
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "A" -- "gnus" __ "De" -- _ _ _ "i," "A" -- _ _ _ _ _ _ _ _ "gnus" "De" -- _ _ _ _ _ "i," "A" -- _ "gnus" "De" -- _ _ _ _ "i," "qui" "tol" -- _ "lis" "pec" -- "cá" -- _ "ta" __ "mun" -- "di:" "mi" -- "se" -- "ré" -- _ "re" "no" -- _ "bis." "mi" -- "se" -- "ré" -- "re" "no" -- "bis." }
    \new Staff = "staff3" \with { instrumentName = "Tenor 2" shortInstrumentName = "T" }
    \new Voice = "voice3" {
      \key g \mixolydian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 d'2\startGroup^\markup \small "1" | d'1 | e'1 | d'2 b2 | c'1 | b2 c'2~\stopGroup | c'4 d'4 c'2 | b1 | a1 | a1 | c'2 c'2 | d'1 | c'1 | r2 g2 | a1 | b2 g2 | a4 b4 d'2~ | d'4 d'4 c'2 | d'1 | c'1 | r2 b2 | a2. a4 | b1 | b2 a2~ | a4 a4 f2~ | f1 | e4 d4 e2~ | e2 g2 | a1 | g1
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "A" -- "gnus" "De" -- _ _ _ _ _ _ _ _ "i," "A" -- "gnus" "De" -- _ "i," "qui" "tol" -- _ _ _ _ _ "lis" __ _ _ _ "pec" -- "cá" -- "ta" "mun" -- "di:" "mi" -- "se" -- "ré" -- _ _ _ "re" "no" -- "bis." }
    \new Staff = "staff4" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice4" {
      \key g \mixolydian \time 2/2 \clef "bass"
      r1 | r1 | r1 | r1 | r1 | r1 | r2 d2~\startGroup^\markup \small "1" | d2 d2~ | d2 e2~ | e2 d2 | b,2 c2~ | c2 b,2 | c2 d2\stopGroup | g4 f4 e4 d4 | f1 | e1 | f2 d2 | c2. d4 | e1 | a,1 | b,2 e2~ | e4 d4 a,2~ | a,2 b,2 | c2 e2 | g1 | c1 | d2 g2 | f4 e4 f2~ | f2 e2 | d2 d2~ | d4 d4 a2 | g1 | c1 | d2. e4 | f2. f4 | e2 e2~ | e2 a,4 b,4 | c2 d2~ | d1~ | d4 g,4 c2 | c2 b,2 | d1 | g,1
      \bar "|."
    }
    \new Lyrics = "text4" \lyricsto "voice4" { "A" -- "gnus" __ "De" -- _ _ _ _ _ _ "i," __ _ _ _ _ "A" -- _ _ _ "gnus" "De" -- _ _ _ _ _ _ _ _ _ "i," "A" -- "gnus" "De" -- _ _ "i," "qui" "tol" -- "lis" "pec" -- "cá" -- "ta" "mun" -- _ _ "di:" __ _ "mi" -- "se" -- _ _ "ré" -- _ _ "re" "no" -- _ "bis." }
  >>
  \layout {
    \context {
      \Voice
      \consists "Horizontal_bracket_engraver"
      \override HorizontalBracket.direction = #UP
      \override TextScript.outside-staff-priority = #1000
    }
  }
}
