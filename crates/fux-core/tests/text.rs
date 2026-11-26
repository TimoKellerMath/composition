//! Texted free settings (phase 12): every voice of a generated setting sings the whole text by
//! the rules of the underlay, which an oracle checks without the rules' code, and the TXT rules
//! decide every case at the tick the notes show it, window by window and prefix by prefix, as
//! the search needs it. The matrices and the timings run with
//! `cargo test --release -p fux-core --test text -- --ignored --nocapture`.

mod common;

use std::sync::Arc;
use std::time::Instant;

use common::{
    assert_cheapest, assert_prefixes_collect, assert_sung, assert_texted, assert_windows_partition,
    choir, text_rules,
};
use fux_core::generate::{
    CounterpointRequest, FreeRequest, FreeText, GenerateError, Generated, NewVoice, Rng, SungText,
    cadence_formula, cadences_at, default_cadences, generate_counterpoint, generate_free,
};
use fux_core::rules::{Analysis, Checker, Detail, Profile, Violation};
use fux_core::text::plan::{automatic_length, plan};
use fux_core::text::{SylFlags, Text, TextOptions, ordinary, resolve};
use fux_core::{
    Ambitus, Duration, Lyric, Mode, ModeSpec, Note, Pitch, Role, Score, Species, Voice, VoiceType,
};

fn mode(name: &str) -> ModeSpec {
    name.parse().unwrap()
}

/// A text as `fux free --text` takes it: a section of the Ordinary, some of its units or an own
/// Latin text.
fn text(spec: &str) -> Arc<Text> {
    Arc::new(resolve(spec, &TextOptions::default()).unwrap().0)
}

/// A free setting of `voices` voices that all sing `spec`, with the length and the cadences
/// planned from the text.
fn request(spec: &str, mode: ModeSpec, voices: usize, seed: u64, beam: usize) -> FreeRequest {
    FreeRequest {
        seed,
        beam,
        text: Some(FreeText::new(text(spec))),
        ..FreeRequest::new(mode, choir(voices), 0)
    }
}

/// The best texted setting; panics if the search finds none or if it breaks a rule of the
/// underlay ([`assert_texted`]).
fn setting(spec: &str, mode: ModeSpec, voices: usize, seed: u64, beam: usize) -> Generated {
    let label = format!("{spec}, {}, {voices} voices, seed {seed}", mode.id());
    let request = request(spec, mode, voices, seed, beam);
    let results =
        generate_free(&request, &Profile::fux()).unwrap_or_else(|e| panic!("{label}: {e}"));
    let generated = results.into_iter().next().unwrap();
    assert_texted(&generated, &text(spec), &label);
    generated
}

/// The parameter of a TXT rule that a violation's case belongs to.
fn param(violation: &Violation) -> &'static str {
    match violation.detail {
        Detail::Param(param) => param,
        Detail::Count(_) => "long",
        _ => "",
    }
}

/// The acceptance test of phase 12: Kyrie I in two to four voices and in the Phrygian mode,
/// the Christe, Agnus III, two sentences of the Gloria and an own text, each at the length and
/// with the cadences its text plans.
#[test]
fn texted_free_settings_follow_the_rules() {
    for (spec, mode_name, voices) in [
        ("kyrie", "dorian", 2),
        ("kyrie", "dorian", 3),
        ("kyrie", "dorian", 4),
        ("kyrie", "phrygian", 3),
        ("christe", "aeolian", 3),
        ("agnus3", "mixolydian", 3),
        ("gloria:2-3", "ionian", 4),
        ("Ad te le-vá-vi á-ni-mam me-am.", "dorian", 3),
    ] {
        let generated = setting(spec, mode(mode_name), voices, 1, 60);
        let score = &generated.score;
        // The setting is as long as its text plans it, with the cadences planned.
        let planned = plan(&text(spec), mode(mode_name), voices, None, None, true).unwrap();
        assert_eq!(score.measure_count(), planned.measures, "{spec}");
        assert_eq!(score.cadences, planned.cadences, "{spec}");
        assert_eq!(score.voices.len(), voices, "{spec}");
        // The last sonority rests on the final in the bass.
        let bass = score.voices.last().unwrap().pitches();
        assert_eq!(
            bass.last().unwrap().step,
            mode(mode_name).final_step(),
            "{spec}"
        );
    }
}

/// The search sees the voices only up to the current tick: on texted settings the rules, those
/// of the text included, must decide everything as they do in the whole score.
#[test]
fn texted_settings_check_window_by_window() {
    let fux = Checker::new(&Profile::fux());
    let text_rules = text_rules();
    for (spec, mode_name, voices) in [
        ("kyrie", "dorian", 3),
        ("agnus3", "mixolydian", 2),
        ("gloria:2-3", "ionian", 4),
    ] {
        let score = setting(spec, mode(mode_name), voices, 1, 60).score;
        let name = format!("{spec}, {voices} voices");
        assert!(text_rules.check(&score).violations.len() > 3, "{name}");
        assert_windows_partition(&fux, &score, &name);
        assert_prefixes_collect(&fux, &score, &name);
        assert_windows_partition(&text_rules, &score, &name);
        assert_prefixes_collect(&text_rules, &score, &name);
    }
}

/// A free Dorian setting asked of the search directly, as `generate_free` asks for it: the
/// voices in score order, internal cadences arriving in the measures `goals` (from 0) and the
/// final cadence as formulas, with `spec` sung at a syllable start per measure.
fn counterpoint_request(
    voices: &[VoiceType],
    measures: usize,
    goals: &[usize],
    spec: &str,
    seed: u64,
) -> CounterpointRequest {
    let dorian = mode("dorian");
    let mut voices = voices.to_vec();
    voices.sort();
    let mut base = Score::new(dorian, Vec::new());
    base.cadences = cadences_at(dorian, goals);
    let meter = base.meter;
    let mut fixed = cadence_formula(
        dorian,
        dorian.final_step(),
        measures - 1,
        voices.len(),
        meter,
    );
    for cadence in &base.cadences {
        fixed.extend(cadence_formula(
            dorian,
            cadence.target,
            cadence.measure,
            voices.len(),
            meter,
        ));
    }
    CounterpointRequest {
        measures: Some(measures),
        fixed,
        seed,
        text: Some(SungText {
            text: text(spec),
            rate: 50,
        }),
        ..CounterpointRequest::new(
            base,
            voices
                .iter()
                .map(|&voice_type| NewVoice {
                    voice_type,
                    species: Species::Free,
                })
                .collect(),
        )
    }
}

