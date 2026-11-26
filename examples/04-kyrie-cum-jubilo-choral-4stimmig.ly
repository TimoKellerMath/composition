\version "2.24.0"
\language "nederlands"

\header {
  title = "dorisch – Kyrie I über Kyrie IX „Cum jubilo“"
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
      r1 | r1 | r1 | r2 a'2\startGroup^\markup \small "1" | c''1 | d''1 | e''1\stopGroup | c''1 | \once \set suggestAccidentals = ##t bes'2 a'2~ | a'2 g'4 e'4 | f'2 g'2~ | g'4 a'4 b'2 | a'2 \once \set suggestAccidentals = ##t bes'2 | \once \set suggestAccidentals = ##t bes'1 | g'4 a'4 b'4 c''4 | \once \set suggestAccidentals = ##t bes'4 g'4 f'2~ | f'1~ | f'4 e'4 g'2~ | g'4 f'4 e'2~ | e'2 f'2 | f'4 g'4 a'2~ | a'2 \once \set suggestAccidentals = ##t gis'2 | a'1 | b'2. c''4 | d''1 | e''4 c''4 \once \set suggestAccidentals = ##t bes'2~ | \once \set suggestAccidentals = ##t bes'2 a'2 | g'2 g'2~ | g'4 f'4 a'2~ | a'2 g'2 | g'2 f'2~ | f'2 a'2 | \once \set suggestAccidentals = ##t bes'2 d''2~ | d''2 \once \set suggestAccidentals = ##t cis''2 | d''1
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Ký" -- _ _ "ri" -- "e" "e" -- "lé" -- _ _ _ _ _ _ "i" -- "son." "Ký" -- _ _ _ _ _ _ _ _ _ "ri" -- "e" __ "e" -- "lé" -- _ _ "i" -- "son." "Ký" -- _ _ _ _ _ "ri" -- "e" "e" -- _ _ _ "lé" -- _ _ _ _ "i" -- "son." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key d \dorian \time 2/2 \clef "treble"
      d'1\startGroup^\markup \small "1" | f'1 | g'1 | a'1\stopGroup | f'2 f'2~ | f'2 d'2 | c'4 \once \set suggestAccidentals = ##t bes4 c'2~ | c'4 e'4 f'2~ | f'4 f'4 a'2 | g'2 e'2~ | e'2 d'2 | d'2. c'4 | d'1 | e'1 | e'4 c'4 d'4 e'4 | d'2. b4 | a2. c'4 | \once \set suggestAccidentals = ##t bes1 | r1 | r1 | r1 | r1 | r2 c'2\startGroup^\markup \small "2" | d'1 | f'1 | g'1 | f'1 | e'1 | d'2.\stopGroup f'4 | g'1 | r1 | r1 | f'2. g'4 | a'2. a'4 | \once \set suggestAccidentals = ##t fis'1
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Ký" -- _ _ "ri" -- "e" "e" -- "lé" -- _ _ _ _ _ "i" -- "son." "Ký" -- _ _ "ri" -- "e" "e" -- _ "lé" -- _ _ _ _ _ _ "i" -- "son." "e" -- _ "lé" -- _ _ "i" -- "son." __ _ _ "e" -- "lé" -- _ "i" -- "son." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key d \dorian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | r1 | r1 | r2 a2\startGroup^\markup \small "1" | c'1 | d'1 | e'1 | d'2\stopGroup b2 | b4 a4 g2~ | g4 f4 g2~ | g4 a4 \once \set suggestAccidentals = ##t bes2 | c'4 a4 g2~ | g4 g4 b2 | r1 | r2 \once \set suggestAccidentals = ##t bes2~ | \once \set suggestAccidentals = ##t bes4 a4 c'2~ | c'4 \once \set suggestAccidentals = ##t bes4 d'2~ | d'4 c'4 a2 | b1 | a1 | g1 | a2 f2 | r2 g2\startGroup^\markup \small "2" | a1 | c'1 | d'1 | c'1 | b1 | a2.\stopGroup c'4 | d'1 | e'1 | d'1
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Ký" -- _ _ "ri" -- "e" "e" -- "lé" -- _ _ _ _ _ _ _ _ _ "i" -- "son." "Ký" -- _ _ _ _ _ _ "ri" -- "e" __ _ _ _ "e" -- _ "lé" -- _ _ "i" -- "son." __ _ _ _ _ }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key d \dorian \time 2/2 \clef "bass"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 d2\startGroup^\markup \small "1" | f1 | g1 | a1 | g2\stopGroup d2 | d2. f4 | e1 | a,1 | r2 g2 | d1 | c2 g,2 | r1 | r1 | r1 | r1 | d1 | f4 e4 d4 c4 | \once \set suggestAccidentals = ##t bes,2 \once \set suggestAccidentals = ##t bes,2 | a,1 | d1
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Ký" -- _ _ "ri" -- "e" "e" -- "lé" -- "i" -- "son." __ _ "e" -- "lé" -- "i" -- "son." "e" -- "lé" -- _ _ _ _ "i" -- "son." __ _ }
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
