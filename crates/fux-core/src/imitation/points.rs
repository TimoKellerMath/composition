//! The points of a text: the parts that the points of imitation set, from the starts of its
//! sense units and tails, and the words that the head of each point declaims.
//!
//! The text is cut at every start of a unit or a tail; a marked break (`|`) never cuts.
//! Consecutive pieces of one word each form one part (San-ctus, San-ctus, San-ctus), and a part
//! of fewer than four syllables (six in a long text) joins the next one, the last part the one
//! before it. Each part is one point, with a subject of its own (Jeppesen, Counterpoint, p. 241:
//! "each line or division of the text has its independent theme"). A short text that is planned
//! to be sung twice ([`crate::text::plan::Plan::statements`]) gets a second point: its last tail
//! once more with a new subject, or now and then the first point restated in closer stretto.
//!
//! The head of a point declaims its opening (SOG-SCOPE): from its first syllable to the end of
//! its first word of two syllables or more that is not a function word, and the next syllable,
//! six at most, within the point and its first unit. So a point of one-word units sings its word
//! alone (San-ctus), and a head reaches past monosyllables (Ad te le-vá-vi á-, Tu es Pe-trus),
//! which a shorter head would load with its melisma: Palestrina declaims Ad te levavi
//! syllabically up to the accent of levavi (Jeppesen, p. 86).

use std::ops::Range;

use super::{Imitation, PointKind, Toggle};
use crate::generate::{GenerateError, Rng};
use crate::text::underlay::{Model, UState};
use crate::text::{SylFlags, Text, TextKind};

/// The least syllables of a part of a short text; a shorter part joins its neighbour.
const SHORT_PART: usize = 4;

/// The same in a long text.
const LONG_PART: usize = 6;

/// The most syllables a head declaims.
const HEAD_SYLLABLES: usize = 6;

/// How often the second statement of a short text restates the first point instead of setting
/// the last tail with a subject of its own (`--restate auto`).
const RESTATE: f64 = 0.25;

/// What a point of imitation sets: its words, what kind of point it is and the words its head
/// declaims.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Part {
    /// The point's syllables, by their positions in the text.
    pub words: Range<u16>,
    pub kind: PointKind,
    /// The syllables that the head declaims, from the point's first on ([`head_words`]).
    pub head: Range<u16>,
}

/// The syllables in `range` as a list of indices.
fn indices(range: &Range<u16>) -> Range<usize> {
    usize::from(range.start)..usize::from(range.end)
}

/// Whether the syllables in `range` belong to one word.
fn one_word(text: &Text, range: &Range<usize>) -> bool {
    range.end > range.start
        && text.syllables[range.start].word == text.syllables[range.end - 1].word
}

/// The parts of a text, without a second statement: the text cut at the starts of its units and
/// tails, consecutive pieces of one word joined, and parts of fewer than four syllables (six in a
/// long text) joined to the next part, the last one to the part before it.
pub(crate) fn parts(text: &Text) -> Vec<Range<u16>> {
    let n = text.syllables.len();
    let cut = |s: usize| {
        s == 0
            || text.syllables[s]
                .flags
                .intersects(SylFlags::UNIT_START | SylFlags::TAIL_START)
    };
    let starts: Vec<usize> = (0..n).filter(|&s| cut(s)).collect();
    let pieces: Vec<Range<usize>> = starts
        .iter()
        .enumerate()
        .map(|(i, &start)| start..starts.get(i + 1).copied().unwrap_or(n))
        .collect();
    // Consecutive pieces of one word each form one part.
    let mut joined: Vec<(Range<usize>, bool)> = Vec::new();
    for piece in pieces {
        let lone = one_word(text, &piece);
        match joined.last_mut() {
            Some((last, true)) if lone => last.end = piece.end,
            _ => joined.push((piece, lone)),
        }
    }
    // Short parts join the next one, the last one the part before it.
    let least = match text.kind {
        TextKind::Short => SHORT_PART,
        TextKind::Long => LONG_PART,
    };
    let mut merged: Vec<Range<usize>> = Vec::new();
    let mut pending: Option<Range<usize>> = None;
    for (piece, _) in joined {
        let part = match pending.take() {
            Some(before) => before.start..piece.end,
            None => piece,
        };
        if part.len() < least {
            pending = Some(part);
        } else {
            merged.push(part);
        }
    }
    if let Some(rest) = pending {
        match merged.last_mut() {
            Some(last) => last.end = rest.end,
            None => merged.push(rest),
        }
    }
    merged
        .into_iter()
        .map(|r| r.start as u16..r.end as u16)
        .collect()
}