/// Kyrie I in two and three voices over sixteen measures, searched directly with the cadence at
/// the end of the first statement (measure 7): every voice sings the text as the oracle wants
/// it, with the cheapest underlay of its notes.
#[test]
fn kyrie_in_two_and_three_voices() {
    use VoiceType::*;
    for voices in [&[Soprano, Tenor][..], &[Alto, Tenor, Bass]] {
        let label = format!("{} voices", voices.len());
        let request = counterpoint_request(voices, 16, &[6], "kyrie", 1);
        let mut results = generate_counterpoint(&request, &Profile::fux())
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let generated = &mut results[0];
        assert_eq!(generated.score.measure_count(), 16, "{label}");
        // The search leaves the syllables to write out to `generate_free`.
        assert!(
            generated
                .score
                .voices
                .iter()
                .flat_map(|v| &v.notes)
                .all(|n| n.syllable.is_none()),
            "{label}"
        );
        generated.score.fill_syllables(true);
        assert_texted(generated, &text("kyrie"), &label);
    }
}

/// A long text whose words go on in the next part of its section (phase 14), searched directly
/// over more measures than one statement needs. Where the words end, both voices restate the
/// last unit freely, like an Amen: "Lau-dá-mus te. Lau-dá-mus te." Where they go on, that
/// restatement costs as one of any other unit (TXT-REPEAT `long`), and the voices sing the
/// words once, as the oracle wants it; the search, the check of the result and the dynamic
/// program price the underlay alike ([`assert_texted`]).
#[test]
fn continued_texts_are_set_and_priced_alike() {
    use VoiceType::*;
    let repeats = |score: &Score| -> Vec<(u32, usize)> {
        Checker::new(&Profile::fux())
            .check(score)
            .violations
            .iter()
            .filter(|v| v.rule == "TXT-REPEAT")
            .map(|v| {
                assert_eq!(param(v), "long");
                (v.tick, v.voice)
            })
            .collect()
    };
    let ends = text("gloria:2");
    let mut goes_on = (*ends).clone();
    goes_on.continued = true;
    let goes_on = Arc::new(goes_on);
    let mut results = Vec::new();
    for sung in [&ends, &goes_on] {
        let label = format!("continued {}", sung.continued);
        let mut request = counterpoint_request(&[Soprano, Tenor], 10, &[], "gloria:2", 1);
        request.text.as_mut().unwrap().text = Arc::clone(sung);
        let mut generated = generate_counterpoint(&request, &Profile::fux())
            .unwrap_or_else(|e| panic!("{label}: {e}"))
            .remove(0);
        generated.score.fill_syllables(true);
        assert_texted(&generated, sung, &label);
        assert_eq!(repeats(&generated.score), [], "{label}");
        results.push(generated);
    }
    let [ending, continued] = &results[..] else {
        unreachable!()
    };
    for v in 0..2 {
        let restated = ending.score.voice_lyrics(v);
        assert_eq!(restated.as_deref(), Some("Lau-dá-mus te. Lau-dá-mus te."));
        let once = continued.score.voice_lyrics(v);
        assert_eq!(once.as_deref(), Some("Lau-dá-mus te."));
    }
    // The setting of the text that ends, checked as one that goes on: each restatement costs.
    let mut checked = ending.score.clone();
    checked.text.as_mut().unwrap().text = Arc::clone(&goes_on);
    let restatements = repeats(&checked);
    assert_eq!(restatements.len(), 2, "{restatements:?}");
    assert!(continued.cost > ending.cost);
}

/// A new voice sings over a given cantus of Fux, which does not: the new voice sings the text as
/// the oracle wants it, with the cheapest underlay of its notes.
#[test]
fn a_texted_voice_over_a_cantus() {
    use fux_core::corpus::fux_cantus;
    let cantus = fux_cantus().into_iter().find(|c| c.id == "dorian").unwrap();
    let shift = VoiceType::Alto.octave_shift_for(&cantus.notes) as i8;
    let notes = cantus
        .notes
        .iter()
        .map(|p| {
            Note::new(
                Pitch::new(p.step, p.alter, p.octave + shift),
                Duration::WHOLE,
            )
        })
        .collect();
    let base = Score::new(
        cantus.mode,
        vec![Voice::new(VoiceType::Alto, Role::CantusFirmus, notes)],
    );
    let request = CounterpointRequest {
        text: Some(SungText {
            text: text("kyrie"),
            rate: 50,
        }),
        ..CounterpointRequest::new(
            base,
            vec![NewVoice {
                voice_type: VoiceType::Soprano,
                species: Species::First,
            }],
        )
    };
    let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
    let score = &results[0].score;
    score.validate().unwrap();
    assert_eq!(results[0].report.forbidden(), 0);
    let sung = score.text.as_ref().unwrap();
    let counterpoint: Vec<bool> = score.voices.iter().map(|v| !v.is_cantus_firmus()).collect();
    assert_eq!(sung.voices, counterpoint);
    let (c, _) = score.cantus_firmus().unwrap();
    assert_sung(&sung.text, &score.voices[1 - c].notes, &[], "first species");
    assert!(score.voices[c].notes.iter().all(|n| n.lyric.is_none()));
    assert_cheapest(&results[0], "first species");
}

/// Free voices around one of Fux's cantus in `cantus_voice`, as `fux cp --species 0 --text`
/// asks the search for them: the cantus fixes the length, and the text is planned over its
/// measures for voices without cadence formulas, as for a single voice without internal
/// cadences.
fn around_cantus(
    id: &str,
    cantus_voice: VoiceType,
    voices: &[VoiceType],
    spec: &str,
    seed: u64,
) -> CounterpointRequest {
    use fux_core::corpus::fux_cantus;
    let cantus = fux_cantus().into_iter().find(|c| c.id == id).unwrap();
    let shift = cantus_voice.octave_shift_for(&cantus.notes) as i8;
    let notes = cantus
        .notes
        .iter()
        .map(|p| {
            Note::new(
                Pitch::new(p.step, p.alter, p.octave + shift),
                Duration::WHOLE,
            )
        })
        .collect();
    let base = Score::new(
        cantus.mode,
        vec![Voice::new(cantus_voice, Role::CantusFirmus, notes)],
    );
    let planned = plan(
        &text(spec),
        cantus.mode,
        1,
        Some(base.measure_count()),
        Some(Vec::new()),
        true,
    )
    .unwrap();
    CounterpointRequest {
        seed,
        text: Some(SungText {
            text: text(spec),
            rate: planned.rate,
        }),
        ..CounterpointRequest::new(
            base,
            voices
                .iter()
                .map(|&voice_type| NewVoice {
                    voice_type,
                    species: Species::Free,
                })
                .collect(),
        )
    }
}

