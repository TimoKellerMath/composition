\version "2.24.0"
\language "nederlands"

\header {
  title = "dorisch – Kyrie I ohne Imitation"
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
      a'1 | c''2 c''2~ | c''2 b'2 | a'2 c''2~ | c''4 \once \set suggestAccidentals = ##t bes'4 a'2~ | a'2 \once \set suggestAccidentals = ##t gis'2 | a'1 | r2 a'2 | f'1 | g'2. a'4 | g'1 | e'4 f'4 g'4 \once \set suggestAccidentals = ##t fis'4 | g'2 g'2~ | g'2 d'2~ | d'2 \once \set suggestAccidentals = ##t cis'2 | d'1
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Ký" -- "ri" -- "e" __ "e" -- "lé" -- _ _ _ "i" -- "son." "Ký" -- _ _ "ri" -- "e" "e" -- _ _ _ _ "lé" -- _ "i" -- "son." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key d \dorian \time 2/2 \clef "treble"
      r2 a'2~ | a'2 f'4 e'4 | d'2. d'4 | \once \set suggestAccidentals = ##t fis'1 | g'2 f'2 | e'2 e'2~ | e'4 c'4 d'2~ | d'4 d'4 c'2 | r2 c'2 | d'4 e'4 g'4 f'4 | e'4 c'4 b4 a4 | g2 g2 | \once \set suggestAccidentals = ##t bes2 c'2 | b4 a4 b2~ | b2 a2 | \once \set suggestAccidentals = ##t fis1
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Ký" -- _ _ _ "ri" -- "e" "e" -- _ _ "lé" -- _ _ "i" -- "son." "Ký" -- _ _ _ _ _ _ _ _ _ "ri" -- "e" "e" -- "lé" -- _ _ "i" -- "son." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key d \dorian \time 2/2 \clef "treble_8"
      r2 a2~ | a2 c'2 | b1 | d'4 c'4 b4 a4 | \once \set suggestAccidentals = ##t bes2 d'2 | b1 | a1 | \once \set suggestAccidentals = ##t bes2 a2 | a2 f2~ | f4 e4 d2 | r2 g2~ | g2 e4 d4 | e1 | d1 | e1 | d1
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Ký" -- _ _ _ _ _ _ _ _ "ri" -- "e" "e" -- _ "lé" -- _ "i" -- "son." "e" -- "lé" -- _ _ _ "i" -- "son." }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key d \dorian \time 2/2 \clef "bass"
      d2. c4 | a,1 | b,4 c4 d2~ | d2 a,2 | g,2 d2 | e1 | a,1 | r2 a,2~ | a,4 g,4 a,2 | g,1 | c2. b,4 | c1 | g,2. a,4 | b,2 b,2 | a,1 | d1
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Ký" -- "ri" -- "e" "e" -- _ _ "lé" -- _ _ "i" -- "son." "e" -- _ _ "lé" -- _ _ _ _ _ _ "i" -- "son." __ _ }
  >>
  \layout { }
}