/// The syllables that the head of a point on `words` declaims (SOG-SCOPE): from the point's
/// first syllable to the end of its first word of two syllables or more that is not a function
/// word, and the next syllable; six at most, within the point and within the unit of its first
/// syllable, unless that unit has no such word. A point whose first unit is one word sings that
/// word alone (San-ctus).
pub(crate) fn head_words(text: &Text, words: Range<u16>) -> Range<u16> {
    let range = indices(&words);
    if range.is_empty() {
        return words;
    }
    let start = range.start;
    let unit_end = usize::from(text.unit_of(start).last) + 1;
    let mut s = start;
    while s < range.end && skipped(text, s) {
        s = text.word_of(s).range().end;
    }
    let content_end = if s < range.end {
        text.word_of(s).range().end
    } else {
        range.end
    };
    let end = (content_end + 1)
        .min(range.end)
        .min(unit_end.max(content_end))
        .min(start + HEAD_SYLLABLES);
    words.start..end as u16
}

/// Whether the head reaches past the word of syllable `s`: a function word, or a word of one
/// syllable (te, es, O), on which a melisma would fall.
fn skipped(text: &Text, s: usize) -> bool {
    text.syllables[s].flags.contains(SylFlags::FUNCTION_WORD) || text.word_of(s).range().len() == 1
}

/// The start of the text's last tail, or of its last unit if it has no tail.
fn last_tail(text: &Text) -> usize {
    let n = text.syllables.len();
    (0..n)
        .rev()
        .find(|&s| text.syllables[s].flags.contains(SylFlags::TAIL_START))
        .unwrap_or_else(|| text.units.last().map_or(0, |u| usize::from(u.first)))
}

/// The points of a text: one per part ([`parts`]), and for a short text planned to be sung
/// `statements` times, from two on, a second point: the last tail with a new subject, or the
/// first point restated (with `restate` on, or now and then with it automatic).
///
/// `imitation.points` changes the number: fewer points merge the shortest neighbouring parts,
/// more restate the parts of a short text in order where the profile allows `restatements`
/// (as [`crate::text::plan::plan`] takes them). Another number fails with
/// [`GenerateError::PointCount`].
pub(crate) fn points_of(
    text: &Text,
    statements: usize,
    restatements: bool,
    imitation: &Imitation,
    rng: &mut Rng,
) -> Result<Vec<Part>, GenerateError> {
    let mut base = parts(text);
    let n = text.syllables.len() as u16;
    let second = (statements >= 2 && !base.is_empty()).then(|| {
        let restate = match imitation.restate {
            Toggle::On => true,
            Toggle::Off => false,
            Toggle::Auto => rng.next_f64() < RESTATE,
        };
        if restate {
            (base[0].clone(), PointKind::Restatement { of: 0 })
        } else {
            (last_tail(text) as u16..n, PointKind::Tail)
        }
    });
    let automatic = base.len() + usize::from(second.is_some());
    // More points restate the parts of a short text, where restatements are allowed.
    let most = match text.kind {
        TextKind::Short if restatements => automatic + base.len(),
        _ => automatic,
    };
    let wanted = imitation.points.unwrap_or(automatic);
    if !(1..=most).contains(&wanted) {
        return Err(GenerateError::PointCount { most });
    }
    let mut extra: Vec<(Range<u16>, PointKind)> = Vec::new();
    if wanted == automatic {
        extra.extend(second);
    } else if wanted < automatic {
        // Fewer points: no second statement, and the shortest neighbours merged.
        while base.len() > wanted {
            let i = (0..base.len() - 1)
                .min_by_key(|&i| (base[i].len() + base[i + 1].len(), i))
                .unwrap_or(0);
            let next = base.remove(i + 1);
            base[i].end = next.end;
        }
    } else {
        // More points: the parts of a short text restated in order.
        extra.extend(second);
        let mut of = 0;
        while base.len() + extra.len() < wanted {
            extra.push((base[of].clone(), PointKind::Restatement { of }));
            of += 1;
        }
    }
    let mut points: Vec<Part> = base
        .into_iter()
        .map(|words| Part {
            head: head_words(text, words.clone()),
            words,
            kind: PointKind::New,
        })
        .collect();
    for (words, kind) in extra {
        let head = match kind {
            PointKind::Restatement { of } => points[of].head.clone(),
            _ => head_words(text, words.clone()),
        };
        points.push(Part { words, kind, head });
    }
    Ok(points)
}