/// Free voices around a cantus sing their text by the rules of the underlay, and the cantus
/// sings none: no prohibition, also under every TXT rule; every free voice sings the text as
/// the oracle wants it, with the cheapest underlay of its notes; and the rules decide everything
/// window by window and prefix by prefix, the text's among them.
#[test]
fn texted_free_voices_around_a_cantus() {
    use VoiceType::*;
    let fux = Checker::new(&Profile::fux());
    let text_rules = text_rules();
    for (id, cantus_voice, voices, spec) in [
        ("dorian", Alto, &[Soprano][..], "kyrie"),
        ("phrygian", Alto, &[Tenor, Bass], "christe"),
        (
            "ionian",
            Tenor,
            &[Soprano],
            "Ad te le-vá-vi á-ni-mam me-am.",
        ),
    ] {
        let label = format!("{spec} around fux:{id}, {} voices", voices.len() + 1);
        let request = around_cantus(id, cantus_voice, voices, spec, 1);
        let mut results = generate_counterpoint(&request, &Profile::fux())
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let generated = &mut results[0];
        generated.score.fill_syllables(true);
        let score = &generated.score;
        assert_eq!(generated.report.forbidden(), 0, "{label}");
        score.validate().unwrap_or_else(|e| panic!("{label}: {e}"));
        assert!(!score.is_free(), "{label}");
        let sung = score.text.as_ref().expect("a text");
        assert_eq!(*sung.text, *text(spec), "{label}");
        let free: Vec<bool> = score.voices.iter().map(|v| !v.is_cantus_firmus()).collect();
        assert_eq!(sung.voices, free, "{label}");
        // The search priced the free voices under a copy of the profile; the profile itself
        // finds the same in them, and no prohibition in the whole score.
        let report = fux.check(score);
        assert_eq!(report.forbidden(), 0, "{label}: {:?}", report.violations);
        let in_free: Vec<&Violation> = report.violations.iter().filter(|v| free[v.voice]).collect();
        let found: Vec<&Violation> = generated.report.violations.iter().collect();
        assert_eq!(in_free, found, "{label}");
        assert_eq!(text_rules.check(score).forbidden(), 0, "{label}");
        for (v, voice) in score.voices.iter().enumerate() {
            let label = format!("{label}, voice {v}");
            if voice.is_cantus_firmus() {
                assert!(
                    voice
                        .notes
                        .iter()
                        .all(|n| n.lyric.is_none() && n.syllable.is_none()),
                    "{label}"
                );
                assert_eq!(score.voice_lyrics(v), None, "{label}");
                continue;
            }
            assert_sung(&sung.text, &voice.notes, &[], &label);
            for (i, note) in voice.notes.iter().enumerate() {
                assert_eq!(
                    note.syllable.is_some(),
                    note.lyric.is_some(),
                    "{label}, note {i}"
                );
            }
            let lyrics = score.voice_lyrics(v).unwrap();
            let last = sung
                .text
                .syllable_label(sung.text.syllables.len() - 1, true);
            assert!(lyrics.ends_with(&last), "{label}: {lyrics}");
        }
        assert_cheapest(generated, &label);
        let score = &generated.score;
        assert_windows_partition(&fux, score, &label);
        assert_prefixes_collect(&fux, score, &label);
        assert_windows_partition(&text_rules, score, &label);
        assert_prefixes_collect(&text_rules, score, &label);
    }
}

/// Where each note of a voice begins.
fn starts(notes: &[Note]) -> Vec<u32> {
    notes
        .iter()
        .scan(0, |tick, note| {
            let start = *tick;
            *tick += note.duration.ticks();
            Some(start)
        })
        .collect()
}

/// Whether note `i` is an attack: the first written note of a sounding.
fn attack(notes: &[Note], i: usize) -> bool {
    !notes[i].is_rest() && !(i > 0 && notes[i - 1].tie)
}

/// Whether attack `i` is an entry: the voice's first attack or the first after a rest.
fn entry(notes: &[Note], i: usize) -> bool {
    i == 0 || notes[i - 1].is_rest()
}

/// The sounding that ends where note `i` begins, tied notes merged: its pitch and length.
fn sounding_before(notes: &[Note], i: usize) -> Option<(Pitch, u32)> {
    let before = notes.get(i.checked_sub(1)?)?;
    let pitch = before.pitch?;
    let mut ticks = before.duration.ticks();
    let mut j = i - 1;
    while j > 0 && notes[j - 1].tie {
        j -= 1;
        ticks += notes[j].duration.ticks();
    }
    Some((pitch, ticks))
}

/// Whether note `i` strikes again the pitch of a white sounding right before it: it needs a new
/// syllable.
fn restruck(notes: &[Note], i: usize) -> bool {
    sounding_before(notes, i).is_some_and(|(pitch, ticks)| {
        notes[i].pitch == Some(pitch) && ticks >= Duration::HALF.ticks()
    })
}

/// Whether attack `i` could begin a syllable without any case of the TXT rules: a white note on
/// a minim beat after a white sounding.
fn may_begin(notes: &[Note], i: usize) -> bool {
    let at = starts(notes)[i];
    attack(notes, i)
        && notes[i].duration.ticks() >= Duration::HALF.ticks()
        && at.is_multiple_of(Duration::HALF.ticks())
        && sounding_before(notes, i).is_none_or(|(_, ticks)| ticks >= Duration::HALF.ticks())
}

/// The syllable a lyric names.
fn syllable(note: &Note) -> Option<usize> {
    note.lyric.and_then(|lyric| lyric.index())
}

/// A generated setting with one fault in the underlay of a voice, and the case of the TXT rules
/// that must report it: the rule, its parameter, the note it marks (a case of a syllable's
/// notes marks its first) and the tick it is decided at.
struct Falsified {
    what: &'static str,
    score: Score,
    voice: usize,
    rule: &'static str,
    param: &'static str,
    note: usize,
    tick: u32,
}

