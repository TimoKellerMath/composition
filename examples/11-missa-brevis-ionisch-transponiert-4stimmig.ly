\version "2.24.0"
\language "nederlands"

#(set-global-staff-size 18)

\header {
  tagline = ##f
}

\paper {
  #(set-paper-size "a4")
  min-systems-per-page = #2
}

\layout {
  \context {
    \Lyrics
    \override LyricHyphen.minimum-distance = #0.5
    \override VerticalAxisGroup.nonstaff-unrelatedstaff-spacing.minimum-distance = #7
  }
}

\bookpart {
\header { title = "Missa brevis"
  subtitle = "ionisch, transponiert nach F, S A T B"
  composer = "fux-kontrapunkt, Seed 2"
}
\markup \fill-line { \override #'(font-size . 4) \bold "Kyrie" }
\markup \fill-line { \center-column { \italic "Kyrie I, Christe und Kyrie II stehen je für drei Anrufungen" \italic "(Wiederholung oder Wechsel mit dem Choral nach Ortsbrauch)." } }

\score {
  \header { piece = "Kyrie I" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | f'1\startGroup^\markup \small "1" | f'2 a'2 | a'1 | g'1\stopGroup | f'2 a'2 | g'2 g'2~ | g'2 e'2 | f'1 | a'4 g'4 f'2~ | f'4 f'4 g'2~ | g'2 f'2 | f'2 g'2 | g'2. a'4 | g'2 bes'2~ | bes'2 a'4 g'4 | a'2 g'2~ | g'2 f'4 e'4 | e'4 e'4 f'2~ | f'2 e'2 | f'1~ | f'1 | e'2 g'2 | a'2 f'2 | e'2 g'2 | a'2. a'4 | g'2. e'4 | f'2 a'2~ | a'4 g'4 f'2~ | f'4 f'4 e'2~ | e'2 f'2~ | f'2 e'2 | f'1\fermata
      \bar "||"
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Ký" -- "ri" -- "e" "e" -- _ _ _ _ "lé" -- _ _ _ _ _ "i" -- "son." __ "Ký" -- "ri" -- "e" "e" -- _ _ "lé" -- _ _ _ _ _ _ _ _ _ "i" -- "son." __ "Ký" -- _ _ _ _ _ "ri" -- "e" "e" -- _ _ "lé" -- _ _ "i" -- "son." __ _ _ _ }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      f1\startGroup^\markup \small "1" | f2 a2 | a1 | g1 | bes1 | a1\stopGroup | a2 f2 | g4 a4 bes4 g4 | a1 | c'4 bes4 c'2~ | c'2 c'2 | a1 | a2. bes4 | a2 bes2 | c'2 c'2~ | c'4 d'4 c'2 | r1 | r1 | r1 | r1 | r1 | r1 | c'2. c'4 | a1 | r2 c'2~\startGroup^\markup \small "2" | c'2 d'2~ | d'2 c'2 | bes1 | c'2 c'2\stopGroup | r1 | r1 | r1 | r1 | c'2 bes2 | c'2. c'4 | a1\fermata
      \bar "||"
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Ký" -- "ri" -- "e" "e" -- _ _ _ "lé" -- _ _ _ _ _ _ _ _ _ "i" -- "son." "Ký" -- "ri" -- "e" "e" -- _ "lé" -- "i" -- "son." "Ký" -- "ri" -- "e" "e" -- "lé" -- _ _ "i" -- "son." "e" -- "lé" -- _ "i" -- "son." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 c'2~\startGroup^\markup \small "1" | c'2 c'2 | e'2 e'2~ | e'2 d'2~ | d'2 f'2~ | f'2 e'2~ | e'2\stopGroup d'2 | bes1 | g1 | f1 | a2 a2 | c'4 bes4 a4 g4 | f2 a2~ | a4 g4 bes2 | r2 f2\startGroup^\markup \small "2" | g1 | f2 e2~ | e2 f2 | f2\stopGroup a2 | g2 f2 | g1 | f1\fermata
      \bar "||"
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Ký" -- "ri" -- "e" "e" -- _ _ _ "lé" -- _ "i" -- "son." "e" -- "lé" -- _ _ _ _ _ _ "i" -- "son." "e" -- "lé" -- _ _ "i" -- "son." "e" -- "lé" -- _ "i" -- "son." }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key f \ionian \time 2/2 \clef "bass"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 c2~\startGroup^\markup \small "1" | c2 c2 | e2 e2~ | e2 d2~ | d2 f2~ | f2 e2~ | e2\stopGroup a2 | a2 e2~ | e4 d4 e2~ | e4 c4 d2~ | d4 g4 f2 | d2 e2~ | e4 f4 g2~ | g2 d2 | c1 | f1 | r1 | r1 | r1 | r2 g2 | f2 f2~ | f4 e4 c2~ | c4 bes,4 c2 | c2 d2~ | d4 bes,4 c2~ | c2 d2 | c1 | f1\fermata
      \bar "||"
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Ký" -- "ri" -- "e" "e" -- _ _ _ _ "lé" -- _ _ _ _ _ _ _ _ _ "i" -- "son." __ _ _ _ "Ký" -- _ "ri" -- "e" "e" -- _ _ "lé" -- _ _ _ _ "i" -- "son." }
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

\score {
  \header { piece = "Christe" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \set Score.currentBarNumber = #37
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | r1 | c''1\startGroup^\markup \small "1" | bes'2 a'2 | d''1 | c''2\stopGroup e''2~ | e''4 d''4 e''2~ | e''2 d''2 | d''4 c''4 d''2~ | d''2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | bes'2 a'2~ | a'4 g'4 f'2 | e'2. g'4 | f'4 e'4 f'2~ | f'2 e'2 | e'2 g'2~ | g'2 f'2 | g'2 g'2 | a'2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1\fermata
      \bar "||"
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Chri" -- "ste" "e" -- _ _ "lé" -- "i" -- "son." __ "e" -- "lé" -- _ _ _ "i" -- "son." "Chri" -- _ _ _ _ _ _ _ _ _ "ste" "e" -- _ _ "lé" -- _ _ "i" -- "son." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | c'1\startGroup^\markup \small "1" | bes2 a2 | d'1 | c'2\stopGroup c'2 | \once \set suggestAccidentals = ##t b2 d'2 | r1 | d'1 | d'2 c'2~ | c'1 | r2 c'2\startGroup^\markup \small "2" | bes2. a4 | bes1 | c'1 | d'1~\stopGroup | d'2 c'2 | d'2 d'2 | e'1\fermata
      \bar "||"
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Chri" -- "ste" "e" -- _ _ "lé" -- "i" -- "son." "Chri" -- "ste" __ _ "e" -- "lé" -- _ _ "i" -- "son." __ "e" -- "lé" -- "i" -- "son." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      c'1\startGroup^\markup \small "1" | bes2 a2 | d'1 | c'2\stopGroup bes2 | c'2 d'2 | bes2 c'2~ | c'2 e'2 | f'2. f'4 | d'2 d'2 | e'1 | e'2 c'4 bes4 | g2 a2 | f1 | f4 e4 g4 a4 | d'1 | c'1 | r1 | r1 | r1 | r1 | r2 g2~ | g4 g4 bes2~ | bes4 a4 c'2~ | c'2 bes2 | c'2. a4 | d'1 | c'1\fermata
      \bar "||"
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Chri" -- "ste" "e" -- _ _ "lé" -- _ _ _ _ _ _ "i" -- "son." "e" -- _ "lé" -- _ _ _ _ "i" -- "son." __ _ _ _ _ _ "Chri" -- "ste" "e" -- _ _ "lé" -- _ "i" -- "son." __ _ }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key f \ionian \time 2/2 \clef "bass"
      r1 | r1 | r1 | f1\startGroup^\markup \small "1" | e2 d2 | g1 | f2\stopGroup e2~ | e4 d4 f2~ | f4 g4 bes2~ | bes4 a4 g2 | r1 | bes,2 f2~ | f2 e4 d4 | e2. f4 | g1 | c1 | r2 f2\startGroup^\markup \small "2" | e2. d4 | e1 | f1 | g1\stopGroup | r1 | f2 a2 | g2 bes2 | a2. a4 | g1 | c1\fermata
      \bar "||"
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Chri" -- "ste" "e" -- _ _ "lé" -- _ _ _ _ "i" -- "son." "Chri" -- _ _ _ _ _ _ "ste" "e" -- "lé" -- _ _ "i" -- "son." "e" -- "lé" -- _ _ _ "i" -- "son." __ _ }
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

\score {
  \header { piece = "Kyrie II" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \set Score.currentBarNumber = #64
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 f'2~\startGroup^\markup \small "1" | f'2 a'2 | a'2 g'2 | a'2. bes'4 | c''2 a'2~ | a'4\stopGroup g'4 e'2~ | e'2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r2 f'2~ | f'4 e'4 f'2~ | f'2 g'2 | g'4 f'4 g'2~ | g'2 f'2~ | f'4 e'4 f'2~ | f'2 e'2 | f'1\fermata
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Ký" -- "ri" -- "e" "e" -- _ _ _ _ "lé" -- _ _ "i" -- "son." "e" -- _ _ _ "lé" -- _ _ _ _ _ "i" -- "son." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | bes1\startGroup^\markup \small "1" | d'2 d'2 | c'2 d'2~ | d'4 e'4\stopGroup d'2 | c'2 c'2~ | c'2 bes4 a4 | c'2. d'4 | e'4 d'4 c'2~ | c'4 \once \set suggestAccidentals = ##t b4 g2~ | g2 g2 | f1 | r2 f2\startGroup^\markup \small "2" | f2 bes2 | a2 g2 | f2\stopGroup a2 | bes2. bes4 | a1~ | a1\fermata
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Ký" -- "ri" -- "e" "e" -- _ _ _ _ "lé" -- _ _ _ _ _ _ _ _ _ "i" -- "son." "e" -- "lé" -- _ _ "i" -- "son." "e" -- "lé" -- "i" -- "son." __ }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      r1 | f1\startGroup^\markup \small "1" | a2 a2 | g2 a2~ | a4 bes4 c'2 | a2.\stopGroup g4 | a1 | c'4 bes4 c'2~ | c'4 a4 bes2~ | bes4 bes4 g2 | a2 a2~ | a2 f2 | g2. bes4 | a2. g4 | a2. f4 | g1 | g2 e2 | d1 | c1 | r2 f2\startGroup^\markup \small "2" | f2 bes2 | a2 g2 | f2\stopGroup e2~ | e1 | d2 f4 g4 | g1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Ký" -- "ri" -- "e" "e" -- _ _ _ _ "lé" -- _ _ _ _ _ _ "i" -- "son." "e" -- "lé" -- _ _ _ _ _ _ _ _ "i" -- "son." __ _ _ "e" -- "lé" -- _ _ "i" -- "son." __ _ _ _ _ _ _ }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key f \ionian \time 2/2 \clef "bass"
      bes,1\startGroup^\markup \small "1" | d2 d2 | c2 d2~ | d4\stopGroup bes,4 c2 | c4 d4 c2~ | c4 bes,4 c2~ | c2 f4 e4 | c1 | d1 | g2 g2~ | g2 f4 e4 | f1 | bes,4 c4 g2~ | g2 f4 e4 | c2 d2 | c2 e2 | e2. c4 | g,1 | c1 | d2 bes,2~ | bes,1 | r1 | d2 e2~ | e2 d2 | bes,2. bes,4 | c1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Ký" -- "ri" -- "e" "e" -- _ _ _ "lé" -- _ _ _ _ _ _ _ "i" -- "son." "Ký" -- _ _ _ _ _ _ _ _ _ "ri" -- "e" "e" -- "lé" -- "i" -- "son." __ _ _ _ "e" -- "lé" -- _ _ "i" -- "son." __ _ }
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
}

\bookpart {
\markup \fill-line { \override #'(font-size . 4) \bold "Gloria" }
\markup \fill-line { \center-column { \italic "Intonatio: Glória in excélsis Deo." } }

\score {
  \header { piece = "Et in terra pax" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | c''1\startGroup^\markup \small "1" | c''2 bes'2~ | bes'2 a'2 | d''2\stopGroup d''2~ | d''4 e''4 d''2 | d''1 | bes'1~ | bes'2 a'4 g'4 | f'2. f'4 | f'1~ | f'2 e'2 | f'1 | g'4 a'4 bes'2~ | bes'4 c''4 bes'2 | d''4 c''4 d''2~ | d''4 d''4 c''2 | bes'2 a'2~ | a'4 g'4 f'2 | f'1 | f'2 a'2 | a'1 | f'2 e'4 c'4~ | c'2 \once \set suggestAccidentals = ##t b2 | c'1 | r2 bes'2~ | bes'4 a'4 g'2 | bes'2 c''2 | \once \set suggestAccidentals = ##t b'1 | \once \set suggestAccidentals = ##t b'1~ | \once \set suggestAccidentals = ##t b'2 a'2 | c''2 bes'2 | a'2 f'2~ | f'2 e'2 | f'1 | r2 f'2~ | f'2 f'2 | a'2 a'2~ | a'2 g'4 f'4 | g'2. g'4 | f'2 f'2~ | f'2 e'2 | f'1 | f'2 e'2~ | e'2 e'2 | f'2. a'4 | bes'1~ | bes'2 f'2~ | f'2 e'2 | f'1~ | f'2 e'2 | f'2. a'4 | bes'2 d''2~ | d''2 \once \set suggestAccidentals = ##t cis''2 | d''1 | d''2 c''4 d''4~ | d''2 \once \set suggestAccidentals = ##t cis''2 | d''1 | r2 g'2 | g'2 f'2 | g'2. g'4 | bes'2 bes'2 | d''2. \once \set suggestAccidentals = ##t cis''4 | d''2 e''4 c''4~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r1 | r1 | r1 | r1 | r2 c''2~\startGroup^\markup \small "4" | c''2 bes'2 | g'1 | a'2 f'2~ | f'2 g'2\stopGroup | bes'2. bes'4 | g'2 f'2~ | f'4 f'4 a'2~ | a'4 g'4 e'2 | e'1 | g'4 f'4 g'2~ | g'4 a'4 g'2~ | g'2 a'2 | g'2 f'2~ | f'2 e'2 | f'1 | r1 | r2 f'2 | g'2. a'4 | g'2 a'2 | g'2 f'2~ | f'4 e'4 f'2 | f'1 | a'2 a'2 | f'2. f'4 | g'2 f'2~ | f'4 e'4 f'2~ | f'2 e'2 | f'1 | r1 | \mark \markup \italic "Qui tollis" f'1 | e'2 e'2~ | e'2 g'2 | g'4 a'4 g'2~ | g'4 f'4 a'2~ | a'2 g'2 | a'1 | g'2. g'4 | bes'2 a'2~ | a'4 g'4 bes'2~ | bes'2 a'2 | bes'1 | bes'2 d''2~ | d''4 c''4 a'2~ | a'4 bes'4 a'2 | d''2 d''2 | a'2 bes'2~ | bes'2 a'2 | bes'1 | bes'2 a'4 f'4~ | f'2 e'2 | f'1 | f'2. e'8 f'8 | g'4 a'4 bes'2~ | bes'2 bes'2 | d''4 c''4 bes'2~ | bes'4 g'4 a'2 | f'2 e'4 c'4~ | c'2 \once \set suggestAccidentals = ##t b2 | c'1 | r1 | r1 | r1 | r2 c''2~\startGroup^\markup \small "6" | c''2 d''2 | c''1 | bes'2 g'2 | a'2 f'2\stopGroup | g'2. bes'4 | c''2 a'2 | bes'2 a'2~ | a'2 g'4 f'4 | g'2 f'2~ | f'4 a'4 g'2 | a'1 | a'1~ | a'4 g'4 f'2~ | f'2 e'2 | f'1 | g'2 g'2~ | g'4 f'4 g'2~ | g'2 f'2 | g'2 c''2~ | c''4 bes'4 d''2~ | d''4 d''4 e''2~ | e''2 d''2 | a'2 a'2~ | a'1 | r2 c''2\startGroup^\markup \small "7" | a'2 g'2 | f'2 a'2 | g'2 a'2 | bes'1\stopGroup | c''2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | e''2 d''2~ | d''1~ | d''2 c''2 | r2 g'2 | g'2 f'2 | a'2. g'4 | a'2 f'2~ | f'2 e'2 | f'1 | r2 c''2~\startGroup^\markup \small "9" | c''2 c''2 | d''2 bes'2~\stopGroup | bes'2 a'2 | f'2 e'2~ | e'4 g'4 f'2 | e'1 | g'1 | g'4 a'4 bes'4 c''8 d''8 | c''2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r2 f'2~ | f'2 e'2 | g'2. f'4 | e'2. g'4 | a'4 bes'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1~ | c''2 bes'2 | bes'2 bes'2~ | bes'2 a'2 | bes'1 | a'2 bes'2~ | bes'2 bes'2 | g'4 f'4 a'2~ | a'4 g'4 a'2~ | a'1 | c''2 c''2 | r2 e'2 | e'2. f'4 | g'2. g'4 | f'2 f'2~ | f'2 e'2 | f'1 | f'1~ | f'2 e'2 | f'1\fermata
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Et" "in" "ter" -- "ra" "pax" "ho" -- _ _ "mí" -- _ _ _ _ "ni" -- "bus" __ "bo" -- _ _ _ _ "nae" "vo" -- _ _ _ "lun" -- "tá" -- "tis." "Lau" -- _ _ "dá" -- "mus" "te." "Be" -- "ne" -- "dí" -- _ "ci" -- "mus" "te." __ _ _ "Ad" -- "o" -- "rá" -- "mus" __ "te." "Glo" -- "ri" -- "fi" -- "cá" -- "mus" "te." "Grá" -- "ti" -- "as" "á" -- _ _ _ "gi" -- "mus" "ti" -- "bi" "pro" -- "pter" "ma" -- "gnam" "gló" -- "ri" -- "am" __ "tu" -- _ "am." __ "Dó" -- _ "mi" -- "ne" "De" -- "us," "Rex" "cae" -- "lé" -- _ _ "stis," "De" -- "us" "Pa" -- _ "ter" "o" -- "mní" -- _ _ _ _ _ "pot" -- "ens." "Dó" -- "mi" -- "ne" "Fi" -- _ _ _ "li" "u" -- _ "ni" -- "gé" -- "ni" -- "te" "Je" -- _ _ _ "su" "Chri" -- _ _ _ _ "ste." "Dó" -- _ "mi" -- "ne" "De" -- "us," "A" -- _ _ "gnus" "De" -- "i," "Fí" -- "li" -- "us" "Pa" -- _ _ _ "tris." "Qui" "tol" -- "lis" __ "pec" -- "cá" -- _ _ "ta" "mun" -- _ "di," "mi" -- "se" -- "ré" -- _ "re" "no" -- _ "bis." "Qui" "tol" -- _ _ "lis" "pec" -- "cá" -- "ta" "mun" -- _ _ "di," "sús" -- _ _ "ci" -- "pe" "de" -- _ _ _ _ _ "pre" -- "ca" -- _ _ "ti" -- "ó" -- "nem" "no" -- _ _ "stram." "Qui" __ "se" -- _ _ _ "des" "ad" "déx" -- "te" -- "ram" "Pa" -- _ _ _ _ _ _ _ "tris," "Qui" "se" -- _ _ _ "des" "ad" "déx" -- _ _ _ _ _ _ _ "te" -- "ram" __ "Pa" -- _ "tris," __ "mi" -- "se" -- "ré" -- _ _ "re" "no" -- _ _ "bis." __ _ _ _ _ _ "Quó" -- "ni" -- "am" "tu" "so" -- "lus" "san" -- _ "ctus." "Tu" __ "so" -- "lus" "Dó" -- "mi" -- "nus." "Tu" __ _ _ "so" -- "lus" "Dó" -- _ _ _ _ _ "mi" -- "nus." __ _ "Tu" __ "so" -- "lus" "Al" -- "tís" -- _ _ _ _ "si" -- "mus," __ "Je" -- "su" "Chri" -- _ "ste." "Cum" "San" -- "cto" "Spí" -- _ _ _ _ "ri" -- "tu," "in" "gló" -- "ri" -- "a" "De" -- "i" "Pa" -- _ "tris." "A" -- _ "men." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | c'1\startGroup^\markup \small "1" | c'2 bes2~ | bes2 a2\stopGroup | c'2 c'2~ | c'2 d'2 | bes2 c'2 | d'2 bes2~ | bes2 bes2 | d'1 | d'2. bes4 | c'2. c'4 | a2 a2~ | a2 \once \set suggestAccidentals = ##t b2 | c'4 bes4 c'2~ | c'2 d'2 | bes1 | r1 | r1 | r1 | r2 f2\startGroup^\markup \small "2" | bes2. a4 | f1 | f1 | a1\stopGroup | r2 c'2 | \once \set suggestAccidentals = ##t b2 d'2 | e'2 c'2 | d'1 | r2 bes2\startGroup^\markup \small "3" | bes2 \once \set suggestAccidentals = ##t es'2 | d'2 e'2 | d'1 | c'2.\stopGroup c'4 | c'1 | a2 bes2 | c'2. c'4 | a1 | r2 c'2~ | c'2 d'4 \once \set suggestAccidentals = ##t b4 | c'2 a2 | a1 | g1 | a2 bes2 | c'2 c'2~ | c'4 bes4 a2 | c'2 c'2~ | c'2 a4 \once \set suggestAccidentals = ##t b4 | c'2. c'4 | d'1 | bes1 | c'2 c'2~ | c'4 d'4 c'2~ | c'4 bes4 c'2 | d'1 | d'4 bes4 a2~ | a2 a2 | f2. f4 | a2 g2 | a2 a2 | f2. f4 | g1 | a4 bes4 d'2~ | d'4 c'4 bes2 | bes2 d'2~ | d'1~ | d'2 c'2 | bes4 a4 g2~ | g1 | r1 | r1 | r1 | r1 | r1 | r2 f'2~\startGroup^\markup \small "4" | f'2 e'2 | c'1 | d'2\stopGroup c'2 | d'2 bes2~ | bes4 bes4 a2 | c'2 c'2~ | c'4 bes4 g2~ | g4 a4 g2~ | g2 g2 | r1 | r1 | r1 | r1 | r1 | r2 c'2~\startGroup^\markup \small "5" | c'2 bes2 | d'2 c'2~ | c'4 bes4 c'2\stopGroup | c'4 bes4 a2~ | a2 c'2 | d'2 d'2~ | d'4 c'4 d'2 | c'1 | d'2 d'2 | c'2 d'2~ | d'2 c'2 | a1 | r1 | c'1 | \once \set suggestAccidentals = ##t b1 | c'2. bes4 | c'1 | d'2. c'4 | d'2. d'4 | e'1 | d'2. c'4 | d'2. d'4 | c'2 bes2 | r2 c'2 | d'2 bes2~ | bes2 bes2 | d'2 d'2 | r2 d'2~ | d'2 d'2 | c'2 d'2~ | d'2 c'2 | d'2 d'2 | c'4 bes4 c'2~ | c'2 c'2 | a2. a4 | c'1 | d'2. d'4 | d'2. c'4 | d'2. d'4 | c'1 | d'2 c'2 | r2 g2 | g1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 f'2~\startGroup^\markup \small "6" | f'2 g'2 | f'1 | e'2 c'2 | d'2 bes2\stopGroup | c'1 | a1 | g1 | a2 bes2~ | bes2 a2 | bes2 a2~ | a4 g4 bes2 | r2 c'2 | d'1 | d'4 c'4 a2 | r2 c'2\startGroup^\markup \small "7" | a2 g2 | f2\stopGroup g2 | a2 f2 | r1 | r1 | r1 | r2 d'2 | e'2 d'2 | c'2 d'2~ | d'2 bes2 | r2 f2~\startGroup^\markup \small "8" | f2 g2 | g2 a2~ | a4\stopGroup c'4 bes2 | a1 | c'2 c'2~ | c'4 bes4 a2 | g2 a2~ | a4 bes4 a2 | bes2 bes2~ | bes4 a8 g8 f2 | a1 | a4 g4 bes2 | r2 c'2\startGroup^\markup \small "9" | c'2 d'2 | bes2\stopGroup bes2 | g1~ | g1~ | g1 | r2 c'2~ | c'2 c'2 | d'2. d'4 | c'1~ | c'1 | \once \set suggestAccidentals = ##t b2 d'2 | e'1 | d'2 d'2~ | d'1~ | d'2 c'2 | d'2 d'2~ | d'4 c'4 bes2 | d'1 | r1 | r1 | c'1 | c'2. bes4 | a1 | c'1 | c'2. bes4 | a1 | c'2 c'2~ | c'2 bes2 | c'2 d'2 | d'2 c'2 | a1\fermata
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Et" "in" "ter" -- "ra" "pax" "ho" -- "mí" -- _ _ _ _ "ni" -- "bus" "bo" -- _ _ "nae" "vo" -- "lun" -- "tá" -- _ _ _ _ "tis." "Lau" -- "dá" -- _ _ "mus" "te." "Be" -- "ne" -- "dí" -- "ci" -- "mus" "te." "Ad" -- "o" -- "rá" -- _ _ "mus" "te." "Glo" -- "ri" -- "fi" -- "cá" -- _ "mus" "te." "Grá" -- _ _ _ "ti" -- "as" "á" -- _ "gi" -- "mus" "ti" -- "bi" "pro" -- "pter" "ma" -- _ _ _ "gnam" "gló" -- "ri" -- "am" "tu" -- "am." "Dó" -- "mi" -- "ne" "De" -- "us," __ _ _ "Rex" __ _ "cae" -- "lé" -- _ "stis," "De" -- _ "us" "Pa" -- _ _ _ "ter" "o" -- "mní" -- _ "pot" -- "ens." __ _ _ "Dó" -- "mi" -- "ne" "Fi" -- "li" "u" -- _ "ni" -- "gé" -- _ "ni" -- "te" "Je" -- "su" "Chri" -- "ste." "Dó" -- "mi" -- "ne" "De" -- _ _ "us," __ _ _ "A" -- "gnus" "De" -- _ _ "i," "Fí" -- "li" -- "us" "Pa" -- _ "tris." "Qui" "tol" -- _ _ _ _ _ _ "lis" "pec" -- "cá" -- _ _ "ta" "mun" -- "di," "mi" -- "se" -- "ré" -- "re" "no" -- "bis." "Qui" __ "tol" -- "lis" "pec" -- "cá" -- _ "ta" "mun" -- _ _ "di," "sús" -- "ci" -- "pe" "de" -- "pre" -- "ca" -- _ _ "ti" -- "ó" -- _ "nem" "no" -- "stram." "Qui" __ "se" -- _ _ _ "des" "ad" "déx" -- _ _ _ _ _ _ _ "te" -- "ram" "Pa" -- _ "tris," __ _ _ "mi" -- "se" -- "ré" -- _ "re" "no" -- "bis." "mi" -- "se" -- "ré" -- "re" "no" -- "bis." "Quó" -- "ni" -- "am" "tu" __ _ _ _ _ "so" -- _ _ _ _ _ _ "lus" "san" -- _ _ _ _ "ctus." __ _ _ "Tu" "so" -- "lus" "Dó" -- "mi" -- "nus." __ "Tu" __ "so" -- _ "lus" "Al" -- "tís" -- _ _ "si" -- "mus," __ "Je" -- "su" "Chri" -- _ _ "ste." "Cum" "San" -- "cto" "Spí" -- "ri" -- "tu," "in" "gló" -- "ri" -- "a" __ "De" -- "i" "Pa" -- "tris." "A" -- "men." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | r1 | f1\startGroup^\markup \small "1" | f2 e2~ | e2 d2 | g2\stopGroup f2~ | f4 e4 f2 | g2 f2~ | f4 e4 f2~ | f2 a2 | a2 f2 | g1 | f1 | r2 bes2\startGroup^\markup \small "2" | \once \set suggestAccidentals = ##t es'2. d'4 | bes1 | bes1 | d'1\stopGroup | r2 f2~ | f4 e4 f2~ | f2 f2~ | f2 a2 | a2. c'4 | d'1 | c'1 | bes1 | r1 | f2 a2~ | a4 g4 \once \set suggestAccidentals = ##t b2~ | \once \set suggestAccidentals = ##t b2 \once \set suggestAccidentals = ##t b2 | c'2 c'2~ | c'4 a4 g2~ | g2 f2 | g1 | f1 | r2 a2~ | a2 f2 | f2 e2 | e2. f4 | e1 | f2 f2 | g1 | f1 | g2. g4 | a2 a2~ | a1~ | a2 g2 | f2 f2 | g1 | f1 | g1 | f2. e4 | f1 | e1 | d1 | e2 g2 | e1 | d1 | e1 | f2. a4 | bes2 g2~ | g2 g2 | f1 | f2 e2 | d1 | c1 | r2 f'2~\startGroup^\markup \small "4" | f'2 e'2 | c'1 | d'2 bes2~ | bes2 c'2\stopGroup | bes2. a4 | bes2 g2~ | g4 f4 a2~ | a4 g4 e2 | f4 g4 e2~ | e2 f4 g4 | a1 | g1 | e1 | e1 | g4 f4 e2~ | e4 g4 f2~ | f4 e4 f2 | g1 | f1~ | f1 | r2 f2~\startGroup^\markup \small "5" | f2 e2 | g2 f2~ | f4 e4 f2~\stopGroup | f4 g4 a2~ | a2 bes2 | c'2 a2~ | a2 a2 | bes2 bes2 | g2 f2 | g1 | f1 | r1 | a1 | a2. g4 | a2 g2 | e2. e4 | f2. e4 | f2 g2 | c'2 c'2~ | c'4 bes4 g2 | f2. a4 | g2. bes4 | c'1 | bes1 | bes1 | a4 g4 a2~ | a2 a2 | bes2 a2~ | a4 c'4 bes2 | c'1 | bes1 | a1 | g1 | f1 | g2 a2 | bes4 c'4 d'2~ | d'4 d'4 bes2~ | bes4 a4 bes2~ | bes2 a2 | a2 c'2 | d'1 | c'1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | c'1\startGroup^\markup \small "6" | d'2 c'2~ | c'2 bes2 | g2 a2 | f2\stopGroup e2 | f4 g4 a4 bes4 | c'2 a2 | bes2. f4 | g1 | f1~ | f2 e2 | f2 e2 | f4 e4 f2~ | f2 e2 | r2 f'2\startGroup^\markup \small "7" | d'2 c'2 | bes2 d'2 | c'2 d'2 | e'1\stopGroup | c'2 r2 | r1 | r1 | c'1 | d'2. f'4 | e'2 e'2 | d'1 | c'1 | r2 f2\startGroup^\markup \small "8" | g2 g2 | a2.\stopGroup f4 | f4 f4 e2~ | e2 f2 | f1 | e4 f8 e8 f2 | g1 | f1 | r1 | r2 f2\startGroup^\markup \small "9" | f2 g2 | e2\stopGroup f2~ | f4 g4 a2 | r1 | r1 | e2 f2 | g2. g4 | e2 e2 | d1 | c1 | r2 a2~ | a2 a2 | bes2 bes2~ | bes4 a4 \once \set suggestAccidentals = ##t b2 | c'2 c'2 | d'1 | c'1 | bes4 a4 bes2~ | bes2 bes2 | c'1 | bes1 | r1 | bes1 | bes2 a2 | bes2 a2~ | a1 | g4 f4 g2~ | g4 f8 g8 a2 | g1 | e2. e4 | f2 f2 | g1 | f1 | a2. g8 f8 | g1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Et" "in" "ter" -- "ra" "pax" "ho" -- "mí" -- _ _ "ni" -- "bus" "bo" -- "nae" "vo" -- "lun" -- "tá" -- "tis." "Lau" -- "dá" -- _ _ "mus" "te." "Be" -- "ne" -- "dí" -- "ci" -- "mus" "te." __ _ _ _ _ "Ad" -- "o" -- "rá" -- "mus" __ "te." "Glo" -- "ri" -- "fi" -- "cá" -- _ "mus" "te." "Grá" -- _ "ti" -- "as" "á" -- _ _ "gi" -- "mus" "ti" -- "bi" "pro" -- "pter" "ma" -- "gnam" __ "gló" -- "ri" -- "am" "tu" -- "am." "Dó" -- "mi" -- "ne" "De" -- _ "us," "Rex" "cae" -- "lé" -- "stis," "De" -- _ "us" "Pa" -- _ "ter" "o" -- "mní" -- _ "pot" -- "ens." "Dó" -- "mi" -- "ne" "Fi" -- _ _ _ "li" "u" -- _ _ _ "ni" -- "gé" -- _ _ _ _ _ _ "ni" -- "te" "Je" -- _ _ _ "su" "Chri" -- _ _ _ "ste." __ "Dó" -- "mi" -- "ne" "De" -- _ _ _ _ "us," "A" -- "gnus" __ "De" -- "i," "Fí" -- "li" -- "us" "Pa" -- "tris." "Qui" "tol" -- "lis" "pec" -- "cá" -- _ "ta" "mun" -- _ _ _ "di," "mi" -- "se" -- "ré" -- _ "re" "no" -- _ _ "bis." "Qui" "tol" -- _ _ "lis" "pec" -- "cá" -- "ta" "mun" -- _ "di," "sús" -- "ci" -- "pe" "de" -- "pre" -- "ca" -- _ _ "ti" -- "ó" -- _ _ "nem" "no" -- _ _ "stram." "Qui" "se" -- _ _ _ "des" "ad" "déx" -- _ _ _ _ _ _ "te" -- "ram" "Pa" -- _ _ _ _ _ _ _ "tris," "mi" -- "se" -- "ré" -- _ _ "re" "no" -- _ "bis." "mi" -- _ "se" -- "ré" -- "re" "no" -- "bis." "Quó" -- "ni" -- "am" "tu" __ _ _ _ _ "so" -- "lus" "san" -- _ _ _ _ "ctus." "Tu" "so" -- "lus" "Dó" -- _ "mi" -- "nus." "Tu" "so" -- _ "lus" "Dó" -- "mi" -- "nus." __ _ "Tu" __ "so" -- _ "lus" __ "Al" -- "tís" -- "si" -- "mus," "Je" -- "su" "Chri" -- _ "ste." __ "Cum" "San" -- "cto" "Spí" -- "ri" -- "tu," "in" "gló" -- _ _ _ _ _ _ _ "ri" -- "a" "De" -- "i" "Pa" -- "tris." "A" -- _ _ _ "men." }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key f \ionian \time 2/2 \clef "bass"
      f1\startGroup^\markup \small "1" | f2 e2~ | e2 d2 | g2\stopGroup f2~ | f2 e2 | e2 d2~ | d4 d4 c2 | bes,2 bes,2~ | bes,2 d2 | d2 bes,2~ | bes,4 c4 d2 | c1 | f2. e4 | f2 d2 | c1 | f1 | r1 | r1 | r1 | r1 | r1 | d2 d2~ | d2 bes,2 | bes,2 f2~ | f4 e4 c2 | d2 a2 | g1 | c1 | r2 bes,2\startGroup^\markup \small "3" | bes,2 \once \set suggestAccidentals = ##t es2 | d2\stopGroup c2 | g1 | r2 g2~ | g4 f4 c2~ | c4 d4 e2 | f4 c4 d2 | c1 | f1 | r2 f2~ | f2 d2 | c2 c2~ | c4 bes,4 c2~ | c4 c4 e2~ | e2 d2 | c1 | f1 | e1 | c4 d4 c2~ | c1 | bes,2. bes,4 | d2 d2 | c1 | f1 | e4 d4 c2~ | c4 bes,4 d2~ | d2 d2 | a,1 | d1 | c2. bes,4 | a,1 | d1 | c2 e2~ | e2 d4 c4 | bes,1 | bes,2 bes,2~ | bes,1~ | bes,1 | g,1 | c1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 c2\startGroup^\markup \small "4" | bes,2 g,2~ | g,2 a,2 | f,1 | g,2\stopGroup c2~ | c4 c4 e2~ | e2 d4 c4 | e2. d4 | c1 | e2 d2 | c1 | f1 | f2. e4 | d2 d2 | bes,2 c2 | r1 | r2 d2 | f2 f2~ | f2 d2 | c2 f2~ | f4 g4 a2 | g2 d2 | e2 d2 | c1 | f1 | r1 | f1 | g1 | c4 d4 e2~ | e2 e2 | d2 d2 | bes,1 | a,1 | bes,2 bes,2~ | bes,2 f2 | e2 g2 | f1 | bes,1 | r1 | f2 f2 | a4 g4 f4 e4 | d1 | f2. g4 | f1 | bes,1 | c2. bes,4 | c1 | f1 | e2. a4 | g1 | bes2. a4 | f2 g2 | a4 e4 f2~ | f4 g4 a2 | g1 | c1 | r2 f2~\startGroup^\markup \small "6" | f2 g2 | f1 | e2 c2\stopGroup | f2. e4 | f2 f2~ | f2 e2 | d1 | c2 g2~ | g4 f4 a2 | g2 a2 | e4 f4 g2~ | g4 e4 f2~ | f4 d4 e2~ | e2 d2 | d4 c4 f4 e4 | d1 | c1 | f1 | r2 c2\startGroup^\markup \small "7" | a,2 g,2 | f,2 a,2 | g,2 a,2 | bes,1\stopGroup | f2 e2 | r2 d2 | f1 | c4 bes,4 c2~ | c4 f4 e2~ | e4 d4 e2~ | e2 d2 | d2. c4 | bes,2 bes,2~ | bes,4 a,4 e2 | g1 | c1 | r1 | r1 | r2 f2~ | f4 d4 e2 | e2 d2~ | d4 c4 d2~ | d1 | c1 | f1 | r1 | r1 | bes,1 | c2 d2~ | d2 c2 | c2 d2 | a4 g4 f4 e4 | c2 bes,2~ | bes,2 bes,2 | c2 c2 | g,1 | c1 | r2 f2~ | f2 a2 | g2. d4 | e2. e4 | c2 c2 | g1 | c1 | d1 | d2 g2 | f1 | bes,1 | c2 d2 | bes,2 bes,2~ | bes,4 d4 c2 | bes,2 c2~ | c1~ | c4 d4 e2 | f4 d4 c2~ | c1~ | c2 c2 | d1 | c1 | f1 | f2 d2 | c1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Et" "in" "ter" -- "ra" "pax" __ _ "ho" -- "mí" -- _ "ni" -- "bus" "bo" -- "nae" __ "vo" -- "lun" -- "tá" -- _ _ _ _ _ _ _ _ "tis." "Lau" -- "dá" -- "mus" "te." "Be" -- "ne" -- "dí" -- _ "ci" -- "mus" "te." "Ad" -- "o" -- "rá" -- _ "mus" "te." "Glo" -- "ri" -- _ "fi" -- "cá" -- _ _ _ "mus" "te." "Grá" -- "ti" -- "as" "á" -- "gi" -- "mus" __ "ti" -- "bi" __ "pro" -- "pter" "ma" -- "gnam" "gló" -- _ _ "ri" -- "am" "tu" -- "am." "Dó" -- "mi" -- "ne" __ _ _ "De" -- "us," __ "Rex" "cae" -- "lé" -- "stis," "De" -- _ "us" "Pa" -- "ter" __ _ _ _ "o" -- "mní" -- "pot" -- "ens." "Dó" -- "mi" -- "ne" __ "Fi" -- _ _ _ "li" "u" -- _ _ _ "ni" -- "gé" -- _ _ "ni" -- "te" "Je" -- _ _ "su" "Chri" -- "ste." "Dó" -- "mi" -- "ne" __ "De" -- "us," "A" -- "gnus" "De" -- "i," "Fí" -- "li" -- "us" "Pa" -- "tris." "Qui" "tol" -- "lis" __ _ _ "pec" -- "cá" -- "ta" "mun" -- "di," "mi" -- "se" -- "ré" -- _ "re" "no" -- "bis." "Qui" "tol" -- _ _ _ _ _ "lis" "pec" -- "cá" -- "ta" "mun" -- _ _ "di," "sús" -- "ci" -- "pe" "de" -- "pre" -- "ca" -- "ti" -- "ó" -- _ _ "nem" "no" -- _ "stram." "Qui" __ "se" -- _ _ _ _ _ _ "des" __ _ _ "ad" "déx" -- _ _ _ _ _ _ _ "te" -- "ram" __ _ _ _ "Pa" -- _ _ _ _ _ "tris," "mi" -- "se" -- "ré" -- _ _ "re" "no" -- _ _ "bis." "mi" -- "se" -- "ré" -- _ _ _ _ _ _ _ "re" __ _ _ "no" -- _ _ _ "bis." "Quó" -- "ni" -- "am" "tu" "so" -- "lus" "san" -- _ "ctus." "Tu" __ _ _ _ "so" -- _ _ _ _ _ _ "lus" __ "Dó" -- "mi" -- "nus." __ _ _ "Tu" __ "so" -- _ _ _ "lus" "Al" -- "tís" -- _ "si" -- "mus," "Je" -- "su" "Chri" -- "ste." "Cum" "San" -- "cto" "Spí" -- "ri" -- "tu," "in" "gló" -- _ _ _ _ _ "ri" -- "a" "De" -- "i" "Pa" -- "tris." "A" -- "men." }
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
}

\bookpart {
\markup \fill-line { \override #'(font-size . 4) \bold "Credo" }
\markup \fill-line { \center-column { \italic "Intonatio: Credo in unum Deum," } }

\score {
  \header { piece = "Patrem omnipotentem" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | f'1\startGroup^\markup \small "1" | e'1 | a'1 | g'2\stopGroup f'2 | e'2 e'2~ | e'2 g'2 | g'4 f'4 g'2~ | g'2 f'2 | g'1 | f'1 | g'1 | g'4 a'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | a'1~ | a'1 | a'2. c''4 | bes'4 a'4 c''2~ | c''4 bes'4 g'2~ | g'2 f'2~ | f'2 a'2~ | a'4 bes'4 d''2~ | d''2 \once \set suggestAccidentals = ##t cis''2 | d''1 | d''2 c''2 | a'1 | r2 c''2\startGroup^\markup \small "3" | c''2 d''2 | c''2 g'2 | a'2\stopGroup f'2~ | f'4 e'4 d'2~ | d'2 \once \set suggestAccidentals = ##t cis'2 | d'1~ | d'2 r2 | r2 f'2\startGroup^\markup \small "4" | g'2 g'2~ | g'2 a'2 | e'1 | e'1\stopGroup | e'2 a'4 f'4~ | f'2 e'2 | f'1 | r2 f'2~ | f'2 e'2 | f'2 f'2 | g'2. a'4 | f'1~ | f'2 e'2 | f'1~ | f'2 e'2 | f'2. f'4 | a'2 g'2 | bes'4 a'4 bes'2~ | bes'2 a'2 | a'4 bes'4 c''2~ | c''2 c''2 | a'2. a'4 | c''1 | c''2 bes'4 f'4~ | f'2 e'2 | f'1 | f'2 e'2 | e'2. f'4 | g'2 bes'2~ | bes'2 a'2 | bes'1~ | bes'1 | a'2 f'2~ | f'1~ | f'2 e'2 | e'1 | e'2 f'2~ | f'2 e'2 | f'1 | r2 f'2~ | f'2 g'2 | bes'1 | a'4 g'4 f'2~ | f'2 g'2 | a'2 a'2 | bes'4 c''4 d''2~ | d''2 bes'2~ | bes'2 a'2 | bes'1 | r2 c''2~ | c''2 a'4 g'4 | f'2. f'4 | a'2 g'2~ | g'4 g'4 bes'2 | a'2 f'2~ | f'2 e'2 | f'1 | r2 g'2~ | g'2 f'2 | f'1 | e'2 e'2 | f'2. g'4 | a'2 f'2~ | f'4 e'4 f'2 | e'2 e'2~ | e'4 f'4 g'2 | a'2 c''4 bes'4~ | bes'2 a'2 | bes'1~ | bes'2 a'2 | a'2 g'2~ | g'4 f'4 a'2~ | a'2 g'2~ | g'4 f'4 a'2~ | a'4 bes'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r2 f'2~ | f'2 f'2 | e'2 e'2~ | e'4 g'4 f'2~ | f'4 e'4 f'2~ | f'2 e'2 | f'1 | f'1 | e'2 g'2~ | g'2 f'2 | e'2. g'4 | f'2. f'4 | c''1~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | c''1 | bes'1~ | bes'4 g'4 a'4 g'8 a'8 | g'2 f'2~ | f'2 e'2 | f'1 | r1 | \mark \markup \italic "Et incarnatus est" f'1 | g'1~ | g'2 f'2~ | f'2 e'2 | f'1 | f'4 g'4 a'2~ | a'4 f'4 g'2~ | g'2 f'2 | a'4 g'4 f'2~ | f'2 e'2 | f'1~ | f'2 e'2 | f'1 | g'4 a'4 bes'2~ | bes'4 c''4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | c''2 \once \set suggestAccidentals = ##t b'2~ | \once \set suggestAccidentals = ##t b'4 a'4 \once \set suggestAccidentals = ##t b'2 | c''2 e''2~ | e''4 d''4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r1 | \mark \markup \italic "Crucifixus" f'1 | f'2 a'2~ | a'4 g'8 f'8 g'2 | g'2 f'2~ | f'2 e'2 | e'2 g'2~ | g'4 f'4 g'2~ | g'4 a'4 c''2~ | c''1 | bes'4 g'4 f'4 e'4 | f'2 f'2~ | f'2 g'2 | g'4 f'4 a'2~ | a'2 g'2~ | g'4 g'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1~ | c''1 | bes'2 bes'2~ | bes'4 g'4 f'2~ | f'2 e'2 | f'1 | r2 f'2~ | f'2 g'2 | e'2 e'2~ | e'4 g'4 a'2~ | a'4 bes'4 c''2 | g'2. a'4 | a'4 bes'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | c''2. d''4 | d''4 c''4 bes'2~ | bes'2 a'2 | f'4 g'4 f'2~ | f'2 e'2 | f'1 | f'2 a'2~ | a'2 g'4 e'4 | f'1 | f'4 g'4 a'2~ | a'2 g'2 | f'2 f'2 | a'1 | a'1 | g'2 e'2 | e'2 f'2 | g'2 \once \set suggestAccidentals = ##t b'2~ | \once \set suggestAccidentals = ##t b'2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r2 f'2~ | f'2 f'2 | e'2. e'4 | g'2 g'2~ | g'2 f'2 | e'2 f'2 | a'4 bes'4 c''4 d''4 | bes'2. a'4 | c''1~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r2 f'2~ | f'4 e'4 f'2~ | f'2 e'2 | g'1 | f'1~ | f'4 a'4 g'2~ | g'2 f'2~ | f'2 e'2 | f'1 | f'4 e'4 f'2~ | f'2 e'2 | g'2 f'2~ | f'2 a'2 | a'4 g'4 f'2~ | f'4 e'4 g'2 | f'2 f'2~ | f'2 e'2 | f'1 | r2 f'2~ | f'2 e'2 | g'1 | g'2. e'4 | f'2 e'4 d'4~ | d'2 \once \set suggestAccidentals = ##t cis'2 | d'1 | r2 a'2~ | a'4 bes'4 c''2~ | c''2 c''2~ | c''2 e''2 | e''2. d''8 c''8 | bes'4 g'4 f'2~ | f'2 e'2 | f'1~ | f'2 e'2 | f'1 | a'4 g'4 e'2~ | e'2 e'2 | f'2. a'4 | bes'2. d''4 | bes'2 a'2 | a'1 | g'2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r2 f'2~ | f'2 a'2 | a'1 | g'1~ | g'2 f'4 e'4 | a'2 g'4 c'4~ | c'2 \once \set suggestAccidentals = ##t b2 | c'1~ | c'1 | bes1~ | bes2 c'2~ | c'2 \once \set suggestAccidentals = ##t b2 | c'1 | r2 d''2~ | d''4 c''4 bes'2~ | bes'4 g'4 a'2~ | a'2 f'2~ | f'2 e'2 | f'1 | f'2 e'2 | f'2 g'2~ | g'4 g'4 f'2 | e'2 e'2 | g'2 a'2 | bes'2 bes'2~ | bes'2 f'2~ | f'2 e'2 | f'1 | r2 f'2~ | f'2 f'2 | g'2 bes'2~ | bes'2 a'2 | bes'1 | bes'2 a'2~ | a'2 g'4 f'4 | g'2. a'4 | bes'2 bes'2~ | bes'1 | a'2. g'4 | a'2 f'2~ | f'2 g'2 | a'2. a'4 | e'2 f'2~ | f'2 e'2 | f'1 | r1 | e'2. f'4 | g'2. a'4 | bes'2 bes'2~ | bes'4 d''4 e''2 | d''2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1~ | c''1 | bes'2. g'4 | f'2 f'2~ | f'4 g'4 a'2~ | a'4 bes'4 c''2~ | c''2 bes'2 | a'2 a'2~ | a'4 g'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r2 f'2~ | f'2 f'2 | g'1 | a'2 bes'2~ | bes'2 a'2 | f'1 | f'2. g'4 | f'1 | e'2 e'2 | g'2. g'4 | e'2 f'2~ | f'2 e'2 | f'1 | e'2 f'2 | g'1 | g'1~ | g'2 a'2 | bes'4 c''4 d''4 bes'4 | bes'4 a'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | c''4 bes'4 f'2~ | f'2 e'2 | f'1\fermata
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Pa" -- "trem" "o" -- _ "mni" -- "pot" -- "én" -- _ "tem," __ _ _ "Pa" -- _ "trem" "o" -- "mni" -- _ _ _ _ "pot" -- "én" -- _ _ _ _ "tem," "fa" -- "ctó" -- _ "rem" "cae" -- "li" "et" "ter" -- _ "rae," "vi" -- "si" -- "bí" -- "li" -- "um" "ó" -- _ _ _ "mni" -- "um," __ "et" "in" -- "vi" -- "si" -- "bí" -- "li" -- "um." __ _ _ _ _ "Et" __ "in" "u" -- "num" "Dó" -- _ _ "mi" -- "num" __ "Je" -- _ "sum" "Chri" -- "stum," "Fí" -- _ _ "li" -- "um" __ _ _ "De" -- _ "i" "u" -- "ni" -- "gé" -- _ "ni" -- "tum." "Et" "ex" "Pa" -- "tre" "na" -- "tum" __ "an" -- _ "te" "ó" -- "mni" -- "a" "saé" -- _ "cu" -- "la." "De" -- "um" "de" "De" -- _ "o," __ "lu" -- "men" "de" "lú" -- _ _ _ "mi" -- "ne," "De" -- _ _ _ "um" "ve" -- "rum" __ "de" "De" -- "o" "ve" -- _ "ro." "Gé" -- "ni" -- "tum," "non" "fa" -- _ _ "ctum," "con" -- "sub" -- "stan" -- "ti" -- "á" -- "lem" "Pa" -- _ _ _ _ "tri:" __ "per" "quem" "ó" -- "mni" -- "a" __ "fa" -- _ _ _ _ "cta" "sunt." "Qui" __ "pro" -- "pter" "nos" __ "hó" -- _ _ _ "mi" -- "nes," "et" "pro" -- "pter" __ "no" -- "stram" "sa" -- "lú" -- "tem" "de" -- "scén" -- "dit" "de" "cae" -- _ _ _ _ _ _ _ "lis." "Et" "in" -- _ _ "car" -- "ná" -- _ _ "tus" "est" __ "de" "Spí" -- _ _ "ri" -- "tu" __ "San" -- _ _ _ _ "cto" "ex" __ "Ma" -- "rí" -- "a" "Vír" -- "gi" -- "ne:" "Et" "ho" -- "mo" "fa" -- "ctus" "est." "Cru" -- "ci" -- "fí" -- _ _ _ "xus" "ét" -- _ "i" -- "am" __ "pro" "no" -- _ "bis:" __ "sub" __ _ _ _ _ "Pón" -- "ti" -- "o" __ _ _ _ "Pi" -- "lá" -- "to" "pas" -- "sus," "et" __ "se" -- "púl" -- "tus" "est." "Et" __ "re" -- "sur" -- "ré" -- "xit" "tér" -- "ti" -- "a" "di" -- _ _ _ "e," __ "se" -- "cún" -- "dum" __ _ _ _ _ "Scri" -- "ptú" -- _ _ _ "ras." "Et" "a" -- "scén" -- _ _ "dit" __ _ _ "in" "cae" -- "lum:" "se" -- "det" "ad" "déx" -- "te" -- "ram" "Pa" -- _ _ _ "tris." "Et" __ "í" -- _ "te" -- "rum" "ven" -- "tú" -- _ "rus" "est" __ _ _ _ _ "cum" "gló" -- "ri" -- "a," "ju" -- "di" -- "cá" -- "re" "vi" -- _ _ _ _ "vos" "et" "mór" -- _ _ "tu" -- "os:" "cu" -- "jus" "re" -- _ _ "gni" "non" "e" -- "rit" __ "fi" -- "nis." "Et" __ "in" "Spí" -- "ri" -- "tum" "San" -- _ _ _ "ctum," "Dó" -- "mi" -- "num," __ "et" __ "vi" -- "vi" -- _ _ _ _ _ "fi" -- "cán" -- "tem:" "qui" "ex" __ _ _ "Pa" -- _ "tre" "Fi" -- "li" -- "ó" -- "que" "pro" -- "cé" -- _ _ "dit." "Qui" __ "cum" "Pa" -- _ _ _ _ _ _ "tre" "et" __ "Fí" -- _ "li" -- "o" "si" -- "mul" "ad" -- "o" -- "rá" -- "tur," __ "et" "con" -- "glo" -- "ri" -- "fi" -- "cá" -- "tur:" "qui" "lo" -- "cú" -- "tus" "est" "per" "Pro" -- "phé" -- _ "tas." "Et" __ "u" -- "nam" "san" -- _ "ctam" "ca" -- "thó" -- _ _ _ "li" -- "cam" "et" __ "a" -- "po" -- "stó" -- _ "li" -- "cam" "Ec" -- "clé" -- _ "si" -- "am." "Con" -- "fí" -- _ "te" -- "or" "u" -- "num" "ba" -- "ptís" -- _ _ "ma" __ "in" "re" -- _ "mis" -- "si" -- "ó" -- "nem" "pec" -- _ "ca" -- "tó" -- _ _ _ "rum." "Et" __ "ex" -- "spé" -- "cto" "re" -- _ "sur" -- "re" -- "cti" -- "ó" -- "nem" "mor" -- _ "tu" -- "ó" -- "rum." __ "Et" "vi" -- "tam" "ven" -- "tú" -- "ri" __ "saé" -- _ _ _ _ _ _ _ "cu" -- "li." "A" -- _ _ _ "men." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      bes1\startGroup^\markup \small "1" | a1 | d'1 | c'2\stopGroup c'2~ | c'4 d'4 c'2~ | c'2 \once \set suggestAccidentals = ##t b2 | c'2. a4 | bes4 c'4 bes2~ | bes2 d'2 | c'2 c'2~ | c'1~ | c'1 | r2 d'2 | d'2 c'2 | d'2 d'2 | e'2 d'2 | c'2. c'4 | a2 g2 | f2 r2 | r2 f2~\startGroup^\markup \small "2" | f2 g2 | f2 bes2 | a1 | f2 g2\stopGroup | a2 a2 | f4 g4 a2 | r2 c'2\startGroup^\markup \small "3" | c'2 d'2 | c'2 g2 | a2\stopGroup \once \set suggestAccidentals = ##t b2 | c'4 bes4 c'2~ | c'4 a4 bes2 | bes2 a2~ | a1 | r2 bes2~\startGroup^\markup \small "4" | bes2 c'2 | c'1 | d'2\stopGroup c'2~ | c'1 | \once \set suggestAccidentals = ##t b1~ | \once \set suggestAccidentals = ##t b1 | c'1 | c'1 | a1 | r2 c'2~ | c'2 bes2 | c'1 | d'1 | d'1 | c'2. c'4 | a2. bes4 | c'2. c'4 | d'2 bes2 | c'2. c'4 | bes1 | c'2 c'2 | a2. bes4 | c'2 c'2~ | c'1~ | c'1 | bes2. a4 | c'4 bes4 c'2~ | c'4 d'4 c'2~ | c'4 bes4 c'2 | r1 | d'2 d'2~ | d'2 c'2 | d'1 | c'1 | c'2. bes8 c'8 | d'2. d'4 | c'2. bes4 | c'1 | c'2 bes2 | c'2 c'2 | a1 | r2 c'2~ | c'2 c'2 | d'2 d'2~ | d'2 c'4 bes4 | d'4 c'4 d'2~ | d'2 c'2 | d'2. d'4 | bes1 | r2 c'2 | d'2 bes2~ | bes4 a4 c'2~ | c'4 c'4 d'2 | c'2. \once \set suggestAccidentals = ##t b4 | c'1 | d'2 d'2~ | d'4 c'4 \once \set suggestAccidentals = ##t b2 | c'2 c'2~ | c'1~ | c'2 d'2 | r2 bes2~ | bes4 bes4 d'2 | c'2 bes2 | d'2. c'4 | a2 a2 | c'4 bes4 a2~ | a4 g4 \once \set suggestAccidentals = ##t b2~ | \once \set suggestAccidentals = ##t b1 | c'2. d'4 | d'2 c'2 | d'2. d'4 | c'1 | c'1 | d'2. d'4 | c'2. bes4 | d'2 d'2~ | d'2 c'2 | \once \set suggestAccidentals = ##t b2 d'2 | e'1 | r2 c'2~ | c'2 d'2 | \once \set suggestAccidentals = ##t b1 | c'1 | c'4 bes4 a4 \once \set suggestAccidentals = ##t b4 | c'2 c'2~ | c'4 d'4 c'2 | c'2 d'2~ | d'2 c'2 | bes2 d'2~ | d'1 | c'1~ | c'1 | r1 | r1 | d'2. c'4 | d'2. bes4 | c'2 a2~ | a4 g4 \once \set suggestAccidentals = ##t b2 | c'1 | a1 | r1 | c'1 | c'1 | bes2 d'2 | d'2 c'2~ | c'1 | c'1 | d'2 bes2~ | bes2 a2 | a2. \once \set suggestAccidentals = ##t b4 | c'1 | a2. a4 | c'2. bes4 | c'2. c'4 | d'2 d'2~ | d'1 | r1 | r1 | d'2. d'4 | c'2 d'2 | c'2 c'2~ | c'2 a2 | \once \set suggestAccidentals = ##t b2 d'2 | e'1 | r1 | c'1 | c'1~ | c'2 d'2 | e'4 c'4 d'2 | e'1 | c'2 c'2~ | c'2 \once \set suggestAccidentals = ##t b2 | c'2 a2~ | a1 | bes4 c'4 d'2~ | d'4 d'4 c'2 | \once \set suggestAccidentals = ##t b1~ | \once \set suggestAccidentals = ##t b2 a2 | c'2. d'4 | e'2 c'2 | r2 d'2 | e'1 | d'2. c'4 | d'1 | d'2. bes4 | c'2. c'4 | a1 | r2 c'2~ | c'2 d'2 | d'2 c'2~ | c'2 bes4 a4 | c'4 bes4 g2~ | g1 | a2 c'2 | r1 | r2 c'2~ | c'2 c'2 | d'2. c'4 | d'2 d'2~ | d'2 c'4 \once \set suggestAccidentals = ##t b4 | c'1 | a2 bes2 | bes2 a2 | c'2 \once \set suggestAccidentals = ##t b2~ | \once \set suggestAccidentals = ##t b2 a4 bes4 | c'2 c'2~ | c'4 d'4 bes2~ | bes2 a2 | c'2 bes2 | a2 bes2 | c'1 | c'2. bes4 | g2 g2~ | g2 a2 | d'2 d'2 | e'1 | r2 c'2~ | c'2 d'2 | c'2 c'2~ | c'2 bes2 | d'2 d'2 | c'4 bes4 d'4 c'4 | a1 | bes2. d'4 | c'1 | r1 | r2 c'2~ | c'2 bes2 | d'2 d'2~ | d'2 c'2 | d'2 d'2 | c'4 bes4 c'2~ | c'2 bes2 | d'2 d'2 | c'4 bes4 c'2~ | c'1~ | c'2 d'2 | d'2 c'2~ | c'2 c'2 | d'2 c'2~ | c'4 c'4 d'2 | e'1 | d'1 | c'1 | a1 | r2 c'2~ | c'2 c'2 | d'2 bes2~ | bes1~ | bes4 a4 bes2~ | bes2 a2~ | a1~ | a4 g4 a2~ | a2 a2 | c'2. bes4 | c'2. d'4 | e'1~ | e'2 f'2 | r2 c'2~ | c'4 c'4 bes2 | c'2 c'2~ | c'2 \once \set suggestAccidentals = ##t b2 | c'1 | a2. bes4 | c'1 | d'2 d'2~ | d'1 | bes2 c'2 | c'1 | \once \set suggestAccidentals = ##t b2 d'2 | e'1 | r2 c'2~ | c'2 a2 | bes1 | d'4 c'4 bes2~ | bes4 a4 g2 | a2 c'2 | r1 | r2 c'2~ | c'2 bes2 | bes2. a4 | bes2 a2 | r1 | r1 | r1 | r2 d'2~ | d'2 c'2 | c'2 d'2~ | d'2 c'2 | a4 g4 a2~ | a2 bes4 c'4 | d'2. bes4 | c'2. d'4 | c'1 | d'1 | bes1 | bes4 c'4 d'4 bes4 | c'1 | a1 | r2 c'2~ | c'2 c'2 | d'2 d'2~ | d'2 c'2 | d'2. d'4 | c'2 c'2~ | c'4 bes4 c'2 | d'2 bes2~ | bes4 bes4 d'2~ | d'1 | c'2 d'2~ | d'2 c'2 | c'2. bes4 | c'1 | r1 | r2 c'2~ | c'2 a2 | \once \set suggestAccidentals = ##t b1 | c'1 | bes1 | d'4 c'4 bes4 a4 | bes1 | a2 a2 | r1 | r2 c'2~ | c'2 bes2 | bes2 d'2~ | d'4 c'8 bes8 c'2 | d'1 | c'1 | d'2 d'2~ | d'4 c'4 d'2 | e'2 c'2 | d'2 d'2 | e'1 | r2 c'2~ | c'2 c'2 | d'1 | d'1 | c'1 | a1 | a2 c'2 | d'2 d'2~ | d'2 c'2 | d'2 bes2~ | bes4 c'4 d'2 | r2 c'2~ | c'1~ | c'2 \once \set suggestAccidentals = ##t b2 | c'1 | c'2. bes4 | c'2. c'4 | d'2 bes2 | c'2. a4 | g1 | r1 | d'1 | c'1 | a1\fermata
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Pa" -- "trem" "o" -- _ "mni" -- _ _ "pot" -- "én" -- _ _ _ _ _ _ "tem," __ "Pa" -- "trem" "o" -- _ "mni" -- _ _ _ "pot" -- "én" -- _ "tem," "fa" -- "ctó" -- _ _ "rem" "cae" -- _ "li" "et" "ter" -- _ "rae," "vi" -- "si" -- "bí" -- "li" -- "um" "ó" -- _ _ _ _ _ _ "mni" -- "um," __ "et" __ "in" -- "vi" -- "si" -- "bí" -- _ _ "li" -- "um." "Et" __ "in" "u" -- "num" "Dó" -- "mi" -- "num" "Je" -- _ _ "sum" "Chri" -- "stum," "Fí" -- "li" -- "um" "De" -- "i" "u" -- _ _ "ni" -- "gé" -- _ _ _ _ _ _ "ni" -- "tum." "Et" "ex" __ "Pa" -- _ "tre" "na" -- _ _ _ "tum" "an" -- "te" "ó" -- "mni" -- "a" "saé" -- "cu" -- "la." "De" -- "um" "de" "De" -- _ _ _ _ _ "o," "lu" -- "men" __ _ "de" "lú" -- _ _ _ "mi" -- "ne," "De" -- "um" "ve" -- _ "rum" __ "de" "De" -- "o" "ve" -- "ro." "Gé" -- "ni" -- "tum," "non" "fa" -- _ _ "ctum," "con" -- _ _ _ "sub" -- "stan" -- _ "ti" -- "á" -- _ _ "lem" "Pa" -- "tri:" "per" "quem" "ó" -- "mni" -- "a" "fa" -- _ _ "cta" "sunt." "Qui" __ "pro" -- _ "pter" "nos" __ _ _ _ _ "hó" -- "mi" -- "nes," "et" "pro" -- "pter" "no" -- _ "stram" __ "sa" -- "lú" -- _ "tem" "de" -- "scén" -- "dit" "de" "cae" -- "lis." "Et" "in" -- _ "car" -- "ná" -- "tus" __ "est" __ _ _ "de" "Spí" -- _ _ "ri" -- "tu" "San" -- "cto" "ex" "Ma" -- "rí" -- "a" __ "Vír" -- "gi" -- "ne:" "Et" "ho" -- "mo" __ "fa" -- _ "ctus" "est." "Cru" -- "ci" -- "fí" -- _ _ _ _ "xus" "ét" -- "i" -- "am" "pro" __ "no" -- _ _ "bis:" "sub" "Pón" -- "ti" -- "o" "Pi" -- "lá" -- "to" "pas" -- _ _ _ "sus," "et" "se" -- "púl" -- "tus" "est." "Et" __ "re" -- "sur" -- "ré" -- _ _ _ _ _ _ "xit" "tér" -- "ti" -- "a" __ _ _ "di" -- _ _ _ "e," "se" -- "cún" -- "dum" "Scri" -- "ptú" -- "ras." __ _ _ "Et" __ "a" -- "scén" -- "dit" "in" "cae" -- _ _ "lum:" "se" -- "det" "ad" "déx" -- "te" -- "ram" "Pa" -- "tris." "Et" __ "í" -- "te" -- "rum" __ "ven" -- "tú" -- "rus" "est" __ _ _ _ _ _ _ _ "cum" __ "gló" -- "ri" -- "a," __ "ju" -- "di" -- "cá" -- "re" __ _ _ "vi" -- "vos" "et" "mór" -- _ _ "tu" -- "os:" "cu" -- "jus" "re" -- _ "gni" "non" "e" -- "rit" "fi" -- "nis." "Et" __ "in" "Spí" -- _ _ _ _ _ _ "ri" -- "tum" "San" -- "ctum," "Dó" -- "mi" -- "num," "et" __ "vi" -- "vi" -- "fi" -- "cán" -- _ "tem:" "qui" "ex" "Pa" -- "tre" "Fi" -- "li" -- "ó" -- "que" "pro" -- "cé" -- "dit." "Qui" __ "cum" "Pa" -- "tre" __ _ _ "et" "Fí" -- "li" -- "o" "si" -- "mul" "ad" -- "o" -- "rá" -- "tur," "et" __ "con" -- "glo" -- "ri" -- "fi" -- "cá" -- _ "tur:" __ "qui" __ _ _ "lo" -- "cú" -- "tus" "est" "per" "Pro" -- "phé" -- _ _ _ _ "tas." "Et" __ "u" -- "nam" "san" -- _ _ "ctam" "ca" -- "thó" -- "li" -- "cam" "et" "a" -- "po" -- "stó" -- "li" -- "cam" __ "Ec" -- "clé" -- "si" -- "am." "Con" -- _ "fí" -- _ _ _ _ _ _ _ "te" -- "or" "u" -- "num" "ba" -- "ptís" -- _ _ _ "ma" "in" "re" -- "mis" -- "si" -- "ó" -- "nem" "pec" -- "ca" -- "tó" -- "rum." "Et" __ "ex" -- "spé" -- "cto" "re" -- "sur" -- "re" -- "cti" -- "ó" -- "nem" __ "mor" -- "tu" -- "ó" -- _ "rum." "Et" __ "vi" -- _ "tam" "ven" -- "tú" -- "ri" "saé" -- _ _ "cu" -- "li." "A" -- _ "men." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | f1\startGroup^\markup \small "1" | e1 | a1 | g2\stopGroup g2~ | g4 a4 g2 | a2 c'2~ | c'4 bes4 c'2~ | c'1~ | c'2 \once \set suggestAccidentals = ##t b2 | c'2. a4 | d'1 | c'1 | r2 c'2\startGroup^\markup \small "2" | d'2 c'2 | f'2 e'2~ | e'2 c'2 | d'2\stopGroup d'2 | bes2. a8 g8 | f1~ | f2 e4 d4 | e1 | d1~ | d2 r2 | r2 f2\startGroup^\markup \small "3" | f2 g2 | f2\stopGroup f2~ | f2 e2 | f2. g4 | a4 bes4 d'2 | e'1 | d'1 | r2 g2~ | g4 g4 f2 | f2 e2 | g2 e2 | g2 g2 | e2 f2 | g2 f2 | g1 | f1 | r2 a2~ | a2 g2 | a4 bes4 c'2~ | c'1 | bes1 | g1 | f1 | g2. g4 | a2 f2~ | f2 e2 | f2 f2~ | f2 e2 | e4 f4 e2~ | e4 g4 a2~ | a1~ | a1 | g2 f2 | g1 | f1 | a1 | g4 f4 g2~ | g4 a4 bes2 | c'1 | bes1 | a2 g2 | f2 f2~ | f1 | e2 e2 | g2. f4 | g2 f2 | g1 | f1 | r2 a2~ | a2 g4 e4 | f1 | e2 f2~ | f4 g4 f2 | e2. e4 | f2. g4 | f2. bes4 | c'1 | bes1 | a2 a2~ | a4 g4 a2~ | a1~ | a4 bes4 c'2~ | c'4 bes4 g2 | f2. f4 | g1 | f1 | g1 | bes1 | bes2 a2~ | a4 g4 bes2~ | bes2 a4 g4 | f2. f4 | a2 a2 | g1 | g2 \once \set suggestAccidentals = ##t b2~ | \once \set suggestAccidentals = ##t b2 a4 bes4 | c'1 | bes1 | a4 g4 a2~ | a4 bes4 c'2 | a1 | g2 g2 | a2. a4 | c'1 | d'1 | c'1 | r2 a2~ | a2 a2 | g2 g2~ | g2 a2 | a2 f2 | g1 | f1 | a2 bes2 | g4 a4 g2~ | g2 bes2 | bes2. g4 | a4 bes4 c'4 \once \set suggestAccidentals = ##t b4 | c'1 | d'1 | c'1 | bes2. a4 | g2. f4 | g2 e2 | e2 f2 | g1 | f1 | r1 | a1 | \once \set suggestAccidentals = ##t b2. c'4 | d'2. bes4 | g1 | f1 | a2 a2 | bes2. bes4 | a1 | f2 f2 | g1 | f1 | g1 | a2. a4 | bes2 d'2~ | d'4 c'4 bes4 a4 | d'1 | c'1 | \once \set suggestAccidentals = ##t b1 | c'2 \once \set suggestAccidentals = ##t b2~ | \once \set suggestAccidentals = ##t b1 | a2. a4 | d'1 | c'1 | r1 | a1 | a2. bes4 | c'2. \once \set suggestAccidentals = ##t b4 | c'1 | c'4 bes4 g2~ | g4 a4 g2 | bes4 a4 g2~ | g4 f4 a2~ | a1~ | a2 g2 | \once \set suggestAccidentals = ##t b1~ | \once \set suggestAccidentals = ##t b1 | c'4 a4 g4 f4 | g1 | g2. c'4 | d'1 | c'1 | bes2. a4 | bes1 | bes2. f4 | g1 | f1 | r2 a2~ | a2 bes2 | g4 f4 g2~ | g4 g4 e2~ | e4 e4 g2~ | g1~ | g2 f4 g4 | d'1 | c'1 | c'2 a2~ | a4 g4 f2 | f4 e4 f2~ | f2 f2 | g1 | f1 | a1 | g2. g4 | a2 a2 | f2. f4 | g2. bes4 | a1 | a2. f4 | e1~ | e2 e2~ | e1~ | e2 e2~ | e2 e2 | d1 | c1 | r2 a2~ | a2 bes2 | g1 | g1 | bes2 bes2~ | bes2 a2 | f2 f2 | g2. a4 | g2 c'2 | d'1 | c'1 | bes2. a4 | bes2. bes4 | a2 g2~ | g2 bes2 | a2. g4 | a2 bes2~ | bes2 a2 | g1 | f1 | a2. a4 | c'2 c'2~ | c'2 a2 | bes2 a2 | c'2. bes4 | c'2 c'2~ | c'4 a4 bes2 | g1 | f1 | r2 a2~ | a2 c'2 | c'4 bes4 g4 a4 | bes2. g4 | a2 g2 | e1 | d1 | e2 f2 | e1 | e4 f4 e2~ | e1 | g2. a4 | bes1 | g1 | f1 | g1 | a2 f2~ | f4 e4 g2~ | g4 f4 g2 | a2 f2~ | f4 f4 g2~ | g2 f2 | e2 e2~ | e4 g4 a2 | d'1 | c'1 | r2 a2~ | a2 f2 | g4 e4 f2~ | f2 e2 | g4 f4 g2~ | g4 f4 g2 | d'1 | c'1 | bes2 bes2~ | bes4 a4 g2~ | g4 f4 a2 | d'1 | c'1 | bes2 bes2 | d'2 d'2 | c'2. bes4 | a2 bes2 | g1 | f1 | a2 g2 | a2 bes2~ | bes2 a2 | g4 f4 g2~ | g2 f2 | g2 bes2~ | bes4 a4 g4 f4 | g1 | f1 | r2 a2~ | a2 c'2 | c'2 bes2 | c'1 | bes1 | a2. g4 | f2 g2~ | g2 bes4 a4 | f4 g4 f2~ | f1~ | f4 a4 \once \set suggestAccidentals = ##t b2 | c'2. bes4 | a1 | a2. f4 | g4 a4 c'4 a4 | g1 | f1 | g2 g2 | e1 | e1 | f2 f2 | g2. g4 | f2. a4 | d'1 | c'1 | bes2 bes2~ | bes4 a4 bes2 | a1 | bes2 a2~ | a2 a2 | bes2 d'2 | c'2 c'2 | bes2 a2 | d'1 | c'1 | r2 a2~ | a2 a2 | bes4 c'4 d'2~ | d'4 d'4 bes2 | a4 g4 a2~ | a4 a4 f2~ | f2 g2 | a2. a4 | g2 a2 | bes2 bes2~ | bes2 a4 f4 | g1 | f1 | r1 | e2 e2~ | e2 g2 | g2. e4 | f2. f4 | a2 a2 | d'1 | c'1 | bes1 | g1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Pa" -- "trem" "o" -- _ "mni" -- _ _ "pot" -- "én" -- _ _ _ _ _ _ "tem," "fa" -- "ctó" -- _ _ "rem" __ "cae" -- _ "li" "et" __ _ _ _ "ter" -- _ _ "rae," __ "vi" -- "si" -- "bí" -- "li" -- "um" __ "ó" -- _ _ _ _ _ "mni" -- "um," "vi" -- "si" -- "bí" -- "li" -- "um" "ó" -- "mni" -- "um," "et" "in" -- "vi" -- "si" -- "bí" -- "li" -- "um." "Et" __ "in" "u" -- _ _ "num" "Dó" -- _ "mi" -- "num" "Je" -- _ _ "sum" "Chri" -- "stum," "Fí" -- _ _ "li" -- "um" __ "De" -- "i" "u" -- "ni" -- "gé" -- _ _ _ "ni" -- "tum." "Et" "ex" "Pa" -- "tre" "na" -- "tum" __ "an" -- "te" "ó" -- "mni" -- "a" "saé" -- "cu" -- "la." "De" -- _ _ _ "um" "de" __ "De" -- "o," "lu" -- "men" __ _ "de" "lú" -- "mi" -- "ne," __ _ _ "De" -- "um" "ve" -- _ _ "rum" "de" "De" -- "o" "ve" -- "ro." "Gé" -- "ni" -- "tum," "non" __ "fa" -- "ctum," __ "con" -- _ _ "sub" -- "stan" -- "ti" -- "á" -- "lem" "Pa" -- _ _ _ "tri:" "per" __ _ _ "quem" "ó" -- _ "mni" -- "a" "fa" -- "cta" __ _ _ "sunt." "Qui" __ "pro" -- "pter" "nos" __ _ "hó" -- _ "mi" -- "nes," "et" "pro" -- _ _ _ "pter" "no" -- _ _ _ _ _ _ "stram" "sa" -- "lú" -- _ _ "tem" "de" -- "scén" -- "dit" "de" "cae" -- "lis." "Et" "in" -- "car" -- "ná" -- "tus" "est" __ _ "de" "Spí" -- _ "ri" -- "tu" "San" -- "cto" "ex" "Ma" -- "rí" -- _ "a" "Vír" -- _ _ _ _ _ "gi" -- "ne:" "Et" "ho" -- "mo" "fa" -- "ctus" "est." "Cru" -- "ci" -- "fí" -- "xus" "ét" -- "i" -- "am" __ _ _ "pro" "no" -- "bis:" __ _ _ "sub" "Pón" -- "ti" -- "o" __ "Pi" -- _ _ _ _ "lá" -- "to" "pas" -- "sus," "et" "se" -- "púl" -- "tus" __ _ _ "est." "Et" __ "re" -- _ _ _ "sur" -- "ré" -- "xit" "tér" -- _ _ _ "ti" -- "a" "di" -- "e," "se" -- "cún" -- _ _ "dum" "Scri" -- "ptú" -- "ras." "Et" "a" -- "scén" -- "dit" __ _ "in" "cae" -- _ "lum:" "se" -- "det" "ad" __ "déx" -- "te" -- "ram" "Pa" -- "tris." "Et" __ "í" -- _ "te" -- "rum" "ven" -- "tú" -- _ "rus" "est" "cum" "gló" -- _ "ri" -- "a," "ju" -- "di" -- "cá" -- "re" "vi" -- "vos" __ "et" "mór" -- _ _ _ _ "tu" -- "os:" "cu" -- "jus" "re" -- "gni" __ "non" __ _ _ _ _ _ "e" -- "rit" "fi" -- _ "nis." "Et" __ "in" "Spí" -- _ _ _ _ "ri" -- "tum" "San" -- _ "ctum," "Dó" -- _ "mi" -- "num," __ _ _ "et" "vi" -- "vi" -- "fi" -- "cán" -- _ "tem:" "qui" __ "ex" "Pa" -- "tre" "Fi" -- _ _ "li" -- "ó" -- "que" "pro" -- "cé" -- _ _ _ "dit." "Qui" __ "cum" "Pa" -- _ _ _ _ _ _ "tre" "et" "Fí" -- "li" -- "o" "si" -- "mul" "ad" -- "o" -- "rá" -- _ "tur," "et" "con" -- "glo" -- "ri" -- _ "fi" -- "cá" -- _ _ "tur:" "qui" "lo" -- "cú" -- "tus" __ "est" __ _ _ _ "per" "Pro" -- "phé" -- _ _ _ _ "tas." "Et" __ "u" -- "nam" "san" -- _ "ctam" "ca" -- _ _ "thó" -- _ _ _ _ _ _ _ "li" -- "cam" "et" "a" -- _ _ _ _ _ _ _ "po" -- "stó" -- _ "li" -- "cam" "Ec" -- "clé" -- "si" -- "am." "Con" -- "fí" -- "te" -- "or" "u" -- "num" "ba" -- "ptís" -- "ma" "in" __ "re" -- "mis" -- "si" -- "ó" -- "nem" "pec" -- "ca" -- "tó" -- "rum." "Et" __ "ex" -- "spé" -- _ _ "cto" "re" -- _ _ _ "sur" -- "re" -- "cti" -- "ó" -- "nem" "mor" -- _ "tu" -- "ó" -- _ _ _ "rum." "Et" "vi" -- _ "tam" "ven" -- "tú" -- "ri" "saé" -- "cu" -- "li." __ _ "A" -- _ "men." }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key f \ionian \time 2/2 \clef "bass"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | f1\startGroup^\markup \small "1" | e1 | a1 | g2\stopGroup g2~ | g4 f4 a2 | g1 | c1~ | c4 d4 e2 | f1~ | f4 d4 c2 | g4 f4 f4 e4 | d2 bes,2~ | bes,2 bes,2 | d2 d2~ | d2 bes,2 | a,1 | d1 | d2 f2~ | f4 e4 f2~ | f4 f4 e2~ | e2 d2 | e1 | d2 d2~ | d2 c4 bes,4 | a,1 | d1 | d2 c2~ | c4 e4 f2 | bes,2 c2 | bes,2 c2 | e2 e2~ | e1~ | e2 f4 f4 | c1 | f1 | r2 f2~ | f2 g2 | f1 | bes,2. c4 | d2 bes,2 | c1 | f1 | e2. e4 | d2 d2 | c2 e2~ | e2 d2 | c2 c2~ | c4 d4 c2~ | c2 f4 g4 | a2. a4 | e1 | e2 d2 | c1 | f1 | f2 c2~ | c2 c2 | bes,2 d2 | f1 | bes,1 | c1 | d1 | bes,2. bes,4 | c2 c2~ | c4 bes,4 c2 | c2 d2 | c1 | f1 | r2 f2~ | f2 e4 c4 | bes,2. bes,4 | c2. d4 | bes,2 bes,2 | c1 | bes,1 | d4 c4 d2 | f1 | bes,1 | c1 | f2. e4 | f1 | c2. c4 | bes,2. bes,4 | d2 d2 | c1 | f1 | e4 c4 bes,2~ | bes,4 bes,4 d2~ | d2 c4 bes,4 | c2 g2 | f2 f2~ | f4 e4 d2 | c1 | e1 | e2 d2 | c2. g4 | f1 | bes,1 | c2 c2~ | c2 e2 | f4 d4 c4 bes,4 | c2. g4 | f2. f4 | c2. c4 | g1 | c1 | r2 f2~ | f2 d2 | e1 | e2 d2 | c2 d2 | c1 | f1 | f2 d2 | e1 | g2. f4 | g2. e4 | f1 | e2 a2 | g1 | c1 | d1 | d1 | c4 bes,4 c2~ | c2 d2 | c1 | f1 | r1 | f1 | e2. d4 | bes,1 | c1 | f1 | f1 | bes,1 | c4 d4 f2~ | f4 e4 d2 | c1 | f1 | e2. g4 | f2. a4 | g2 d2~ | d4 e4 f2 | g1 | c1 | d1 | a2 g2~ | g4 f4 e2 | f2 a2 | g1 | c1 | r1 | f1 | f1 | e2. d4 | c2 f2 | e1 | e1 | d2 e2~ | e4 f4 c2 | d2. d4 | g2 g2 | d2. c4 | d2. g4 | f2 f2 | c1 | c2 e2 | g1 | c1 | d2. a4 | g1 | g2 d2 | c1 | f1 | r2 f2~ | f2 bes,2 | c1 | c4 bes,4 c2~ | c2 e4 f4 | g1 | f4 e4 d4 c4 | g,1 | c1 | c2 f2~ | f2 e4 d4 | bes,4 c4 d2~ | d2 f4 d4 | c1 | f1 | f1 | e2. g4 | f2 f2~ | f4 e4 f2~ | f2 e2 | f2 f2~ | f4 c4 d2~ | d4 c4 d2 | e2 c2 | c2 d2 | e2 e2~ | e4 d4 c2 | g,1 | c1 | r2 f2~ | f2 bes,2 | c1 | bes,2 bes,2~ | bes,4 c4 d2 | a4 g4 f2~ | f1~ | f1 | e1 | g1 | c1 | d1 | bes,4 c4 d4 bes,4 | c2 c2 | bes,2. bes,4 | f2 e2 | d2 g2 | f4 d4 c4 bes,4 | c1 | f1 | f1 | e2. e4 | c2. c4 | bes,2 c2~ | c4 c4 f2 | e2 c2~ | c2 bes,2 | c1 | f1 | r2 f2~ | f2 a2 | g1 | g1 | f2 bes,2 | a,1 | d1 | c2. bes,4 | c1 | c2 e2 | e2 e2~ | e2 f2 | g2 d2 | c1 | f1 | e2. e4 | d1 | c2. bes,4 | c2 c2~ | c1 | bes,1 | d2 d2~ | d2 c2 | c2. f4 | g1 | c1 | r2 f2~ | f2 f2 | bes,4 c4 d2~ | d1 | c2. e4 | d2 e2 | g1 | c1 | d2 g2~ | g4 f4 e2 | d2 f2 | g1 | c1 | d1 | bes,1 | c1 | c2 bes,2 | c1 | f1 | f2 g2 | d1 | c2. \once \set suggestAccidentals = ##t b,4 | c2 c2 | bes,2 f2~ | f4 e4 g2 | g2 d2 | c1 | f1 | r2 f2~ | f2 a2 | g1 | f1 | bes,1 | c1 | d1 | bes,1 | bes,2. c4 | d2. e4 | f2 d2 | c2 f2~ | f4 e4 c2~ | c4 d8 e8 f2 | c1 | c1 | f1 | e2 e2~ | e1~ | e1 | d1 | bes,2 c2 | d4 e4 f4 f4 | g1 | c1 | d2 g2~ | g2 f4 e4 | f1 | d2. e4 | f2. a4 | g2. g4 | a2 f2 | g2 a2 | g1 | c1 | r2 f2~ | f2 a2 | g1 | f4 d4 c4 bes,4 | c1 | f1 | f2 e2 | d1 | e2. a4 | g4 f4 g2~ | g2 d2 | c1 | f1 | g2. d4 | c1 | c2 e2 | e1 | d1 | c2. f4 | g1 | c1 | d2. bes,4 | c1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Pa" -- "trem" "o" -- _ "mni" -- "pot" -- "én" -- _ _ _ _ _ _ _ "tem," __ _ _ _ _ "fa" -- "ctó" -- _ "rem" __ "cae" -- _ _ "li" __ _ _ _ "et" "ter" -- "rae," "vi" -- "si" -- "bí" -- _ _ _ "li" -- "um" "ó" -- "mni" -- "um," "et" "in" -- "vi" -- "si" -- "bí" -- "li" -- "um." __ _ _ _ "Et" __ "in" "u" -- _ "num" "Dó" -- _ "mi" -- "num" "Je" -- "sum" "Chri" -- "stum," "Fí" -- _ "li" -- "um" "De" -- "i" "u" -- _ _ _ "ni" -- "gé" -- "ni" -- "tum." __ _ _ "Et" "ex" __ "Pa" -- "tre" "na" -- _ "tum" "an" -- _ _ "te" __ _ "ó" -- "mni" -- "a" "saé" -- _ "cu" -- "la." "De" -- "um" __ _ _ "de" "De" -- "o," "lu" -- "men" "de" "lú" -- _ _ _ "mi" -- "ne," "De" -- _ "um" "ve" -- "rum" "de" "De" -- "o" __ _ "ve" -- _ "ro." "Gé" -- _ _ "ni" -- "tum," __ "non" __ _ _ "fa" -- "ctum," "con" -- "sub" -- "stan" -- "ti" -- "á" -- "lem" "Pa" -- _ _ _ "tri:" "per" "quem" __ "ó" -- _ _ _ _ _ "mni" -- "a" "fa" -- _ "cta" __ _ "sunt." "Qui" __ "pro" -- _ "pter" "nos" "hó" -- _ "mi" -- "nes," "et" "pro" -- _ _ "pter" "no" -- "stram" "sa" -- "lú" -- _ _ "tem" "de" -- "scén" -- "dit" __ _ _ "de" "cae" -- "lis." "Et" "in" -- "car" -- "ná" -- "tus" "est" "de" "Spí" -- _ _ _ "ri" -- "tu" "San" -- "cto" "ex" "Ma" -- "rí" -- "a" "Vír" -- _ _ _ "gi" -- "ne:" "Et" "ho" -- _ "mo" "fa" -- _ _ "ctus" "est." "Cru" -- "ci" -- "fí" -- _ _ _ "xus" "ét" -- "i" -- "am" __ "pro" "no" -- _ "bis:" "sub" "Pón" -- _ _ _ "ti" -- "o" "Pi" -- "lá" -- "to" "pas" -- _ "sus," "et" "se" -- "púl" -- "tus" __ _ _ "est." "Et" __ "re" -- "sur" -- "ré" -- _ _ "xit" __ _ _ "tér" -- _ _ _ _ "ti" -- "a" "di" -- _ _ _ _ _ _ _ _ "e," "se" -- "cún" -- _ _ "dum" __ "Scri" -- "ptú" -- _ "ras." "Et" __ "a" -- "scén" -- "dit" "in" "cae" -- "lum:" "se" -- "det" "ad" "déx" -- "te" -- "ram" "Pa" -- "tris." "Et" __ "í" -- _ "te" -- "rum" __ "ven" -- "tú" -- "rus" __ _ _ "est" "cum" "gló" -- "ri" -- "a," __ _ _ _ _ "ju" -- _ "di" -- "cá" -- "re" "vi" -- _ _ _ _ _ _ "vos" "et" "mór" -- "tu" -- "os:" "cu" -- "jus" "re" -- "gni" "non" "e" -- "rit" __ "fi" -- _ "nis." "Et" __ "in" "Spí" -- "ri" -- "tum" "San" -- _ "ctum," "Dó" -- "mi" -- "num," "et" "vi" -- "vi" -- "fi" -- "cán" -- _ _ _ "tem:" "qui" "ex" "Pa" -- _ "tre" "Fi" -- "li" -- "ó" -- _ "que" __ "pro" -- "cé" -- _ _ "dit." "Qui" __ "cum" "Pa" -- _ _ "tre" "et" "Fí" -- _ "li" -- "o" "si" -- _ "mul" "ad" -- "o" -- "rá" -- _ "tur," "et" "con" -- "glo" -- "ri" -- "fi" -- "cá" -- "tur:" "qui" "lo" -- "cú" -- _ _ _ "tus" "est" __ _ _ _ "per" "Pro" -- "phé" -- "tas." "Et" __ "u" -- "nam" "san" -- "ctam" "ca" -- "thó" -- "li" -- "cam" "et" "a" -- "po" -- "stó" -- _ "li" -- "cam" __ "Ec" -- "clé" -- _ _ _ _ "si" -- "am." "Con" -- "fí" -- "te" -- "or" "u" -- _ _ _ _ _ "num" "ba" -- "ptís" -- _ _ _ _ "ma" "in" "re" -- "mis" -- "si" -- "ó" -- "nem" "pec" -- "ca" -- "tó" -- "rum." "Et" __ "ex" -- "spé" -- _ _ _ _ _ "cto" "re" -- "sur" -- "re" -- _ _ _ _ _ "cti" -- "ó" -- "nem" "mor" -- "tu" -- "ó" -- "rum." "Et" "vi" -- "tam" "ven" -- "tú" -- "ri" "saé" -- "cu" -- "li." "A" -- "men." }
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
}

\bookpart {
\markup \fill-line { \override #'(font-size . 4) \bold "Sanctus" }

\score {
  \header { piece = "Sanctus" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | bes'1\startGroup^\markup \small "1" | g'1 | f'1 | bes'1 | a'2. g'4 | a'2 bes'2 | c''1\stopGroup | bes'2 c''2 | bes'2. g'4 | g'4 a'4 bes'2 | a'2. a'4 | f'2. f'4 | a'2 a'2~ | a'2 f'2~ | f'4 e'4 g'2~ | g'4 f'4 bes'2~ | bes'2 a'2 | bes'1 | a'2 a'2~ | a'2 bes'4 g'4 | f'2 a'2~ | a'4 g'4 bes'2~ | bes'4 a'4 f'2~ | f'2 g'2 | g'2 a'2 | f'1~ | f'2 e'2 | f'1 | r2 f'2\startGroup^\markup \small "3" | a'2 a'2 | c''1\stopGroup | c''4 bes'4 c''2~ | c''2 bes'2 | bes'2 f'2~ | f'2 e'2 | f'1 | r2 c''2~\startGroup^\markup \small "4" | c''2 bes'2 | g'1 | f'2 a'2~\stopGroup | a'2 bes'2 | r2 c''2 | bes'4 a'4 f'4 g'4 | a'2. bes'4 | c''2 d''2~ | d''4 d''4 c''2~ | c''2 a'4 g'4 | f'2 g'2~ | g'2 f'2 | e'2 f'2~ | f'2 e'2 | f'1 | r1 | bes'2 bes'2~ | bes'2 d''2 | d''2 bes'2 | bes'2. a'8 g'8 | f'4 e'4 f'2~ | f'2 e'2 | f'1\fermata
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "San" -- _ _ _ _ _ _ _ "ctus," "San" -- "ctus," "San" -- _ _ _ _ _ "ctus" "San" -- "ctus" "San" -- "ctus," __ "San" -- _ _ _ _ _ "ctus," "San" -- "ctus" __ _ _ _ "Dó" -- _ _ "mi" -- "nus" __ "De" -- "us" "Sá" -- _ "ba" -- "oth." "Ple" -- "ni" "sunt" __ _ "cae" -- _ _ "li" "et" "ter" -- _ "ra" "gló" -- "ri" -- "a" "tu" -- _ "a." "gló" -- _ _ _ _ _ _ _ "ri" -- "a" "tu" -- _ _ _ _ _ _ _ _ "a." "Ho" -- "sán" -- "na" "in" "ex" -- "cél" -- _ _ _ _ _ _ "sis." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | bes1\startGroup^\markup \small "1" | g1 | f1 | bes1 | a2. g4 | a2 bes2 | c'1\stopGroup | d'1~ | d'2 c'2 | d'2 d'2~ | d'2 c'2 | r1 | r2 c'2~\startGroup^\markup \small "2" | c'2 d'2 | bes1 | a2 g2~ | g2 f2~ | f2\stopGroup a4 bes4 | c'2. c'4 | a2. a4 | c'1 | r2 f2\startGroup^\markup \small "3" | a2 a2 | c'1\stopGroup | c'4 bes4 d'4 c'4 | bes2 a2 | r2 c'2~ | c'2 bes2 | c'2. c'4 | d'1 | r1 | r2 c'2~\startGroup^\markup \small "4" | c'2 bes2 | g1 | f2 a2\stopGroup | a2 c'2~ | c'4 bes4 a2~ | a2 c'4 bes4 | c'1 | d'2 d'2~ | d'4 c'4 d'2 | bes2. a4 | c'1 | r2 c'2~\startGroup^\markup \small "5" | c'2 d'2~ | d'4 e'4 f'2\stopGroup | g'2 d'2~ | d'2 d'2 | d'1~ | d'4 c'4 d'2~ | d'2 c'2 | a1\fermata
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "San" -- _ _ _ _ _ _ _ "ctus," "San" -- _ "ctus," "San" -- "ctus" "Dó" -- "mi" -- "nus" "De" -- _ _ _ _ _ "us" "Sá" -- "ba" -- "oth." "Ple" -- "ni" "sunt" __ _ "cae" -- _ _ _ _ "li" "et" __ "ter" -- _ "ra" __ _ "gló" -- "ri" -- "a" "tu" -- _ "a." "gló" -- _ _ _ _ _ _ "ri" -- "a" "tu" -- _ _ "a." "Ho" -- "sán" -- _ _ "na" "in" __ "ex" -- "cél" -- _ _ _ "sis." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      bes1\startGroup^\markup \small "1" | g1 | f1 | bes1 | a2. g4\stopGroup | f2 g2~ | g4 c'4 bes2 | a2 bes2~ | bes2 d'2 | c'1 | e'4 f'4 e'2~ | e'1 | d'4 c'4 bes4 a4 | f2 g2~ | g2 g2 | r2 f2~ | f2 f2 | e2 e2~ | e2 f2 | g2 e2 | f2. g4 | c'1 | bes1 | r2 c'2\startGroup^\markup \small "2" | d'2 bes2~ | bes2 a2 | g1 | f1\stopGroup | f2 e2~ | e2 f2 | f1 | g1 | f1 | r1 | r2 f2~ | f2 e2 | g2. a4 | f4 g4 f2~ | f4 e4 f2 | g1 | f1 | g2 g2 | a4 bes4 d'2~ | d'4 c'4 d'2~ | d'4 bes4 a2~ | a4 g4 f2 | f2 e2~ | e1 | d2 e2~ | e4 g4 f2~ | f4 f4 a2~ | a4 g4 a2 | a2 g2 | f2. a4 | g2 f2 | g1 | f1~ | f1 | r1 | d'2. c'4 | d'2 d'2~ | d'1 | c'2 bes2 | g1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "San" -- _ _ _ _ _ _ _ _ _ "ctus," "San" -- "ctus," "San" -- _ _ _ _ _ _ _ _ _ "ctus" "San" -- "ctus," "San" -- "ctus," __ "San" -- _ _ _ _ _ "ctus" "Dó" -- "mi" -- "nus" __ "De" -- _ _ "us" __ _ _ "Sá" -- "ba" -- "oth." "Ple" -- "ni" "sunt" __ _ _ _ _ _ _ "cae" -- "li" "et" "ter" -- _ _ _ "ra" "Ple" -- "ni" "sunt" __ _ _ "cae" -- "li" __ "et" "ter" -- _ _ "ra" "gló" -- "ri" -- "a" "tu" -- _ _ _ _ _ _ "a." __ "Ho" -- "sán" -- _ "na" __ "in" "ex" -- "cél" -- "sis." }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key f \ionian \time 2/2 \clef "bass"
      r1 | r1 | r1 | r1 | f1\startGroup^\markup \small "1" | d1 | c1 | f1\stopGroup | g2 g2~ | g4 f4 c2~ | c4 d4 g2 | f1 | g2. c4 | d2 bes,2~ | bes,2 bes,2 | d2 d2~ | d4 c4 d2~ | d2 c2 | c2 d2 | c2. e4 | d2. e4 | f1 | bes,1 | c2 f2~ | f2 g2 | a2 f2~ | f4 e4 d4 c4 | d2 d2~ | d2 e2 | e2 d2~ | d2 d2 | c1 | f1 | e2 f2~ | f2 d2 | c4 bes,4 c2~ | c2 c2 | d1 | d4 c4 d2 | c1 | f1 | e2. e4 | d2. bes,4 | c2 bes,2~ | bes,2 c2 | f4 e4 d2~ | d4 bes,4 c2 | d2 d2~ | d2 c2 | c2 r2 | f1\startGroup^\markup \small "4" | e2 c2~ | c2 bes,2 | d2\stopGroup d2~ | d1 | c1 | f1 | r2 f2\startGroup^\markup \small "5" | g2. a4 | bes2 f2~ | f2 g2 | g1\stopGroup | a2 d2 | c1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "San" -- _ _ _ "ctus," "San" -- _ _ _ _ _ _ _ _ "ctus," __ "San" -- "ctus" "San" -- _ _ "ctus," "San" -- _ _ _ _ _ _ "ctus," "San" -- _ _ _ _ _ _ _ _ "ctus" __ "Dó" -- "mi" -- "nus" __ "De" -- _ _ _ _ _ _ _ _ "us" __ _ "Sá" -- _ _ "ba" -- "oth." "Ple" -- "ni" "sunt" __ _ _ "cae" -- _ _ _ _ _ _ "li" "et" __ "ter" -- "ra" "gló" -- "ri" -- "a" __ "tu" -- _ "a." __ _ _ "Ho" -- "sán" -- _ _ _ "na" "in" "ex" -- "cél" -- _ "sis." }
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
}

\bookpart {
\markup \fill-line { \override #'(font-size . 4) \bold "Benedictus" }

\score {
  \header { piece = "Benedictus" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | f'1\startGroup^\markup \small "1" | a'2 g'2 | bes'1 | a'2 f'2 | g'1\stopGroup | g'2 f'2~ | f'2 e'2 | f'1~ | f'1 | g'2 bes'2 | r2 c''2\startGroup^\markup \small "2" | bes'2 a'2 | bes'2 bes'2~ | bes'4 a'4\stopGroup f'2~ | f'4 g'4 a'2 | a'2 g'2~ | g'4 f'4 g'2 | a'2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | c''2 a'2~ | a'4 g'4 a'2~ | a'4 a'4 f'2~ | f'4 e'4 g'2~ | g'4 f'4 a'2~ | a'2 g'4 f'4 | g'2 f'2~ | f'2 e'2 | f'1 | e'2 f'2 | g'2 f'2~ | f'2 e'2 | g'4 f'4 a'4 g'4 | f'2 f'2~ | f'2 e'2 | e'4 f'4 a'2~ | a'4 bes'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1\fermata
      \bar "||"
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Be" -- "ne" -- "dí" -- _ _ "ctus" "qui" "ve" -- _ "nit" __ _ _ _ "in" "nó" -- "mi" -- "ne" "Dó" -- _ _ "mi" -- "ni." "in" "nó" -- "mi" -- "ne" "Dó" -- _ "mi" -- "ni." "Be" -- _ _ _ "ne" -- "dí" -- _ _ _ _ _ _ _ _ _ "ctus" "qui" "ve" -- "nit" "in" __ "nó" -- _ _ _ _ _ "mi" -- "ne" "Dó" -- _ _ _ _ "mi" -- "ni." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | c'1\startGroup^\markup \small "1" | e'2 d'2 | f'1 | e'2 c'2 | d'1\stopGroup | d'2 c'2~ | c'4 a4 g2 | f2 a2 | r2 d'2 | c'2 e'2~ | e'2 c'2 | r1 | r2 c'2\startGroup^\markup \small "2" | bes2 a2 | bes2 bes2~ | bes4 a4 bes2 | a2\stopGroup a2~ | a2 g2 | r1 | r1 | r2 f2~\startGroup^\markup \small "3" | f2 a2 | g2 bes2~ | bes2 a2 | f2 g2~ | g2\stopGroup a2~ | a2 c'2~ | c'4 bes4 a2 | c'2 d'2 | r1 | r2 c'2\startGroup^\markup \small "4" | bes2 a2 | bes2 bes2~ | bes4 a4 bes2 | a2\stopGroup a2~ | a2 c'4 d'4 | e'1~ | e'1\fermata
      \bar "||"
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Be" -- "ne" -- "dí" -- _ _ "ctus" "qui" "ve" -- "nit" __ "Be" -- "ne" -- "dí" -- "ctus" "qui" "ve" -- _ "nit" "in" "nó" -- "mi" -- "ne" "Dó" -- _ _ _ "mi" -- "ni." "Be" -- "ne" -- "dí" -- _ _ "ctus" "qui" __ "ve" -- _ _ _ _ "nit" "in" "nó" -- "mi" -- "ne" "Dó" -- _ _ _ "mi" -- "ni." __ _ _ }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      c'1\startGroup^\markup \small "1" | e'2 d'2 | f'1 | e'2 c'2 | d'1\stopGroup | a4 bes4 c'2 | g4 f4 g2~ | g4 f4 a2~ | a4 g4 c'2~ | c'4 bes4 d'2 | c'1 | f1 | r2 f'2\startGroup^\markup \small "2" | e'2 d'2 | e'2 e'2~ | e'4 d'4 e'2 | d'2\stopGroup bes2 | bes2 a2 | r1 | g2 g2 | a2 g2~ | g2 f2 | g1 | c'1 | r2 c'2\startGroup^\markup \small "3" | e'2 d'2 | f'1 | e'2\stopGroup d'2 | a2 a2~ | a4 f4 e2~ | e2 f4 f4 | c'1 | f1 | r2 f'2\startGroup^\markup \small "4" | e'2 d'2 | e'2 e'2~ | e'4 d'4 e'2 | d'2\stopGroup bes2 | a4 f4 g2~ | g2 f2 | a1 | g1 | c'1\fermata
      \bar "||"
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Be" -- "ne" -- "dí" -- _ _ "ctus" "qui" "ve" -- _ _ _ _ _ _ _ _ _ "nit" __ _ _ _ "in" "nó" -- "mi" -- "ne" "Dó" -- _ _ _ "mi" -- "ni." "in" "nó" -- "mi" -- "ne" "Dó" -- _ "mi" -- "ni." "Be" -- "ne" -- "dí" -- _ _ "ctus" "qui" "ve" -- _ _ _ _ _ "nit" "in" "nó" -- "mi" -- "ne" "Dó" -- _ _ _ _ _ _ _ _ _ "mi" -- "ni." }
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

\score {
  \header { piece = "Osanna" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \set Score.currentBarNumber = #44
      \key f \ionian \time 2/2 \clef "treble"
      f'1 | f'2 g'2~ | g'2 f'2 | f'1 | e'2. e'4 | f'2 g'2~ | g'4 a'4 bes'2~ | bes'2 bes'2 | c''1~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r2 a'2~ | a'2 f'4 g'4 | a'2 a'2 | f'2. g'8 f'8 | g'2. a'4 | g'2 bes'2~ | bes'1 | a'1 | c''2 c''2 | a'4 bes'4 c''2~ | c''2 a'2 | a'4 g'4 f'2~ | f'2 e'2 | f'1\fermata
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "Ho" -- "sán" -- _ _ "na" "in" "ex" -- "cél" -- _ _ _ "sis." __ _ _ _ "Ho" -- _ _ _ "sán" -- _ _ _ _ _ _ _ _ "na" "in" "ex" -- _ _ _ "cél" -- _ _ _ "sis." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      c'1 | c'2. bes4 | d'2 c'2~ | c'2 d'2 | c'1 | d'1 | d'4 c'4 d'2~ | d'4 bes4 a4 g4 | a2 a2 | r1 | r2 c'2 | bes2 a2~ | a2 c'4 bes4 | c'2 c'2~ | c'1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 d'2 | d'2 c'2 | a1\fermata
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "Ho" -- "sán" -- _ _ _ "na" "in" "ex" -- "cél" -- _ _ _ _ _ _ "sis." "in" "ex" -- "cél" -- _ _ _ "sis." __ "in" "ex" -- "cél" -- "sis." }
    \new Staff = "staff2" \with { instrumentName = "Tenor" shortInstrumentName = "T" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      a1 | bes2 bes2 | a1 | c'2 bes2~ | bes4 g4 a2~ | a2 g2 | bes4 a4 g2~ | g4 g4 f2 | f2 a2 | d'1 | c'1 | r1 | a1 | a4 bes4 c'2~ | c'1~ | c'2 d'2 | bes1 | c'2. e'4 | f'4 d'4 c'2~ | c'4 a4 g2 | a2 g2~ | g4 f4 a2~ | a2 g4 f4 | g1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "Ho" -- _ "sán" -- _ _ _ _ _ _ _ _ _ "na" "in" "ex" -- "cél" -- _ "sis." "Ho" -- "sán" -- _ _ _ _ _ _ _ _ _ "na" "in" "ex" -- "cél" -- _ _ _ _ _ "sis." }
    \new Staff = "staff3" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice3" {
      \key f \ionian \time 2/2 \clef "bass"
      f1 | d1 | d2 f2 | f2. d4 | e2 c2~ | c1 | bes,1 | bes,1~ | bes,2 a,4 g,4 | g,1 | c1 | d2. e4 | f2 a2~ | a2 g4 f4 | a2. g4 | e2 f2 | g1 | e1 | r1 | r2 c2~ | c4 d4 e2 | f1 | f2. d4 | c1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "Ho" -- _ "sán" -- _ "na" __ _ _ "in" __ "ex" -- "cél" -- _ _ _ "sis." "in" "ex" -- "cél" -- _ _ _ _ _ _ _ _ "sis." "in" __ "ex" -- _ _ "cél" -- _ _ "sis." }
  >>
  \layout { }
}
}

\bookpart {
\markup \fill-line { \override #'(font-size . 4) \bold "Agnus Dei" }
\markup \fill-line { \center-column { \italic "Agnus Dei I gilt für die erste und zweite Anrufung," \italic "Agnus Dei II (dona nobis pacem) für die dritte." } }

\score {
  \header { piece = "Agnus Dei I" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | f'1\startGroup^\markup \small "1" | f'1 | c''1 | bes'1 | d''2 c''2 | a'1\stopGroup | bes'2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | a'2. g'4 | a'2 bes'2~ | bes'2 a'2 | a'2. g'4 | e'1~ | e'1 | f'2 f'2 | g'4 a'4 bes'2~ | bes'4 c''4 d''2~ | d''2 \once \set suggestAccidentals = ##t cis''2 | d''1~ | d''2 r2 | r1 | r1 | r2 f'2\startGroup^\markup \small "3" | g'2 d'2 | e'1 | e'1 | f'2.\stopGroup g'4 | f'1~ | f'1 | e'2 e'2 | r1 | r1 | r1 | r1 | r1 | f'2 a'2~ | a'4 g'4 a'2~ | a'2 g'2 | bes'4 a'4 f'4 g'4 | a'4 g'4 f'2~ | f'2 e'2 | f'1 | a'1 | a'2 f'2~ | f'2 f'2 | g'1 | g'2 f'2~ | f'4 f'4 e'2~ | e'4 g'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | r1 | r2 bes'2~ | bes'4 a'4 c''2~ | c''4 bes'4 a'2 | r2 e'2~ | e'2 g'2 | g'2 f'2~ | f'1 | e'2 g'2 | f'4 e'4 g'2~ | g'4 g'4 f'2~ | f'2 e'2 | f'1\fermata
      \bar "||"
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "A" -- "gnus" "De" -- _ _ _ _ _ _ _ "i," "A" -- "gnus" "De" -- "i," __ "qui" "tol" -- _ _ "lis" "pec" -- "cá" -- _ _ "ta" "mun" -- _ "di:" __ "mi" -- "se" -- "ré" -- _ "re" "no" -- _ _ _ "bis." "mi" -- _ "se" -- "ré" -- "re" "no" -- _ _ _ _ _ _ _ "bis." "mi" -- "se" -- "ré" -- "re" "no" -- "bis." "mi" -- "se" -- "ré" -- "re" "no" -- _ "bis." "mi" -- "se" -- "ré" -- "re" __ _ "no" -- "bis." "mi" -- "se" -- "ré" -- _ _ _ _ "re" "no" -- _ "bis." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | r1 | r1 | r1 | f1\startGroup^\markup \small "1" | f1 | bes1 | a1 | c'2 bes2 | g1\stopGroup | bes2. c'4 | d'4 bes4 c'2~ | c'1 | d'2 c'2~ | c'4 \once \set suggestAccidentals = ##t b4 g2~ | g4 f4 g2 | a2 a2~ | a2 g2 | a2 c'2~ | c'4 bes4 c'2~ | c'2 r2 | c'1\startGroup^\markup \small "2" | a1 | g1 | g1\stopGroup | c'4 bes4 a2~ | a4 a4 bes2~ | bes2 c'2 | r2 c'2 | c'2 bes2~ | bes4 bes4 d'2~ | d'2 d'2 | c'4 bes4 c'2~ | c'1~ | c'2 bes2 | bes2. a4 | g4 f4 g2~ | g4 a4 g2~ | g4 f4 a2~ | a2 g2 | g4 a4 bes4 bes4 | g1~ | g1 | r2 c'2\startGroup^\markup \small "3" | d'2 a2 | bes1 | bes1 | c'2.\stopGroup a4 | a4 g4 c'2~ | c'2 d'2 | c'1 | r2 f2\startGroup^\markup \small "4" | f1 | bes1 | a1 | c'2 bes2 | g1\stopGroup | g1 | r2 c'2 | d'2 c'2~ | c'4 bes4 d'2 | r2 c'2\startGroup^\markup \small "5" | a1 | g1 | g1\stopGroup | a2. \once \set suggestAccidentals = ##t b4 | c'2. d'4 | c'2. bes4 | c'2 d'2 | c'2. \once \set suggestAccidentals = ##t b4 | c'1 | a1\fermata
      \bar "||"
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "A" -- "gnus" "De" -- _ _ _ _ _ _ _ _ _ "i," "A" -- _ _ _ _ "gnus" "De" -- _ _ _ _ "i," __ "qui" "tol" -- "lis" "pec" -- "cá" -- _ _ "ta" "mun" -- "di:" "qui" "tol" -- _ "lis" "pec" -- "cá" -- _ _ _ "ta" "mun" -- _ _ _ _ _ _ _ _ _ "di:" __ _ _ _ _ "mi" -- "se" -- "ré" -- _ "re" "no" -- _ _ _ _ _ "bis." "A" -- "gnus" "De" -- _ _ _ _ "i," "A" -- "gnus" "De" -- _ "i," "qui" "tol" -- "lis" "pec" -- "cá" -- _ _ "ta" "mun" -- "di:" "mi" -- "se" -- "ré" -- "re" "no" -- "bis." }
    \new Staff = "staff2" \with { instrumentName = "Tenor 1" shortInstrumentName = "T1" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | c'1\startGroup^\markup \small "1" | c'1 | f'1 | e'1\stopGroup | d'1 | r2 f2~ | f4 e4 g2~ | g1~ | g4 f4 e2~ | e1 | f1 | d1 | c1~ | c2 r2 | r1 | c'1\startGroup^\markup \small "2" | a1 | g1 | g1\stopGroup | f1 | f1 | e2 g2 | e1 | d1 | r2 c'2\startGroup^\markup \small "3" | d'2 a2 | bes1 | bes1\stopGroup | d'2 d'2 | r1 | r1 | r1 | r1 | r1 | r2 g2 | f2. a4 | g4 f4 g2~ | g2 f2 | e2 f2 | g2. g4 | a1 | f4 g4 e4 f4 | g2. g4 | f1 | f4 e4 f2 | g1 | f1 | r2 c'2\startGroup^\markup \small "4" | c'1 | f'1 | e'1\stopGroup | e'2 a2~ | a2 bes2 | c'2. e'4 | d'1 | c'1 | bes2 a2 | bes1 | c'2 a2~ | a4 g4 f2~ | f4 e4 g2~ | g4 f4 e2 | f2. f4 | a1 | g2 g2 | a2 g2 | e2 f2 | g1 | f1\fermata
      \bar "||"
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "A" -- "gnus" "De" -- _ "i," "A" -- "gnus" "De" -- _ _ _ _ "i," __ "qui" "tol" -- "lis" "pec" -- "cá" -- "ta" "mun" -- _ _ "di:" "mi" -- "se" -- "ré" -- _ "re" "no" -- "bis." "mi" -- _ _ _ _ _ _ _ _ _ "se" -- "ré" -- _ _ _ _ _ "re" __ _ "no" -- _ _ _ "bis." "A" -- "gnus" "De" -- _ "i," "A" -- "gnus" "De" -- _ _ "i," "qui" "tol" -- _ _ "lis" __ "pec" -- "cá" -- _ _ _ _ _ "ta" "mun" -- "di:" "mi" -- "se" -- "ré" -- "re" "no" -- _ "bis." }
    \new Staff = "staff4" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice4" {
      \key f \ionian \time 2/2 \clef "bass"
      c1\startGroup^\markup \small "1" | c1 | f1 | e1\stopGroup | e1 | d2 f2~ | f4 e4 d4 c4 | d2 f2~ | f4 e4 d2 | e2 e2~ | e1 | d2 e2~ | e1 | d4 c4 bes,4 a,4 | g,1 | c1 | r2 f2\startGroup^\markup \small "2" | d1 | c1 | c1\stopGroup | e1 | e2 c2~ | c1 | bes,1~ | bes,1 | a,1 | d1 | d2 c2 | bes,2 f2~ | f4 e4 d2~ | d2 d2 | bes,4 c4 d2~ | d2 c2 | c2. e4 | d4 c4 d2~ | d1 | e1 | r2 c2\startGroup^\markup \small "3" | d2 a,2 | bes,1 | bes,1 | c2.\stopGroup d4 | e4 f4 g4 e4 | d2 c2~ | c4 bes,4 c2 | bes,1~ | bes,2 d2 | c1 | c1 | f1 | r1 | r1 | d2 f2~ | f4 e4 g2 | g4 c4 f2~ | f2 g2 | e2. e4 | g1 | c1 | r2 f2\startGroup^\markup \small "5" | d1 | c1 | c1\stopGroup | e2. d4 | e2 e2~ | e2 d2 | c2. \once \set suggestAccidentals = ##t b,4 | c2 c2~ | c2 \once \set suggestAccidentals = ##t b,2 | c2. d4 | c1 | f1\fermata
      \bar "||"
    }
    \new Lyrics = "text4" \lyricsto "voice4" { "A" -- "gnus" "De" -- _ "i," "A" -- _ _ _ _ _ _ _ _ "gnus" "De" -- _ _ _ _ _ _ _ "i," "qui" "tol" -- "lis" "pec" -- "cá" -- "ta" "mun" -- _ _ "di:" "qui" "tol" -- _ _ "lis" "pec" -- "cá" -- _ _ _ "ta" "mun" -- _ _ _ _ "di:" "mi" -- "se" -- "ré" -- _ "re" "no" -- _ _ _ _ _ "bis." "mi" -- "se" -- "ré" -- _ _ "re" "no" -- "bis." "mi" -- "se" -- _ _ "ré" -- _ _ _ _ "re" "no" -- "bis." "qui" "tol" -- "lis" "pec" -- "cá" -- _ _ "ta" __ "mun" -- _ _ "di:" "mi" -- "se" -- "ré" -- "re" "no" -- "bis." }
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

\score {
  \header { piece = "Agnus Dei II" }
  \new StaffGroup <<
    \new Staff = "staff0" \with { instrumentName = "Sopran" shortInstrumentName = "S" }
    \new Voice = "voice0" {
      \set Score.currentBarNumber = #73
      \key f \ionian \time 2/2 \clef "treble"
      c''1~\startGroup^\markup \small "1" | c''1 | bes'1 | g'1 | a'2 g'2 | f'2 a'2 | bes'1~\stopGroup | bes'1 | a'1 | f'4 g'4 a'4 bes'4 | c''1 | c''2. a'4 | g'2. e'4 | e'4 f'4 g'2~ | g'4 f'4 g'2~ | g'2 f'2 | g'2. a'8 bes'8 | a'2 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | c''2. a'4 | bes'4 c''4 d''4 e''4 | c''2. d''4 | c''2 bes'2 | bes'4 g'4 f'4 e'4 | e'4 f'4 g'4 a'4 | a'4 bes'4 c''2~ | c''2 \once \set suggestAccidentals = ##t b'2 | c''1 | c''1~ | c''2 g'2 | a'2. a'4 | f'2 f'2~ | f'4 e'4 f'2~ | f'2 e'2 | e'2. f'4 | f'4 e'4 f'2~ | f'4 a'4 g'2~ | g'4 f'4 a'2~ | a'2 a'2 | f'4 e'4 f'2~ | f'2 e'2 | f'1\fermata
      \bar "|."
    }
    \new Lyrics = "text0" \lyricsto "voice0" { "A" -- "gnus" "De" -- _ _ _ _ _ _ "i," __ _ _ _ _ "A" -- _ _ _ _ _ _ "gnus" "De" -- _ _ _ _ _ _ _ "i," "A" -- _ _ _ _ _ _ _ _ "gnus" "De" -- _ _ _ _ _ _ _ _ _ _ _ "i," "qui" __ "tol" -- _ "lis" "pec" -- "cá" -- "ta" "mun" -- "di:" "do" -- _ _ _ _ "na" "no" -- _ _ "bis" "pa" -- _ _ _ "cem." }
    \new Staff = "staff1" \with { instrumentName = "Alt" shortInstrumentName = "A" }
    \new Voice = "voice1" {
      \key f \ionian \time 2/2 \clef "treble"
      r1 | f'1~\startGroup^\markup \small "1" | f'1 | e'1 | c'1 | d'2 c'2\stopGroup | d'2. bes4 | c'2 c'2~ | c'2 bes2 | a2 a2 | c'2 c'2~ | c'2 r2 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r2 c'2~\startGroup^\markup \small "2" | c'2 c'2 | d'2. bes4 | c'1\stopGroup | c'2 r2 | d'2 c'2 | c'2 d'2 | c'2 a2 | r2 g2~ | g2 a4 \once \set suggestAccidentals = ##t b4 | c'1 | c'2 bes2 | c'2 c'2~ | c'2 r2 | c'2 d'2 | e'2 e'2~ | e'2 r2 | f1\startGroup^\markup \small "3" | f2 g2 | f1 | a1\stopGroup | a1 | c'2 c'2~ | c'1\fermata
      \bar "|."
    }
    \new Lyrics = "text1" \lyricsto "voice1" { "A" -- "gnus" "De" -- _ _ _ _ _ "i," __ "A" -- "gnus" "De" -- _ "i," __ "qui" __ "tol" -- _ _ _ "lis" "pec" -- "cá" -- "ta" "mun" -- "di:" "qui" "tol" -- _ _ _ "lis" "pec" -- "cá" -- "ta" __ "mun" -- _ _ "di:" __ "do" -- "na" "no" -- _ _ "bis" "pa" -- "cem." __ }
    \new Staff = "staff2" \with { instrumentName = "Tenor 1" shortInstrumentName = "T1" }
    \new Voice = "voice2" {
      \key f \ionian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | c'1~\startGroup^\markup \small "1" | c'1 | bes1 | g1 | a2 g2 | f2 a2\stopGroup | a2 c'2~ | c'4 c'4 d'2 | e'2 e'2~ | e'2 d'4 c'4 | e'4 d'4 c'4 bes4 | a2 c'2 | d'1 | c'2 e'2~ | e'2 d'2 | e'2 c'2~ | c'4 bes4 a2~ | a2 g2 | r2 c'2\startGroup^\markup \small "2" | c'2 d'2~ | d'4 bes4 c'2~ | c'2 bes2 | a2\stopGroup a2~ | a2 g2 | e2. f4 | g2 a2 | a2 bes2 | r2 c'2\startGroup^\markup \small "3" | c'2 d'2 | c'1 | e'1\stopGroup | e'4 d'4 c'2~ | c'2 f2~ | f2 e2 | f2 f2 | r1 | r2 a2~ | a4 g4 c'2 | a1\fermata
      \bar "|."
    }
    \new Lyrics = "text2" \lyricsto "voice2" { "A" -- "gnus" "De" -- _ _ _ _ "i," "A" -- "gnus" "De" -- "i," "A" -- _ _ _ _ _ _ _ "gnus" "De" -- _ _ _ _ _ _ _ "i," "qui" "tol" -- _ _ _ "lis" "pec" -- "cá" -- _ _ _ _ "ta" "mun" -- "di:" "do" -- "na" "no" -- _ _ "bis" __ _ _ "pa" -- "cem." "do" -- "na" "no" -- "bis" "pa" -- "cem." }
    \new Staff = "staff3" \with { instrumentName = "Tenor 2" shortInstrumentName = "T2" }
    \new Voice = "voice3" {
      \key f \ionian \time 2/2 \clef "treble_8"
      r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 | c'1~\startGroup^\markup \small "1" | c'1 | bes1 | g1 | a2 g2 | f2 a2 | bes1\stopGroup | c'2. e'4 | d'1 | c'1 | a2. a4 | g2 g2~ | g2 a2 | g1 | g2. f4 | g1 | e1 | d1 | c1~ | c4 e4 f4 g4 | a2 g2~ | g4 f4 a2~ | a2 \once \set suggestAccidentals = ##t b2 | c'2. bes4 | c'2 c'2 | a2 a2 | r1 | r1 | f2 f2~ | f2 a2 | a2 f2 | g1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text3" \lyricsto "voice3" { "A" -- "gnus" "De" -- _ _ _ _ _ _ _ _ "i," "A" -- "gnus" "De" -- "i," __ "qui" "tol" -- "lis" "pec" -- "cá" -- "ta" "mun" -- _ _ _ _ _ _ _ _ "di:" "do" -- "na" "no" -- "bis" "pa" -- "cem." "do" -- "na" __ "no" -- "bis" "pa" -- _ "cem." }
    \new Staff = "staff4" \with { instrumentName = "Bass" shortInstrumentName = "B" }
    \new Voice = "voice4" {
      \key f \ionian \time 2/2 \clef "bass"
      r1 | r1 | r1 | r1 | r1 | f1~\startGroup^\markup \small "1" | f1 | e1 | c1 | d2 c2~\stopGroup | c4 bes,4 c2 | c2 f2 | g1 | g2 c2~ | c4 d4 e2 | f2. a4 | g1 | a2. e4 | g1 | c1~ | c1 | r2 g2~ | g2 f2 | f4 e4 g2~ | g4 g4 e2~ | e4 d4 bes,2 | c2. c4 | g,1 | c1 | r2 f2\startGroup^\markup \small "3" | f2 g2 | f1 | a1\stopGroup | a2 f2 | e2 c2~ | c4 bes,4 c2~ | c2 bes,2 | r2 e2~ | e1 | d2 d2~ | d4 c4 d2 | c1 | f1\fermata
      \bar "|."
    }
    \new Lyrics = "text4" \lyricsto "voice4" { "A" -- "gnus" "De" -- _ _ _ _ "i," "A" -- _ "gnus" "De" -- _ _ _ _ _ _ _ _ "i," __ "A" -- "gnus" "De" -- _ "i," __ "qui" "tol" -- "lis" "pec" -- "cá" -- "ta" "mun" -- "di:" "do" -- "na" "no" -- _ _ "bis" "pa" -- _ _ _ _ "cem." "do" -- "na" "no" -- "bis" "pa" -- _ "cem." }
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
}
