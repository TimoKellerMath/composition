\version "2.24.0"
\language "nederlands"

\header {
  title = "dorisch – Kyrie I"
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
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key d \dorian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | a'1\startGroup^\markup \small "1" | c''2 d''2 | e''1 | d''2 c''2 | d''1\stopGroup | d''4 c''4 \once \set suggestAccidentals = ##t bes'4 a'4 | f'1 | g'4 a'4 c''2~ | c''4 \once \set suggestAccidentals = ##t bes'4 c''2~ | c''4 a'4 \once \set suggestAccidentals = ##t bes'2~ | \once \set suggestAccidentals = ##t bes'2 a'2~ | a'2 \once \set suggestAccidentals = ##t gis'2 | a'1 | r2 d'2~ | d'4 c'4 d'2 | c'2. e'4 | d'1 | f'4 e'4 d'4 c'4 | d'2 d'2~ | d'2 \once \set suggestAccidentals = ##t cis'2 | d'1
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Ký" -- "ri" -- "e" "e" -- _ _ _ "lé" -- _ _ _ _ _ _ _ _ _ "i" -- "son." __ _ _ _ "e" -- _ _ "lé" -- _ _ _ _ _ _ _ "i" -- "son." __ _ }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key d \dorian \time 2/2 \clef "treble"
      d'1\startGroup^\markup \small "1" | f'2 g'2 | a'1\stopGroup | a'4 g'4 e'2~ | e'4 f'4 e'2~ | e'4 g'4 f'2 | g'2 e'2 | r2 a'2 | g'2 a'2 | f'1 | r1 | r1 | r2 a'2~ | a'2 f'2 | f'1 | e'4 d'4 c'4 b4 | c'2 a2 | g1 | r2 a2\startGroup^\markup \small "2" | a1 | b2 b2 | c'1~\stopGroup | c'2 \once \set suggestAccidentals = ##t bes2 | a2. a4 | \once \set suggestAccidentals = ##t fis1
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Ký" -- "ri" -- "e" "e" -- "lé" -- _ _ _ _ _ _ "i" -- "son." "e" -- "lé" -- "i" -- "son." "e" -- _ "lé" -- _ _ _ _ _ "i" -- "son." "e" -- "lé" -- _ "i" -- "son." __ "e" -- "lé" -- "i" -- "son." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key d \dorian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | a1\startGroup^\markup \small "1" | c'2 d'2 | e'1 | d'2 c'2 | d'1\stopGroup | d'1 | b1 | a1 | r2 d2\startGroup^\markup \small "2" | d1 | e2 e2 | f1\stopGroup | a1 | \once \set suggestAccidentals = ##t bes2 a4 g4 | e1 | d1
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Ký" -- "ri" -- "e" "e" -- _ _ _ "lé" -- "i" -- "son." "e" -- "lé" -- _ "i" -- "son." __ _ _ _ _ _ _ }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key d \dorian \time 2/2 \clef "bass"
      r1 | r1 | d1\startGroup^\markup \small "1" | f2 g2 | a1\stopGroup | e2 f2 | e2. e4 | b2 a2~ | a4 g4 d2~ | d4 e4 f2~ | f4 f4 \once \set suggestAccidentals = ##t bes2~ | \once \set suggestAccidentals = ##t bes2 a2 | \once \set suggestAccidentals = ##t bes2 a2 | f1 | f2. d4 | e1 | a,1 | b,1 | r1 | a,1~ | a,2 g,2 | f,2. c4 | \once \set suggestAccidentals = ##t bes,2 g,2 | a,1 | d1
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Ký" -- "ri" -- "e" "e" -- "lé" -- _ _ "i" -- "son." "Ký" -- _ _ _ _ "ri" -- "e" __ "e" -- "lé" -- _ "i" -- "son." __ _ _ _ _ "e" -- "lé" -- _ _ _ _ "i" -- "son." }
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