/// The faults of the falsification test that can be made in voice `v` of a texted score: a
/// syllable shifted onto the off-beat quarter after it; a syllable moved from a tied note onto its
/// continuation; a note inside a word replaced by a rest; the syllable of a re-struck pitch moved
/// on to the next note; and a melisma on the short penult of Dó-mi-nus. Each fault is made where
/// it causes no other prohibited case.
fn faults(score: &Score, v: usize) -> Vec<Falsified> {
    let sung = score.text.as_ref().unwrap();
    let text = &sung.text;
    let flags = |y: usize| text.syllables[y].flags;
    let notes = &score.voices[v].notes;
    let at = starts(notes);
    let mut out = Vec::new();
    let falsify = |change: &dyn Fn(&mut Vec<Note>)| {
        let mut score = score.clone();
        change(&mut score.voices[v].notes);
        score.fill_syllables(true);
        score
    };
    // The penults of Dó-mi-nus.
    let dominus: Vec<usize> = text
        .words
        .iter()
        .filter(|word| {
            let syllables: Vec<&str> = word
                .range()
                .map(|s| text.syllables[s].text.as_str())
                .collect();
            syllables == ["Dó", "mi", "nus"]
        })
        .map(|word| usize::from(word.first) + 1)
        .collect();
    for i in 0..notes.len() {
        if !attack(notes, i) || i + 1 >= notes.len() {
            continue;
        }
        let y = syllable(&notes[i]);
        let next = &notes[i + 1];
        // The syllable sung before note i.
        let before = (0..i).rev().find_map(|j| syllable(&notes[j]));
        // Attack i begins a syllable in the middle of the voice's line, and without it the
        // syllable before would continue there without any other case.
        let shiftable = y.is_some()
            && !entry(notes, i)
            && before.is_some_and(|x| !flags(x).contains(SylFlags::SHORT_PENULT))
            && !restruck(notes, i);
        if shiftable && !notes[i].tie && !next.is_rest() && next.lyric.is_none() {
            let y = y.unwrap();
            let dactyl = notes[i].duration == Duration::DOTTED_HALF && at[i].is_multiple_of(4);
            if next.duration == Duration::QUARTER && !at[i + 1].is_multiple_of(4) && !dactyl {
                out.push(Falsified {
                    what: "a syllable shifted onto the next attack",
                    score: falsify(&|notes: &mut Vec<Note>| {
                        notes[i].lyric = None;
                        notes[i + 1].lyric = lyric(y);
                    }),
                    voice: v,
                    rule: "TXT-ONSET",
                    param: "level",
                    note: i + 1,
                    tick: at[i + 1],
                });
            }
        }
        if shiftable && notes[i].tie {
            let y = y.unwrap();
            out.push(Falsified {
                what: "a syllable moved onto a tie continuation",
                score: falsify(&|notes: &mut Vec<Note>| {
                    notes[i].lyric = None;
                    notes[i + 1].lyric = lyric(y);
                }),
                voice: v,
                rule: "TXT-ONSET",
                param: "level",
                note: i + 1,
                tick: at[i + 1],
            });
        }
        // A note inside a word, white before it and the word's next syllable right after it.
        if y.is_none() && !notes[i].tie && i > 0 && !next.is_rest() {
            let inside = before.is_some_and(|x| {
                !flags(x).contains(SylFlags::WORD_END) && syllable(next) == Some(x + 1)
            });
            let white =
                sounding_before(notes, i).is_some_and(|(_, ticks)| ticks >= Duration::HALF.ticks());
            if inside && white {
                out.push(Falsified {
                    what: "a note inside a word replaced by a rest",
                    score: falsify(&|notes: &mut Vec<Note>| {
                        notes[i] = Note::rest(notes[i].duration);
                    }),
                    voice: v,
                    rule: "TXT-REST",
                    param: "word",
                    note: i,
                    tick: at[i],
                });
            }
        }
        // A re-struck pitch whose syllable can begin on the next note instead.
        if let Some(y) = y
            && !entry(notes, i)
            && restruck(notes, i)
            && before.is_some_and(|x| !flags(x).contains(SylFlags::SHORT_PENULT))
            && !notes[i].tie
            && next.lyric.is_none()
            && may_begin(notes, i + 1)
        {
            out.push(Falsified {
                what: "the syllable of a re-struck pitch moved on",
                score: falsify(&|notes: &mut Vec<Note>| {
                    notes[i].lyric = None;
                    notes[i + 1].lyric = lyric(y);
                }),
                voice: v,
                rule: "TXT-REPEATED-PITCH",
                param: "level",
                note: i,
                tick: at[i],
            });
        }
        // The mi of Dó-mi-nus on one note takes notes from Dó before it or from nus after it:
        // it begins on a note of Dó's melisma, or nus on a later note of its own.
        if let Some(m) = y.filter(|y| dominus.contains(y))
            && !notes[i].tie
        {
            let melisma = |from: usize, to: usize| (from..to).filter(|&k| attack(notes, k));
            // The last note of Dó's melisma that could begin mi. Its own note stays white, so
            // that nus does not lose the licence after a texted dactyl quarter.
            let dó = (0..i).rev().find(|&k| notes[k].lyric.is_some());
            if let Some(k) =
                dó.and_then(|first| (first + 1..i).rev().find(|&k| may_begin(notes, k)))
                && !entry(notes, i)
                && !restruck(notes, i)
                && sounding(notes, i) >= Duration::HALF.ticks()
            {
                // The case marks the syllable's first note, at its second attack.
                let second = melisma(k + 1, notes.len()).next().unwrap();
                out.push(Falsified {
                    what: "a melisma on the mi of Dó-mi-nus",
                    score: falsify(&|notes: &mut Vec<Note>| {
                        notes[k].lyric = lyric(m);
                        notes[i].lyric = None;
                    }),
                    voice: v,
                    rule: "TXT-ACCENT",
                    param: "short_penult",
                    note: k,
                    tick: at[second],
                });
            }
            // A later note of nus's melisma that could begin it.
            let nus = i + 1;
            let end = (nus + 1..notes.len())
                .find(|&k| notes[k].is_rest() || notes[k].lyric.is_some())
                .unwrap_or(notes.len());
            if syllable(next) == Some(m + 1)
                && !restruck(notes, nus)
                && let Some(k) = melisma(nus + 1, end).find(|&k| may_begin(notes, k))
            {
                out.push(Falsified {
                    what: "a melisma on the mi of Dó-mi-nus",
                    score: falsify(&|notes: &mut Vec<Note>| {
                        notes[nus].lyric = None;
                        notes[k].lyric = lyric(m + 1);
                    }),
                    voice: v,
                    rule: "TXT-ACCENT",
                    param: "short_penult",
                    note: i,
                    tick: at[nus],
                });
            }
        }
    }
    out
}

