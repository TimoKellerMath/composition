\version "2.24.0"
\language "nederlands"

\header {
  title = "ionisch – Gloria: Et in terra … Glorificamus te"
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
  \header { piece = "Freier Satz 1 – ionisch" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key c \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | r1 | f'1\startGroup^\markup \small "1" | a'1 | g'2 g'2~\stopGroup | g'2 f'2 | e'2 e'2 | d'2 c'2~ | c'2 e'2 | f'1~ | f'4 g'4 g'2~ | g'2 \once \set suggestAccidentals = ##t fis'2 | g'1 | b'4 c''4 d''2~ | d''4 b'4 c''2~ | c''4 c''4 a'2~ | a'2 g'4 f'4 | g'2 f'2 | g'1 | r2 g'2\startGroup^\markup \small "2" | g'1 | a'1 | g'1 | b'1\stopGroup | b'2 g'2 | g'4 f'4 d'4 e'4 | f'2 e'2~ | e'2 d'2 | e'1 | r1 | d'2 f'2 | f'2 e'2~ | e'4 d'4 e'2 | r2 d''2~ | d''4 b'4 a'2~ | a'2 g'2 | a'2 c''2~ | c''2 b'2 | c''1
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Et" "in" "ter" -- "ra" __ "pax" "ho" -- "mí" -- "ni" -- "bus" __ "bo" -- _ _ _ "nae" "vo" -- _ _ _ _ _ "lun" -- "tá" -- _ _ _ _ "tis." "Lau" -- "dá" -- _ "mus" "te." "Be" -- "ne" -- "dí" -- _ _ _ _ "ci" -- "mus" "te." "Ad" -- "o" -- "rá" -- _ "mus" "te." "Glo" -- _ _ "ri" -- "fi" -- "cá" -- "mus" "te." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key c \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | c'1\startGroup^\markup \small "1" | e'1 | d'2 d'2 | b2.\stopGroup c'4 | a4 g4 a2~ | a2 g2 | b1 | b2. c'4 | d'2 d'2~ | d'2 c'2 | b2. d'4 | e'2 g'2~ | g'2 f'2 | r2 c'2\startGroup^\markup \small "2" | c'1 | d'1 | c'1 | e'1\stopGroup | d'1~ | d'2 d'2 | b2. a4 | b1 | d'2 d'2~ | d'2 b2 | d'1 | r2 c'2\startGroup^\markup \small "3" | b2 g2 | a2 b2 | a1\stopGroup | r2 c'2~ | c'4 b4 d'2~ | d'2 c'2 | a4 b4 c'2~ | c'4 e'4 \once \set suggestAccidentals = ##t fis'2 | g'2. g'4 | e'1
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Et" "in" "ter" -- "ra" "pax" __ _ _ _ _ "ho" -- "mí" -- "ni" -- "bus" "bo" -- "nae" __ "vo" -- _ "lun" -- "tá" -- _ "tis." "Lau" -- "dá" -- _ "mus" "te." "Be" -- "ne" -- "dí" -- _ _ _ "ci" -- "mus" "te." "Ad" -- "o" -- "rá" -- _ "mus" "te." "Glo" -- _ _ "ri" -- _ _ _ "fi" -- "cá" -- _ "mus" "te." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key c \ionian \time 2/2 \clef "treble_8"
      f1\startGroup^\markup \small "1" | a1 | g2 g2\stopGroup | a4 b4 d'2~ | d'2 c'2 | b4 a4 f2~ | f4 g4 a2~ | a2 a2 | c'2. b4 | a4 g4 a2~ | a2 g2 | f4 d4 e2~ | e4 f4 g2 | f2. e4 | f2 g2 | a1 | g1 | r2 g2\startGroup^\markup \small "2" | g1 | a1 | g1\stopGroup | g4 e4 f2~ | f1 | e2 e2 | g2. g4 | \once \set suggestAccidentals = ##t fis1 | g1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 g2~\startGroup^\markup \small "3" | g2 f2 | d2 e2 | f2 e2~ | e2\stopGroup d2 | d2 f2 | e2 e2 | f4 g4 a4 b8 c'8 | d'1 | c'1
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Et" "in" "ter" -- "ra" "pax" __ _ _ "ho" -- "mí" -- _ _ "ni" -- "bus" __ "bo" -- _ _ _ _ _ "nae" "vo" -- _ _ "lun" -- "tá" -- _ _ _ _ _ "tis." "Lau" -- "dá" -- _ "mus" "te." __ _ _ "Be" -- "ne" -- "dí" -- "ci" -- "mus" "te." "Ad" -- "o" -- "rá" -- _ "mus" "te." __ "Glo" -- "ri" -- "fi" -- "cá" -- "mus" "te." __ _ _ _ _ _ _ }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key c \ionian \time 2/2 \clef "bass"
      r1 | c1\startGroup^\markup \small "1" | e1 | d2 d2 | b,2.\stopGroup c4 | d1 | d1 | c4 b,4 c2~ | c2 c2 | f2 f2 | g2. c4 | d4 b,4 c4 d4 | e2 e2~ | e1 | d2. e4 | d1 | g,1 | r1 | r1 | r1 | r2 e2~ | e2 d4 c4 | b,2. g,4 | a,2 c2~ | c4 b,4 c2~ | c1 | b,2 g,2~ | g,1~ | g,4 a,4 b,2~ | b,4 c4 d2~ | d4 d4 g2 | f1 | e1 | g1 | f2 d2~ | d2 c4 b,4 | a,2. c4 | b,2 b,2~ | b,4 g,4 a,2 | c2. c4 | a,1 | g,1 | c1
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Et" "in" "ter" -- "ra" "pax" __ _ _ "ho" -- "mí" -- _ _ "ni" -- "bus" "bo" -- _ _ _ _ _ _ _ "nae" __ "vo" -- "lun" -- "tá" -- "tis." "Lau" -- "dá" -- _ _ "mus" "te." "Be" -- "ne" -- "dí" -- _ _ _ _ _ _ "ci" -- "mus" "te." __ _ _ "Ad" -- "o" -- "rá" -- _ _ "mus" "te." "Glo" -- _ _ "ri" -- "fi" -- "cá" -- "mus" "te." }
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