/// The state of a voice's underlay before the head of point `p`: the start for the first point,
/// otherwise after the last syllable of the point before and a rest.
pub(crate) fn state_before(model: &Model, points: &[Part], p: usize) -> UState {
    match p.checked_sub(1).map(|q| &points[q]) {
        None => UState::START,
        Some(before) => model.after_rest(Some(usize::from(before.words.end) - 1)),
    }
}

/// The syllables of `range` as written, with hyphens within words and the words' punctuation:
/// `Ký-ri-e e-`, `San-ctus, San-ctus,`.
pub fn spelled(text: &Text, range: &Range<u16>) -> String {
    let mut out = String::new();
    for s in indices(range) {
        let flags = text.syllables[s].flags;
        if s > usize::from(range.start) && flags.contains(SylFlags::WORD_START) {
            out.push(' ');
        }
        out.push_str(&text.syllables[s].text);
        if !flags.contains(SylFlags::WORD_END) {
            out.push('-');
        } else if let Some(p) = text.word_of(s).punct {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{TextOptions, ordinary, resolve};

    fn text(spec: &str) -> Text {
        resolve(spec, &TextOptions::default()).unwrap().0
    }

    /// The points of a text as their words, with the kind of the second statement.
    fn points(spec: &str, statements: usize, restate: Toggle) -> Vec<String> {
        let text = text(spec);
        let imitation = Imitation {
            restate,
            ..Imitation::default()
        };
        points_of(&text, statements, true, &imitation, &mut Rng::new(1))
            .unwrap()
            .iter()
            .map(|part| {
                let words = spelled(&text, &part.words);
                match part.kind {
                    PointKind::New => words,
                    PointKind::Tail => format!("tail {words}"),
                    PointKind::Restatement { of } => format!("restated {of}: {words}"),
                }
            })
            .collect()
    }

    /// The words the head of each point declaims (SOG-SCOPE), through the first word
    /// that is not a function word and the next syllable.
    #[test]
    fn head_words_of_every_section() {
        let table = [
            ("kyrie", "Ký-ri-e e-lé-i-son.", "Ký-ri-e e-"),
            ("kyrie", "e-lé-i-son.", "e-lé-i-son."),
            ("christe", "Chri-ste e-lé-i-son.", "Chri-ste e-"),
            ("agnus", "A-gnus De-i,", "A-gnus De-"),
            ("sanctus", "San-ctus, San-ctus, San-ctus", "San-ctus,"),
            ("sanctus", "Dó-mi-nus De-us Sá-ba-oth.", "Dó-mi-nus De-"),
            ("agnus", "qui tol-lis pec-cá-ta mun-di:", "qui tol-lis pec-"),
            ("agnus", "mi-se-ré-re no-bis.", "mi-se-ré-re no-"),
            (
                "benedictus",
                "Be-ne-dí-ctus qui ve-nit",
                "Be-ne-dí-ctus qui",
            ),
            ("benedictus", "in nó-mi-ne Dó-mi-ni.", "in nó-mi-ne Dó-"),
            ("osanna", "Ho-sán-na in ex-cél-sis.", "Ho-sán-na in"),
            ("gloria", "Et in ter-ra pax", "Et in ter-ra pax"),
            // Monosyllables take one note each before the first longer word.
            (
                "Ad te le-vá-vi á-ni-mam me-am.",
                "Ad te le-vá-vi á-ni-mam me-am.",
                "Ad te le-vá-vi á-",
            ),
            (
                "In te Dó-mi-ne spe-rá-vi.",
                "In te Dó-mi-ne spe-rá-vi.",
                "In te Dó-mi-ne spe-",
            ),
            (
                "O quan-tus luc-tus hó-mi-num.",
                "O quan-tus luc-tus hó-mi-num.",
                "O quan-tus luc-",
            ),
            ("Tu es Pe-trus.", "Tu es Pe-trus.", "Tu es Pe-trus."),
            ("qui-tollis", "Tu so-lus Dó-mi-nus.", "Tu so-lus Dó-"),
        ];
        for (id, point, head) in table {
            let text = text(id);
            let n = text.syllables.len() as u16;
            let found = (0..n)
                .flat_map(|a| (a + 1..=n).map(move |b| a..b))
                .find(|r| {
                    spelled(&text, r) == point
                        && (r.start == 0 || text.is_opening(usize::from(r.start)))
                });
            let words = found.unwrap_or_else(|| panic!("{id}: no point {point}"));
            assert_eq!(spelled(&text, &head_words(&text, words)), head, "{id}");
        }
        // In every section of the Ordinary every head lies in its point and its first unit,
        // begins with the point and declaims six syllables at most; it reaches past the first
        // word of two syllables or more that is not a function word only by one syllable.
        for section in ordinary::sections(&TextOptions::default()) {
            for words in parts(section) {
                let head = head_words(section, words.clone());
                let label = format!("{} {}", section.id, spelled(section, &words));
                assert_eq!(head.start, words.start, "{label}");
                assert!(head.end <= words.end && head.len() <= 6, "{label}");
                assert!(!head.is_empty(), "{label}");
                let first = usize::from(head.start);
                let unit = section.unit_of(first);
                assert!(
                    usize::from(head.end) <= usize::from(unit.last) + 1,
                    "{label}"
                );
                let content = indices(&head)
                    .find(|&s| !skipped(section, s))
                    .map(|s| section.word_of(s).range().end);
                if let Some(end) = content {
                    assert!(usize::from(head.end) <= end + 1, "{label}");
                }
            }
        }
    }

    /// The points of the sections of the Ordinary and of an own text, with
    /// the second statement of a short text as its tail or as the first point restated.
    #[test]
    fn points_of_the_ordinary() {
        let kyrie = ["Ký-ri-e e-lé-i-son.", "tail e-lé-i-son."];
        assert_eq!(points("kyrie", 2, Toggle::Off), kyrie);
        assert_eq!(points("kyrie2", 2, Toggle::Off), kyrie);
        assert_eq!(
            points("kyrie", 2, Toggle::On),
            ["Ký-ri-e e-lé-i-son.", "restated 0: Ký-ri-e e-lé-i-son."]
        );
        assert_eq!(
            points("christe", 2, Toggle::Off),
            ["Chri-ste e-lé-i-son.", "tail e-lé-i-son."]
        );
        for osanna in ["osanna", "osanna2"] {
            assert_eq!(
                points(osanna, 2, Toggle::Off),
                ["Ho-sán-na in ex-cél-sis.", "tail in ex-cél-sis."]
            );
        }
        assert_eq!(
            points("sanctus", 1, Toggle::Auto),
            ["San-ctus, San-ctus, San-ctus", "Dó-mi-nus De-us Sá-ba-oth."]
        );
        assert_eq!(
            points("pleni", 1, Toggle::Auto),
            ["Ple-ni sunt cae-li et ter-ra", "gló-ri-a tu-a."]
        );
        assert_eq!(
            points("benedictus", 1, Toggle::Auto),
            ["Be-ne-dí-ctus qui ve-nit", "in nó-mi-ne Dó-mi-ni."]
        );
        assert_eq!(
            points("agnus", 1, Toggle::Auto),
            [
                "A-gnus De-i,",
                "qui tol-lis pec-cá-ta mun-di:",
                "mi-se-ré-re no-bis."
            ]
        );
        assert_eq!(points("agnus3", 1, Toggle::Auto)[2], "do-na no-bis pa-cem.");
        assert_eq!(
            points("gloria:1-5", 1, Toggle::Auto),
            [
                "Et in ter-ra pax ho-mí-ni-bus bo-nae vo-lun-tá-tis.",
                "Lau-dá-mus te. Be-ne-dí-ci-mus te.",
                "Ad-o-rá-mus te. Glo-ri-fi-cá-mus te."
            ]
        );
        assert_eq!(
            points("Ad te le-vá-vi á-ni-mam me-am.", 1, Toggle::Auto),
            ["Ad te le-vá-vi á-ni-mam me-am."]
        );
        // The planned statements of Phase 12 decide whether a second point follows.
        for (id, statements, count) in [
            ("kyrie", 2, 2),
            ("kyrie2", 2, 2),
            ("christe", 2, 2),
            ("osanna", 2, 2),
            ("osanna2", 2, 2),
            ("sanctus", 1, 2),
            ("pleni", 1, 2),
            ("benedictus", 1, 2),
            ("agnus", 1, 3),
            ("agnus3", 1, 3),
            ("gloria:1-5", 1, 3),
        ] {
            let plan =
                crate::text::plan::plan(&text(id), "dorian".parse().unwrap(), 4, None, None, true)
                    .unwrap();
            assert_eq!(plan.statements, statements, "{id}");
            assert_eq!(
                points(id, plan.statements, Toggle::Auto).len(),
                count,
                "{id}"
            );
        }
        // Automatic, the first point is restated now and then: about a quarter of the seeds.
        let text = text("kyrie");
        let restated = (0..400)
            .filter(|&seed| {
                let parts =
                    points_of(&text, 2, true, &Imitation::default(), &mut Rng::new(seed)).unwrap();
                matches!(parts[1].kind, PointKind::Restatement { of: 0 })
            })
            .count();
        assert!((70..130).contains(&restated), "{restated} of 400");
    }

    /// `--points` merges the shortest neighbouring parts or restates the parts of a short text,
    /// and fails beyond that.
    #[test]
    fn points_can_be_counted() {
        let text = text("agnus");
        let count = |points: usize| {
            let imitation = Imitation {
                points: Some(points),
                ..Imitation::default()
            };
            points_of(&text, 1, true, &imitation, &mut Rng::new(1)).map(|parts| {
                parts
                    .iter()
                    .map(|p| match p.kind {
                        PointKind::Restatement { of } => format!("R{of}"),
                        _ => spelled(&text, &p.words),
                    })
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(
            count(2).unwrap(),
            [
                "A-gnus De-i, qui tol-lis pec-cá-ta mun-di:",
                "mi-se-ré-re no-bis."
            ]
        );
        assert_eq!(count(1).unwrap().len(), 1);
        assert_eq!(count(5).unwrap()[3..], ["R0", "R1"]);
        assert_eq!(count(6).unwrap().len(), 6);
        assert_eq!(count(7), Err(GenerateError::PointCount { most: 6 }));
        assert_eq!(count(0), Err(GenerateError::PointCount { most: 6 }));
        // A long text has no room for restatements.
        let gloria = self::text("gloria:1-5");
        assert_eq!(parts(&gloria).len(), 3);
        let imitation = Imitation {
            points: Some(4),
            ..Imitation::default()
        };
        assert_eq!(
            points_of(&gloria, 1, true, &imitation, &mut Rng::new(1)),
            Err(GenerateError::PointCount { most: 3 })
        );
        // Nor has a text whose profile forbids restatements (TXT-REPEAT `short` V): Kyrie I is
        // then sung once, in one point.
        let kyrie = self::text("kyrie");
        let plan = crate::text::plan::plan(&kyrie, "dorian".parse().unwrap(), 4, None, None, false)
            .unwrap();
        assert_eq!(plan.statements, 1);
        let imitation = Imitation {
            points: Some(2),
            ..Imitation::default()
        };
        assert_eq!(
            points_of(&kyrie, plan.statements, false, &imitation, &mut Rng::new(1)),
            Err(GenerateError::PointCount { most: 1 })
        );
        let restated = points_of(&kyrie, 1, true, &imitation, &mut Rng::new(1)).unwrap();
        assert_eq!(restated[1].kind, PointKind::Restatement { of: 0 });
    }

    /// Each point's head begins from the state the voice is in: the first from the start, a
    /// later one after the last syllable of the point before and a rest, from which a tail or a
    /// restatement begins again.
    #[test]
    fn heads_begin_after_the_point_before() {
        let text = text("kyrie");
        let model = Model::new(
            &text,
            "dorian".parse().unwrap(),
            crate::time::Meter::ALLA_BREVE,
            &[],
        );
        for restate in [Toggle::On, Toggle::Off] {
            let imitation = Imitation {
                restate,
                ..Imitation::default()
            };
            let parts = points_of(&text, 2, true, &imitation, &mut Rng::new(1)).unwrap();
            assert_eq!(state_before(&model, &parts, 0), UState::START);
            let before = state_before(&model, &parts, 1);
            assert_eq!(before, model.after_rest(Some(6)));
            let first = usize::from(parts[1].words.start);
            assert_eq!(model.to_reach(&before, first as u16), Some(0));
        }
    }
}