/// The lyric of syllable `y`.
fn lyric(y: usize) -> Option<Lyric> {
    Some(Lyric::new(y))
}

/// Falsified underlays of generated settings: each fault is reported once, by the rule that
/// forbids it, on the note it concerns and at the earliest tick at which the notes show it, and
/// windows and prefixes find it there too. This is the guard against cases decided late.
#[test]
fn falsified_underlays_are_reported_at_their_tick() {
    const KINDS: [&str; 5] = [
        "a syllable shifted onto the next attack",
        "a syllable moved onto a tie continuation",
        "a note inside a word replaced by a rest",
        "the syllable of a re-struck pitch moved on",
        "a melisma on the mi of Dó-mi-nus",
    ];
    let fux = Checker::new(&Profile::fux());
    let text_rules = text_rules();
    let count = |tested: &[&str], what: &str| tested.iter().filter(|&&w| w == what).count();
    // A few faults of each kind in a score.
    let falsify = |score: &Score, tested: &mut Vec<&'static str>| {
        for v in 0..score.voices.len() {
            for fault in faults(score, v) {
                if count(tested, fault.what) < 3 {
                    tested.push(fault.what);
                    assert_reported(&fault, &fux, &text_rules);
                }
            }
        }
    };
    let mut tested: Vec<&'static str> = Vec::new();
    falsify(
        &setting("kyrie", mode("dorian"), 3, 1, 60).score,
        &mut tested,
    );
    falsify(
        &setting("christe", mode("aeolian"), 3, 1, 60).score,
        &mut tested,
    );
    // A text with Dó-mi-nus, until every kind of fault has found a place.
    for seed in 11..19 {
        if KINDS.iter().all(|what| count(&tested, what) > 0) {
            break;
        }
        let score = setting("Dó-mi-nus vo-bís-cum.", mode("dorian"), 2, seed, 60).score;
        falsify(&score, &mut tested);
    }
    for what in KINDS {
        assert!(count(&tested, what) > 0, "no place for {what}");
        println!("{what}: {}", count(&tested, what));
    }
}

/// A fault is the only prohibited case of the TXT rules in its score, reported by its rule, on
/// its note and at its tick, and windows and prefixes find the same.
fn assert_reported(fault: &Falsified, fux: &Checker, text_rules: &Checker) {
    let label = format!("{}, voice {}, note {}", fault.what, fault.voice, fault.note);
    fault.score.validate().unwrap();
    let forbidden: Vec<(&str, &str, usize, usize, u32)> = fux
        .check(&fault.score)
        .violations
        .iter()
        .filter(|x| x.rule.starts_with("TXT-") && x.is_forbidden())
        .map(|x| (x.rule, param(x), x.voice, x.note, x.tick))
        .collect();
    assert_eq!(
        forbidden,
        [(fault.rule, fault.param, fault.voice, fault.note, fault.tick)],
        "{label}"
    );
    assert_windows_partition(text_rules, &fault.score, &label);
    assert_prefixes_collect(text_rules, &fault.score, &label);
    assert_windows_partition(fux, &fault.score, &label);
    assert_prefixes_collect(fux, &fault.score, &label);
}

/// Without a text a free setting is what it was before phase 12: the two-voice Dorian setting
/// of tests/free.rs, note for note, without lyrics and without a text in the score.
#[test]
fn untexted_requests_stay_untexted() {
    let dorian = mode("dorian");
    let request = FreeRequest {
        cadences: default_cadences(dorian, 10),
        beam: 100,
        ..FreeRequest::new(dorian, choir(2), 10)
    };
    assert_eq!(request.text, None);
    let results = generate_free(&request, &Profile::fux()).unwrap();
    let score = &results[0].score;
    assert_eq!(score.text, None);
    assert!(
        score
            .voices
            .iter()
            .flat_map(|v| &v.notes)
            .all(|n| n.lyric.is_none() && n.syllable.is_none())
    );
    let voices: Vec<String> = score
        .voices
        .iter()
        .map(|voice| {
            let notes: Vec<String> = voice
                .notes
                .iter()
                .map(|n| {
                    let pitch = n.pitch.map_or("r".to_string(), |p| p.to_string());
                    let tie = if n.tie { "~" } else { "" };
                    format!("{pitch}:{}{tie}", n.duration.ticks())
                })
                .collect();
            notes.join(" ")
        })
        .collect();
    assert_eq!(
        voices,
        [
            "D5:8 E5:8 C5:2 B4:2 A4:2 G4:2 A4:8~ A4:4 G#4:4 A4:8 Bb4:6 G4:2 F4:4 D4:4~ D4:4 \
             C#4:4 D4:8",
            "r:4 D4:4~ D4:4 C4:4 E4:6 D4:2 C4:6 A3:2 B3:8 A3:8 G3:2 F3:2 D3:2 E3:2 A3:2 G3:2 \
             F3:4 E3:8 D3:8",
        ]
    );
    assert_eq!(results[0].cost, 3);
}

/// The notes of each voice as text: pitch (`r` for a rest), ticks, `~` for a tie and, after `=`,
/// the position of the syllable the note begins.
fn sung_notes(score: &Score) -> Vec<String> {
    score
        .voices
        .iter()
        .map(|voice| {
            let notes: Vec<String> = voice
                .notes
                .iter()
                .map(|n| {
                    let pitch = n.pitch.map_or("r".to_string(), |p| p.to_string());
                    let tie = if n.tie { "~" } else { "" };
                    let lyric = n
                        .lyric
                        .map_or(String::new(), |l| format!("={}", l.index().unwrap()));
                    format!("{pitch}:{}{tie}{lyric}", n.duration.ticks())
                })
                .collect();
            notes.join(" ")
        })
        .collect()
}

