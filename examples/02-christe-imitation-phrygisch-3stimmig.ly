\version "2.24.0"
\language "nederlands"

\header {
  title = "phrygisch – Christe"
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
  \header { piece = "Freier Satz 1 – phrygisch" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice0" {
      \key e \phrygian \time 2/2 \clef "treble"
      e'1\startGroup^\markup \small "1" | g'2 f'2 | e'2 f'2 | e'1 | c'1~\stopGroup | c'2 d'2 | f'2 g'2 | a'2. f'4 | g'2 f'2 | e'2 f'2 | g'4 e'4 d'2~ | d'2 c'2 | e'4 d'4 c'2~ | c'4 b4 a2~ | a2 \once \set suggestAccidentals = ##t gis2 | a1 | r1 | r1 | r1 | g'2 e'2~ | e'4 d'4 c'2~ | c'4 b4 c'2 | d'2 b2~ | b4 b4 d'2~ | d'4 e'4 e'2~ | e'2 d'2 | e'1
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Chri" -- "ste" "e" -- _ _ _ _ "lé" -- _ _ _ _ _ _ _ "i" -- "son." __ _ _ "e" -- "lé" -- _ _ _ _ "i" -- "son." "e" -- "lé" -- _ _ _ _ _ _ "i" -- "son." __ _ _ _ _ }
    \new Staff = "staff1" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice1" {
      \key e \phrygian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | a1\startGroup^\markup \small "1" | c'2 b2 | a2 b2 | a1\stopGroup | b2 b2~ | b4 g4 a2 | b4 c'4 a2~ | a2 g4 f4 | g2. a4 | f1 | r1 | r2 e'2\startGroup^\markup \small "2" | f'2 e'2~ | e'4 d'4 e'2 | d'1 | b2.\stopGroup c'4 | a2 g2 | b1 | r1 | r1 | b2 b2 | a2. a4 | \once \set suggestAccidentals = ##t gis1
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Chri" -- "ste" "e" -- _ _ _ _ "lé" -- _ _ _ _ _ _ _ _ "i" -- "son." "e" -- "lé" -- _ _ _ "i" -- "son." __ _ _ _ _ "e" -- "lé" -- _ "i" -- "son." }
    \new Staff = "staff2" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice2" {
      \key e \phrygian \time 2/2 \clef "bass"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | e1\startGroup^\markup \small "1" | g2 f2 | e2 f2 | e1 | c1\stopGroup | d2 f2 | e1 | a,1 | d2 c2 | b,2 c2 | g1 | r2 e2\startGroup^\markup \small "2" | f2 e2~ | e4 d4 e2 | d1 | b,2.\stopGroup a,4 | b,2 e2 | f1 | e1
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Chri" -- "ste" "e" -- _ _ _ _ "lé" -- _ "i" -- "son." "Chri" -- _ _ _ "ste" "e" -- "lé" -- _ _ _ "i" -- "son." __ _ _ _ _ _ }
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
