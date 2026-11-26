\version "2.24.0"
\language "nederlands"

\header {
  title = "äolisch – 5. Gattung, Cantus im Tenor"
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
  \header { piece = "Kontrapunkt 1 – äolisch" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key a \aeolian \time 2/2 \clef "treble"
      r2 a'2~ | a'4 g'4 f'2~ | f'4 g'8 a'8 b'2~ | b'4 c''4 d''2~ | d''4 c''8 b'8 c''2~ | c''4 d''4 e''2~ | e''2 d''2~ | d''4 c''4 b'2~ | b'4 a'8 g'8 f'2~ | f'4 e'4 e''2~ | e''2 b'2 | \once \set suggestAccidentals = ##t cis''1
      \bar "|."
    }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key a \aeolian \time 2/2 \clef "treble"
      e'1 | f'1 | d'1 | g'1 | e'1 | a'1 | a'1 | g'1 | f'1 | e'1 | \once \set suggestAccidentals = ##t gis'1 | a'1
      \bar "|."
    }
    \new Staff = "staff2" \with { instrumentName = \markup \center-column { "Tenor" \small "C. f." } shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key a \aeolian \time 2/2 \clef "treble_8"
      a1 | c'1 | b1 | d'1 | c'1 | e'1 | f'1 | e'1 | d'1 | c'1 | b1 | a1
      \bar "|."
    }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key a \aeolian \time 2/2 \clef "bass"
      a1 | a,1 | d1 | b,1 | c1 | c1 | d1 | e1 | f1 | a1 | e1 | a,1
      \bar "|."
    }
  >>
  \layout { }
}