/// Without imitation a texted free setting is what it has been since the bare sixth and a final
/// chord without a major third cost in free counterpoint (L3, L6; before, the settings of phase
/// 12 at costs 27 and 24, both ending without a third): Kyrie I, Dorian, three voices, beam 40,
/// seeds 1 and 2, note for note and syllable for syllable, at the same cost.
#[test]
fn texted_results_are_unchanged() {
    let dorian = mode("dorian");
    let golden = |seed: u64, voices: [&str; 3], cost: u32| {
        let request = request("kyrie", dorian, 3, seed, 40);
        assert_eq!(request.imitation, None);
        let results = generate_free(&request, &Profile::fux()).unwrap();
        let score = &results[0].score;
        assert_eq!(score.cadences, cadences_at(dorian, &[6]), "seed {seed}");
        assert_eq!(score.imitation, None, "seed {seed}");
        assert_eq!(sung_notes(score), voices, "seed {seed}");
        assert_eq!(results[0].cost, cost, "seed {seed}");
    };
    golden(
        1,
        [
            "r:4 A4:4~=0 A4:4 F4:4~=1 F4:2 E4:2=2 G4:4=3 A4:4=4 G4:4~ G4:2 F4:2 A4:4~ A4:4 \
             G#4:4=5 A4:8=6 F4:8=0 E4:2 D4:2 F4:4~ F4:2 E4:2 C4:4~ C4:2 Bb3:2=1 C4:4=2 E4:4=3 \
             E4:4~=4 E4:2 F4:2 D4:4~ D4:2 E4:2 D4:4~ D4:4 C#4:4=5 D4:8=6",
            "D4:6=0 D4:2=1 C4:8=2 E4:6=3 D4:2 E4:6 C#4:2 D4:4 D4:4=4 C#4:8~ C#4:2 B3:2 C4:4 D4:2 \
             Bb3:2 A3:2 B3:2 G3:4 F3:4~=5 F3:8 E3:2=6 D3:2 E3:4~ E3:4 r:4 B3:6=3 A3:2 F3:4 \
             F3:4~=4 F3:2 E3:2=5 F#3:4~=6 F#3:8",
            "r:8 A3:8=0 G3:4 C3:4~=1 C3:2 D3:2=2 E3:4=3 D3:8=4 E3:8=5 A2:8=6 r:4 D3:4~=3 D3:8 \
             C3:2 G2:2 A2:2 Bb2:2 C3:8 G2:8 G2:2=4 A2:2 B2:2 C#3:2 D3:4 Bb2:4 A2:8=5 D3:8=6",
        ],
        26,
    );
    golden(
        2,
        [
            "A3:6=0 C4:2 C4:2 B3:2 C4:4~ C4:2 A3:2=1 B3:4~=2 B3:4 A3:4~=3 A3:2 G3:2=4 A3:4~ A3:4 \
             G#3:4=5 A3:8=6 r:4 F4:4~=0 F4:4 A4:2 G4:2 F4:6 F4:2=1 G4:4=2 G4:4~=3 G4:4 E4:4=4 \
             D4:6 E4:2 F4:4 D4:4~ D4:4 C#4:4=5 D4:8=6",
            "A3:8=0 B3:4=1 G3:4~=2 G3:8 F#3:8=3 G3:4=4 F3:4~ F3:4 E3:4~=5 E3:8 D3:8=6 r:8 A3:8=3 \
             Bb3:4 G3:4 G3:2=4 A3:2 C4:4~ C4:2 Bb3:2=5 D4:4~=6 D4:8 C4:2 B3:2 A3:4~ A3:8",
            "A3:6=0 F3:2 E3:8 D3:2 C3:2 B2:2 G2:2 A2:6 D3:2=1 C3:6=2 F3:2 E3:8 A2:8 Bb2:8=3 \
             D3:6=4 C#3:2 D3:6 A3:2 G3:2 F3:2 G3:4~ G3:8 Bb3:6 Bb3:2=5 A3:4=6 B3:4 A3:8 D3:8",
        ],
        34,
    );
}

/// Where the profile forbids restatements (`fux free --repeat keine`), the text is planned to
/// be sung once and every voice sings each syllable once.
#[test]
fn texts_sung_once_without_restatements() {
    let mut profile = Profile::fux();
    for repeat in ["TXT-REPEAT.short=V", "TXT-REPEAT.long=V"] {
        profile.apply_override(repeat).unwrap();
    }
    let kyrie = text("kyrie");
    let results = generate_free(&request("kyrie", mode("dorian"), 2, 1, 60), &profile).unwrap();
    let score = &results[0].score;
    assert_eq!(results[0].report.forbidden(), 0);
    assert_eq!(score.measure_count(), automatic_length(&kyrie, false));
    for (v, voice) in score.voices.iter().enumerate() {
        assert_sung(&kyrie, &voice.notes, &[], &format!("voice {v}"));
        let sung: Vec<usize> = voice.notes.iter().filter_map(syllable).collect();
        assert_eq!(
            sung,
            (0..kyrie.syllables.len()).collect::<Vec<_>>(),
            "voice {v}"
        );
    }
}

/// A texted request whose given length is too short fails before the search, with the length
/// the text needs.
#[test]
fn lengths_the_text_needs() {
    let mut request = request("agnus", mode("dorian"), 2, 1, 60);
    request.measures = 6;
    request.text.as_mut().unwrap().auto_length = false;
    let error = generate_free(&request, &Profile::fux()).unwrap_err();
    assert!(
        matches!(error, GenerateError::TextTooLong { measures: 10.. }),
        "{error:?}"
    );
}

/// The settings of a random sample: seeds, modes, texts and choirs.
fn random_settings(cases: usize) {
    const TEXTS: [&str; 5] = [
        "kyrie",
        "christe",
        "agnus3",
        "Ad te le-vá-vi á-ni-mam me-am.",
        "Dó-mi-nus vo-bís-cum.",
    ];
    let mut rng = Rng::new(12);
    let below = |rng: &mut Rng, n: usize| (rng.next_u64() % n as u64) as usize;
    for _ in 0..cases {
        let spec = TEXTS[below(&mut rng, TEXTS.len())];
        let mode = ModeSpec::new(Mode::ALL[below(&mut rng, Mode::ALL.len())])
            .with_ambitus(if below(&mut rng, 2) == 0 {
                Ambitus::Authentic
            } else {
                Ambitus::Plagal
            })
            .with_transposition(below(&mut rng, 2) == 0);
        let voices = 2 + below(&mut rng, 3);
        let seed = rng.next_u64() % 1000;
        setting(spec, mode, voices, seed, 60);
    }
}

