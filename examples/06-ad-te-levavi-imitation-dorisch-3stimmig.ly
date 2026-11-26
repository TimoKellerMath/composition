\version "2.24.0"
\language "nederlands"

\header {
  title = "dorisch – Ad te levavi"
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
  \header { piece = "Freier Satz 1 – dorisch" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice0" {
      \key d \dorian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | d'1\startGroup^\markup \small "1" | f'2 e'2~ | e'2 a'2 | g'2 e'2 | f'1\stopGroup | e'2. d'4 | c'2 d'2~ | d'2 \once \set suggestAccidentals = ##t cis'2 | d'1
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Ad" "te" "le" -- "vá" -- _ "vi" "á" -- "ni" -- "mam" "me" -- _ _ "am." }
    \new Staff = "staff1" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice1" {
      \key d \dorian \time 2/2 \clef "treble_8"
      d1\startGroup^\markup \small "1" | f2 e2~ | e2 a2 | g2\stopGroup b2~ | b4 c'4 b2~ | b4 b4 d'2~ | d'2 c'4 a4 | g4 f4 e4 \once \set suggestAccidentals = ##t fis4 | g2. b4 | a2 b2~ | b1 | a2 g2 | f4 e4 \once \set suggestAccidentals = ##t fis2~ | \once \set suggestAccidentals = ##t fis1
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Ad" "te" "le" -- "vá" -- _ _ _ _ "vi" "á" -- _ _ _ _ _ _ _ "ni" -- "mam" "me" -- _ _ "am." __ _ _ }
    \new Staff = "staff2" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice2" {
      \key d \dorian \time 2/2 \clef "bass"
      r1 | r1 | a,1\startGroup^\markup \small "1" | c2 b,2~ | b,2 e2 | d2 b,2\stopGroup | a,4 \once \set suggestAccidentals = ##t bes,4 c2~ | c4 b,4 c2~ | c4 d4 e2~ | e2 d2 | e1 | f4 c4 b,4 g,4 | a,1 | d1
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Ad" "te" "le" -- "vá" -- _ "vi" "á" -- _ _ _ _ "ni" -- "mam" __ "me" -- _ _ _ _ _ _ "am." }
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
