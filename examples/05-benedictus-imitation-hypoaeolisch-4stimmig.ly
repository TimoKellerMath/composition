\version "2.24.0"
\language "nederlands"

\header {
  title = "hypoäolisch – Benedictus"
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
  \header { piece = "Freier Satz 1 – hypoäolisch" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key a \aeolian \time 2/2 \clef "treble"
      a'1\startGroup^\markup \small "1" | g'1 | f'1 | a'2 b'2 | c''1\stopGroup | d''1 | e''2 d''2 | c''2 d''2~ | d''4 d''4 c''2~ | c''2 d''2 | c''2 b'2 | g'4 a'4 c''4 b'4 | a'2. a'4 | c''2. d''4 | e''2 e''2~ | e''4 d''4 b'2~ | b'4 a'4 g'2~ | g'2 a'2 | g'1 | a'2 c''2~ | c''2 b'2 | c''1~ | c''2 d''2~ | d''2 c''2 | r2 g'2 | e'2. d'4 | e'2. f'4 | g'2. g'4 | b'2 a'2~ | a'2 \once \set suggestAccidentals = ##t gis'2 | a'1
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Be" -- "ne" -- "dí" -- _ "ctus" "qui" "ve" -- _ _ _ _ "nit" "Be" -- _ _ _ _ _ _ _ _ "ne" -- "dí" -- "ctus" "qui" "ve" -- _ _ _ _ _ _ _ _ "nit" __ _ _ _ "in" "nó" -- _ _ _ _ "mi" -- "ne" "Dó" -- "mi" -- "ni." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key a \aeolian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | e'1\startGroup^\markup \small "1" | d'1 | c'1 | e'2 f'2 | g'1\stopGroup | a'2. f'4 | e'2 g'2~ | g'4 f'4 g'2 | f'1 | g'2 a'2 | a'2 g'2~ | g'2 f'4 d'4 | e'4 f'4 g'2~ | g'4 g'4 f'2 | e'2 e'2 | f'2 a'2~ | a'2 g'2~ | g'4 f'4 g'2~ | g'2 a'2 | r2 a2\startGroup^\markup \small "2" | g2 g2 | b2 c'2 | b1 | d'2.\stopGroup c'4 | d'2 f'2~ | f'2 e'2 | \once \set suggestAccidentals = ##t cis'1
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Be" -- "ne" -- "dí" -- _ "ctus" "qui" "ve" -- _ _ _ _ _ _ _ "nit" "Be" -- _ _ _ _ _ _ "ne" -- "dí" -- "ctus" "qui" "ve" -- _ _ _ _ "nit" "in" "nó" -- "mi" -- "ne" "Dó" -- _ _ _ _ _ "mi" -- "ni." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key a \aeolian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | a1\startGroup^\markup \small "1" | g1 | f1 | a2 b2 | c'1\stopGroup | d'2 f'2~ | f'4 e'4 d'2 | c'2. e'4 | f'2 d'2~ | d'4 c'4 e'2~ | e'4 e'4 c'2~ | c'2 b2 | c'2. c'4 | d'1 | c'1 | c'2 a2~ | a4 g4 e2~ | e2 e2 | g2. f4 | g1 | b1 | b2 d'2 | b1 | a1
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Be" -- "ne" -- "dí" -- _ "ctus" "qui" "ve" -- _ _ _ _ _ _ _ _ _ "nit" "Be" -- _ _ "ne" -- "dí" -- "ctus" "qui" "ve" -- "nit" "in" __ "nó" -- _ "mi" -- "ne" __ _ "Dó" -- _ "mi" -- "ni." }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key a \aeolian \time 2/2 \clef "bass"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | e1\startGroup^\markup \small "1" | d1 | c1 | e2 f2 | g1\stopGroup | f2. a4 | g1 | c1 | r2 d2\startGroup^\markup \small "2" | c2 c2 | e2\stopGroup e2~ | e4 d4 e2~ | e4 e4 g2~ | g1~ | g2 f4 d4 | e1 | a1
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Be" -- "ne" -- "dí" -- _ "ctus" "qui" "ve" -- _ _ "nit" "in" "nó" -- "mi" -- "ne" "Dó" -- _ _ "mi" -- "ni." __ _ _ _ _ }
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