/// A random sample of texted settings, of which more run with the ignored tests.
#[test]
fn random_texted_settings() {
    random_settings(4);
}

#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn many_random_texted_settings() {
    random_settings(64);
}

/// Kyrie I in every mode, plagal and transposed ones included, with two to four voices.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn every_mode_with_text() {
    let clock = Instant::now();
    let mut runs = 0;
    for mode in Mode::ALL {
        for ambitus in [Ambitus::Authentic, Ambitus::Plagal] {
            for transposed in [false, true] {
                for voices in 2..=4 {
                    let spec = ModeSpec::new(mode)
                        .with_ambitus(ambitus)
                        .with_transposition(transposed);
                    setting("kyrie", spec, voices, 1, 200);
                    runs += 1;
                }
            }
        }
    }
    println!("{runs} settings in {:.1} s", clock.elapsed().as_secs_f64());
}

/// Agnus III and the first five units of the Gloria in five and six voices. Six voices
/// singing the Gloria take from half a minute to several, as untexted settings of six voices
/// and that length do.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn five_and_six_voices_with_text() {
    for (spec, mode_name, voices) in [
        ("agnus3", "dorian", 5),
        ("agnus3", "mixolydian", 6),
        ("gloria:1-5", "aeolian", 5),
        ("gloria:1-5", "ionian", 6),
    ] {
        let clock = Instant::now();
        let generated = setting(spec, mode(mode_name), voices, 1, 200);
        println!(
            "{spec}, {mode_name}, {voices} voices: {} measures, cost {}, {:.1} s",
            generated.score.measure_count(),
            generated.cost,
            clock.elapsed().as_secs_f64()
        );
    }
}

/// Every section of the Ordinary at its planned length, in four voices.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn every_section() {
    for (i, section) in ordinary::sections(&TextOptions::default())
        .iter()
        .enumerate()
    {
        let mode = ModeSpec::new(Mode::ALL[i % Mode::ALL.len()]);
        let clock = Instant::now();
        let generated = setting(&section.id, mode, 4, 1, 200);
        println!(
            "{}, {}: {} measures, cadences {:?}, cost {}, {:.1} s",
            section.id,
            mode.id(),
            generated.score.measure_count(),
            generated
                .score
                .cadences
                .iter()
                .map(|c| c.measure + 1)
                .collect::<Vec<_>>(),
            generated.cost,
            clock.elapsed().as_secs_f64()
        );
    }
}

/// Measures of declamation and line in settings: per voice note attacks per syllable, the share
/// of stressed syllables (of words of two or more) that begin on a downbeat or on a note longer
/// than the one before, the share of white notes that strike the pitch of a white note before
/// again, and of all moves that leap, and the mean range of a voice over four measures (steps).
#[derive(Default)]
struct Style {
    attacks: usize,
    syllables: usize,
    stressed: usize,
    accented: usize,
    white: usize,
    restruck: usize,
    moves: usize,
    leaps: usize,
    ranges: i64,
    windows: usize,
}

impl Style {
    fn add(&mut self, score: &Score) {
        let analysis = Analysis::new(score);
        let per = analysis.per_measure;
        let text = score.text.as_ref().map(|t| t.text.clone());
        for (v, voice) in score.voices.iter().enumerate() {
            let notes = &voice.notes;
            let at = starts(notes);
            for i in 0..notes.len() {
                if !attack(notes, i) {
                    continue;
                }
                self.attacks += 1;
                // As counted in Palestrina's voices: the attack before, without a rest between,
                // is written white too and has the same pitch.
                if notes[i].duration.ticks() >= Duration::HALF.ticks() {
                    self.white += 1;
                    let before = (0..i).rev().find(|&j| attack(notes, j));
                    if before.is_some_and(|j| {
                        !notes[i - 1].is_rest()
                            && notes[j].pitch == notes[i].pitch
                            && notes[j].duration.ticks() >= Duration::HALF.ticks()
                    }) {
                        self.restruck += 1;
                    }
                }
                let (Some(text), Some(y)) = (&text, syllable(&notes[i])) else {
                    continue;
                };
                self.syllables += 1;
                let word = text.word_of(y);
                if word.len >= 2 && text.syllables[y].flags.contains(SylFlags::STRESS) {
                    self.stressed += 1;
                    let longer = sounding_before(notes, i)
                        .is_some_and(|(_, ticks)| sounding(notes, i) > ticks);
                    if at[i].is_multiple_of(per) || longer {
                        self.accented += 1;
                    }
                }
            }
            let line = &analysis.lines[v];
            for pair in line.windows(2) {
                if pair[0].end == pair[1].start && pair[0].pitch != pair[1].pitch {
                    self.moves += 1;
                    let steps = (pair[1].pitch.diatonic() - pair[0].pitch.diatonic()).abs();
                    if steps >= 2 {
                        self.leaps += 1;
                    }
                }
            }
            let span = 4 * per;
            let mut from = 0;
            while from + span <= analysis.total {
                let inside: Vec<i32> = line
                    .iter()
                    .filter(|s| s.start >= from && s.start < from + span)
                    .map(|s| s.pitch.diatonic())
                    .collect();
                if let (Some(lo), Some(hi)) = (inside.iter().min(), inside.iter().max()) {
                    self.ranges += i64::from(hi - lo);
                    self.windows += 1;
                }
                from += span;
            }
        }
    }

    fn row(&self, name: &str) -> String {
        let share = |a: usize, b: usize| 100.0 * a as f64 / b.max(1) as f64;
        let per_syllable = match self.syllables {
            0 => "-".to_string(),
            n => format!("{:.2}", self.attacks as f64 / n as f64),
        };
        format!(
            "{name:<34} {per_syllable:>8} {:>9.1} {:>9.1} {:>7.1} {:>7.2}",
            share(self.accented, self.stressed),
            share(self.restruck, self.white),
            share(self.leaps, self.moves),
            self.ranges as f64 / self.windows.max(1) as f64,
        )
    }
}

/// The length of the sounding that begins at attack `i`, tied notes merged.
fn sounding(notes: &[Note], i: usize) -> u32 {
    let mut ticks = notes[i].duration.ticks();
    let mut j = i;
    while notes[j].tie && j + 1 < notes.len() {
        j += 1;
        ticks += notes[j].duration.ticks();
    }
    ticks
}

