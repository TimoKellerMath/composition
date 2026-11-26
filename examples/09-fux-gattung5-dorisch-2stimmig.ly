\version "2.24.0"
\language "nederlands"

\header {
  title = "dorisch – 5. Gattung"
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
  \header { piece = "Kontrapunkt 1 – dorisch" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key d \dorian \time 2/2 \clef "treble"
      r2 d''2~ | d''4 c''4 b'4 a'4 | c''4 b'4 c''2~ | c''4 b'8 a'8 b'2~ | b'4 c''4 d''2~ | d''4 c''8 b'8 c''2~ | c''4 d''4 e''2~ | e''2 d''2~ | d''4 c''4 d''2~ | d''2 \once \set suggestAccidentals = ##t cis''2 | d''1
      \bar "|."
    }
    \new Staff = "staff1" \with { instrumentName = \markup \center-column { "Alt" \small "C. f." } shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key d \dorian \time 2/2 \clef "treble"
      d'1 | f'1 | e'1 | d'1 | g'1 | f'1 | a'1 | g'1 | f'1 | e'1 | d'1
      \bar "|."
    }
  >>
  \layout { }
}