/// Declamation and line of texted settings against untexted ones and against Palestrina: notes
/// per syllable (PAL-DENS-1: the Christe about 2, the homophonic Gloria about 1.1), stressed
/// syllables with a musical accent, re-struck white notes (Palestrina 10.6 % of his white
/// notes), leaps and the range of a voice in four measures (Palestrina 3.1 steps, from the
/// `style` test of tests/palestrina.rs). `OVERRIDES` takes profile overrides, such as
/// `MEL-REPEAT.free_texted=P1`:
/// `cargo test --release -p fux-core --test text -- --ignored --nocapture style_with_text`.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn style_with_text() {
    let mut profile = Profile::fux();
    for rule in std::env::var("OVERRIDES")
        .unwrap_or_default()
        .split_whitespace()
    {
        profile.apply_override(rule).unwrap();
    }
    println!(
        "{:<34} {:>8} {:>9} {:>9} {:>7} {:>7}",
        "4 voices, seeds 1-3", "att/syl", "accent %", "re-st. %", "leap %", "range"
    );
    let modes = [Mode::Dorian, Mode::Phrygian, Mode::Mixolydian, Mode::Ionian];
    for spec in [
        None,
        Some("kyrie"),
        Some("christe"),
        Some("agnus3"),
        Some("gloria:1-5"),
    ] {
        let mut style = Style::default();
        for (m, mode) in modes.into_iter().enumerate() {
            for seed in 1..=3 {
                let mode = ModeSpec::new(mode);
                let request = match spec {
                    Some(spec) => request(spec, mode, 4, seed + 10 * m as u64, 200),
                    None => FreeRequest {
                        cadences: default_cadences(mode, 16),
                        seed: seed + 10 * m as u64,
                        ..FreeRequest::new(mode, choir(4), 16)
                    },
                };
                match generate_free(&request, &profile) {
                    Ok(results) => style.add(&results[0].score),
                    Err(e) => println!("{spec:?}, {}: {e}", mode.id()),
                }
            }
        }
        println!("{}", style.row(spec.unwrap_or("untexted, 16 measures")));
    }
}

/// The least time of `rounds` runs of a free setting, and its result's measures and cost.
fn timed(request: &FreeRequest, rounds: usize) -> (f64, usize, String) {
    let profile = Profile::fux();
    let mut least = f64::INFINITY;
    let mut outcome = (0, String::new());
    for _ in 0..rounds {
        let clock = Instant::now();
        let result = generate_free(request, &profile);
        least = least.min(clock.elapsed().as_secs_f64());
        outcome = match result {
            Ok(results) => (
                results[0].score.measure_count(),
                results[0].cost.to_string(),
            ),
            Err(e) => (0, format!("({e})")),
        };
    }
    (least, outcome.0, outcome.1)
}

/// An untexted free setting with the length and the cadences that `spec` plans.
fn untexted_like(spec: &str, mode: ModeSpec, voices: usize, seed: u64) -> FreeRequest {
    let planned = plan(&text(spec), mode, voices, None, None, true).unwrap();
    FreeRequest {
        cadences: planned.cadences,
        seed,
        ..FreeRequest::new(mode, choir(voices), planned.measures)
    }
}

/// The guard of the plan: a texted setting of Kyrie I in four voices takes at most half as long
/// again as the untexted setting of the same sixteen measures and cadences, the least of three
/// rounds over seeds 1 to 3:
/// `cargo test --release -p fux-core --test text -- --ignored --nocapture texted_search_is_fast`.
#[test]
#[ignore = "timing: cargo test --release -- --ignored"]
fn texted_search_is_fast_enough() {
    let dorian = mode("dorian");
    let (mut untexted, mut texted) = (0.0, 0.0);
    for seed in 1..=3 {
        let (seconds, measures, cost) = timed(&untexted_like("kyrie", dorian, 4, seed), 3);
        assert_eq!(measures, 16, "untexted, seed {seed}: {cost}");
        untexted += seconds;
        let (seconds, measures, cost) = timed(&request("kyrie", dorian, 4, seed, 200), 3);
        assert_eq!(measures, 16, "Kyrie I, seed {seed}: {cost}");
        texted += seconds;
    }
    let factor = texted / untexted;
    println!(
        "4 voices, 16 measures: {:.3} s per measure untexted, {:.3} s with Kyrie I: {factor:.2} \
         times",
        untexted / 48.0,
        texted / 48.0
    );
    assert!(factor <= 1.5, "{factor:.2} times");
}

/// The timings of texted settings against untexted ones of the same length and cadences, two to
/// six voices, Dorian, seeds 1 and 2: seconds per planned measure, the
/// least of two rounds up to four voices, one round beyond (a texted search that needs a longer
/// plan counts with all its time):
/// `cargo test --release -p fux-core --test text -- --ignored --nocapture timings_with_text`.
#[test]
#[ignore = "timing: cargo test --release -- --ignored"]
fn timings_with_text() {
    let dorian = mode("dorian");
    println!(
        "{:<12} {:>6} {:>8} {:>12} {:>12} {:>7}   costs",
        "text", "voices", "measures", "untexted s/m", "texted s/m", "factor"
    );
    for spec in ["kyrie", "christe", "agnus3", "gloria:1-5"] {
        let measures = automatic_length(&text(spec), true);
        for voices in 2..=6 {
            let rounds = if voices <= 4 { 2 } else { 1 };
            let (mut untexted, mut texted) = (0.0, 0.0);
            let mut costs = Vec::new();
            for seed in 1..=2 {
                let plain = untexted_like(spec, dorian, voices, seed);
                let (seconds, _, cost) = timed(&plain, rounds);
                untexted += seconds;
                let (seconds, length, sung) =
                    timed(&request(spec, dorian, voices, seed, 200), rounds);
                texted += seconds;
                let longer = if length > measures { " (longer)" } else { "" };
                costs.push(format!("{cost}/{sung}{longer}"));
            }
            let per = |seconds: f64| seconds / 2.0 / measures as f64;
            println!(
                "{spec:<12} {voices:>6} {measures:>8} {:>12.3} {:>12.3} {:>7.2}   {}",
                per(untexted),
                per(texted),
                texted / untexted,
                costs.join(", ")
            );
        }
    }
}
