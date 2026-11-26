//! The mass (phase 14, docs/design/phase-14/spec.md §§ 3, 4 and 8).
//!
//! The plan: the forms `brevis` and `normal` with their sections, parts, joins and voices, the
//! cut of the long texts at the ends of sentences, the endings in every mode, the restarts with
//! their opening chords, the lengths, the options and the addresses that name parts, and the
//! errors of options that fail before any search. Planning runs no search, so these tests run in
//! debug.
//!
//! The parts and the mass: the profile of the mass, the request of every part, the events of a
//! cancelled mass, the joins of a section and of a movement, the fields of the files, a part read
//! back from its file and the parts split from a movement run in debug, on parts made by hand
//! where they need scores. The acceptance sets whole
//! masses in release: `cargo test --release -p fux-core --test mass -- --ignored --nocapture
//! --test-threads=1` (`a_missa_brevis`, `missa_brevis_in_four_modes`, and on the same four masses,
//! set once in the process, `mass_style` and `mass_timings`; `a_kyrie_in_three_voices` in a
//! minute, and `a_credo_in_three_voices`, whose section of twelve parts joins at seams and
//! Breaks, in a few). `overlay_strictness` measured the level of the overlay (D7) in phase 14a.

mod common;

use std::ops::ControlFlow;
use std::sync::{Arc, Mutex};
use std::time::{Duration as Elapsed, Instant};

use common::{assert_cheapest, assert_detected, assert_imitated, assert_sung, gate_restarts};
use fux_core::generate::{
    Budget, FreeRequest, FreeText, GenerateError, Generated, Rng, SearchControl, Warning,
    generate_free, part_endings,
};
use fux_core::imitation::chant::ChantFallback;
use fux_core::imitation::planner::plan_imitation;
use fux_core::imitation::{Answer, Imitation, Origin, SubjectSource, Toggle};
use fux_core::mass::plan::part_text;
use fux_core::mass::preset::Scoring;
use fux_core::mass::tables;
use fux_core::mass::{
    ChantMass, FIELD_MASS, FIELD_PART, FieldError, GENERATOR, Join, KeptError, KeptPart, Lengths,
    Mass, MassEvent, MassField, MassOption, MassOptionError, MassPlan, MassRequest, Motif,
    Movement, Opening, Override, PartField, PartRef, PartResult, Preset, Repeat, choir,
    fields_of_part, generate_mass, generate_part, mass_field, mass_of_parts, motif_of,
    movement_parts, movement_score, movement_sections, opening_chord, parse_chant, parse_chunk,
    parse_endings, parse_imitation, parse_movements, parse_scoring, part_chant, part_field,
    part_fields, part_from_parts, part_profile, part_request, plan_mass, profile_hash,
    section_score, split_movement, voice_names,
};
use fux_core::rules::{Analysis, Checker, Detail, Level, Profile};
use fux_core::text::{Text, TextKind, TextOptions};
use fux_core::{
    Ambitus, Cadence, Duration, Lang, Lyric, Mode, ModeSpec, Note, Pitch, Role, Score, ScoreEntry,
    ScoreError, ScoreImitation, ScorePoint, ScoreText, SectionKind, SectionMark, Species, Step,
    Voice, VoiceType, parse_lilypond,
};

fn mode(name: &str) -> ModeSpec {
    name.parse().unwrap()
}

/// Every mode authentic and plagal, untransposed and transposed.
fn every_mode() -> Vec<ModeSpec> {
    let mut modes = Vec::new();
    for m in Mode::ALL {
        for transposed in [false, true] {
            for ambitus in [Ambitus::Authentic, Ambitus::Plagal] {
                modes.push(
                    ModeSpec::new(m)
                        .with_transposition(transposed)
                        .with_ambitus(ambitus),
                );
            }
        }
    }
    modes
}

/// The parts of a plan, one line each: address, section, text, join, start and voices.
fn outline(plan: &MassPlan) -> Vec<String> {
    plan.parts()
        .map(|(at, part)| {
            let movement = &plan.movements[at.movement];
            let section = &movement.sections[at.section];
            let voices: Vec<&str> = section
                .voices
                .iter()
                .map(|&v| movement.voices[v].abbreviation())
                .collect();
            let join = match part.join {
                Join::Start => "start".to_string(),
                Join::Section => "section".to_string(),
                Join::Break(label) => format!("break {label}"),
                Join::Seam => "seam".to_string(),
            };
            format!(
                "{} {} {} | {join} {} | {}",
                plan.address(at),
                section.id,
                part.text,
                part.start,
                voices.join(" ")
            )
        })
        .collect()
}

/// The Missa brevis in four voices: six movements, ten sections, 26 parts of 657 syllables. The
/// Gloria has one section with a Break at Qui tollis, the Credo one with Breaks at Et
/// incarnatus and the Crucifixus, the Sanctus sets Pleni and Osanna through; the Benedictus
/// sings in three voices, Agnus Dei II in five. (The example of `fux mass --plan` in § 3.10
/// says nine sections; the forms of § 3.1 give ten.)
#[test]
fn brevis_plan() {
    let plan = plan_mass(&MassRequest::default()).unwrap();
    assert_eq!(plan.title, "Missa brevis");
    assert_eq!(
        (
            plan.movements.len(),
            plan.section_count(),
            plan.part_count(),
            plan.syllables()
        ),
        (6, 10, 26, 657)
    );
    assert_eq!(
        outline(&plan),
        [
            "kyrie/1 kyrie kyrie | start 0 | S A T B",
            "kyrie/2 christe christe | section 0 | S A T B",
            "kyrie/3 kyrie2 kyrie2 | section 0 | S A T B",
            "gloria/1 gloria gloria:1-5 | start 0 | S A T B",
            "gloria/2 gloria gloria:6-9 | seam 4 | S A T B",
            "gloria/3 gloria gloria:10-13 | seam 4 | S A T B",
            "gloria/4 gloria qui-tollis:1-4 | break Qui tollis 8 | S A T B",
            "gloria/5 gloria qui-tollis:5-8 | seam 4 | S A T B",
            "gloria/6 gloria qui-tollis:9-13 | seam 4 | S A T B",
            "credo/1 credo credo:1-4 | start 0 | S A T B",
            "credo/2 credo credo:5-7 | seam 4 | S A T B",
            "credo/3 credo credo:8-14 | seam 4 | S A T B",
            "credo/4 credo credo:15-16 | seam 4 | S A T B",
            "credo/5 credo incarnatus | break Et incarnatus est 8 | S A T B",
            "credo/6 credo crucifixus | break Crucifixus 8 | S A T B",
            "credo/7 credo resurrexit:1-4 | seam 4 | S A T B",
            "credo/8 credo resurrexit:5-7 | seam 4 | S A T B",
            "credo/9 credo spiritum:1-4 | seam 4 | S A T B",
            "credo/10 credo spiritum:5-7 | seam 4 | S A T B",
            "credo/11 credo spiritum:8-9 | seam 4 | S A T B",
            "credo/12 credo spiritum:10-12 | seam 4 | S A T B",
            "sanctus/1 sanctus sanctus+pleni+osanna | start 0 | S A T B",
            "benedictus/1 benedictus benedictus | start 0 | S A T",
            "benedictus/2 osanna2 osanna2 | section 0 | S A T B",
            "agnus/1 agnus agnus | start 0 | S A T B",
            "agnus/2 agnus3 agnus3 | section 0 | S A T T B",
        ]
    );
    let titles: Vec<&str> = plan
        .movements
        .iter()
        .flat_map(|m| m.sections.iter().map(|s| s.title))
        .collect();
    assert_eq!(
        titles,
        [
            "Kyrie I",
            "Christe",
            "Kyrie II",
            "Et in terra pax",
            "Patrem omnipotentem",
            "Sanctus",
            "Benedictus",
            "Osanna",
            "Agnus Dei I",
            "Agnus Dei II"
        ]
    );
    // The movement of Agnus Dei has the voices of both its sections, the second tenor only in
    // Agnus Dei II.
    let agnus = &plan.movements[5];
    assert_eq!(agnus.names, ["Sopran", "Alt", "Tenor 1", "Tenor 2", "Bass"]);
    assert_eq!(agnus.sections[0].voices, [0, 1, 2, 4]);
    assert_eq!(agnus.sections[1].voices, [0, 1, 2, 3, 4]);
    // The text goes on after every part but the last of its section.
    for (at, part) in plan.parts() {
        let section = &plan.movements[at.movement].sections[at.section];
        assert_eq!(
            part.continued,
            at.part + 1 < section.parts.len(),
            "{}",
            part.text
        );
        assert_eq!(part.start, part.join.start());
    }
    // Every part has a seed of its own.
    let mut seeds: Vec<u64> = plan.parts().map(|(_, part)| part.seed).collect();
    seeds.sort_unstable();
    seeds.dedup();
    assert_eq!(seeds.len(), 26);
    // The intonations of Gloria and Credo and the remarks on Kyrie and Agnus Dei.
    let intonations: Vec<Option<&str>> = plan.movements.iter().map(|m| m.intonation).collect();
    assert_eq!(
        intonations,
        [
            None,
            Some("Glória in excélsis Deo."),
            Some("Credo in unum Deum,"),
            None,
            None,
            None
        ]
    );
    assert!(plan.movements[0].notes_in(Lang::De)[0].starts_with("Kyrie I, Christe und Kyrie II"));
    assert!(plan.movements[5].notes_in(Lang::En)[0].starts_with("Agnus Dei I serves"));
}

/// The form `normal`: 27 parts in fifteen sections. The Crucifixus is a section of its own in
/// three voices up to „non erit finis“, cut at its sentences, and the Benedictus is followed by
/// „Osanna ut supra“, a copy of the Sanctus' Osanna without parts of its own.
#[test]
fn normal_plan() {
    let request = MassRequest {
        preset: Preset::Normal,
        ..MassRequest::default()
    };
    let plan = plan_mass(&request).unwrap();
    assert_eq!(plan.title, "Missa");
    assert_eq!(
        (plan.section_count(), plan.part_count(), plan.syllables()),
        (15, 27, 650)
    );
    let lines = outline(&plan);
    for line in [
        "gloria/4 qui-tollis qui-tollis:1-4 | section 0 | S A T B",
        "credo/5 credo incarnatus | break Et incarnatus est 8 | S A T B",
        "credo/6 crucifixus crucifixus | section 0 | S A T",
        "credo/7 crucifixus resurrexit:1-4 | seam 4 | S A T",
        "credo/8 crucifixus resurrexit:5-7 | seam 4 | S A T",
        "credo/9 spiritum spiritum:1-4 | section 0 | S A T B",
        "sanctus/2 pleni pleni | section 0 | S A T",
        "sanctus/3 osanna osanna | section 0 | S A T B",
        "benedictus/1 benedictus benedictus | start 0 | S A T",
    ] {
        assert!(lines.iter().any(|l| l == line), "{line} not in {lines:#?}");
    }
    let benedictus = &plan.movements[4];
    let ut_supra = &benedictus.sections[1];
    assert_eq!(
        (ut_supra.id, ut_supra.title),
        ("osanna2", "Osanna ut supra")
    );
    assert_eq!(ut_supra.ut_supra, Some((3, 2)));
    assert!(ut_supra.parts.is_empty());
    assert_eq!(benedictus.names, ["Sopran", "Alt", "Tenor", "Bass"]);
    assert_eq!(plan.movements[3].sections[2].id, "osanna");
}

/// MASS-PARTS at 45 syllables: the Gloria in 3 + 3 parts, Patrem in 4, Et incarnatus in 1 and
/// Crucifixus to the Amen in 7; every part a whole number of sentences, every text sung once.
/// Without a limit each stretch between Breaks is one part; at 55 the Gloria and Credo have 11
/// joins instead of 16.
#[test]
fn long_texts_are_cut_at_sentence_ends() {
    let parts = |chunk: Option<usize>, preset: Preset| -> Vec<String> {
        let request = MassRequest {
            preset,
            chunk,
            movements: vec![Movement::Gloria, Movement::Credo],
            ..MassRequest::default()
        };
        plan_mass(&request)
            .unwrap()
            .parts()
            .map(|(_, part)| part.text.clone())
            .collect()
    };
    let at_45 = parts(Some(45), Preset::Brevis);
    let stretches: Vec<usize> = ["gloria", "qui-tollis", "credo", "incarnatus", "crucifixus"]
        .iter()
        .map(|start| {
            at_45
                .iter()
                .position(|text| text.starts_with(start))
                .unwrap()
        })
        .chain([at_45.len()])
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| w[1] - w[0])
        .collect();
    assert_eq!(stretches, [3, 3, 4, 1, 7]);
    assert_eq!(
        parts(None, Preset::Brevis),
        [
            "gloria",
            "qui-tollis",
            "credo",
            "incarnatus",
            "crucifixus+resurrexit+spiritum"
        ]
    );
    assert_eq!(
        parts(None, Preset::Normal),
        [
            "gloria",
            "qui-tollis",
            "credo",
            "incarnatus",
            "crucifixus+resurrexit",
            "spiritum"
        ]
    );
    let at_55 = parts(Some(55), Preset::Brevis);
    let joins = at_55.len() - 2;
    assert_eq!(joins, 11, "{at_55:?}");
    assert_eq!(at_45.len() - 2, 16);
    // Each part of a long text is at most as long as the limit asks (no unit of the Ordinary
    // has more than 19 syllables), its spec names its text, and the parts of a section sing
    // its text once, in order.
    let options = TextOptions::default();
    for chunk in [20, 31, 45, 60, 100, 200] {
        let request = MassRequest {
            chunk: Some(chunk),
            ..MassRequest::default()
        };
        let plan = plan_mass(&request).unwrap();
        assert_eq!(plan.syllables(), 657, "{chunk}");
        for movement in &plan.movements {
            for section in &movement.sections {
                let mut sung = String::new();
                for part in &section.parts {
                    let text = part_text(&part.text, &options).unwrap();
                    assert_eq!(text.id, part.text);
                    assert_eq!(text.syllables.len(), part.syllables, "{}", part.text);
                    if text.kind == TextKind::Long {
                        assert!(part.syllables <= chunk, "{chunk}: {}", part.text);
                    }
                    sung = format!("{sung} {}", text.render(false, false));
                }
                let whole: Vec<String> =
                    fux_core::mass::preset::form(Preset::Brevis, movement.movement)
                        .sections
                        .iter()
                        .find(|form| form.id == section.id)
                        .unwrap()
                        .pieces
                        .iter()
                        .map(|piece| part_text(piece.id, &options).unwrap().render(false, false))
                        .collect();
                assert_eq!(sung.trim(), whole.join(" "), "{chunk} {}", section.id);
            }
        }
    }
}

/// MASS-ENDING in every mode, plagal and transposed, in both forms and every choir: each
/// ending has a clausula and is the final, the fifth or the fourth; each movement ends on the
/// final (the Benedictus of `normal` before its „Osanna ut supra“ may end on the fifth); parts
/// in two voices end on the final.
#[test]
fn endings_are_cadential() {
    for mode in every_mode() {
        let allowed = part_endings(mode);
        assert!(allowed.contains(&mode.final_step()));
        for preset in [Preset::Brevis, Preset::Normal] {
            for voices in 2..=6 {
                for seed in 1..=12 {
                    let request = MassRequest {
                        preset,
                        mode: Some(mode),
                        choir: choir(voices),
                        seed,
                        ..MassRequest::default()
                    };
                    let plan = plan_mass(&request).unwrap();
                    for (at, part) in plan.parts() {
                        let section = &plan.movements[at.movement].sections[at.section];
                        let label = format!("{mode} {voices} {seed} {}", plan.address(at));
                        assert!(allowed.contains(&part.ending), "{label}");
                        assert!(mode.clausula(part.ending).is_some(), "{label}");
                        if section.voices.len() == 2 {
                            assert_eq!(part.ending, mode.final_step(), "{label}");
                        }
                    }
                    for movement in &plan.movements {
                        let last = movement.sections.last().unwrap();
                        if last.ut_supra.is_some() {
                            continue;
                        }
                        let ending = last.parts.last().unwrap().ending;
                        assert_eq!(ending, mode.final_step(), "{mode} {:?}", movement.movement);
                    }
                }
            }
        }
    }
}

/// The restart of every part: a tutti part has a full triad on the final, an imitative one none.
/// `--imitation` overrides the table, even with six voices.
#[test]
fn restarts_follow_the_options() {
    let request = |imitation: &str, voices: usize| MassRequest {
        choir: choir(voices),
        overrides: parse_imitation(imitation).unwrap(),
        ..MassRequest::default()
    };
    let plan = plan_mass(&request("aus", 4)).unwrap();
    for (_, part) in plan.parts() {
        assert_eq!(part.opening, Opening::Tutti);
        assert_eq!(part.points, None);
    }
    let plan = plan_mass(&request("auto; incarnatus=an; credo/2=aus; gloria=an", 4)).unwrap();
    let opening = |at: &str| plan.part(plan.find(at).unwrap()[0]).opening;
    assert_eq!(opening("incarnatus"), Opening::Imitative);
    assert_eq!(opening("credo/2"), Opening::Tutti);
    assert_eq!(opening("crucifixus"), Opening::Tutti);
    for at in plan.find("gloria").unwrap() {
        assert_eq!(plan.part(at).opening, Opening::Imitative);
    }
    let plan = plan_mass(&request("an", 6)).unwrap();
    for (_, part) in plan.parts() {
        assert_eq!((part.opening, &part.chord), (Opening::Imitative, &None));
    }
    // A later option wins.
    let plan = plan_mass(&request("credo/2=an; aus", 4)).unwrap();
    assert_eq!(
        plan.part(plan.find("credo/2").unwrap()[0]).opening,
        Opening::Tutti
    );
    let plan = plan_mass(&request("aus; credo/2=an", 4)).unwrap();
    assert_eq!(
        plan.part(plan.find("credo/2").unwrap()[0]).opening,
        Opening::Imitative
    );
    // Chords sound in the voices of their section.
    let plan = plan_mass(&request("aus", 4)).unwrap();
    for (at, part) in plan.parts() {
        let movement = &plan.movements[at.movement];
        let voices: Vec<VoiceType> = movement.sections[at.section]
            .voices
            .iter()
            .map(|&v| movement.voices[v])
            .collect();
        let chord = part.chord.clone().unwrap();
        assert_eq!(
            chord,
            opening_chord(movement.mode, &voices, movement.mode.final_step())
        );
    }
}

/// The opening chord of a tutti part (§ 3.7) in every mode and in choirs of three to six
/// voices, also those of the reduced and enlarged sections: the root of the final in the
/// lowest voice, the mode's own third (once up to four voices), the fifth from four voices,
/// every voice in its staff, no voice below the one under it, no two voices on one pitch, and
/// adjacent upper voices at most an octave apart. The Mixolydian bass begins in its octave of the
/// mode, on G2.
#[test]
fn opening_chord_is_a_full_triad() {
    use VoiceType::*;
    let choirs: Vec<Vec<VoiceType>> = vec![
        choir(3),
        choir(4),
        choir(5),
        choir(6),
        vec![Soprano, Alto, Tenor],
        vec![Soprano, Alto, Tenor, Tenor],
        vec![Soprano, Soprano, Alto, Tenor],
        vec![Soprano, Alto, Tenor, Tenor, Bass],
        vec![Soprano, Soprano, Alto, Tenor, Tenor, Bass],
        vec![MezzoSoprano, Alto, Baritone, Bass],
    ];
    for mode in every_mode() {
        let root = mode.final_pitch(4);
        let third = mode.scale_pitch(root.diatonic() + 2);
        let fifth = mode.scale_pitch(root.diatonic() + 4);
        for voices in &choirs {
            let chord = opening_chord(mode, voices, mode.final_step());
            let label = format!("{mode} {voices:?}: {chord:?}");
            assert_eq!(chord.len(), voices.len(), "{label}");
            let last = chord[chord.len() - 1];
            assert!(last.same_class(root), "{label}");
            let count = |tone: fux_core::Pitch| chord.iter().filter(|p| p.same_class(tone)).count();
            assert!(count(third) >= 1, "{label}");
            if voices.len() <= 4 {
                assert_eq!(count(third), 1, "{label}");
            }
            if voices.len() >= 4 {
                assert!(count(fifth) >= 1, "{label}");
            }
            assert_eq!(
                count(root) + count(third) + count(fifth),
                voices.len(),
                "{label}"
            );
            for (pitch, voice) in chord.iter().zip(voices) {
                assert!(mode.is_scale_pitch(*pitch), "{label}");
                assert_eq!(voice.clef().steps_outside(*pitch), 0, "{label}");
            }
            for pair in chord.windows(2) {
                assert!(pair[0].midi() > pair[1].midi(), "{label}");
            }
            for pair in chord[..chord.len() - 1].windows(2) {
                assert!(pair[0].midi() - pair[1].midi() <= 12, "{label}");
            }
        }
    }
    // Dorian in four voices: D3 A3 D4 F4 from the bass up.
    let dorian = opening_chord(mode("dorian"), &choir(4), Step::D);
    let names: Vec<String> = dorian.iter().map(|p| p.to_string()).collect();
    assert_eq!(names, ["F4", "D4", "A3", "D3"]);
    // The Mixolydian bass in its octave of the mode, on G2 (FREE-AMBITUS).
    let mixolydian = opening_chord(mode("mixolydian"), &choir(4), Step::G);
    let names: Vec<String> = mixolydian.iter().map(|p| p.to_string()).collect();
    assert_eq!(names, ["G4", "D4", "B3", "G2"]);
}

/// Two voices begin on a perfect consonance over the final, as CAD-START demands of every
/// two-voice setting: the lower voice on the root, the upper one on the root or the fifth, never
/// the third; in every mode and every pair of voices, each in its staff, the upper not below the
/// lower. The Dorian S T opens on D4 and A4.
#[test]
fn two_voices_open_on_a_perfect_consonance() {
    use VoiceType::*;
    let all = [
        HighSoprano,
        Soprano,
        MezzoSoprano,
        Alto,
        Tenor,
        Baritone,
        Bass,
    ];
    for mode in every_mode() {
        let root = mode.final_pitch(4);
        let fifth = mode.scale_pitch(root.diatonic() + 4);
        for (i, &upper) in all.iter().enumerate() {
            for &lower in &all[i..] {
                let voices = [upper, lower];
                let chord = opening_chord(mode, &voices, mode.final_step());
                let label = format!("{mode} {voices:?}: {chord:?}");
                assert_eq!(chord.len(), 2, "{label}");
                assert!(chord[1].same_class(root), "{label}");
                assert!(
                    chord[0].same_class(root) || chord[0].same_class(fifth),
                    "{label}"
                );
                assert!(chord[0].midi() >= chord[1].midi(), "{label}");
                for (pitch, voice) in chord.iter().zip(&voices) {
                    assert!(mode.is_scale_pitch(*pitch), "{label}");
                    assert_eq!(voice.clef().steps_outside(*pitch), 0, "{label}");
                }
            }
        }
    }
    let dorian = opening_chord(mode("dorian"), &choir(2), Step::D);
    let names: Vec<String> = dorian.iter().map(|p| p.to_string()).collect();
    assert_eq!(names, ["A4", "D4"]);
}

/// D10 (MASS-LENGTH): with `palestrina` the Benedictus and Agnus Dei I get two points more than
/// the imitation plans by itself (two and three); with `natürlich`, without restatements or
/// where they begin together, the imitation plans their points.
#[test]
fn short_sections_are_lengthened() {
    let points = |request: MassRequest| -> Vec<(String, Option<usize>)> {
        let plan = plan_mass(&request).unwrap();
        plan.parts()
            .filter(|(_, part)| part.points.is_some())
            .map(|(at, part)| (plan.address(at), part.points))
            .collect()
    };
    let palestrina = points(MassRequest::default());
    assert_eq!(
        palestrina,
        [
            ("benedictus/1".to_string(), Some(4)),
            ("agnus/1".to_string(), Some(5))
        ]
    );
    // The estimate grows with the points.
    let plan = plan_mass(&MassRequest::default()).unwrap();
    let natural = plan_mass(&MassRequest {
        lengths: Lengths::Natural,
        ..MassRequest::default()
    })
    .unwrap();
    let benedictus = |plan: &MassPlan| plan.part(plan.find("benedictus").unwrap()[0]).estimate;
    assert!(benedictus(&plan).measures > benedictus(&natural).measures);
    assert!(
        points(MassRequest {
            lengths: Lengths::Natural,
            ..MassRequest::default()
        })
        .is_empty()
    );
    assert!(
        points(MassRequest {
            repeat: Repeat::None,
            ..MassRequest::default()
        })
        .is_empty()
    );
    assert_eq!(
        points(MassRequest {
            overrides: parse_imitation("benedictus=aus").unwrap(),
            ..MassRequest::default()
        }),
        [("agnus/1".to_string(), Some(5))]
    );
}

/// D11: with the head motif the first parts of Gloria, Credo, Sanctus and Agnus Dei I take it,
/// where they imitate.
#[test]
fn motif_marks_the_later_movements() {
    let plan = plan_mass(&MassRequest {
        motif: true,
        ..MassRequest::default()
    })
    .unwrap();
    let marked: Vec<String> = plan
        .parts()
        .filter(|(_, part)| part.motif)
        .map(|(at, _)| plan.address(at))
        .collect();
    assert_eq!(marked, ["gloria/1", "credo/1", "sanctus/1", "agnus/1"]);
    assert!(
        plan_mass(&MassRequest::default())
            .unwrap()
            .parts()
            .all(|(_, part)| !part.motif)
    );
}

/// The head motif and a chant in a mode of the mass's own change no draw (D15): the parts of
/// such a mass are planned as those of the mass without, but for the mark of the parts that take
/// the motif, so that Kyrie I and every part that takes no motif sets the same notes. Without a
/// mode, a mass on chant draws in the modes of its chants (D12).
#[test]
fn motif_and_chant_change_no_draw() {
    let without = plan_mass(&MassRequest::default()).unwrap();
    let alike = |request: MassRequest| {
        let plan = plan_mass(&request).unwrap();
        assert_eq!(plan.part_count(), without.part_count());
        for ((at, part), (_, plain)) in plan.parts().zip(without.parts()) {
            let unmarked = fux_core::mass::PartPlan {
                motif: false,
                ..part.clone()
            };
            assert_eq!(unmarked, *plain, "{}", plan.address(at));
        }
    };
    alike(MassRequest {
        motif: true,
        ..MassRequest::default()
    });
    alike(MassRequest {
        chant: Some(ChantMass::IX),
        mode: Some(mode("dorian")),
        ..MassRequest::default()
    });
    let modes = plan_mass(&MassRequest {
        chant: Some(ChantMass::IX),
        ..MassRequest::default()
    })
    .unwrap();
    assert_ne!(modes.part(modes.find("gloria/1").unwrap()[0]).seed, {
        let at = without.find("gloria/1").unwrap()[0];
        without.part(at).seed
    });
}

/// Options change the scoring and the endings where they say: a section's id names its voices
/// and its last part's ending; the k-th voice of a type is the k-th of the movement's.
#[test]
fn options_shape_the_plan() {
    let mut overrides = parse_scoring("benedictus=S A T B; agnus3 = S, A, T, B").unwrap();
    overrides.extend(parse_endings("christe=G; gloria/3=D; incarnatus=G").unwrap());
    let request = MassRequest {
        overrides,
        ..MassRequest::default()
    };
    let plan = plan_mass(&request).unwrap();
    let voices = |id: &str| -> Vec<VoiceType> {
        let (m, s) = plan
            .movements
            .iter()
            .enumerate()
            .find_map(|(m, mv)| mv.sections.iter().position(|s| s.id == id).map(|s| (m, s)))
            .unwrap();
        let movement = &plan.movements[m];
        movement.sections[s]
            .voices
            .iter()
            .map(|&v| movement.voices[v])
            .collect()
    };
    assert_eq!(voices("benedictus"), choir(4));
    assert_eq!(voices("agnus3"), choir(4));
    assert_eq!(plan.movements[5].names, ["Sopran", "Alt", "Tenor", "Bass"]);
    let ending = |at: &str| plan.part(plan.find(at).unwrap()[0]).ending;
    assert_eq!(ending("christe"), Step::G);
    assert_eq!(ending("gloria/3"), Step::D);
    assert_eq!(ending("incarnatus"), Step::G);
    // A two-voice section ends on the final and begins together.
    let request = MassRequest {
        overrides: parse_scoring("christe=S T").unwrap(),
        movements: vec![Movement::Kyrie],
        ..MassRequest::default()
    };
    for seed in 1..20 {
        let plan = plan_mass(&MassRequest {
            seed,
            ..request.clone()
        })
        .unwrap();
        let christe = &plan.movements[0].sections[1];
        assert_eq!(christe.parts[0].ending, Step::D);
        assert_eq!(christe.parts[0].opening, Opening::Tutti);
    }
    // Five voices: Agnus Dei II gains a soprano; Agnus Dei I sings with the first.
    let plan = plan_mass(&MassRequest {
        choir: choir(5),
        movements: vec![Movement::Agnus],
        ..MassRequest::default()
    })
    .unwrap();
    let agnus = &plan.movements[0];
    assert_eq!(
        agnus.names,
        ["Sopran 1", "Sopran 2", "Alt", "Tenor 1", "Tenor 2", "Bass"]
    );
    assert_eq!(agnus.sections[0].voices, [0, 2, 3, 4, 5]);
    assert_eq!(
        voice_names(&agnus.voices, Lang::En),
        [
            "Soprano 1",
            "Soprano 2",
            "Alto",
            "Tenor 1",
            "Tenor 2",
            "Bass"
        ]
    );
    // Six voices: the Christe and the Benedictus in four.
    let plan = plan_mass(&MassRequest {
        choir: choir(6),
        movements: vec![Movement::Kyrie, Movement::Benedictus],
        ..MassRequest::default()
    })
    .unwrap();
    assert_eq!(plan.movements[0].sections[1].voices, [0, 1, 2, 3]);
    assert_eq!(plan.movements[1].sections[0].voices, [0, 1, 2, 3]);
    assert_eq!(
        tables::scoring(&choir(6), Scoring::Reduced),
        [
            VoiceType::Soprano,
            VoiceType::Soprano,
            VoiceType::Alto,
            VoiceType::Tenor
        ]
    );
}

/// Addresses: `movement/n`, a section's id, a Break by the id of its text or its label; the
/// address of a part names it again.
#[test]
fn addresses_name_parts() {
    let plan = plan_mass(&MassRequest::default()).unwrap();
    let names = |at: &str| -> Vec<String> {
        plan.find(at)
            .unwrap()
            .into_iter()
            .map(|part| plan.address(part))
            .collect()
    };
    assert_eq!(names("credo/5"), ["credo/5"]);
    assert_eq!(names("incarnatus"), ["credo/5"]);
    assert_eq!(names("Et incarnatus est"), ["credo/5"]);
    assert_eq!(names("qui-tollis"), ["gloria/4"]);
    assert_eq!(names("Qui tollis"), ["gloria/4"]);
    assert_eq!(names("crucifixus"), ["credo/6"]);
    assert_eq!(names("christe"), ["kyrie/2"]);
    assert_eq!(names("Agnus/2"), ["agnus/2"]);
    assert_eq!(names("credo").len(), 12);
    assert_eq!(names("").len(), 26);
    for (at, _) in plan.parts() {
        assert_eq!(plan.find(&plan.address(at)).unwrap(), [at]);
    }
    let normal = plan_mass(&MassRequest {
        preset: Preset::Normal,
        ..MassRequest::default()
    })
    .unwrap();
    let found: Vec<String> = normal
        .find("crucifixus")
        .unwrap()
        .into_iter()
        .map(|at| normal.address(at))
        .collect();
    assert_eq!(found, ["credo/6", "credo/7", "credo/8"]);
    assert_eq!(normal.find("qui-tollis").unwrap().len(), 3);
    // The Osanna ut supra has no part of its own: `--only osanna2` fails as `--imitation` does.
    assert_eq!(
        normal.find("osanna2").unwrap_err(),
        MassOptionError::UtSupra {
            at: "osanna2".into()
        }
    );
    assert_eq!(names("osanna2"), ["benedictus/2"]);
    // The ending before a part is the previous part's in its movement.
    let at = plan.find("kyrie/2").unwrap()[0];
    assert_eq!(
        plan.ending_before(at),
        Some(plan.part(plan.find("kyrie/1").unwrap()[0]).ending)
    );
    assert_eq!(plan.ending_before(plan.find("gloria/1").unwrap()[0]), None);
}

/// D12: with a chant and no mode each movement takes the mode of its chant; with a mode the
/// mass has that one.
#[test]
fn chant_masses_change_their_mode() {
    let modes = |request: MassRequest| -> Vec<String> {
        plan_mass(&request)
            .unwrap()
            .movements
            .iter()
            .map(|m| m.mode.id())
            .collect()
    };
    let ix = modes(MassRequest {
        chant: Some(ChantMass::IX),
        ..MassRequest::default()
    });
    // Kyrie IX mode 1, Gloria IX mode 7, Credo I mode 4, Sanctus and Agnus IX mode 5 with B♭.
    assert_eq!(
        ix,
        [
            "dorian",
            "mixolydian",
            "hypophrygian",
            "ionian-transposed",
            "ionian-transposed",
            "ionian-transposed"
        ]
    );
    let given = modes(MassRequest {
        chant: Some(ChantMass::IV),
        mode: Some(mode("phrygian")),
        ..MassRequest::default()
    });
    assert!(given.iter().all(|m| m == "phrygian"), "{given:?}");
}

/// The estimates (§ 3.10): every part takes some measures and seconds, and the mass between 600
/// and 1200 measures; side by side on four workers it takes less than one part after the other.
/// (A part together is not always quicker than in imitation: the melismatic Sanctus and Osanna
/// together take a little longer, as their imitative parts are hardly longer than their text.)
#[test]
fn estimates_add_up() {
    let plan = plan_mass(&MassRequest::default()).unwrap();
    let total = plan.estimate();
    assert!(total.measures > 600 && total.measures < 1200, "{total:?}");
    for (_, part) in plan.parts() {
        assert!(
            part.estimate.measures >= 8 && part.estimate.seconds > 0.0,
            "{}",
            part.text
        );
    }
    let one = plan.pool_seconds(1, 1.0);
    assert!((one - total.seconds).abs() < 0.01 * total.seconds);
    let four = plan.pool_seconds(4, tables::ESTIMATE_WASM);
    assert!(four < tables::ESTIMATE_WASM * total.seconds / 2.0);
    assert!(four >= tables::ESTIMATE_WASM * total.seconds / 4.0);
}

/// The seconds of the parts of `plan` in `order` on `workers` workers, each part `factor` times
/// its estimate and to the worker free first, without the head motif.
fn in_order(plan: &MassPlan, order: &[PartRef], workers: usize, factor: f32) -> f32 {
    let mut loads = vec![0.0_f32; workers];
    for &at in order {
        *loads.iter_mut().min_by(|a, b| a.total_cmp(b)).unwrap() +=
            plan.part(at).estimate.seconds * factor;
    }
    loads.into_iter().fold(0.0, f32::max)
}

/// The parts of `plan` longest first, of equal ones the earlier in the mass.
fn longest_first(plan: &MassPlan) -> Vec<PartRef> {
    let mut parts: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
    parts.sort_by(|a, b| {
        let seconds = |at: &PartRef| plan.part(*at).estimate.seconds;
        seconds(b).total_cmp(&seconds(a)).then(a.cmp(b))
    });
    parts
}

/// The order of the pool of the web version (`pool_order`, web/pool.js; K12 (a)): Kyrie I first
/// where it gives the head motif, then the `workers − 1` longest parts, then the others movement
/// by movement in the order of the mass, each movement's longest first, the last three per worker
/// longest first; every part once. A plan with few parts goes longest first after Kyrie I.
#[test]
fn the_pool_starts_the_longest_then_whole_movements() {
    for request in [
        MassRequest::default(),
        MassRequest {
            mode: Some(mode("ionian-transposed")),
            seed: 2,
            ..MassRequest::default()
        },
        MassRequest {
            motif: true,
            seed: 1,
            ..MassRequest::default()
        },
        MassRequest {
            preset: Preset::Normal,
            choir: choir(5),
            seed: 2,
            ..MassRequest::default()
        },
    ] {
        let plan = plan_mass(&request).unwrap();
        let seconds = |at: &PartRef| plan.part(*at).estimate.seconds;
        for workers in [1, 2, 4, 7] {
            let order = plan.pool_order(workers);
            let mut sorted = order.clone();
            sorted.sort();
            let all: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
            assert_eq!(sorted, all, "{workers} workers: every part once");
            let mut others = &order[..];
            if let Some(kyrie) = plan.motif_source() {
                assert_eq!(order[0], kyrie, "{workers} workers: Kyrie I first");
                others = &order[1..];
            }
            let (longest, rest) = others.split_at(workers - 1);
            let shortest = longest.iter().map(seconds).fold(f32::INFINITY, f32::min);
            assert!(
                longest.windows(2).all(|w| seconds(&w[0]) >= seconds(&w[1]))
                    && rest.iter().all(|at| seconds(at) <= shortest),
                "{workers} workers: the longest first"
            );
            let (fill, tail) =
                rest.split_at(rest.len().saturating_sub(tables::POOL_TAIL * workers));
            assert!(
                fill.windows(2).all(|w| {
                    w[0].movement < w[1].movement
                        || (w[0].movement == w[1].movement && seconds(&w[0]) >= seconds(&w[1]))
                }),
                "{workers} workers: movement by movement, each movement's longest first"
            );
            assert!(
                tail.windows(2).all(|w| seconds(&w[0]) >= seconds(&w[1])),
                "{workers} workers: the last parts longest first"
            );
        }
    }
    // The Dorian Missa brevis of seed 1 on four workers, as web/smoke-test.mjs has it for the
    // page: Sanctus and Agnus Dei, the longest; Kyrie, Gloria and the longest parts of the Credo;
    // the last twelve longest first.
    let plan = plan_mass(&MassRequest::default()).unwrap();
    let order: Vec<String> = plan
        .pool_order(4)
        .into_iter()
        .map(|at| plan.address(at))
        .collect();
    assert_eq!(
        order.join(" "),
        "sanctus/1 agnus/2 agnus/1 kyrie/1 kyrie/3 kyrie/2 gloria/1 gloria/5 gloria/3 gloria/2 \
         gloria/4 gloria/6 credo/11 credo/8 credo/1 credo/3 credo/2 benedictus/1 credo/7 \
         credo/10 credo/9 benedictus/2 credo/5 credo/12 credo/6 credo/4"
    );
    // Five parts on four workers: the three longest, then the two others, longest first.
    let small = plan_mass(&MassRequest {
        movements: vec![Movement::Kyrie, Movement::Agnus],
        choir: choir(3),
        seed: 4,
        ..MassRequest::default()
    })
    .unwrap();
    assert_eq!(small.pool_order(4), longest_first(&small));
}

/// The parts of a run go in the pool's order by the same rule as a whole mass (`pool_order_of`;
/// web/pool.js, `order` on the jobs of a run; `fux mass` begins them so with several at once,
/// K6): over all parts it is `pool_order`, and Kyrie I goes first only where it is among them.
/// Two runs of the Dorian Missa brevis of seed 1 with the head motif, as web/smoke-test.mjs has
/// them for the page: without the Kyrie on four workers, as `--resume` sets the rest of a mass
/// whose Kyrie is set, the three longest, then whole movements, the last twelve longest first;
/// Kyrie I, the Credo and the Agnus Dei on two workers, Kyrie I first, then the longest, then
/// movement by movement, the last six longest first.
#[test]
fn the_pool_orders_the_parts_of_a_run_alike() {
    let plan = plan_mass(&MassRequest {
        motif: true,
        seed: 1,
        ..MassRequest::default()
    })
    .unwrap();
    let all: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
    for workers in [1, 2, 4, 7] {
        assert_eq!(plan.pool_order_of(&all, workers), plan.pool_order(workers));
    }
    let kyrie = plan.motif_source().unwrap();
    let addresses = |order: &[PartRef]| -> String {
        let addresses: Vec<String> = order.iter().map(|&at| plan.address(at)).collect();
        addresses.join(" ")
    };
    let seconds = |at: &PartRef| plan.part(*at).estimate.seconds;
    let without: Vec<PartRef> = all
        .iter()
        .copied()
        .filter(|at| at.movement != kyrie.movement)
        .collect();
    let order = plan.pool_order_of(&without, 4);
    let mut longest = without.clone();
    longest.sort_by(|a, b| seconds(b).total_cmp(&seconds(a)).then(a.cmp(b)));
    assert_eq!(order[..3], longest[..3]);
    assert_eq!(
        addresses(&order),
        "sanctus/1 agnus/2 agnus/1 gloria/1 gloria/5 gloria/3 gloria/2 gloria/4 gloria/6 \
         credo/11 credo/8 credo/1 credo/3 credo/2 benedictus/1 credo/7 credo/10 credo/9 \
         benedictus/2 credo/5 credo/12 credo/6 credo/4"
    );
    let movement = |at: &PartRef| plan.movements[at.movement].movement;
    let some: Vec<PartRef> = all
        .iter()
        .copied()
        .filter(|at| *at == kyrie || matches!(movement(at), Movement::Credo | Movement::Agnus))
        .collect();
    let order = plan.pool_order_of(&some, 2);
    assert_eq!(order[0], kyrie);
    assert_eq!(
        addresses(&order),
        "kyrie/1 agnus/2 credo/11 credo/8 credo/1 credo/3 credo/2 credo/7 credo/10 agnus/1 \
         credo/9 credo/5 credo/12 credo/6 credo/4"
    );
}

/// The estimate for the browser is that of the pool of the web version (`pool_seconds`,
/// web/pool.js): without the head motif each part of the pool's order to the worker free first;
/// a plan whose order is the longest first takes the time of phase 14b to the bit, and the
/// others hardly longer (K12 (a) takes the last parts longest first for that). With the head
/// motif Kyrie I first, and no part that opens with the motif before Kyrie I is set, so that the
/// parts are done no sooner than Kyrie I and the longest of them one after the other. On one
/// worker the parts take their sum either way.
#[test]
fn the_pool_waits_for_the_head_motif() {
    for request in [
        MassRequest::default(),
        MassRequest {
            mode: Some(mode("ionian-transposed")),
            seed: 2,
            ..MassRequest::default()
        },
        MassRequest {
            preset: Preset::Normal,
            choir: choir(5),
            seed: 2,
            ..MassRequest::default()
        },
        MassRequest {
            movements: vec![Movement::Kyrie, Movement::Agnus],
            choir: choir(3),
            seed: 4,
            ..MassRequest::default()
        },
    ] {
        let plan = plan_mass(&request).unwrap();
        for (workers, factor) in [(1, 1.0), (2, 1.0), (4, tables::ESTIMATE_WASM), (7, 2.5)] {
            let seconds = plan.pool_seconds(workers, factor);
            let order = plan.pool_order(workers);
            assert_eq!(
                seconds.to_bits(),
                in_order(&plan, &order, workers, factor).to_bits(),
                "{workers} workers"
            );
            let longest = in_order(&plan, &longest_first(&plan), workers, factor);
            if order == longest_first(&plan) {
                assert_eq!(seconds.to_bits(), longest.to_bits(), "{workers} workers");
            }
            // At most a second later natively, five in the browser.
            assert!(
                seconds <= longest + factor,
                "{workers} workers: {seconds} s against {longest} s longest first"
            );
        }
    }
    for seed in [1, 11] {
        let plan = plan_mass(&MassRequest {
            motif: true,
            choir: choir(5),
            seed,
            ..MassRequest::default()
        })
        .unwrap();
        let kyrie = plan.motif_source().unwrap();
        let first = plan.part(kyrie).estimate.seconds;
        let opening = plan
            .parts()
            .filter(|(_, part)| part.motif)
            .map(|(_, part)| part.estimate.seconds)
            .fold(0.0, f32::max);
        assert!(opening > 0.0);
        let four = plan.pool_seconds(4, 1.0);
        assert!(four >= first + opening, "{four} {first} {opening}");
        let total = plan.estimate().seconds;
        assert!((plan.pool_seconds(1, 1.0) - total).abs() < 0.01 * total);
    }
}

/// `plan_mass` runs no search: a plan takes milliseconds even in debug.
#[test]
fn plans_take_milliseconds() {
    let request = MassRequest {
        preset: Preset::Normal,
        choir: choir(6),
        ..MassRequest::default()
    };
    plan_mass(&request).unwrap();
    let start = Instant::now();
    for seed in 0..20 {
        plan_mass(&MassRequest {
            seed,
            ..request.clone()
        })
        .unwrap();
    }
    let each = start.elapsed() / 20;
    assert!(each < Elapsed::from_millis(100), "{each:?}");
}

/// The parsers of the options read German and English and keep the order of the items.
#[test]
fn options_parse() {
    assert_eq!(
        parse_movements("Kyrie, agnus dei,credo").unwrap(),
        [Movement::Kyrie, Movement::Agnus, Movement::Credo]
    );
    assert_eq!(
        parse_movements("gloria  agnus-dei").unwrap(),
        [Movement::Gloria, Movement::Agnus]
    );
    assert_eq!("Normal".parse::<Preset>().unwrap(), Preset::Normal);
    assert_eq!("natürlich".parse::<Lengths>().unwrap(), Lengths::Natural);
    assert_eq!(
        "Palestrina".parse::<Lengths>().unwrap(),
        Lengths::Palestrina
    );
    assert_eq!(parse_chant("keiner").unwrap(), None);
    assert_eq!(parse_chant("IX").unwrap(), Some(ChantMass::IX));
    assert_eq!(parse_chunk("aus").unwrap(), None);
    assert_eq!(parse_chunk(" 55 ").unwrap(), Some(55));
    use VoiceType::*;
    assert_eq!(
        parse_scoring("benedictus=S A T; agnus3=S A T T B;").unwrap(),
        [
            Override::Scoring {
                at: "benedictus".into(),
                voices: vec![Soprano, Alto, Tenor]
            },
            Override::Scoring {
                at: "agnus3".into(),
                voices: vec![Soprano, Alto, Tenor, Tenor, Bass]
            }
        ]
    );
    assert_eq!(
        parse_imitation("auto; incarnatus=aus; credo/3=an").unwrap(),
        [
            Override::Imitation {
                at: String::new(),
                toggle: Toggle::Auto
            },
            Override::Imitation {
                at: "incarnatus".into(),
                toggle: Toggle::Off
            },
            Override::Imitation {
                at: "credo/3".into(),
                toggle: Toggle::On
            }
        ]
    );
    assert_eq!(
        parse_endings("christe=A; gloria/3=d; agnus=Bb; credo/2=H").unwrap(),
        [
            Override::Ending {
                at: "christe".into(),
                step: Step::A
            },
            Override::Ending {
                at: "gloria/3".into(),
                step: Step::D
            },
            Override::Ending {
                at: "agnus".into(),
                step: Step::B
            },
            Override::Ending {
                at: "credo/2".into(),
                step: Step::B
            }
        ]
    );
    assert_eq!(parse_scoring("").unwrap(), []);
    assert_eq!(Repeat::None.overrides().len(), 2);
}

/// Every option that the plan cannot have fails before any search, with its message in German
/// and English.
#[test]
fn options_fail_early() {
    let dorian = mode("dorian");
    let brevis = MassRequest::default();
    let with = |overrides: Vec<Override>| MassRequest {
        overrides,
        ..MassRequest::default()
    };
    // An option the plan cannot have fails the plan as `GenerateError::MassOption`.
    let planned = |request: MassRequest| match plan_mass(&request).unwrap_err() {
        GenerateError::MassOption(error) => error,
        other => panic!("{other:?}"),
    };
    let valid = "kyrie, christe, kyrie2, gloria, qui-tollis, credo, incarnatus, crucifixus, \
                 sanctus, benedictus, osanna2, agnus, agnus3";
    let cases: Vec<(MassOptionError, MassOptionError, String, String)> = vec![
        (
            planned(MassRequest {
                choir: vec![VoiceType::Tenor; 7],
                ..brevis.clone()
            }),
            MassOptionError::Voices { voices: 7 },
            "Eine Messe braucht zwei bis sechs Stimmen, nicht 7".into(),
            "a mass needs two to six voices, not 7".into(),
        ),
        (
            planned(MassRequest {
                movements: Vec::new(),
                ..brevis.clone()
            }),
            MassOptionError::NoMovement,
            "Kein Satz gewählt; die Sätze sind kyrie, gloria, credo, sanctus, benedictus, agnus"
                .into(),
            "no movement chosen; the movements are kyrie, gloria, credo, sanctus, benedictus, \
             agnus"
                .into(),
        ),
        (
            planned(MassRequest {
                movements: vec![Movement::Kyrie, Movement::Credo, Movement::Kyrie],
                ..brevis.clone()
            }),
            MassOptionError::MovementTwice {
                movement: Movement::Kyrie,
            },
            "Der Satz Kyrie ist zweimal genannt".into(),
            "the movement Kyrie is named twice".into(),
        ),
        (
            parse_movements("kyrie, glroia").unwrap_err(),
            MassOptionError::UnknownMovement {
                name: "glroia".into(),
            },
            "Unbekannter Satz „glroia“; die Sätze sind kyrie, gloria, credo, sanctus, \
             benedictus, agnus"
                .into(),
            "unknown movement \"glroia\"; the movements are kyrie, gloria, credo, sanctus, \
             benedictus, agnus"
                .into(),
        ),
        (
            "kurz".parse::<Preset>().unwrap_err(),
            MassOptionError::UnknownPreset {
                name: "kurz".into(),
            },
            "Unbekannte Form der Messe „kurz“; die Formen sind brevis und normal".into(),
            "unknown form of the mass \"kurz\"; the forms are brevis and normal".into(),
        ),
        (
            "lang".parse::<Lengths>().unwrap_err(),
            MassOptionError::UnknownLengths {
                name: "lang".into(),
            },
            "Unbekannte Länge „lang“; die Längen sind palestrina und natürlich".into(),
            "unknown lengths \"lang\"; the lengths are palestrina and natürlich (natural)".into(),
        ),
        (
            parse_chant("xi").unwrap_err(),
            MassOptionError::UnknownChant { name: "xi".into() },
            "Unbekannter Choral „xi“; zur Wahl stehen keiner, ix und iv".into(),
            "unknown chant \"xi\"; the choices are keiner (none), ix and iv".into(),
        ),
        (
            planned(MassRequest {
                chunk: Some(10),
                ..brevis.clone()
            }),
            MassOptionError::Chunk { given: "10".into() },
            "Die Größe der Teile „10“ ist weder eine Zahl von 20 bis 200 Silben noch „aus“".into(),
            "the size of the parts \"10\" is neither a number of 20 to 200 syllables nor \"aus\" \
             (off)"
                .into(),
        ),
        (
            parse_scoring("benedictus S A T").unwrap_err(),
            MassOptionError::Item {
                option: MassOption::Scoring,
                item: "benedictus S A T".into(),
            },
            "--scoring: „benedictus S A T“ hat nicht die Form Abschnitt=Stimmen".into(),
            "--scoring: \"benedictus S A T\" is not of the form section=voices".into(),
        ),
        (
            parse_imitation("auto; christe").unwrap_err(),
            MassOptionError::Item {
                option: MassOption::Imitation,
                item: "christe".into(),
            },
            "--imitation: „christe“ hat nicht die Form Stelle=auto|an|aus".into(),
            "--imitation: \"christe\" is not of the form place=auto|on|off".into(),
        ),
        (
            parse_endings("christe").unwrap_err(),
            MassOptionError::Item {
                option: MassOption::Endings,
                item: "christe".into(),
            },
            "--endings: „christe“ hat nicht die Form Stelle=Stufe".into(),
            "--endings: \"christe\" is not of the form place=degree".into(),
        ),
        (
            planned(with(parse_imitation("pleni=aus").unwrap())),
            MassOptionError::UnknownAddress {
                at: "pleni".into(),
                valid: valid.split(", ").map(String::from).collect(),
            },
            format!("Unbekannte Stelle „pleni“; gültig sind Satz/Teil wie credo/3 und {valid}"),
            format!("unknown place \"pleni\"; valid are movement/part such as credo/3 and {valid}"),
        ),
        (
            planned(MassRequest {
                movements: vec![Movement::Kyrie],
                ..with(parse_endings("credo/3=A").unwrap())
            }),
            MassOptionError::NotPlanned {
                at: "credo/3".into(),
                movement: Movement::Credo,
            },
            "„credo/3“: Das Credo wird nicht gesetzt".into(),
            "\"credo/3\": the Credo is not set".into(),
        ),
        (
            planned(with(parse_imitation("credo/13=an").unwrap())),
            MassOptionError::NoPart {
                at: "credo/13".into(),
                movement: Movement::Credo,
                parts: 12,
            },
            "„credo/13“: Das Credo hat 12 Teile".into(),
            "\"credo/13\": the Credo has 12 parts".into(),
        ),
        (
            planned(with(parse_imitation("sanctus/2=an").unwrap())),
            MassOptionError::NoPart {
                at: "sanctus/2".into(),
                movement: Movement::Sanctus,
                parts: 1,
            },
            "„sanctus/2“: Das Sanctus hat nur einen Teil".into(),
            "\"sanctus/2\": the Sanctus has one part only".into(),
        ),
        (
            planned(with(parse_scoring("incarnatus=S A T").unwrap())),
            MassOptionError::ScoringAddress {
                at: "incarnatus".into(),
            },
            "--scoring nimmt Abschnitte wie benedictus oder agnus3, nicht „incarnatus“".into(),
            "--scoring takes sections such as benedictus or agnus3, not \"incarnatus\"".into(),
        ),
        (
            planned(with(parse_scoring("agnus3=S").unwrap())),
            MassOptionError::ScoringVoices {
                at: "agnus3".into(),
                voices: 1,
            },
            "„agnus3“: Ein Abschnitt hat zwei bis sechs Stimmen, nicht 1".into(),
            "\"agnus3\": a section has two to six voices, not 1".into(),
        ),
        (
            parse_scoring("agnus3=S X").unwrap_err(),
            MassOptionError::UnknownVoice {
                at: "agnus3".into(),
                name: "X".into(),
            },
            "„agnus3“: Unbekannte Stimmlage „X“".into(),
            "\"agnus3\": unknown voice type \"X\"".into(),
        ),
        (
            parse_imitation("credo/3=vielleicht").unwrap_err(),
            MassOptionError::UnknownToggle {
                at: "credo/3".into(),
                value: "vielleicht".into(),
            },
            "„credo/3“: „vielleicht“ ist nicht auto, an oder aus".into(),
            "\"credo/3\": \"vielleicht\" is not auto, an (on) or aus (off)".into(),
        ),
        (
            parse_endings("christe=A#").unwrap_err(),
            MassOptionError::UnknownStep {
                at: "christe".into(),
                value: "A#".into(),
            },
            "„christe“: „A#“ ist keine Stufe von A bis G".into(),
            "\"christe\": \"A#\" is no degree from A to G".into(),
        ),
        (
            planned(with(parse_endings("christe=B").unwrap())),
            MassOptionError::EndingClausula {
                at: "christe".into(),
                step: Step::B,
                mode: dorian,
            },
            "„christe“: Auf H gibt es in diesem Modus keine Klausel (verminderte Quinte)".into(),
            "\"christe\": no clausula on B in this mode (diminished fifth)".into(),
        ),
        (
            planned(with(parse_endings("christe=E").unwrap())),
            MassOptionError::EndingDegree {
                at: "christe".into(),
                step: Step::E,
                mode: dorian,
            },
            "„christe“: Ein Teil endet nicht auf E, sondern auf der Finalis, der Quinte oder der \
             Quarte des Modus mit einer Klausel: D, A oder G"
                .into(),
            "\"christe\": a part ends not on E but on the final, the fifth or the fourth of the \
             mode with a clausula: D, A or G"
                .into(),
        ),
        (
            planned(with(parse_endings("kyrie2=A").unwrap())),
            MassOptionError::EndingLast {
                at: "kyrie2".into(),
                movement: Movement::Kyrie,
            },
            "„kyrie2“: Der letzte Teil des Kyrie endet auf der Finalis".into(),
            "\"kyrie2\": the last part of the Kyrie ends on the final".into(),
        ),
        (
            planned(MassRequest {
                choir: choir(2),
                ..with(parse_endings("christe=A").unwrap())
            }),
            MassOptionError::EndingTwoVoices {
                at: "christe".into(),
            },
            "„christe“: Ein zweistimmiger Teil endet auf der Finalis".into(),
            "\"christe\": a part in two voices ends on the final".into(),
        ),
        (
            planned(MassRequest {
                preset: Preset::Normal,
                ..with(parse_imitation("osanna2=aus").unwrap())
            }),
            MassOptionError::UtSupra {
                at: "osanna2".into(),
            },
            "„osanna2“: Das Osanna ut supra wiederholt das Osanna des Sanctus und wird nicht \
             eigens gesetzt"
                .into(),
            "\"osanna2\": the Osanna ut supra repeats the Osanna of the Sanctus and is not set \
             on its own"
                .into(),
        ),
        (
            planned(MassRequest {
                preset: Preset::Normal,
                movements: vec![Movement::Benedictus],
                ..brevis.clone()
            }),
            MassOptionError::UtSupraWithoutSanctus,
            "Das Benedictus der Form normal schließt mit dem Osanna des Sanctus („Osanna ut \
             supra“); dazu gehört das Sanctus"
                .into(),
            "the Benedictus of the form normal ends with the Osanna of the Sanctus (\"Osanna ut \
             supra\"); choose the Sanctus as well"
                .into(),
        ),
        (
            planned(MassRequest {
                motif: true,
                chant: Some(ChantMass::IX),
                ..brevis.clone()
            }),
            MassOptionError::MotifWithChant,
            "Kopfmotiv und Choralparaphrase schließen einander aus".into(),
            "the head motif and the chant paraphrase exclude each other".into(),
        ),
        (
            planned(MassRequest {
                motif: true,
                movements: vec![Movement::Gloria],
                ..brevis.clone()
            }),
            MassOptionError::MotifWithoutKyrie,
            "Das Kopfmotiv kommt aus dem Kyrie I; dazu gehört das Kyrie".into(),
            "the head motif comes from Kyrie I; choose the Kyrie as well".into(),
        ),
    ];
    for (error, expected, de, en) in cases {
        assert_eq!(error, expected);
        assert_eq!(error.message(Lang::De), de);
        assert_eq!(error.message(Lang::En), en);
        assert_eq!(error.to_string(), en);
        // As an error of the generator, a sentence with its full stop.
        let generated = GenerateError::MassOption(error);
        assert_eq!(generated.message(Lang::De), format!("{de}."));
        let mut capital = en.chars();
        let first = capital.next().unwrap().to_uppercase().collect::<String>();
        assert_eq!(
            generated.message(Lang::En),
            format!("{first}{}.", capital.as_str())
        );
    }
    // The same errors from the other ways in: a section of an unplanned movement, an ending of
    // the whole mass, a scoring at a part.
    assert!(matches!(
        planned(MassRequest {
            movements: vec![Movement::Kyrie],
            ..with(parse_imitation("incarnatus=aus").unwrap())
        }),
        MassOptionError::NotPlanned {
            movement: Movement::Credo,
            ..
        }
    ));
    assert!(matches!(
        planned(with(parse_endings("=A").unwrap())),
        MassOptionError::UnknownAddress { .. }
    ));
    assert!(matches!(
        planned(with(parse_scoring("credo/3=S A T").unwrap())),
        MassOptionError::ScoringAddress { .. }
    ));
    assert_eq!(
        parse_movements("kyrie kyrie").unwrap_err(),
        MassOptionError::MovementTwice {
            movement: Movement::Kyrie
        }
    );
    assert_eq!(
        parse_movements(" ,").unwrap_err(),
        MassOptionError::NoMovement
    );
    assert!(parse_chunk("300").is_err() && parse_chunk("viel").is_err());
    assert!(parse_endings("christe=").is_err());
    // A mode flattens B only: a flat after another letter is no degree.
    for value in ["Ab", "Eb", "e♭", "Hb", "Bbb"] {
        assert_eq!(
            parse_endings(&format!("christe={value}")).unwrap_err(),
            MassOptionError::UnknownStep {
                at: "christe".into(),
                value: value.into()
            }
        );
    }
    assert_eq!(
        parse_endings("christe=B♭; agnus=bb").unwrap(),
        [
            Override::Ending {
                at: "christe".into(),
                step: Step::B
            },
            Override::Ending {
                at: "agnus".into(),
                step: Step::B
            }
        ]
    );
}

/// The profile of a mass (D7) gives the parameters for settings without a plan of imitation the
/// levels of their counterparts with a plan, V, P3 and P1 in Fux's profile and those of another
/// profile in it, and changes nothing else; given twice it is the same. The profile of the parts
/// adds the overrides of `--repeat`, and its number tells profiles apart.
#[test]
fn profile_overlays_the_unplanned_levels() {
    const PARAMS: [(&str, &str, &str); 3] = [
        ("IMI-HELD", "unplanned_struck", "struck"),
        ("IMI-HELD", "unplanned_passing", "passing"),
        ("IMI-SILENCE", "unplanned_thin", "thin"),
    ];
    let levels = |profile: &Profile, planned: bool| -> Vec<Level> {
        PARAMS
            .iter()
            .map(|&(rule, unplanned, counterpart)| {
                let param = if planned { counterpart } else { unplanned };
                profile.settings(rule).level(param)
            })
            .collect()
    };
    let fux = Profile::fux();
    assert_eq!(levels(&fux, false), [Level::Off; 3]);
    let mass = fux_core::mass::profile(&fux);
    assert_eq!(
        levels(&mass, false),
        [Level::Forbidden, Level::P3, Level::P1]
    );
    let changed: Vec<String> = mass
        .to_toml(Some(&fux))
        .lines()
        .filter(|line| line.contains(" = ") && !line.starts_with("name"))
        .map(String::from)
        .collect();
    assert_eq!(
        changed,
        [
            "unplanned_thin = \"P1\"",
            "unplanned_struck = \"V\"",
            "unplanned_passing = \"P3\""
        ]
    );
    assert_eq!(
        fux_core::mass::profile(&mass).to_toml(None),
        mass.to_toml(None)
    );
    let mut strict = Profile::fux();
    strict.apply_override("IMI-HELD.passing=V").unwrap();
    strict.apply_override("IMI-SILENCE.thin=P2").unwrap();
    assert_eq!(
        levels(&fux_core::mass::profile(&strict), false),
        [Level::Forbidden, Level::Forbidden, Level::P2]
    );
    let jeppesen = Profile::jeppesen();
    assert_eq!(
        levels(&fux_core::mass::profile(&jeppesen), false),
        levels(&jeppesen, true)
    );
    let plain = part_profile(&MassRequest::default(), &mass);
    assert_eq!(plain.to_toml(None), mass.to_toml(None));
    let never = part_profile(
        &MassRequest {
            repeat: Repeat::None,
            ..MassRequest::default()
        },
        &mass,
    );
    assert_eq!(never.settings("TXT-REPEAT").level("long"), Level::Forbidden);
    assert_eq!(profile_hash(&plain), profile_hash(&mass));
    assert_ne!(profile_hash(&never), profile_hash(&mass));
    assert_ne!(profile_hash(&mass), profile_hash(&fux));
}

/// Every part of brevis and normal plans in several modes and choirs asks the search for what
/// the plan says (§ 4.1): its movement's mode; its section's voices from the top; its text,
/// going on where it is not the last of its section, with the request's accents; imitation with
/// the plan's points (as `fux free --imitation auto` asks for it) where it begins in imitation,
/// an opening chord only where it begins together; its ending, start and chain; the portable
/// budget, its seed, the request's beam and noise. A part in two voices ends on the final. With
/// the control cancelled before, the requests pass the checks of the search, of their text's plan
/// and of their imitation's planner and then end as cancelled, before any search begins (so did
/// all 6360 parts of both forms in every mode, two to six voices, when this was first checked):
/// every part that begins together, and the imitative parts of a choir in each form, whose
/// planner takes most of the time (10 s for all of them in debug).
#[test]
fn part_request_matches_the_plan() {
    let profile = fux_core::mass::profile(&Profile::fux());
    let control = Arc::new(SearchControl::new());
    control.cancel();
    let mut parts = 0;
    for preset in [Preset::Brevis, Preset::Normal] {
        for (name, voices) in [
            ("dorian", 4),
            ("phrygian", 3),
            ("mixolydian", 5),
            ("ionian-transposed", 2),
            ("aeolian", 6),
        ] {
            let planned = match preset {
                Preset::Brevis => voices == 4,
                Preset::Normal => voices == 3,
            };
            let request = MassRequest {
                preset,
                mode: Some(mode(name)),
                choir: choir(voices),
                accents: voices % 2 == 0,
                beam: 120,
                noise: 1.5,
                seed: 3,
                ..MassRequest::default()
            };
            let plan = plan_mass(&request).unwrap();
            for (at, part) in plan.parts() {
                let label = format!("{preset:?} {name} {voices}: {}", plan.address(at));
                let movement = &plan.movements[at.movement];
                let section = &movement.sections[at.section];
                let free = part_request(&request, &plan, at, None);
                let types: Vec<VoiceType> =
                    section.voices.iter().map(|&v| movement.voices[v]).collect();
                assert_eq!(
                    (free.mode, &free.voices),
                    (movement.mode, &types),
                    "{label}"
                );
                let sung = free.text.as_ref().unwrap();
                assert_eq!(sung.text.spec(), part.text, "{label}");
                assert_eq!(sung.text.continued, part.continued, "{label}");
                assert_eq!(sung.text.syllables.len(), part.syllables, "{label}");
                assert!(sung.auto_length && sung.auto_cadences, "{label}");
                assert_eq!(sung.accents, request.accents, "{label}");
                let imitation = (part.opening == Opening::Imitative).then(|| Imitation {
                    points: part.points,
                    ..Imitation::default()
                });
                assert_eq!(free.imitation, imitation, "{label}");
                assert_eq!(
                    part.chord.is_some(),
                    part.opening == Opening::Tutti,
                    "{label}"
                );
                assert_eq!(free.part.chord, part.chord, "{label}");
                assert_eq!(free.part.ending, Some(part.ending), "{label}");
                assert_eq!(free.part.start, part.start, "{label}");
                assert_eq!(free.part.chain, part.chain, "{label}");
                assert_eq!(free.budget, Budget::PORTABLE, "{label}");
                assert_eq!(
                    (free.seed, free.beam, free.noise, free.count),
                    (part.seed, 120, 1.5, 1),
                    "{label}"
                );
                assert!(free.control.is_none(), "{label}");
                if types.len() == 2 {
                    assert_eq!(part.ending, movement.mode.final_step(), "{label}");
                }
                if free.imitation.is_some() && !planned {
                    continue;
                }
                let cancelled = FreeRequest {
                    control: Some(Arc::clone(&control)),
                    ..free
                };
                assert_eq!(
                    generate_free(&cancelled, &profile).unwrap_err(),
                    GenerateError::Cancelled,
                    "{label}"
                );
                parts += 1;
            }
        }
    }
    assert!(parts > 150, "{parts} parts searched");
    assert_eq!(control.reached(), 0, "no search began");
}

/// A part that fails every attempt (§§ 3.8, 4.1, 4.4): the Agnus Dei begun together under a
/// profile that forbids every entry off the final and the fifth (FREE-ENTRY V), so that the third
/// of each opening chord fails at once. Agnus Dei I has its three attempts as planned; Agnus Dei
/// II in five voices its three and three more in the choir of the mass, numbered on (3 to 5),
/// each attempt seeded with `splitmix64(part seed + attempt)`. Every attempt begins and fails as
/// an event; the error of a part's last attempt names the part, its words, its attempts and what
/// may help, beginning in imitation among it, the earlier ones are the search's own. The mass is
/// incomplete, without either part.
#[test]
fn failed_attempts_take_the_fallbacks() {
    let request = MassRequest {
        movements: vec![Movement::Agnus],
        overrides: parse_imitation("aus").unwrap(),
        beam: 40,
        ..MassRequest::default()
    };
    let mut profile = fux_core::mass::profile(&Profile::fux());
    profile.apply_override("FREE-ENTRY=V").unwrap();
    let mut events = Vec::new();
    let mass = generate_mass(&request, &profile, None, &mut |event| {
        events.push(event);
        ControlFlow::Continue(())
    })
    .unwrap();
    let plan = Arc::clone(&mass.plan);
    assert!(mass.results.iter().all(Option::is_none) && !mass.complete);
    let mut started: Vec<(String, u8)> = Vec::new();
    let mut failed: Vec<(String, u8)> = Vec::new();
    for event in &events {
        match event {
            MassEvent::Planned(_) => {}
            MassEvent::Started { at, attempt, seed } => {
                let part = plan.part(*at);
                assert_eq!(
                    *seed,
                    Rng::new(part.seed + u64::from(*attempt)).next_u64(),
                    "{}",
                    plan.address(*at)
                );
                started.push((plan.address(*at), *attempt));
            }
            MassEvent::Failed { at, attempt, error } => {
                let address = plan.address(*at);
                let last = if address == "agnus/1" { 2 } else { 5 };
                match error {
                    GenerateError::MassPart {
                        part,
                        attempts,
                        error,
                        remedies,
                        words,
                    } => {
                        assert_eq!(*attempt, last, "{address}");
                        assert_eq!((part, *attempts), (&address, last + 1));
                        assert!(words.starts_with("A-gnus De-i"), "{words}");
                        assert!(
                            matches!(**error, GenerateError::NoCounterpoint { .. }),
                            "{error:?}"
                        );
                        assert!(
                            remedies.contains(&format!("--imitation {address}=an")),
                            "{remedies:?}"
                        );
                    }
                    GenerateError::NoCounterpoint { .. } => {
                        assert!(*attempt < last, "{address}: {error:?}")
                    }
                    other => panic!("{address}: {other:?}"),
                }
                failed.push((address, *attempt));
            }
            other => panic!("{other:?}"),
        }
    }
    let attempts: Vec<(String, u8)> = (0..3)
        .map(|a| ("agnus/1".to_string(), a))
        .chain((0..6).map(|a| ("agnus/2".to_string(), a)))
        .collect();
    assert_eq!(started, attempts);
    assert_eq!(failed, attempts);
}

/// Parts in two voices begin together on their opening chord, a perfect consonance over the
/// final (CAD-START), and are set: the Kyrie of a mass for S T, every part on its first attempt,
/// and the Christe for S T in a mass for four voices (`--scoring christe=S T`). Each is a texted
/// setting without a prohibition, strikes the planned chord at its start in both voices and ends
/// on the final. With the search narrower than a mass's, in a debug build.
#[test]
fn two_voices_set_their_parts() {
    let profile = fux_core::mass::profile(&Profile::fux());
    for request in [
        MassRequest {
            choir: choir(2),
            movements: vec![Movement::Kyrie],
            beam: 40,
            ..MassRequest::default()
        },
        MassRequest {
            movements: vec![Movement::Kyrie],
            overrides: parse_scoring("christe=S T").unwrap(),
            beam: 40,
            ..MassRequest::default()
        },
    ] {
        let plan = plan_mass(&request).unwrap();
        let profile = part_profile(&request, &profile);
        for (at, part) in plan.parts() {
            let movement = &plan.movements[at.movement];
            let section = &movement.sections[at.section];
            if section.voices.len() != 2 {
                continue;
            }
            let label = plan.address(at);
            assert_eq!(part.opening, Opening::Tutti, "{label}");
            let chord = part.chord.clone().unwrap();
            let result = generate_part(&request, &plan, at, &profile, None, None)
                .unwrap_or_else(|error| panic!("{label}: {error}"));
            assert_eq!(result.attempt, 0, "{label}");
            let generated = &result.generated;
            assert_part_texted(generated, &part.sung_text(&request.text), &profile, &label);
            let score = &generated.score;
            for (v, voice) in score.voices.iter().enumerate() {
                let (start, first) = voice.timed().find(|(_, note)| !note.is_rest()).unwrap();
                assert_eq!(
                    (start, first.pitch),
                    (part.start, Some(chord[v])),
                    "{label}"
                );
            }
            let bass = score.voices.last().unwrap().pitches();
            assert_eq!(bass.last().unwrap().step, Step::D, "{label}");
        }
    }
}

/// A mass whose control is cancelled before it begins is planned and says so, then ends without
/// a part: `Planned`, then `Cancelled`. A callback that breaks off at the plan ends it the same
/// way. An option the plan cannot have fails the mass before any event, and a part whose control
/// is cancelled fails before its search. The events can go to another thread.
#[test]
fn a_cancelled_mass_sets_no_part() {
    fn sent<T: Send + 'static>() {}
    sent::<MassEvent>();
    let request = MassRequest {
        movements: vec![Movement::Kyrie, Movement::Gloria],
        ..MassRequest::default()
    };
    let profile = fux_core::mass::profile(&Profile::fux());
    let control = Arc::new(SearchControl::new());
    control.cancel();
    let mut events = Vec::new();
    let mass = generate_mass(
        &request,
        &profile,
        Some(Arc::clone(&control)),
        &mut |event| {
            events.push(event);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(
        matches!(
            events.as_slice(),
            [MassEvent::Planned(_), MassEvent::Cancelled]
        ),
        "{events:?}"
    );
    let MassEvent::Planned(planned) = &events[0] else {
        unreachable!()
    };
    assert!(Arc::ptr_eq(planned, &mass.plan));
    assert_eq!(**planned, plan_mass(&request).unwrap());
    assert_eq!(mass.results.len(), 9);
    assert!(mass.results.iter().all(Option::is_none) && !mass.complete);
    assert_eq!(control.reached(), 0);
    let mut told = 0;
    let mass = generate_mass(&request, &profile, None, &mut |event| {
        told += 1;
        match event {
            MassEvent::Planned(_) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    })
    .unwrap();
    assert_eq!(told, 2);
    assert!(mass.results.iter().all(Option::is_none) && !mass.complete);
    let alone = MassRequest {
        choir: vec![VoiceType::Tenor],
        ..request.clone()
    };
    let mut told = false;
    let error = generate_mass(&alone, &profile, None, &mut |_| {
        told = true;
        ControlFlow::Continue(())
    })
    .unwrap_err();
    assert_eq!(
        error,
        GenerateError::MassOption(MassOptionError::Voices { voices: 1 })
    );
    assert!(!told);
    let plan = plan_mass(&request).unwrap();
    for (at, _) in plan.parts() {
        let error = generate_part(
            &request,
            &plan,
            at,
            &profile,
            None,
            Some(Arc::clone(&control)),
        );
        assert_eq!(error.unwrap_err(), GenerateError::Cancelled);
    }
}

/// `pitches` with a pitch repeated at once counted once.
fn merged(pitches: impl IntoIterator<Item = Pitch>) -> Vec<Pitch> {
    let mut merged: Vec<Pitch> = Vec::new();
    for pitch in pitches {
        if merged.last() != Some(&pitch) {
            merged.push(pitch);
        }
    }
    merged
}

/// The pitches of the subject of the first point of `generated` in `mode`, as its leader sings
/// it at the unison, a pitch repeated at once counted once; and the pitches its leader sings,
/// fewer where its head is cut short (fuga sciolta), counted so.
fn first_heads(generated: &Generated, mode: ModeSpec) -> (Vec<Pitch>, Vec<Pitch>) {
    let subject = &generated.subjects[0];
    let first = mode.final_pitch(3).diatonic() + i32::from(subject.degree);
    let pitches = subject.pitches(mode, first, 0, Answer::Real).unwrap();
    let plan = generated
        .score
        .imitation
        .as_deref()
        .expect("a plan of imitation");
    let leader = plan.points[0].entries[0]
        .notes
        .iter()
        .map(|&(pitch, _)| pitch);
    (merged(pitches), merged(leader))
}

/// Whether `head`, the pitches a leader sings, are the first pitches of `motif` in `mode`, in
/// some octave.
fn sings_the_motif(head: &[Pitch], motif: &Motif, mode: ModeSpec) -> bool {
    let all = motif.pitches(mode, head[0].octave);
    let up = motif.pitches(mode, head[0].octave - 1);
    head.len() >= 2 && (all.starts_with(head) || up.starts_with(head))
}

/// A small mass with the head motif (D11): Kyrie and Agnus Dei for A T B, seed 4, the lengths
/// natural, with a narrow search for a debug build.
fn motif_mass() -> MassRequest {
    MassRequest {
        movements: vec![Movement::Kyrie, Movement::Agnus],
        choir: choir(3),
        motif: true,
        lengths: Lengths::Natural,
        beam: 40,
        seed: 4,
        ..MassRequest::default()
    }
}

/// MASS-MOTIF: the head motif is the first subject of Kyrie I as set (`motif_of`), its pitches
/// as the subject has them, a pitch repeated at once counted once, its leader singing the first
/// of them at least; Kyrie I's result and the field of its file keep it, and Kyrie I read back
/// from its file, which keeps no subjects, gives it again from its field. Agnus Dei I, which takes
/// it, opens with a subject on its pitches and keeps the motif it was given in its result and its
/// field, whose motif fits the mass's only; without a motif it begins with a subject from its
/// words and says why. With the search narrower than a mass's, in a debug build (some seconds).
#[test]
fn kyrie_i_gives_the_head_motif() {
    let request = motif_mass();
    let plan = plan_mass(&request).unwrap();
    let profile = part_profile(&request, &fux_core::mass::profile(&Profile::fux()));
    let kyrie = plan.find("kyrie/1").unwrap()[0];
    assert_eq!(plan.motif_source(), Some(kyrie));
    let result = generate_part(&request, &plan, kyrie, &profile, None, None).unwrap();
    let motif = motif_of(&result.generated).expect("a head motif of five to ten tones");
    let mode = plan.movements[kyrie.movement].mode;
    assert!(Motif::TONES.contains(&motif.tones.len()), "{motif}");
    let (subject, leader) = first_heads(&result.generated, mode);
    assert_eq!(subject, motif.pitches(mode, subject[0].octave));
    assert!(sings_the_motif(&leader, &motif, mode), "{leader:?}");
    assert_eq!(result.motif.as_ref(), Some(&motif));
    let field = PartField::of(&plan, kyrie, &result, 1);
    assert_eq!(field.motif.as_ref(), Some(&motif));
    assert!(
        field
            .to_string()
            .contains(&format!(";motif={motif};profile="))
    );
    assert_eq!(field.to_string().parse::<PartField>(), Ok(field.clone()));
    let back = part_from_parts(
        &plan,
        kyrie,
        result.generated.score.clone(),
        &field,
        &profile,
    )
    .unwrap();
    assert!(back.generated.subjects.is_empty());
    assert_eq!(
        (motif_of(&back.generated), back.motif.as_ref()),
        (None, Some(&motif))
    );
    // Agnus Dei I on the motif.
    let agnus = plan.find("agnus/1").unwrap()[0];
    let label = "agnus/1";
    let set = generate_part(&request, &plan, agnus, &profile, Some(&motif), None)
        .unwrap_or_else(|error| panic!("{label}: {error}"));
    let generated = &set.generated;
    assert_eq!(generated.subjects[0].origin, Origin::Motif, "{label}");
    let (subject, leader) = first_heads(generated, mode);
    assert_eq!(subject, motif.pitches(mode, subject[0].octave), "{label}");
    assert!(
        sings_the_motif(&leader, &motif, mode),
        "{label}: {leader:?}"
    );
    assert_eq!(set.warnings, [], "{label}");
    assert_eq!(set.motif.as_ref(), Some(&motif));
    assert_part_texted(
        generated,
        &plan.part(agnus).sung_text(&request.text),
        &profile,
        label,
    );
    let field = PartField::of(&plan, agnus, &set, 1);
    assert_eq!(field.motif.as_ref(), Some(&motif));
    assert!(field.motif_fits(&plan, agnus, Some(&motif)));
    assert!(!field.motif_fits(&plan, agnus, None));
    let other: Motif = "0,2,3,4,3".parse().unwrap();
    assert!(!field.motif_fits(&plan, agnus, Some(&other)));
    let mut read = generated.score.clone();
    if let Some(sung) = read.text.as_mut() {
        sung.text = Arc::new(Text {
            continued: false,
            ..(*sung.text).clone()
        });
    }
    let back = part_from_parts(&plan, agnus, read, &field, &profile).unwrap();
    assert_eq!(back.motif.as_ref(), Some(&motif));
    assert_eq!(back.generated.report, generated.report);
    // Kyrie I keeps the motif it gives, whatever is asked of it; Agnus Dei II takes none.
    let kyrie_field = PartField::of(&plan, kyrie, &result, 1);
    assert!(kyrie_field.motif_fits(&plan, kyrie, None));
    let agnus3 = plan.find("agnus3").unwrap()[0];
    assert!(!field.motif_fits(&plan, agnus3, Some(&motif)));
    // Without a motif from Kyrie I.
    let without = generate_part(&request, &plan, agnus, &profile, None, None).unwrap();
    assert_eq!(without.generated.subjects[0].origin, Origin::Text);
    assert_eq!(
        without.warnings,
        [Warning::NoMotif {
            movement: "Agnus Dei".into()
        }]
    );
    assert_eq!(without.motif, None);
    assert_eq!(
        without.warnings[0].message(Lang::De),
        "Agnus Dei: Das Kyrie I gab kein Kopfmotiv (es beginnt gemeinsam, fehlt, oder sein erstes \
         Soggetto hat nicht 5 bis 10 Töne); der erste Punkt nimmt sein Soggetto aus dem Text"
    );
}

/// MASS-MOTIF in Gloria and Credo: the first point of each takes the motif, its leader with the
/// motif's pitches, its other entries answering it, as their plans of imitation lay it out (no
/// search); a part after them takes its subjects from its words. A motif that no head can sing
/// (a tritone) gives the first point a subject from its words, and the part set with it says so.
#[test]
fn later_movements_open_with_the_motif() {
    let request = MassRequest {
        motif: true,
        seed: 3,
        ..MassRequest::default()
    };
    let plan = plan_mass(&request).unwrap();
    let profile = part_profile(&request, &fux_core::mass::profile(&Profile::fux()));
    let motif: Motif = "0,2,1,4,3,2,0,1".parse().unwrap();
    for at in ["gloria/1", "credo/1", "sanctus/1", "agnus/1"] {
        let place = plan.find(at).unwrap()[0];
        let mode = plan.movements[place.movement].mode;
        let free = part_request(&request, &plan, place, Some(&motif));
        let imitation = free.imitation.clone().unwrap();
        let planner = plan_imitation(&free, &imitation, &profile).unwrap();
        assert_eq!(planner.fallbacks(), [], "{at}");
        let laid = planner.plan();
        let point = &laid.points[0];
        let subject = &point.subject;
        assert_eq!(subject.origin, Origin::Motif, "{at}");
        let first = mode.final_pitch(3).diatonic() + i32::from(subject.degree);
        let pitches = merged(subject.pitches(mode, first, 0, Answer::Real).unwrap());
        assert_eq!(pitches, motif.pitches(mode, pitches[0].octave), "{at}");
        // The leader sings it, as far as its head goes, and every entry four notes of it.
        let leader = merged(point.entries[0].notes.iter().map(|&(p, _, _)| p));
        assert!(sings_the_motif(&leader, &motif, mode), "{at}: {leader:?}");
        for entry in &point.entries {
            assert!(entry.notes.len() >= 4, "{at}");
        }
        if laid.points.len() > 1 {
            assert_ne!(laid.points[1].subject.origin, Origin::Motif, "{at}");
        }
    }
    let tritone: Motif = "0,2,5,4,3".parse().unwrap();
    let place = plan.find("gloria/1").unwrap()[0];
    let free = part_request(&request, &plan, place, Some(&tritone));
    let planner = plan_imitation(&free, free.imitation.as_ref().unwrap(), &profile).unwrap();
    assert_eq!(planner.fallbacks(), [ChantFallback { point: 1 }]);
    assert_eq!(planner.plan().points[0].subject.origin, Origin::Text);
}

/// MASS-MOTIF: where the motif gives the first point no subject, the part takes one from its
/// words and says so, by its movement, as a fallback of the motif (not of a chant). Agnus Dei I
/// for A T B with a motif no head can sing, in a debug build (some seconds).
#[test]
fn an_unsung_motif_falls_back() {
    let request = motif_mass();
    let plan = plan_mass(&request).unwrap();
    let profile = part_profile(&request, &fux_core::mass::profile(&Profile::fux()));
    let tritone: Motif = "0,2,5,4,3".parse().unwrap();
    let agnus = plan.find("agnus/1").unwrap()[0];
    let set = generate_part(&request, &plan, agnus, &profile, Some(&tritone), None).unwrap();
    assert_eq!(set.generated.subjects[0].origin, Origin::Text);
    let fallback = Warning::MotifFallback {
        movement: "Agnus Dei".into(),
    };
    assert_eq!(set.warnings, std::slice::from_ref(&fallback));
    assert_eq!(set.generated.warnings, [fallback]);
    assert_eq!(set.motif, Some(tritone));
}

/// MASS-CHANT: a Credo in parts follows its chant across the seams and Breaks. Every part of the
/// Credo of a Missa brevis on chant IX, each in imitation, takes Credo I from where the chant
/// sings its words: a part that begins within a section of the text from its first words on, so
/// that its points find their phrases there; the points of all parts take phrases one after the
/// other through the chant (or say why they take none), as their plans lay them out. The Credo
/// takes the mode of its chant. The same with `--chunk 20`, whose parts begin at Gé-ni-tum and
/// the like, words the chant sings earlier within others.
#[test]
fn a_credo_in_parts_follows_the_chant() {
    let profile = fux_core::mass::profile(&Profile::fux());
    for chunk in [Some(45), Some(20)] {
        let request = MassRequest {
            chant: Some(ChantMass::IX),
            movements: vec![Movement::Credo],
            overrides: parse_imitation("an").unwrap(),
            chunk,
            ..MassRequest::default()
        };
        let plan = plan_mass(&request).unwrap();
        assert_eq!(plan.movements[0].mode, mode("hypophrygian"));
        let profile = part_profile(&request, &profile);
        let whole = fux_core::imitation::chant::Kyriale::CredoI.chant();
        // The first note of the phrase of the last point, in the whole chant.
        let mut last = 0;
        let mut within = 0;
        for (at, part) in plan.parts() {
            let label = format!("{chunk:?} {}", part.text);
            let chant = part_chant(&plan, at).unwrap();
            let begins = whole.notes.len() - chant.notes.len();
            assert_eq!(chant.notes[..], whole.notes[begins..], "{label}");
            // A part that begins after the first unit of a section of the text (`credo:5-7`).
            let first_unit = part
                .text
                .split('+')
                .next()
                .and_then(|first| first.split_once(':'))
                .map_or(1, |(_, units)| {
                    units.split('-').next().unwrap().parse::<usize>().unwrap()
                });
            if first_unit > 1 {
                assert!(begins > last, "{label}: {begins} after {last}");
                within += 1;
            }
            let free = part_request(&request, &plan, at, None);
            let imitation = free.imitation.as_ref().unwrap();
            assert!(
                matches!(
                    &imitation.subject,
                    SubjectSource::Chant { phrases: None, .. }
                ),
                "{label}"
            );
            let planner = plan_imitation(&free, imitation, &profile).unwrap();
            let fallbacks: Vec<usize> = planner.fallbacks().iter().map(|f| f.point).collect();
            let phrases = chant.phrases();
            for (p, point) in planner.plan().points.iter().enumerate() {
                let Origin::Chant { phrase } = point.subject.origin else {
                    assert!(fallbacks.contains(&(p + 1)), "{label}, point {p}");
                    continue;
                };
                let note = begins + phrases[phrase].range.start;
                assert!(note >= last, "{label}, point {p}: {note} before {last}");
                last = note;
            }
        }
        assert!(within >= 3, "{within}");
    }
}

/// A part's result made by hand, for the joins and the files: in the voices `voices` of its
/// movement it sings its text a syllable a measure, after a measure's rest where it begins after
/// silence, each voice on its pitch of the opening chord of those voices, and holds its last
/// syllable a measure more; it has an internal cadence on the final where its text is long
/// enough and, where `imitative`, a point in which every voice enters with its first note. Its
/// text is divided and printed as the plan has it (eleison in four syllables or in three, the
/// acutes or none), its report is that of `profile`, its seed that of the part's first attempt.
fn handmade(
    plan: &MassPlan,
    at: PartRef,
    voices: &[usize],
    imitative: bool,
    profile: &Profile,
) -> PartResult {
    let movement = &plan.movements[at.movement];
    let part = plan.part(at);
    let text = part.sung_text(&plan.text);
    let kinds: Vec<VoiceType> = voices.iter().map(|&v| movement.voices[v]).collect();
    let chord = opening_chord(movement.mode, &kinds, movement.mode.final_step());
    let silent = usize::from(part.start > 0);
    let syllables = text.syllables.len();
    let lines: Vec<Voice> = kinds
        .iter()
        .zip(&chord)
        .map(|(&kind, &pitch)| {
            let mut notes = vec![Note::rest(Duration::WHOLE); silent];
            notes.extend((0..syllables).map(|i| Note {
                lyric: Some(Lyric::new(i)),
                ..Note::new(pitch, Duration::WHOLE)
            }));
            notes.push(Note::new(pitch, Duration::WHOLE));
            Voice::new(kind, Role::Counterpoint(Species::Free), notes)
        })
        .collect();
    let mut score = Score::new(movement.mode, lines);
    if syllables >= 6 {
        score.cadences = vec![Cadence {
            measure: silent + 3,
            target: movement.mode.final_step(),
        }];
    }
    score.text = Some(ScoreText {
        text: Arc::new(text),
        voices: vec![true; kinds.len()],
    });
    if imitative {
        let start = silent as u32 * 8;
        let entries = chord
            .iter()
            .enumerate()
            .map(|(voice, &pitch)| ScoreEntry {
                voice,
                start,
                rest: 0,
                notes: vec![(pitch, 8)],
            })
            .collect();
        score.imitation = Some(Arc::new(ScoreImitation {
            points: vec![ScorePoint {
                words: 0..1,
                entries,
                goal: None,
            }],
        }));
    }
    score.fill_syllables(plan.accents);
    score.validate().unwrap();
    let report = Checker::new(profile).check(&score);
    PartResult {
        at,
        // The seed of attempt 0: `splitmix64(part seed + 0)`, the first number of its generator.
        seed: Rng::new(part.seed).next_u64(),
        attempt: 0,
        generated: Generated {
            score,
            cost: report.cost(),
            report,
            warnings: Vec::new(),
            subjects: Vec::new(),
        },
        seconds: 0.0,
        warnings: Vec::new(),
        voices: voices.to_vec(),
        profile: profile_hash(profile),
        motif: None,
        kept: None,
    }
}

/// `result`, a part of `plan`, as attempt `attempt` would have set it: with that attempt's seed
/// (MASS-SEED, `splitmix64(part seed + attempt)`), as a fallback sets a part from attempt 3 on.
fn attempted(plan: &MassPlan, result: PartResult, attempt: u8) -> PartResult {
    let part = plan.part(result.at);
    PartResult {
        seed: Rng::new(part.seed + u64::from(attempt)).next_u64(),
        attempt,
        ..result
    }
}

/// A mass of `request` whose every part is made by hand ([`handmade`]) in the voices of its
/// section, imitative where it begins in imitation.
fn handmade_mass(request: &MassRequest, profile: &Profile) -> Mass {
    let plan = Arc::new(plan_mass(request).unwrap());
    let mut mass = Mass::new(Arc::clone(&plan));
    for (at, part) in plan.parts() {
        let voices = &plan.movements[at.movement].sections[at.section].voices;
        let imitative = part.opening == Opening::Imitative;
        mass.insert(handmade(&plan, at, voices, imitative, profile));
    }
    assert!(mass.complete);
    mass
}

/// The measures of each part of a section, and where each begins in it.
fn offsets(mass: &Mass, movement: usize, section: usize) -> Vec<(usize, usize)> {
    let parts = mass.plan.movements[movement].sections[section].parts.len();
    let mut at = 0;
    (0..parts)
        .map(|part| {
            let result = mass
                .result(PartRef {
                    movement,
                    section,
                    part,
                })
                .unwrap();
            let length = result.generated.score.measure_count();
            at += length;
            (length, at - length)
        })
        .collect()
}

/// A section is its parts one after the other (§ 2.2): as many measures as they have together,
/// in every voice of its movement, those that do not sing in it resting; a mark where a part
/// begins after a Break, with its label, or after a seam; one text of the parts' texts, not going
/// on after it, which every voice that sings sings through, every lyric counted in it with its
/// syllable as printed; the parts' internal cadences and the endings of all but the last part;
/// one plan of the imitative parts, their entries in the voices of the movement, a voice's first
/// entry in a later part after its silence there. It validates. An Osanna ut supra is the Osanna
/// it repeats; a section with a part not set is none.
#[test]
fn sections_join_their_parts() {
    let profile = fux_core::mass::profile(&Profile::fux());
    let request = MassRequest {
        movements: vec![Movement::Gloria, Movement::Benedictus, Movement::Agnus],
        overrides: parse_imitation(
            "gloria/2=an; gloria/3=aus; qui-tollis=an; gloria/5=aus; gloria/6=aus",
        )
        .unwrap(),
        ..MassRequest::default()
    };
    let mass = handmade_mass(&request, &profile);
    let plan = &mass.plan;
    for (m, movement) in plan.movements.iter().enumerate() {
        for (s, section) in movement.sections.iter().enumerate() {
            let label = section.id;
            let score = section_score(&mass, m, s).unwrap();
            score.validate().unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!(score.title.as_deref(), Some(section.title), "{label}");
            let places = offsets(&mass, m, s);
            let measures: usize = places.iter().map(|&(length, _)| length).sum();
            assert_eq!(score.measure_count(), measures, "{label}");
            let sung = score.text.as_ref().unwrap();
            assert_eq!(score.voices.len(), movement.voices.len(), "{label}");
            for (v, voice) in score.voices.iter().enumerate() {
                assert_eq!(
                    (&voice.name, voice.voice_type),
                    (&movement.names[v], movement.voices[v])
                );
                let sings = section.voices.contains(&v);
                assert_eq!(sung.voices[v], sings, "{label}, voice {v}");
                if !sings {
                    assert!(voice.notes.iter().all(Note::is_rest), "{label}, voice {v}");
                }
            }
            let marks: Vec<SectionMark> = section
                .parts
                .iter()
                .zip(&places)
                .skip(1)
                .map(|(part, &(_, measure))| match part.join {
                    Join::Break(label) => SectionMark {
                        measure,
                        kind: SectionKind::Break,
                        title: Some(label.to_string()),
                    },
                    _ => SectionMark {
                        measure,
                        kind: SectionKind::Seam,
                        title: None,
                    },
                })
                .collect();
            assert_eq!(score.sections, marks, "{label}");
            let text = &sung.text;
            let specs: Vec<&str> = section.parts.iter().map(|p| p.text.as_str()).collect();
            assert_eq!(text.id, specs.join("+"), "{label}");
            assert!(!text.continued, "{label}");
            let syllables: usize = section.parts.iter().map(|p| p.syllables).sum();
            assert_eq!(text.syllables.len(), syllables, "{label}");
            for &v in &section.voices {
                let lyrics: Vec<usize> = score.voices[v]
                    .notes
                    .iter()
                    .filter_map(|note| {
                        let i = note.lyric?.index()?;
                        let printed = text.syllable_label(i, true);
                        assert_eq!(note.syllable.as_deref(), Some(printed.as_str()), "{label}");
                        Some(i)
                    })
                    .collect();
                assert_eq!(lyrics, (0..syllables).collect::<Vec<_>>(), "{label}");
            }
            let mut cadences = Vec::new();
            for (p, (part, &(length, at))) in section.parts.iter().zip(&places).enumerate() {
                let result = mass
                    .result(PartRef {
                        movement: m,
                        section: s,
                        part: p,
                    })
                    .unwrap();
                cadences.extend(result.generated.score.cadences.iter().map(|c| Cadence {
                    measure: c.measure + at,
                    ..*c
                }));
                if p + 1 < places.len() {
                    cadences.push(Cadence {
                        measure: at + length - 1,
                        target: part.ending,
                    });
                }
            }
            assert_eq!(score.cadences, cadences, "{label}");
            let imitative: Vec<usize> = (0..section.parts.len())
                .filter(|&p| section.parts[p].opening == Opening::Imitative)
                .collect();
            let points = score.imitation.as_deref().map_or(&[][..], |i| &i.points);
            assert_eq!(points.len(), imitative.len(), "{label}");
            for (point, &p) in points.iter().zip(&imitative) {
                let (_, at) = places[p];
                let start = section.parts[p].start.div_ceil(8) * 8 + at as u32 * 8;
                let voices: Vec<usize> = point.entries.iter().map(|e| e.voice).collect();
                assert_eq!(voices, section.voices, "{label}");
                for entry in &point.entries {
                    assert_eq!(entry.start, start, "{label}");
                    let rest = if p == 0 { 0 } else { start - at as u32 * 8 };
                    assert_eq!(entry.rest, rest, "{label}");
                }
                let before: usize = section.parts[..p].iter().map(|q| q.syllables).sum();
                assert_eq!(point.words, before as u16..before as u16 + 1, "{label}");
            }
        }
    }
    // The Gloria: six parts, seams and the Break at Qui tollis, three of them imitative.
    let gloria = section_score(&mass, 0, 0).unwrap();
    let kinds: Vec<(SectionKind, Option<&str>)> = gloria
        .sections
        .iter()
        .map(|mark| (mark.kind, mark.title.as_deref()))
        .collect();
    assert_eq!(
        kinds,
        [
            (SectionKind::Seam, None),
            (SectionKind::Seam, None),
            (SectionKind::Break, Some("Qui tollis")),
            (SectionKind::Seam, None),
            (SectionKind::Seam, None),
        ]
    );
    assert_eq!(gloria.imitation.as_ref().unwrap().points.len(), 3);
    // The Benedictus in three voices, the bass resting; Agnus Dei I without the second tenor.
    let benedictus = section_score(&mass, 1, 0).unwrap();
    assert_eq!(
        benedictus.text.as_ref().unwrap().voices,
        [true, true, true, false]
    );
    let agnus = section_score(&mass, 2, 0).unwrap();
    assert_eq!(
        agnus.text.as_ref().unwrap().voices,
        [true, true, true, false, true]
    );
    // A section with a part not set is none; the others still join.
    let mut partial = mass.clone();
    partial.results[1] = None;
    assert!(section_score(&partial, 0, 0).is_none());
    assert!(section_score(&partial, 1, 0).is_some());
    // The Osanna ut supra of the form normal repeats the Osanna of the Sanctus.
    let normal = handmade_mass(
        &MassRequest {
            preset: Preset::Normal,
            movements: vec![Movement::Sanctus, Movement::Benedictus],
            ..MassRequest::default()
        },
        &profile,
    );
    let osanna = section_score(&normal, 0, 2).unwrap();
    let repeated = section_score(&normal, 1, 1).unwrap();
    assert_eq!(repeated.title.as_deref(), Some("Osanna ut supra"));
    assert_eq!(
        Score {
            title: osanna.title.clone(),
            ..repeated
        },
        osanna
    );
}

/// The masses the tests of whole movements join, made by hand: the Missa brevis for S A T B
/// (its Gloria with seams and the Break at Qui tollis, the Benedictus without the bass, Agnus Dei
/// I without the second tenor); the form normal for five voices, whose Benedictus ends with the
/// Osanna of the Sanctus ut supra; and three voices without imitation.
fn joined_masses(profile: &Profile) -> Vec<Mass> {
    [
        MassRequest::default(),
        MassRequest {
            preset: Preset::Normal,
            choir: choir(5),
            seed: 77,
            ..MassRequest::default()
        },
        MassRequest {
            choir: choir(3),
            mode: Some(mode("phrygian")),
            overrides: parse_imitation("aus").unwrap(),
            ..MassRequest::default()
        },
    ]
    .iter()
    .map(|request| handmade_mass(request, profile))
    .collect()
}

/// The notes of `voice` that begin in the measures `measures` (8 ticks each), without their
/// lyrics.
fn notes_within(voice: &Voice, measures: std::ops::Range<usize>) -> Vec<Note> {
    let ticks = measures.start as u32 * 8..measures.end as u32 * 8;
    voice
        .timed()
        .filter(|(tick, _)| ticks.contains(tick))
        .map(|(_, note)| Note {
            lyric: None,
            ..note.clone()
        })
        .collect()
}

/// `score`, whose measures (8 ticks each) no note crosses, with a measure's rest inserted in every
/// voice before its measure `measure` (from 0), or appended at its end; the marks of its
/// sections after it a measure later, as a notation program inserts a measure.
fn with_rest(score: &Score, measure: usize) -> Score {
    let mut longer = score.clone();
    let tick = measure as u32 * 8;
    for voice in &mut longer.voices {
        let at = voice
            .timed()
            .position(|(start, _)| start >= tick)
            .unwrap_or(voice.notes.len());
        voice.notes.insert(at, Note::rest(Duration::WHOLE));
    }
    for mark in &mut longer.sections {
        if mark.measure >= measure {
            mark.measure += 1;
        }
    }
    longer
}

/// `score`, whose measures (8 ticks each) no note crosses, without its measure `measure` (from
/// 0) in every voice; the marks of its sections after it a measure earlier.
fn without_measure(score: &Score, measure: usize) -> Score {
    let mut shorter = score.clone();
    let ticks = measure as u32 * 8..(measure as u32 + 1) * 8;
    for voice in &mut shorter.voices {
        voice.notes = voice
            .timed()
            .filter(|(start, _)| !ticks.contains(start))
            .map(|(_, note)| note.clone())
            .collect();
    }
    for mark in &mut shorter.sections {
        if mark.measure > measure {
            mark.measure -= 1;
        }
    }
    shorter
}

/// A movement is its sections one after the other (§ 2.2, `movement_score`): each section's
/// score as `section_score` joins it at its place, in every voice of the movement, a voice that
/// does not sing in a section resting through it, so that the movement has as many measures as
/// its parts and the Osanna it repeats; a mark where each section begins, with its title, the
/// first at measure 0, and the marks of its Breaks and seams; one text of all its parts' texts,
/// every lyric counted in it; the internal cadences of its parts and the endings of all but the
/// last; one plan of the imitative parts. It validates, and its title is the movement's. An
/// Osanna ut supra sounds the Osanna of the Sanctus; a movement with a part not set is none.
#[test]
fn movement_score_validates() {
    let profile = fux_core::mass::profile(&Profile::fux());
    for mass in joined_masses(&profile) {
        let plan = &mass.plan;
        for (m, movement) in plan.movements.iter().enumerate() {
            let label = format!("{} {}", plan.preset.id(), movement.movement.id());
            let score = movement_score(&mass, m).unwrap();
            score.validate().unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!(
                score.title.as_deref(),
                Some(movement.movement.title()),
                "{label}"
            );
            assert_eq!(score.voices.len(), movement.voices.len(), "{label}");
            for (v, voice) in score.voices.iter().enumerate() {
                assert_eq!(
                    (&voice.name, voice.voice_type),
                    (&movement.names[v], movement.voices[v]),
                    "{label}"
                );
            }
            let sung = score.text.as_ref().unwrap();
            let mut marks: Vec<SectionMark> = Vec::new();
            let mut texts: Vec<String> = Vec::new();
            let mut cadences: Vec<Cadence> = Vec::new();
            let (mut at, mut syllables, mut points) = (0, 0, 0);
            // The lyrics of each voice, section by section, as the movement counts them.
            let mut lyrics: Vec<Vec<usize>> = vec![Vec::new(); score.voices.len()];
            for (s, section) in movement.sections.iter().enumerate() {
                let joined = section_score(&mass, m, s).unwrap();
                let length = joined.measure_count();
                let own = joined.text.as_ref().unwrap();
                for (v, voice) in score.voices.iter().enumerate() {
                    assert_eq!(
                        notes_within(voice, at..at + length),
                        notes_within(&joined.voices[v], 0..length),
                        "{label}, {}, voice {v}",
                        section.id
                    );
                    if own.voices[v] {
                        lyrics[v].extend(syllables..syllables + own.text.syllables.len());
                    } else {
                        assert!(
                            notes_within(voice, at..at + length)
                                .iter()
                                .all(Note::is_rest),
                            "{label}, {}: voice {v} is tacet",
                            section.id
                        );
                    }
                }
                marks.push(SectionMark {
                    measure: at,
                    kind: SectionKind::Section,
                    title: Some(section.title.to_string()),
                });
                marks.extend(joined.sections.iter().map(|mark| SectionMark {
                    measure: mark.measure + at,
                    ..mark.clone()
                }));
                texts.push(own.text.id.clone());
                cadences.extend(joined.cadences.iter().map(|cadence| Cadence {
                    measure: cadence.measure + at,
                    ..*cadence
                }));
                // The section's ending is an internal cadence of the movement, but for the last.
                let (from, repeated) = section.ut_supra.unwrap_or((m, s));
                let ending = plan.movements[from].sections[repeated]
                    .parts
                    .last()
                    .unwrap()
                    .ending;
                if s + 1 < movement.sections.len() {
                    cadences.push(Cadence {
                        measure: at + length - 1,
                        target: ending,
                    });
                }
                points += joined.imitation.as_ref().map_or(0, |i| i.points.len());
                at += length;
                syllables += own.text.syllables.len();
            }
            assert_eq!(score.measure_count(), at, "{label}");
            let parts: usize = movement
                .sections
                .iter()
                .enumerate()
                .flat_map(|(s, section)| {
                    let (from, repeated) = section.ut_supra.unwrap_or((m, s));
                    let count = plan.movements[from].sections[repeated].parts.len();
                    (0..count).map(move |part| PartRef {
                        movement: from,
                        section: repeated,
                        part,
                    })
                })
                .map(|at| mass.result(at).unwrap().generated.score.measure_count())
                .sum();
            assert_eq!(at, parts, "{label}: the measures of its parts");
            assert_eq!(score.sections, marks, "{label}");
            assert_eq!(sung.text.id, texts.join("+"), "{label}");
            assert_eq!(sung.text.syllables.len(), syllables, "{label}");
            assert!(!sung.text.continued, "{label}");
            for (v, voice) in score.voices.iter().enumerate() {
                let found: Vec<usize> = voice
                    .notes
                    .iter()
                    .filter_map(|note| note.lyric?.index())
                    .collect();
                assert_eq!(found, lyrics[v], "{label}, voice {v}");
                assert_eq!(sung.voices[v], !lyrics[v].is_empty(), "{label}, voice {v}");
            }
            assert_eq!(score.cadences, cadences, "{label}");
            assert_eq!(
                score.imitation.as_ref().map_or(0, |i| i.points.len()),
                points,
                "{label}"
            );
        }
    }
    let brevis = &joined_masses(&profile)[0];
    // The Benedictus without the bass, then the Osanna in all voices; Agnus Dei I without the
    // second tenor.
    let benedictus = movement_score(brevis, 4).unwrap();
    let kinds: Vec<(usize, Option<&str>)> = benedictus
        .sections
        .iter()
        .map(|mark| (mark.measure, mark.title.as_deref()))
        .collect();
    let osanna = section_score(brevis, 4, 0).unwrap().measure_count();
    assert_eq!(kinds, [(0, Some("Benedictus")), (osanna, Some("Osanna"))]);
    let bass = benedictus.voices.last().unwrap();
    assert!(notes_within(bass, 0..osanna).iter().all(Note::is_rest));
    assert!(
        !notes_within(bass, osanna..benedictus.measure_count())
            .iter()
            .all(Note::is_rest)
    );
    // The Gloria: one section with seams and the Break at Qui tollis.
    let gloria = movement_score(brevis, 1).unwrap();
    let kinds: Vec<(SectionKind, Option<&str>)> = gloria
        .sections
        .iter()
        .map(|mark| (mark.kind, mark.title.as_deref()))
        .collect();
    assert_eq!(
        kinds,
        [
            (SectionKind::Section, Some("Et in terra pax")),
            (SectionKind::Seam, None),
            (SectionKind::Seam, None),
            (SectionKind::Break, Some("Qui tollis")),
            (SectionKind::Seam, None),
            (SectionKind::Seam, None),
        ]
    );
    // The Osanna ut supra of the form normal sounds the Osanna of the Sanctus.
    let normal = &joined_masses(&profile)[1];
    let benedictus = movement_score(normal, 4).unwrap();
    let repeated = section_score(normal, 3, 2).unwrap();
    let from = benedictus.sections[1].measure;
    assert_eq!(
        benedictus.sections[1].title.as_deref(),
        Some("Osanna ut supra")
    );
    assert_eq!(benedictus.measure_count() - from, repeated.measure_count());
    for (voice, original) in benedictus.voices.iter().zip(&repeated.voices) {
        assert_eq!(
            notes_within(voice, from..benedictus.measure_count()),
            notes_within(original, 0..repeated.measure_count())
        );
    }
    // A movement with a part not set is none; the others still join.
    let mut partial = brevis.clone();
    let at = brevis.plan.find("credo/5").unwrap()[0];
    let i = brevis.plan.parts().position(|(p, _)| p == at).unwrap();
    partial.results[i] = None;
    assert!(movement_score(&partial, 2).is_none());
    assert!(movement_score(&partial, 1).is_some());
    assert!(movement_score(brevis, 6).is_none());
}

/// The parts of a movement come back from its score and the fields of its file (`split_movement`,
/// D14), each as it was set: its voices with their notes and lyrics, its text going on where it
/// went on, its internal cadences and its plan of imitation, so that the check gives its cost;
/// with its place, the movement by its place in the mass, and the measures before it. A movement
/// as a file gives it back, without its text, gives the parts' notes, cadences and plans, their
/// syllables to be aligned. The fields of each part are those of its file, and the marks of the
/// parts those of the movement. Fields that do not fit say why in German and English, also where
/// the parts no longer cover the score: a gap between two parts, a measure inserted or appended,
/// the field of the last part missing, a measure inserted or deleted before an Osanna ut supra,
/// whose measures alone belong to no part.
#[test]
fn split_returns_the_parts() {
    let profile = fux_core::mass::profile(&Profile::fux());
    let checker = Checker::new(&profile);
    for mass in joined_masses(&profile) {
        let plan = &mass.plan;
        for (m, movement) in plan.movements.iter().enumerate() {
            let score = movement_score(&mass, m).unwrap();
            let fields = part_fields(&mass, m);
            let places: Vec<PartRef> = plan
                .parts()
                .filter(|(at, _)| at.movement == m)
                .map(|(at, _)| at)
                .collect();
            let split = split_movement(&score, &fields).unwrap();
            assert_eq!(split.len(), places.len());
            let mut before = 0;
            for (n, ((at, part, offset), place)) in split.iter().zip(&places).enumerate() {
                let label = plan.address(*place);
                let result = mass.result(*place).unwrap();
                let own = &result.generated.score;
                assert_eq!(
                    *at,
                    PartRef {
                        movement: movement.movement as usize,
                        ..*place
                    },
                    "{label}"
                );
                assert_eq!(*offset, before, "{label}");
                assert_eq!(part.voices.len(), own.voices.len(), "{label}");
                for ((voice, original), &v) in
                    part.voices.iter().zip(&own.voices).zip(&result.voices)
                {
                    assert_eq!(voice.name, movement.names[v], "{label}");
                    assert_eq!(voice.voice_type, original.voice_type, "{label}");
                    assert_eq!(voice.notes, original.notes, "{label}");
                }
                assert_eq!(part.meter, own.meter, "{label}");
                assert_eq!(part.mode, own.mode, "{label}");
                assert_eq!(part.text, own.text, "{label}");
                assert_eq!(part.cadences, own.cadences, "{label}");
                assert_eq!(part.imitation, own.imitation, "{label}");
                assert_eq!(checker.check(part).cost(), result.generated.cost, "{label}");
                // The fields of the part as its file has them, its measures those in the
                // movement.
                let kept = fields_of_part(&fields, n + 1);
                let value = |name: &str| {
                    kept.iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, value)| value.clone())
                };
                assert_eq!(kept[0], mass_field(plan), "{label}");
                let field: PartField = value(FIELD_PART).unwrap().parse().unwrap();
                let file: PartField = part_field(&mass, *place).unwrap().1.parse().unwrap();
                assert_eq!(
                    PartField {
                        bars: file.bars.clone(),
                        ..field.clone()
                    },
                    file,
                    "{label}"
                );
                assert_eq!(
                    *field.bars.start(),
                    before + 1,
                    "{label}: the measures in the movement"
                );
                assert_eq!(value("fux-text"), own.text.as_ref().map(|t| t.text.spec()));
                assert!(kept.iter().all(|(name, _)| !name.contains('.')), "{kept:?}");
                before += own.measure_count();
            }
            // As a file gives the movement back: without text and lyrics.
            let mut read = score.clone();
            read.text = None;
            for note in read.voices.iter_mut().flat_map(|v| v.notes.iter_mut()) {
                note.lyric = None;
            }
            let again = split_movement(&read, &fields).unwrap();
            for ((_, part, _), place) in again.iter().zip(&places) {
                let own = &mass.result(*place).unwrap().generated.score;
                assert_eq!(part.text, None);
                for (voice, original) in part.voices.iter().zip(&own.voices) {
                    let plain: Vec<Note> = original
                        .notes
                        .iter()
                        .map(|note| Note {
                            lyric: None,
                            ..note.clone()
                        })
                        .collect();
                    assert_eq!(voice.notes, plain);
                }
                assert_eq!(
                    (&part.cadences, &part.imitation),
                    (&own.cadences, &own.imitation)
                );
            }
            // The marks of the parts are those of the movement, but for an Osanna ut supra's.
            let parts = movement_parts(&fields).unwrap();
            let utsupra: Vec<usize> = movement
                .sections
                .iter()
                .zip(&score.sections)
                .filter(|(section, _)| section.ut_supra.is_some())
                .map(|(_, mark)| mark.measure)
                .collect();
            let marks: Vec<SectionMark> = score
                .sections
                .iter()
                .filter(|mark| {
                    !(mark.kind == SectionKind::Section && utsupra.contains(&mark.measure))
                })
                .cloned()
                .collect();
            assert_eq!(movement_sections(&parts), marks);
        }
    }
    // Fields that do not fit, on the Gloria and the Benedictus of the Missa brevis.
    let brevis = &joined_masses(&profile)[0];
    let gloria = movement_score(brevis, 1).unwrap();
    let fields = part_fields(brevis, 1);
    let changed = |name: &str, change: &dyn Fn(&str) -> String| -> Vec<(String, String)> {
        fields
            .iter()
            .map(|(n, value)| {
                let value = if n == name {
                    change(value)
                } else {
                    value.clone()
                };
                (n.clone(), value)
            })
            .collect()
    };
    assert_eq!(split_movement(&gloria, &[]), Ok(Vec::new()));
    assert_eq!(
        split_movement(&gloria, &[mass_field(&brevis.plan)]),
        Ok(Vec::new())
    );
    let gap: Vec<(String, String)> = fields
        .iter()
        .filter(|(name, _)| name != "fux-part.2")
        .cloned()
        .collect();
    let error = split_movement(&gloria, &gap).unwrap_err();
    assert_eq!(error, FieldError::PartMissing { number: 2 });
    assert_eq!(
        error.message(Lang::De),
        "Der Datei fehlt das Feld fux-part.2"
    );
    assert_eq!(
        error.message(Lang::En),
        "the file lacks the field fux-part.2"
    );
    let second: PartField = fields
        .iter()
        .find(|(name, _)| name == "fux-part.2")
        .unwrap()
        .1
        .parse()
        .unwrap();
    let (first, last) = (*second.bars.start(), *second.bars.end());
    let overlapping = changed("fux-part.2", &|value: &str| {
        value.replacen(
            &format!("bars={first}-"),
            &format!("bars={}-", first - 1),
            1,
        )
    });
    let error = split_movement(&gloria, &overlapping).unwrap_err();
    let measures = gloria.measure_count();
    assert_eq!(
        error,
        FieldError::Bars {
            part: "gloria/2".into(),
            first: first - 1,
            last,
            measures
        }
    );
    assert_eq!(
        error.message(Lang::De),
        format!(
            "Teil gloria/2: Die Takte {}–{last} liegen nicht in den {measures} Takten der Datei \
             nach denen des Teils davor",
            first - 1
        )
    );
    assert_eq!(
        error.message(Lang::En),
        format!(
            "part gloria/2: measures {}–{last} do not lie within the file's {measures} measures \
             after those of the part before",
            first - 1
        )
    );
    let mut shorter = gloria.clone();
    for voice in &mut shorter.voices {
        voice.notes.truncate(voice.notes.len() - 1);
    }
    assert!(matches!(
        split_movement(&shorter, &fields),
        Err(FieldError::Bars { part, .. }) if part == "gloria/6"
    ));
    // A gap between two parts: the measures between them belong to none.
    let gap = changed("fux-part.2", &|value: &str| {
        value.replacen(
            &format!("bars={first}-"),
            &format!("bars={}-", first + 1),
            1,
        )
    });
    assert_eq!(
        split_movement(&gloria, &gap),
        Err(FieldError::Bars {
            part: "gloria/2".into(),
            first: first + 1,
            last,
            measures
        })
    );
    // A longer score, a measure's rest in every voice inserted in the last part or appended:
    // the parts end before the file does.
    let unclaimed = FieldError::Unclaimed {
        part: "gloria/6".into(),
        last: measures,
        measures: measures + 1,
        ut_supra: None,
    };
    for measure in [measures - 1, measures] {
        let longer = with_rest(&gloria, measure);
        assert_eq!(longer.measure_count(), measures + 1);
        assert_eq!(
            split_movement(&longer, &fields),
            Err(unclaimed.clone()),
            "{measure}"
        );
    }
    assert_eq!(
        unclaimed.message(Lang::De),
        format!(
            "Teil gloria/6: Die Takte der Teile enden mit Takt {measures}, die der Datei mit Takt {}",
            measures + 1
        )
    );
    assert_eq!(
        unclaimed.message(Lang::En),
        format!(
            "part gloria/6: the parts' measures end with measure {measures}, the file's with \
             measure {}",
            measures + 1
        )
    );
    // The field of the last part missing: its measures belong to no part.
    let fifth: PartField = fields
        .iter()
        .find(|(name, _)| name == "fux-part.5")
        .unwrap()
        .1
        .parse()
        .unwrap();
    let without_last: Vec<(String, String)> = fields
        .iter()
        .filter(|(name, _)| name != "fux-part.6")
        .cloned()
        .collect();
    assert_eq!(
        split_movement(&gloria, &without_last),
        Err(FieldError::Unclaimed {
            part: "gloria/5".into(),
            last: *fifth.bars.end(),
            measures,
            ut_supra: None,
        })
    );
    let cadences = changed("fux-cadences.1", &|_| "4:X".to_string());
    let error = split_movement(&gloria, &cadences).unwrap_err();
    assert_eq!(
        error,
        FieldError::Unreadable {
            field: "fux-cadences.1".into(),
            value: "4:X".into()
        }
    );
    assert_eq!(
        error.message(Lang::De),
        "Feld fux-cadences.1: „4:X“ lässt sich nicht lesen"
    );
    assert_eq!(
        error.message(Lang::En),
        "field fux-cadences.1: \"4:X\" cannot be read"
    );
    let own = changed("fux-text.1", &|_| "Ad te le-vá-vi.".to_string());
    assert!(matches!(
        split_movement(&gloria, &own),
        Err(FieldError::Unreadable { field, .. }) if field == "fux-text.1"
    ));
    let other = changed("fux-text.2", &|_| "gloria:1-5".to_string());
    assert!(matches!(
        split_movement(&gloria, &other),
        Err(FieldError::Text { part, planned, .. }) if part == "gloria/2" && planned == "gloria:1-5"
    ));
    let imitation = changed("fux-imitation.1", &|_| "p0-1|0@0/0:X4.8".to_string());
    assert!(matches!(
        split_movement(&gloria, &imitation),
        Err(FieldError::Score { part, error: ScoreError::ImitationSyntax { .. } }) if part == "gloria/1"
    ));
    // The bass sounds in the Benedictus, which its field does not give it.
    let mut benedictus = movement_score(brevis, 4).unwrap();
    let bass = benedictus.voices.last_mut().unwrap();
    bass.notes[0] = Note::new("D3".parse().unwrap(), Duration::WHOLE);
    assert_eq!(
        split_movement(&benedictus, &part_fields(brevis, 4)).unwrap_err(),
        FieldError::Voices {
            part: "benedictus/1".into()
        }
    );
    // The Benedictus of the Missa brevis without the field of its Osanna: its notation marks the
    // section there, but the form has no Osanna ut supra.
    let benedictus = movement_score(brevis, 4).unwrap();
    let fields = part_fields(brevis, 4);
    let parts = movement_parts(&fields).unwrap();
    assert_eq!(parts.len(), 2);
    let osanna = benedictus.sections.last().unwrap();
    assert_eq!(
        (osanna.measure, osanna.title.as_deref()),
        (*parts[0].bars.end(), Some("Osanna"))
    );
    let without_osanna: Vec<(String, String)> = fields
        .iter()
        .filter(|(name, _)| name != "fux-part.2")
        .cloned()
        .collect();
    assert_eq!(
        split_movement(&benedictus, &without_osanna),
        Err(FieldError::Unclaimed {
            part: "benedictus/1".into(),
            last: *parts[0].bars.end(),
            measures: benedictus.measure_count(),
            ut_supra: None,
        })
    );
    // The Benedictus of the form normal ends with the Osanna ut supra, whose measures belong to
    // no part, as long as its section begins right after the parts, also where the file names no
    // mass; with a measure deleted or inserted before it, it does not. For five voices a measure
    // deleted brings the bass of the Osanna into the measures of the reduced Benedictus; for
    // three, which all sing the Benedictus, the parts end before the Osanna ut supra begins.
    let benedictus_of = |mass: &Mass| {
        let m = mass
            .plan
            .movements
            .iter()
            .position(|movement| movement.movement == Movement::Benedictus)
            .unwrap();
        (movement_score(mass, m).unwrap(), part_fields(mass, m))
    };
    let (benedictus, fields) = benedictus_of(&joined_masses(&profile)[1]);
    assert_eq!(
        split_movement(&without_measure(&benedictus, 1), &fields),
        Err(FieldError::Voices {
            part: "benedictus/1".into()
        })
    );
    let three = handmade_mass(
        &MassRequest {
            preset: Preset::Normal,
            choir: choir(3),
            movements: vec![Movement::Sanctus, Movement::Benedictus],
            ..MassRequest::default()
        },
        &profile,
    );
    let (benedictus, fields) = benedictus_of(&three);
    assert_eq!(benedictus.voices.len(), 3);
    let parts = movement_parts(&fields).unwrap();
    assert!(parts.iter().all(|part| part.voices == [0, 1, 2]));
    let last = *parts.last().unwrap().bars.end();
    let total = benedictus.measure_count();
    assert!(last < total);
    assert_eq!(
        split_movement(&benedictus, &fields).unwrap().len(),
        parts.len()
    );
    let unnamed: Vec<(String, String)> = fields
        .iter()
        .filter(|(name, _)| name != "fux-mass")
        .cloned()
        .collect();
    assert_eq!(
        split_movement(&benedictus, &unnamed).unwrap().len(),
        parts.len()
    );
    let address = format!("benedictus/{}", parts.len());
    for (edited, measures) in [
        (without_measure(&benedictus, 1), total - 1),
        (with_rest(&benedictus, 1), total + 1),
    ] {
        let error = split_movement(&edited, &fields).unwrap_err();
        assert_eq!(
            error,
            FieldError::Unclaimed {
                part: address.clone(),
                last,
                measures,
                ut_supra: Some("Osanna ut supra".into()),
            }
        );
        assert_eq!(
            error.message(Lang::De),
            format!(
                "Teil {address}: Die Takte der Teile enden mit Takt {last}, und nach ihm beginnt \
                 kein Osanna ut supra"
            )
        );
        assert_eq!(
            error.message(Lang::En),
            format!(
                "part {address}: the parts' measures end with measure {last}, and no Osanna ut \
                 supra begins after it"
            )
        );
    }
    // The fields of a part: the numbered ones of the part without their number, and those of
    // the whole file.
    let fields = vec![
        ("fux-mode".to_string(), "dorian".to_string()),
        ("fux-mass".to_string(), "seed=1".to_string()),
        ("fux-part.1".to_string(), "bars=1-3".to_string()),
        ("fux-text.1".to_string(), "kyrie".to_string()),
        ("fux-part.2".to_string(), "bars=4-6".to_string()),
        ("fux-lyrics.2".to_string(), "0,1".to_string()),
        ("fux-x.y".to_string(), "z".to_string()),
    ];
    assert_eq!(
        fields_of_part(&fields, 2),
        [
            ("fux-mode", "dorian"),
            ("fux-mass", "seed=1"),
            ("fux-part", "bars=4-6"),
            ("fux-lyrics", "0,1"),
            ("fux-x.y", "z"),
        ]
        .map(|(name, value)| (name.to_string(), value.to_string()))
    );
}

/// The fields of the files of a mass read back as they were written: the mass with its form,
/// mode, choir, principal degree and seed; the part with its measures, voices, join, title,
/// ending, the ending before it, start, opening, text going on, seed, attempt, points, profile
/// and the version of the generator that set it. A part's field fits its plan, also after a
/// fallback set by the attempts of the fallback, but not with another seed, profile or version of
/// the generator, not with the points of the fallback from an attempt as planned, and not where
/// the part before it ends otherwise. A field written before the version was kept was set by
/// version 1. A movement's fields number its parts, their measures one after the other. Fields
/// that miss an item or cannot be read say so in German and English.
#[test]
fn fields_round_trip() {
    let profile = fux_core::mass::profile(&Profile::fux());
    for request in [
        MassRequest::default(),
        MassRequest {
            preset: Preset::Normal,
            mode: Some(mode("ionian-transposed")),
            choir: choir(5),
            overrides: parse_imitation("aus; christe=an").unwrap(),
            seed: 77,
            ..MassRequest::default()
        },
        MassRequest {
            mode: Some(mode("phrygian")),
            choir: vec![VoiceType::MezzoSoprano, VoiceType::Baritone],
            ..MassRequest::default()
        },
    ] {
        let mass = handmade_mass(&request, &profile);
        let plan = &mass.plan;
        let (name, value) = mass_field(plan);
        assert_eq!(name, FIELD_MASS);
        let field: MassField = value.parse().unwrap();
        assert_eq!(field, MassField::of(plan));
        assert_eq!(
            (field.preset, field.seed, &field.choir, field.principal),
            (request.preset, request.seed, &plan.choir, plan.principal)
        );
        for (at, part) in plan.parts() {
            let label = plan.address(at);
            let result = mass.result(at).unwrap();
            let (name, value) = part_field(&mass, at).unwrap();
            assert_eq!(name, FIELD_PART);
            let field: PartField = value.parse().unwrap();
            assert_eq!(
                field,
                PartField::of(plan, at, result, 1),
                "{label}: {value}"
            );
            assert_eq!(field.to_string(), value);
            let measures = result.generated.score.measure_count();
            assert_eq!(field.bars, 1..=measures, "{label}");
            assert_eq!(field.chord, part.chord, "{label}");
            assert_eq!(
                (field.ending, field.start, field.continued, field.points),
                (part.ending, part.start, part.continued, part.points),
                "{label}"
            );
            assert_eq!(field.before, plan.ending_before(at), "{label}");
            assert_eq!(field.generator, GENERATOR, "{label}");
            assert!(
                value.ends_with(&format!(";generator={GENERATOR}")),
                "{label}: {value}"
            );
            assert!(field.fits(plan, at, result.profile), "{label}");
            let other = PartField {
                attempt: 1,
                ..field.clone()
            };
            assert!(!other.fits(plan, at, result.profile), "{label}");
            assert!(!field.fits(plan, at, result.profile ^ 1), "{label}");
            // Another version of the generator may set other notes from the same plan and seed.
            let newer = PartField {
                generator: GENERATOR + 1,
                ..field.clone()
            };
            assert!(!newer.fits(plan, at, result.profile), "{label}");
            // The chain of internal cadences starts from the ending before the part.
            let elsewhere = PartField {
                before: Some(match field.before {
                    Some(Step::D) => Step::A,
                    _ => Step::D,
                }),
                ..field.clone()
            };
            assert!(!elsewhere.fits(plan, at, result.profile), "{label}");
            if part.points.is_some() {
                // Natural points fit as the fallback of the lengths sets them, from attempt 3,
                // not from an attempt as planned (a mass set with `--lengths natürlich`).
                let natural = PartField {
                    points: None,
                    ..field.clone()
                };
                assert!(!natural.fits(plan, at, result.profile), "{label}");
                let fallback = attempted(plan, result.clone(), 3);
                let natural = PartField {
                    points: None,
                    ..PartField::of(plan, at, &fallback, 1)
                };
                assert!(natural.fits(plan, at, result.profile), "{label}");
                let planned = PartField {
                    points: part.points,
                    ..natural.clone()
                };
                assert!(!planned.fits(plan, at, result.profile), "{label}");
                let beyond = PartField::of(plan, at, &attempted(plan, fallback, 6), 1);
                assert!(!beyond.fits(plan, at, result.profile), "{label}");
            }
        }
        for m in 0..plan.movements.len() {
            let fields = part_fields(&mass, m);
            assert_eq!(fields[0], mass_field(plan));
            let mut next = 1;
            let parts = plan.movements[m].part_count();
            for n in 1..=parts {
                let value = &fields
                    .iter()
                    .find(|(name, _)| *name == format!("fux-part.{n}"))
                    .unwrap()
                    .1;
                let field: PartField = value.parse().unwrap();
                assert_eq!(*field.bars.start(), next);
                next = field.bars.end() + 1;
                assert!(
                    fields
                        .iter()
                        .any(|(name, _)| *name == format!("fux-text.{n}"))
                );
            }
            assert!(fields.iter().all(|(name, _)| {
                name == FIELD_MASS
                    || name
                        .rsplit_once('.')
                        .is_some_and(|(_, n)| n.parse::<usize>().is_ok_and(|n| n <= parts))
            }));
        }
    }
    // Agnus Dei II set in the choir of the mass by its fallback fits, with the chord of that
    // choir; in those voices from an attempt as planned it does not.
    let tutti = MassRequest {
        overrides: parse_imitation("agnus3=aus").unwrap(),
        ..MassRequest::default()
    };
    let plan = plan_mass(&tutti).unwrap();
    let at = plan.find("agnus3").unwrap()[0];
    let planned = handmade(&plan, at, &[0, 1, 2, 4], false, &profile);
    assert!(!PartField::of(&plan, at, &planned, 1).fits(&plan, at, planned.profile));
    let base = attempted(&plan, planned, 3);
    let field = PartField::of(&plan, at, &base, 1);
    assert_eq!(field.voices, [0, 1, 2, 4]);
    assert_eq!(
        field.chord.as_ref().map(Vec::len),
        Some(4),
        "{:?}",
        field.chord
    );
    assert!(field.fits(&plan, at, base.profile));
    let other = PartField {
        voices: vec![0, 1, 3, 4],
        ..field.clone()
    };
    assert!(!other.fits(&plan, at, base.profile));
    // Fields that cannot be read.
    let missing = "bars=1-3".parse::<PartField>().unwrap_err();
    assert_eq!(
        missing,
        FieldError::Missing {
            field: FIELD_PART,
            key: "voices"
        }
    );
    assert_eq!(
        missing.message(Lang::De),
        "Dem Feld fux-part fehlt „voices“"
    );
    assert_eq!(
        missing.message(Lang::En),
        "the field fux-part lacks \"voices\""
    );
    let value = field.to_string().replace("open=tutti:", "open=tutti:X9,");
    let wrong = value.parse::<PartField>().unwrap_err();
    assert!(
        matches!(&wrong, FieldError::Value { key: "open", .. }),
        "{wrong:?}"
    );
    assert!(
        wrong
            .message(Lang::De)
            .starts_with("Feld fux-part: „open=tutti:X9,")
    );
    assert!(
        "preset=kurz;mode=dorian;choir=S,A;principal=A;seed=1"
            .parse::<MassField>()
            .is_err()
    );
    // A field written without the ending before its part keeps none: it fits the first part of
    // a movement only. One that cannot be read fails.
    let first = plan.find("kyrie/1").unwrap()[0];
    let kyrie = handmade(&plan, first, &[0, 1, 2, 3], true, &profile);
    let written = PartField::of(&plan, first, &kyrie, 1).to_string();
    assert!(written.contains(";before=;"), "{written}");
    let older: PartField = written.replace("before=;", "").parse().unwrap();
    assert_eq!(older.before, None);
    assert!(older.fits(&plan, first, kyrie.profile));
    let christe = plan.find("christe").unwrap()[0];
    let later = handmade(&plan, christe, &[0, 1, 2, 3], true, &profile);
    let written = PartField::of(&plan, christe, &later, 1).to_string();
    let ending = plan.part(first).ending.letter();
    assert!(written.contains(&format!(";before={ending};")), "{written}");
    let older: PartField = written
        .replace(&format!("before={ending};"), "")
        .parse()
        .unwrap();
    assert!(!older.fits(&plan, christe, later.profile));
    let unreadable = written.replace(&format!("before={ending}"), "before=X");
    assert!(matches!(
        unreadable.parse::<PartField>(),
        Err(FieldError::Value { key: "before", .. })
    ));
    // A field written before the version of the generator was kept was set by version 1, the
    // generator up to then; one whose version cannot be read fails.
    let item = format!(";generator={GENERATOR}");
    assert!(written.ends_with(&item), "{written}");
    let unversioned: PartField = written.replace(&item, "").parse().unwrap();
    assert_eq!(
        unversioned,
        PartField {
            generator: 1,
            ..written.parse().unwrap()
        }
    );
    assert_eq!(
        unversioned.fits(&plan, christe, later.profile),
        GENERATOR == 1
    );
    for bad in ["generator=0", "generator=x", "generator="] {
        let unreadable = written.replace(&item, &format!(";{bad}"));
        assert!(
            matches!(
                unreadable.parse::<PartField>(),
                Err(FieldError::Value {
                    key: "generator",
                    ..
                })
            ),
            "{bad}"
        );
    }
}

/// The chant and the head motif reach the fields of a mass (phase 14c): `chant=ix` and
/// `motif=1` in `fux-mass`, the motif of Kyrie I and of the parts that take it in `fux-part`,
/// before `profile=`; a mass without either keeps the fields of phases 14a and 14b, which read
/// back without them. A file of a mass on chant or with a head motif is no file of a mass
/// without, though its mode is given and its draws are the same.
#[test]
fn motif_and_chant_reach_the_fields() {
    let plain = plan_mass(&MassRequest::default()).unwrap();
    let field = MassField::of(&plain);
    assert_eq!(
        field.to_string(),
        format!(
            "preset=brevis;mode=dorian;choir=S,A,T,B;principal={};seed=1",
            plain.principal.letter()
        )
    );
    assert_eq!((field.chant, field.motif), (None, false));
    let chanted = plan_mass(&MassRequest {
        chant: Some(ChantMass::IX),
        ..MassRequest::default()
    })
    .unwrap();
    let field = MassField::of(&chanted);
    let written = field.to_string();
    assert!(
        written.starts_with("preset=brevis;mode=kyriale-ix;") && written.ends_with(";chant=ix"),
        "{written}"
    );
    assert_eq!(written.parse::<MassField>(), Ok(field));
    let given = |request: MassRequest| MassField::of(&plan_mass(&request).unwrap());
    let dorian = Some(mode("dorian"));
    let on_chant = given(MassRequest {
        chant: Some(ChantMass::IV),
        mode: dorian,
        ..MassRequest::default()
    });
    let without = given(MassRequest {
        mode: dorian,
        ..MassRequest::default()
    });
    assert_ne!(on_chant, without);
    assert_eq!(
        (on_chant.chant, &on_chant.mode, without.chant),
        (Some(ChantMass::IV), &without.mode, None)
    );
    let motif = given(MassRequest {
        motif: true,
        ..MassRequest::default()
    });
    assert!(motif.motif && motif.to_string().ends_with(";seed=1;motif=1"));
    assert_eq!(motif.to_string().parse::<MassField>(), Ok(motif.clone()));
    // The head motif is the mass's, whichever of its movements a run sets: the Kyrie and the
    // Benedictus, which take none, or Agnus Dei I begun together, name the mass with it.
    for (movements, overrides) in [
        (vec![Movement::Kyrie, Movement::Benedictus], ""),
        (vec![Movement::Kyrie, Movement::Agnus], "agnus/1=aus"),
    ] {
        let request = MassRequest {
            motif: true,
            movements,
            overrides: parse_imitation(overrides).unwrap(),
            ..MassRequest::default()
        };
        let plan = plan_mass(&request).unwrap();
        assert!(plan.motif && plan.parts().all(|(_, part)| !part.motif));
        assert_eq!(
            plan.motif_source(),
            plan.find("kyrie/1").ok().map(|at| at[0])
        );
        assert_eq!(MassField::of(&plan), motif);
    }
    for bad in ["chant=xi", "motif=2"] {
        let value = format!("{};{bad}", without);
        assert!(
            matches!(value.parse::<MassField>(), Err(FieldError::Value { .. })),
            "{bad}"
        );
    }
    // The motif of a part.
    let profile = fux_core::mass::profile(&Profile::fux());
    let mass = handmade_mass(&MassRequest::default(), &profile);
    let (_, value) = part_field(&mass, plain.find("kyrie/1").unwrap()[0]).unwrap();
    assert!(!value.contains("motif"), "{value}");
    let field: PartField = value.parse().unwrap();
    assert_eq!(field.motif, None);
    let head: Motif = "4,7,9,10,8".parse().unwrap();
    let with = PartField {
        motif: Some(head.clone()),
        ..field.clone()
    };
    let written = with.to_string();
    assert!(
        written.contains(";points=;motif=4,7,9,10,8;profile="),
        "{written}"
    );
    assert_eq!(written.parse::<PartField>(), Ok(with.clone()));
    let empty = value.replace(";profile=", ";motif=;profile=");
    assert_eq!(empty.parse::<PartField>(), Ok(field.clone()));
    let unreadable = value.replace(";profile=", ";motif=0,2,3;profile=");
    assert!(matches!(
        unreadable.parse::<PartField>(),
        Err(FieldError::Value { key: "motif", .. })
    ));
    // The points that took their subjects from the words, after the points.
    assert!(field.fell.is_empty());
    for (fell, item) in [(vec![1], "fell=1"), (vec![3, 5], "fell=3,5")] {
        let fallen = PartField {
            fell,
            ..with.clone()
        };
        let written = fallen.to_string();
        assert!(
            written.contains(&format!(";points=;{item};motif=4,7,9,10,8;profile=")),
            "{written}"
        );
        assert_eq!(written.parse::<PartField>(), Ok(fallen));
    }
    let empty = value.replace(";profile=", ";fell=;profile=");
    assert_eq!(empty.parse::<PartField>(), Ok(field));
    for bad in ["fell=0", "fell=2,x"] {
        let unreadable = value.replace(";profile=", &format!(";{bad};profile="));
        assert!(
            matches!(
                unreadable.parse::<PartField>(),
                Err(FieldError::Value { key: "fell", .. })
            ),
            "{bad}"
        );
    }
}

/// A part read back from its file is the part as set: its score with the plan's text going on
/// where it goes on, checked again under the profile of the parts, gives the report and the cost
/// the search gave; seed, attempt, voices and profile are those of its field, and a fallback
/// in the field is a warning again. A file with another text or other voices does not fit.
#[test]
fn parts_come_back_from_their_files() {
    let profile = fux_core::mass::profile(&Profile::fux());
    let request = MassRequest {
        movements: vec![Movement::Gloria, Movement::Agnus],
        ..MassRequest::default()
    };
    let profile = part_profile(&request, &profile);
    let mass = handmade_mass(&request, &profile);
    let plan = &mass.plan;
    // As a file gives it back: the text does not say that it goes on.
    let read = |result: &PartResult| -> Score {
        let mut score = result.generated.score.clone();
        let sung = score.text.as_mut().unwrap();
        sung.text = Arc::new(Text {
            continued: false,
            ..(*sung.text).clone()
        });
        score
    };
    for (at, part) in plan.parts() {
        let label = plan.address(at);
        let result = mass.result(at).unwrap();
        let field: PartField = part_field(&mass, at).unwrap().1.parse().unwrap();
        let back = part_from_parts(plan, at, read(result), &field, &profile).unwrap();
        assert_eq!(back.generated.score, result.generated.score, "{label}");
        assert_eq!(
            back.generated.score.text.as_ref().unwrap().text.continued,
            part.continued
        );
        assert_eq!(back.generated.report, result.generated.report, "{label}");
        assert_eq!(back.generated.cost, result.generated.cost, "{label}");
        assert_eq!(
            (back.at, back.seed, back.attempt, &back.voices, back.profile),
            (at, result.seed, 0, &result.voices, profile_hash(&profile)),
            "{label}"
        );
        assert!(back.warnings.is_empty(), "{label}");
    }
    // Agnus Dei I set with its natural points by its fallback, Agnus Dei II in the choir of the
    // mass.
    let agnus = plan.find("agnus/1").unwrap()[0];
    let result = attempted(plan, mass.result(agnus).unwrap().clone(), 3);
    let natural = PartField {
        points: None,
        ..PartField::of(plan, agnus, &result, 1)
    };
    let back = part_from_parts(plan, agnus, read(&result), &natural, &profile).unwrap();
    assert_eq!(
        back.warnings,
        [Warning::LengthFallback {
            part: "agnus/1".into()
        }]
    );
    let enlarged = plan.find("agnus3").unwrap()[0];
    let base = handmade(plan, enlarged, &[0, 1, 2, 4], false, &profile);
    let base = attempted(plan, base, 3);
    let field = PartField::of(plan, enlarged, &base, 1);
    let back = part_from_parts(plan, enlarged, read(&base), &field, &profile).unwrap();
    assert_eq!(
        back.warnings,
        [Warning::ScoringFallback {
            section: "agnus3".into()
        }]
    );
    // Another text, other voices.
    let first = plan.find("gloria/1").unwrap()[0];
    let result = mass.result(first).unwrap();
    let field: PartField = part_field(&mass, first).unwrap().1.parse().unwrap();
    let other = plan.find("gloria/2").unwrap()[0];
    let error = part_from_parts(plan, other, read(result), &field, &profile).unwrap_err();
    assert_eq!(
        error,
        FieldError::Text {
            part: "gloria/2".into(),
            planned: plan.part(other).text.clone(),
            found: plan.part(first).text.clone()
        }
    );
    assert_eq!(
        error.message(Lang::De),
        "Teil gloria/2: Die Datei singt „gloria:1-5“, der Plan „gloria:6-9“"
    );
    let fewer = PartField {
        voices: vec![0, 1, 2],
        ..field.clone()
    };
    assert_eq!(
        part_from_parts(plan, first, read(result), &fewer, &profile).unwrap_err(),
        FieldError::Voices {
            part: "gloria/1".into()
        }
    );
}

/// A part read back from its file warns as it warned when it was set (§ 4.4): the points whose
/// subjects came from the words instead of the chant (`fell=` of `fux-part`), and the first point
/// of a part set on the head motif, which found no plan with it; the last fallback of the motif,
/// which sets that point from the words itself, warns once, as its search did. The warnings come
/// in the order of the search: the fallback's, the plan's, the setting's. A part without such
/// points keeps no item. Parts made by hand, no search.
#[test]
fn setting_warnings_come_back_from_the_files() {
    let base = fux_core::mass::profile(&Profile::fux());
    // As a file gives a part back: the text does not say that it goes on.
    let read = |result: &PartResult| -> Score {
        let mut score = result.generated.score.clone();
        let sung = score.text.as_mut().unwrap();
        sung.text = Arc::new(Text {
            continued: false,
            ..(*sung.text).clone()
        });
        score
    };
    // A Sanctus on chant IV, which takes the chant of Mass IX and says so, two of its points from
    // the words.
    let request = MassRequest {
        movements: vec![Movement::Sanctus],
        chant: Some(ChantMass::IV),
        ..MassRequest::default()
    };
    let profile = part_profile(&request, &base);
    let mass = handmade_mass(&request, &profile);
    let plan = &mass.plan;
    let at = plan.find("sanctus/1").unwrap()[0];
    let plain = mass.result(at).unwrap();
    assert!(PartField::of(plan, at, plain, 1).fell.is_empty());
    let mut result = plain.clone();
    result.warnings = vec![
        Warning::ChantIV,
        Warning::ChantFallback(ChantFallback { point: 2 }),
        Warning::ChantFallback(ChantFallback { point: 4 }),
    ];
    let field = PartField::of(plan, at, &result, 1);
    assert_eq!(field.fell, [2, 4]);
    let written = field.to_string();
    assert!(written.contains(";points=;fell=2,4;profile="), "{written}");
    let field: PartField = written.parse().unwrap();
    let back = part_from_parts(plan, at, read(&result), &field, &profile).unwrap();
    assert_eq!(back.warnings, result.warnings);
    assert_eq!(
        back.warnings[1].message(Lang::De),
        "Punkt 2: kein brauchbarer Kopf aus dem Choral; Soggetto aus dem Text"
    );
    // Agnus Dei I on a head motif whose first point found no plan with it, as planned and in the
    // last fallback (attempt 3: Agnus Dei I has its natural points and its own voices).
    let request = motif_mass();
    let profile = part_profile(&request, &base);
    let mass = handmade_mass(&request, &profile);
    let plan = &mass.plan;
    let agnus = plan.find("agnus/1").unwrap()[0];
    let head: Motif = "4,7,9,10,8".parse().unwrap();
    let fallback = Warning::MotifFallback {
        movement: "Agnus Dei".into(),
    };
    let set = PartResult {
        motif: Some(head.clone()),
        warnings: vec![fallback.clone()],
        ..mass.result(agnus).unwrap().clone()
    };
    for (result, fell) in [
        (set.clone(), vec![1]),
        (attempted(plan, set, 3), Vec::new()),
    ] {
        let label = format!("attempt {}", result.attempt);
        let field = PartField::of(plan, agnus, &result, 1);
        assert_eq!(
            (&field.fell, field.motif.as_ref()),
            (&fell, Some(&head)),
            "{label}"
        );
        let field: PartField = field.to_string().parse().unwrap();
        let back = part_from_parts(plan, agnus, read(&result), &field, &profile).unwrap();
        assert_eq!(back.warnings, std::slice::from_ref(&fallback), "{label}");
        assert_eq!(back.motif.as_ref(), Some(&head), "{label}");
    }
}

/// A part read back from its file sings its text as the plan divides it: a Kyrie set with
/// eleison in four syllables does not fit a mass that divides it in three (`--eleison 3
/// --resume`), and the other way round, so that the part is set again rather than sung otherwise
/// than planned. Its syllables are printed as the mass prints them: with their acutes or, after
/// `--accents aus`, without.
#[test]
fn parts_come_back_as_the_plan_sings() {
    use fux_core::text::Eleison;
    let profile = fux_core::mass::profile(&Profile::fux());
    let request = MassRequest {
        movements: vec![Movement::Kyrie],
        ..MassRequest::default()
    };
    let profile = part_profile(&request, &profile);
    let mass = handmade_mass(&request, &profile);
    let four = &mass.plan;
    let three = plan_mass(&MassRequest {
        text: TextOptions {
            eleison: Eleison::Three,
        },
        ..request.clone()
    })
    .unwrap();
    let plain = plan_mass(&MassRequest {
        accents: false,
        ..request.clone()
    })
    .unwrap();
    let printed = |score: &Score| -> Vec<String> {
        score.voices[0]
            .notes
            .iter()
            .filter_map(|note| note.syllable.clone())
            .collect()
    };
    for (at, part) in four.parts() {
        let label = four.address(at);
        let result = mass.result(at).unwrap();
        let score = result.generated.score.clone();
        let field: PartField = part_field(&mass, at).unwrap().1.parse().unwrap();
        let planned = three.part(at).syllables;
        assert_eq!(planned + 1, part.syllables, "{label}");
        assert_eq!(
            part_from_parts(&three, at, score.clone(), &field, &profile).unwrap_err(),
            FieldError::Division {
                part: label.clone(),
                planned,
                found: part.syllables
            }
        );
        let back = part_from_parts(&plain, at, score.clone(), &field, &profile).unwrap();
        assert_eq!(back.generated.report, result.generated.report, "{label}");
        let acute = |s: &String| s.chars().any(|c| "áéíóúýǽ".contains(c));
        let accented = printed(&score);
        assert!(accented.iter().any(acute), "{accented:?}");
        let without = printed(&back.generated.score);
        assert!(!without.iter().any(acute), "{without:?}");
        assert_eq!(accented.len(), without.len(), "{label}");
    }
    let at = four.find("kyrie").unwrap()[0];
    let error = part_from_parts(
        &three,
        at,
        mass.result(at).unwrap().generated.score.clone(),
        &part_field(&mass, at).unwrap().1.parse().unwrap(),
        &profile,
    )
    .unwrap_err();
    assert_eq!(
        error.message(Lang::De),
        "Teil kyrie/1: Die Datei teilt den Text in 7 Silben, der Plan in 6 (eleison drei- oder \
         viersilbig, --eleison)"
    );
    assert_eq!(
        error.message(Lang::En),
        "part kyrie/1: the file divides the text into 7 syllables, the plan into 6 (eleison in \
         three or four syllables, --eleison)"
    );
}

/// A part of `mass` as its file gives it back, for `mass_of_parts` (`--rewrite`): its movement and
/// its number in the movement, as the name of its file has them, its field as the file writes it,
/// and its score without the text going on after it, as a file does not say so.
fn kept_part(mass: &Mass, at: PartRef) -> KeptPart {
    let plan = &mass.plan;
    let mut score = mass.result(at).unwrap().generated.score.clone();
    if let Some(sung) = score.text.as_mut() {
        sung.text = Arc::new(Text {
            continued: false,
            ..(*sung.text).clone()
        });
    }
    let address = plan.address(at);
    let (_, number) = address.rsplit_once('/').unwrap();
    KeptPart {
        movement: plan.movements[at.movement].movement,
        number: number.parse().unwrap(),
        field: part_field(mass, at).unwrap().1.parse().unwrap(),
        score,
    }
}

/// The parts of `mass` as their files give them back, in the order of the mass.
fn kept_parts(mass: &Mass) -> Vec<KeptPart> {
    mass.plan
        .parts()
        .map(|(at, _)| kept_part(mass, at))
        .collect()
}

/// `back`, the mass assembled from the files of the parts of `set` (`mass_of_parts`), is `set`
/// as it was set: its field, title and division of eleison; its movements with their modes,
/// voices, sections and parts as the plan of `set` has them, the points of a part where its
/// file keeps them; the files of its movements, byte for byte; and each part as `--resume` takes
/// it up from its file under the plan of `set` (`part_from_parts`): its score, report, warnings,
/// seed, attempt, voices and motif, with its field as the file keeps it.
fn assert_set_again(set: &Mass, back: &Mass, profile: &Profile) {
    let (plan, again) = (&set.plan, &back.plan);
    assert_eq!(MassField::of(again), MassField::of(plan));
    assert_eq!(
        (&again.title, again.text, again.accents),
        (&plan.title, plan.text, plan.accents)
    );
    assert_eq!(again.movements.len(), plan.movements.len());
    for (m, (movement, planned)) in again.movements.iter().zip(&plan.movements).enumerate() {
        let label = planned.movement.id();
        assert_eq!(
            (
                movement.movement,
                movement.mode,
                &movement.voices,
                &movement.names
            ),
            (
                planned.movement,
                planned.mode,
                &planned.voices,
                &planned.names
            ),
            "{label}"
        );
        assert_eq!(
            (movement.intonation, &movement.notes),
            (planned.intonation, &planned.notes),
            "{label}"
        );
        assert_eq!(movement.sections.len(), planned.sections.len(), "{label}");
        for (section, planned) in movement.sections.iter().zip(&planned.sections) {
            assert_eq!(
                (section.id, section.title, &section.voices, section.ut_supra),
                (planned.id, planned.title, &planned.voices, planned.ut_supra),
                "{label}"
            );
            assert_eq!(section.parts.len(), planned.parts.len(), "{label}");
            for (part, planned) in section.parts.iter().zip(&planned.parts) {
                assert_eq!(
                    (
                        &part.text,
                        part.syllables,
                        part.join,
                        part.ending,
                        part.opening
                    ),
                    (
                        &planned.text,
                        planned.syllables,
                        planned.join,
                        planned.ending,
                        planned.opening
                    ),
                    "{label}"
                );
                assert_eq!(
                    (
                        part.start,
                        &part.chord,
                        part.continued,
                        part.motif,
                        part.seed
                    ),
                    (
                        planned.start,
                        &planned.chord,
                        planned.continued,
                        planned.motif,
                        planned.seed
                    ),
                    "{label}"
                );
                assert_eq!(part.points.is_some(), planned.points.is_some(), "{label}");
            }
        }
        assert_eq!(part_fields(back, m), part_fields(set, m), "{label}");
        assert_eq!(movement_score(back, m), movement_score(set, m), "{label}");
    }
    for (at, _) in plan.parts() {
        let label = plan.address(at);
        let kept = kept_part(set, at);
        let expected = part_from_parts(plan, at, kept.score, &kept.field, profile).unwrap();
        let result = back.result(at).unwrap();
        assert_eq!(result.generated.score, expected.generated.score, "{label}");
        assert_eq!(
            result.generated.report, expected.generated.report,
            "{label}"
        );
        assert_eq!(
            (
                result.seed,
                result.attempt,
                &result.voices,
                &result.warnings
            ),
            (
                expected.seed,
                expected.attempt,
                &expected.voices,
                &expected.warnings
            ),
            "{label}"
        );
        assert_eq!(result.motif, expected.motif, "{label}");
        assert_eq!(result.kept.as_ref(), Some(&kept.field), "{label}");
        if kept.field.points.is_some() {
            assert_eq!(plan.part(at).points, again.part(at).points, "{label}");
        }
    }
}

/// A mass assembled from the files of its parts as they are (`fux mass --rewrite`) is the mass as
/// it was set, whatever this program plans from its options (`mass_of_parts`): cut into smaller
/// parts than by default (`--chunk 30`), with imitation as the options of its setting asked for,
/// in the form normal with its Osanna ut supra, in five voices, in two voices of their own, with
/// eleison in three syllables, with the head motif, without the acutes, under a title of its own.
/// The parts set by the fallbacks of the lengths and of the scoring come back with the points and
/// the voices of their sections as planned and warn again. Only the movements asked for come back.
/// Parts made by hand, no search.
#[test]
fn a_mass_comes_back_from_its_parts() {
    use fux_core::text::Eleison;
    let base = fux_core::mass::profile(&Profile::fux());
    for request in [
        MassRequest {
            chunk: Some(30),
            ..MassRequest::default()
        },
        MassRequest {
            preset: Preset::Normal,
            mode: Some(mode("ionian-transposed")),
            choir: choir(5),
            overrides: parse_imitation("aus; christe=an").unwrap(),
            seed: 77,
            ..MassRequest::default()
        },
        MassRequest {
            mode: Some(mode("phrygian")),
            choir: vec![VoiceType::MezzoSoprano, VoiceType::Baritone],
            movements: vec![Movement::Kyrie, Movement::Agnus],
            text: TextOptions {
                eleison: Eleison::Three,
            },
            seed: 5,
            ..MassRequest::default()
        },
        MassRequest {
            motif: true,
            movements: vec![Movement::Kyrie, Movement::Gloria, Movement::Agnus],
            accents: false,
            title: Some("Missa sine nomine".into()),
            ..MassRequest::default()
        },
    ] {
        let profile = part_profile(&request, &base);
        let mass = handmade_mass(&request, &profile);
        let plan = &mass.plan;
        let movements: Vec<Movement> = plan.movements.iter().map(|m| m.movement).collect();
        let back = mass_of_parts(
            &MassField::of(plan),
            &movements,
            kept_parts(&mass),
            request.title.clone(),
            request.accents,
            &profile,
        )
        .unwrap();
        assert!(back.complete);
        assert_set_again(&mass, &back, &profile);
    }
    // Agnus Dei I set by the fallback of its lengths, Agnus Dei II by the fallback of the scoring
    // in the choir of the mass: their sections keep the voices as planned, and both warn again.
    let request = MassRequest {
        movements: vec![Movement::Agnus],
        ..MassRequest::default()
    };
    let profile = part_profile(&request, &base);
    let mut mass = handmade_mass(&request, &profile);
    let plan = Arc::clone(&mass.plan);
    let agnus = plan.find("agnus/1").unwrap()[0];
    let lengthened = PartResult {
        warnings: vec![Warning::LengthFallback {
            part: "agnus/1".into(),
        }],
        ..attempted(&plan, mass.result(agnus).unwrap().clone(), 3)
    };
    mass.insert(lengthened);
    let enlarged = plan.find("agnus3").unwrap()[0];
    let choir_of_the_mass = handmade(&plan, enlarged, &[0, 1, 2, 4], true, &profile);
    mass.insert(attempted(&plan, choir_of_the_mass, 3));
    let back = mass_of_parts(
        &MassField::of(&plan),
        &[Movement::Agnus],
        kept_parts(&mass),
        None,
        true,
        &profile,
    )
    .unwrap();
    assert_set_again(&mass, &back, &profile);
    let warnings: Vec<&Warning> = back
        .results
        .iter()
        .flatten()
        .flat_map(|result| &result.warnings)
        .collect();
    assert_eq!(
        warnings,
        [
            &Warning::LengthFallback {
                part: "agnus/1".into()
            },
            &Warning::ScoringFallback {
                section: "agnus3".into()
            }
        ]
    );
    assert_eq!(back.plan.movements[0].sections[1].voices, [0, 1, 2, 3, 4]);
    // The movements asked for, from the parts of more.
    let request = MassRequest::default();
    let profile = part_profile(&request, &base);
    let mass = handmade_mass(&request, &profile);
    let credo = mass_of_parts(
        &MassField::of(&mass.plan),
        &[Movement::Credo],
        kept_parts(&mass),
        None,
        true,
        &profile,
    )
    .unwrap();
    let movements: Vec<Movement> = credo.plan.movements.iter().map(|m| m.movement).collect();
    assert_eq!(movements, [Movement::Credo]);
    assert_eq!(part_fields(&credo, 0), part_fields(&mass, 2));
}

/// Parts that do not make the mass as it was set fail and say which part, in German and English
/// (`mass_of_parts`): a part missing in its movement or at its end, two parts with one number, a
/// part that begins no section of the form where its join says so, one that sings another text
/// than its section goes on with, one set after another ending of the part before it or with
/// another head motif than Kyrie I gives, the parts of a movement in different modes, a part whose
/// score does not hold together, a movement asked for without parts, and the Osanna ut supra
/// without the Sanctus it repeats.
#[test]
fn parts_that_do_not_make_the_mass_fail() {
    let base = fux_core::mass::profile(&Profile::fux());
    let request = MassRequest {
        movements: vec![Movement::Kyrie, Movement::Credo],
        ..MassRequest::default()
    };
    let profile = part_profile(&request, &base);
    let mass = handmade_mass(&request, &profile);
    let plan = &mass.plan;
    let record = MassField::of(plan);
    let movements = [Movement::Kyrie, Movement::Credo];
    let assembled = |kept: Vec<KeptPart>| {
        mass_of_parts(&record, &movements, kept, None, true, &profile).map(|_| ())
    };
    let parts = kept_parts(&mass);
    assert_eq!(assembled(parts.clone()), Ok(()));
    let without = |address: &str| -> Vec<KeptPart> {
        parts
            .iter()
            .filter(|part| format!("{}/{}", part.movement.id(), part.number) != address)
            .cloned()
            .collect()
    };
    let error = assembled(without("kyrie/2")).unwrap_err();
    assert_eq!(
        error,
        KeptError::Missing {
            part: "kyrie/2".into(),
            later: false
        }
    );
    assert_eq!(error.message(Lang::De), "Teil kyrie/2 fehlt");
    assert_eq!(error.message(Lang::En), "part kyrie/2 is missing");
    let last = format!("credo/{}", plan.movements[1].part_count());
    let error = assembled(without(&last)).unwrap_err();
    assert_eq!(
        error,
        KeptError::Missing {
            part: last.clone(),
            later: true
        }
    );
    assert_eq!(
        error.message(Lang::De),
        format!("Die Teile ab {last} fehlen")
    );
    assert_eq!(
        error.message(Lang::En),
        format!("the parts from {last} on are missing")
    );
    let mut twice = parts.clone();
    twice.push(parts[1].clone());
    let error = assembled(twice).unwrap_err();
    assert_eq!(
        error,
        KeptError::Twice {
            part: "kyrie/2".into()
        }
    );
    assert_eq!(
        error.message(Lang::De),
        "Zwei Dateien halten den Teil kyrie/2"
    );
    assert_eq!(error.message(Lang::En), "two files hold part kyrie/2");
    let mut renamed = parts.clone();
    renamed[1].field.title = "Christus".into();
    let error = assembled(renamed).unwrap_err();
    assert_eq!(
        error,
        KeptError::Form {
            part: "kyrie/2".into()
        }
    );
    assert!(
        error
            .message(Lang::De)
            .starts_with("Teil kyrie/2 beginnt keinen Abschnitt")
    );
    assert_eq!(
        error.message(Lang::En),
        "part kyrie/2 begins no section or Break of the form of the mass where its join says so, \
         or follows its last section"
    );
    let mut other = parts.clone();
    other[1].score = parts[2].score.clone();
    let error = assembled(other).unwrap_err();
    assert_eq!(
        error,
        KeptError::Text {
            part: "kyrie/2".into()
        }
    );
    assert_eq!(
        error.message(Lang::De),
        "Teil kyrie/2 singt nicht den Text, mit dem sein Abschnitt nach den Teilen davor \
         weitergeht"
    );
    assert_eq!(
        error.message(Lang::En),
        "part kyrie/2 does not sing the text with which its section goes on after the parts \
         before it"
    );
    let christe = plan.find("christe").unwrap()[0];
    let mut stale = parts.clone();
    stale[2].field.before = Some(match plan.part(christe).ending {
        Step::D => Step::A,
        _ => Step::D,
    });
    let error = assembled(stale).unwrap_err();
    assert_eq!(
        error,
        KeptError::Before {
            part: "kyrie/3".into()
        }
    );
    assert_eq!(
        error.message(Lang::De),
        "Teil kyrie/3 wurde nach einer anderen Schlussstufe des Teils davor gesetzt, als dessen \
         Datei hält"
    );
    assert_eq!(
        error.message(Lang::En),
        "part kyrie/3 was set after another ending of the part before it than that part's file \
         keeps"
    );
    let mut modes = parts.clone();
    modes[0].score.mode = mode("phrygian");
    let error = assembled(modes).unwrap_err();
    assert_eq!(
        error,
        KeptError::Apart {
            movement: Movement::Kyrie
        }
    );
    assert_eq!(
        error.message(Lang::De),
        "Die Teile des Kyrie passen nicht zusammen: Modus, Teilung von eleison oder Stimmen"
    );
    assert_eq!(
        error.message(Lang::En),
        "the parts of the Kyrie do not go together: their modes, their division of eleison or \
         their voices"
    );
    // A part whose voices do not last alike, as its file does not give it back.
    let mut short = parts.clone();
    let last = short[1].score.voices.len() - 1;
    short[1].score.voices[last].notes.pop();
    let error = assembled(short).unwrap_err();
    assert!(
        matches!(&error, KeptError::Field(FieldError::Score { part, .. }) if part == "kyrie/2"),
        "{error:?}"
    );
    assert_eq!(
        error.message(Lang::De),
        "Teil kyrie/2: Stimme 4 dauert 6 Takte, Stimme 1 aber 7 Takte"
    );
    assert_eq!(
        error.message(Lang::En),
        "part kyrie/2: Voice 4 lasts 6 measures, but voice 1 lasts 7"
    );
    let error = mass_of_parts(
        &record,
        &[Movement::Kyrie, Movement::Gloria],
        parts.clone(),
        None,
        true,
        &profile,
    )
    .unwrap_err();
    assert_eq!(
        error,
        KeptError::NoParts {
            movement: Movement::Gloria
        }
    );
    assert_eq!(error.message(Lang::De), "Vom Gloria liegt kein Teil vor");
    assert_eq!(error.message(Lang::En), "no part of the Gloria is there");
    // A part with another head motif than Kyrie I gives.
    let request = MassRequest {
        motif: true,
        movements: vec![Movement::Kyrie, Movement::Agnus],
        ..MassRequest::default()
    };
    let profile = part_profile(&request, &base);
    let mass = handmade_mass(&request, &profile);
    let record = MassField::of(&mass.plan);
    let mut parts = kept_parts(&mass);
    let agnus = parts
        .iter()
        .position(|part| part.movement == Movement::Agnus && part.number == 1)
        .unwrap();
    parts[agnus].field.motif = Some("4,7,9,10,8".parse().unwrap());
    let error = mass_of_parts(
        &record,
        &[Movement::Kyrie, Movement::Agnus],
        parts.clone(),
        None,
        true,
        &profile,
    )
    .unwrap_err();
    assert_eq!(
        error,
        KeptError::Motif {
            part: "agnus/1".into()
        }
    );
    assert_eq!(
        error.message(Lang::De),
        "Teil agnus/1 nahm ein anderes Kopfmotiv, als das Kyrie I gibt"
    );
    assert_eq!(
        error.message(Lang::En),
        "part agnus/1 took another head motif than Kyrie I gives"
    );
    // Without Kyrie I, the Agnus Dei keeps the motif it took.
    assert!(mass_of_parts(&record, &[Movement::Agnus], parts, None, true, &profile).is_ok());
    // The Osanna ut supra of the form normal repeats the Osanna of the Sanctus.
    let request = MassRequest {
        preset: Preset::Normal,
        movements: vec![Movement::Sanctus, Movement::Benedictus],
        ..MassRequest::default()
    };
    let profile = part_profile(&request, &base);
    let mass = handmade_mass(&request, &profile);
    let record = MassField::of(&mass.plan);
    let error = mass_of_parts(
        &record,
        &[Movement::Benedictus],
        kept_parts(&mass),
        None,
        true,
        &profile,
    )
    .unwrap_err();
    assert_eq!(error, KeptError::UtSupra);
    assert_eq!(
        error.message(Lang::De),
        "Das Osanna ut supra des Benedictus wiederholt das Osanna des Sanctus, und das fehlt"
    );
    assert_eq!(
        error.message(Lang::En),
        "the Osanna ut supra of the Benedictus repeats the Osanna of the Sanctus, which is missing"
    );
    let both = mass_of_parts(
        &record,
        &[Movement::Sanctus, Movement::Benedictus],
        kept_parts(&mass),
        None,
        true,
        &profile,
    )
    .unwrap();
    assert_set_again(&mass, &both, &profile);
}

/// A voice of a movement that does not sing in a section rests through it in the section's score
/// (`section_score`): it sings none of the text and has no note, and the rules that count the
/// voices of a choir leave it out. Three voices of a four-voice movement, entering together in
/// a point of imitation or not, have no half choir (IMI-SILENCE `thin`, `unplanned_thin`), and
/// FREE-TEXTURE judges them as three, from their own first notes on: one voice of three sounding
/// is too few. A voice that rests through a score without a text, or that sings the text, still
/// counts, as before phase 14: two voices of four sounding are half the choir, and the texture
/// waits for the fourth voice, which never enters.
#[test]
fn tacet_voices_are_no_part_of_the_choir() {
    let line = |notes: &str, voice_type: VoiceType| {
        let notes = parse_lilypond(notes, Duration::HALF).unwrap();
        Voice::new(voice_type, Role::Counterpoint(Species::Free), notes)
    };
    let score = |text: Option<[bool; 4]>, planned: bool| {
        let mut score = Score::new(
            mode("dorian"),
            vec![
                line("a'1 a'1 a'1 a'1 a'1", VoiceType::Soprano),
                line("d'1 d'1 r1 r1 d'1", VoiceType::Alto),
                line("f1 f1 f1 r1 f1", VoiceType::Tenor),
                line("r1 r1 r1 r1 r1", VoiceType::Bass),
            ],
        );
        score.text = text.map(|voices| ScoreText {
            text: Arc::new(part_text("kyrie", &TextOptions::default()).unwrap()),
            voices: voices.to_vec(),
        });
        if planned {
            let entries = ["A4", "D4", "F3"]
                .iter()
                .enumerate()
                .map(|(voice, pitch)| ScoreEntry {
                    voice,
                    start: 0,
                    rest: 0,
                    notes: vec![(pitch.parse().unwrap(), 8)],
                })
                .collect();
            score.imitation = Some(Arc::new(ScoreImitation {
                points: vec![ScorePoint {
                    words: 0..1,
                    entries,
                    goal: None,
                }],
            }));
        }
        score
    };
    let profile = fux_core::mass::profile(&Profile::fux());
    let checker = Checker::new(&profile);
    let found = |score: &Score| -> Vec<(&'static str, Level, usize, u32)> {
        checker
            .check(score)
            .violations
            .iter()
            .filter(|v| v.rule == "IMI-SILENCE" || v.rule == "FREE-TEXTURE")
            .map(|v| (v.rule, v.level, v.voice, v.tick))
            .collect()
    };
    let texture = profile.settings("FREE-TEXTURE").level("level");
    let thin = profile.settings("IMI-SILENCE").level("thin");
    for planned in [false, true] {
        // The bass tacet: three voices, one of them sounding in the fourth measure.
        assert_eq!(
            found(&score(Some([true, true, true, false]), planned)),
            [("FREE-TEXTURE", texture, 1, 24)],
            "planned {planned}"
        );
        // Counted as a fourth voice.
        let four: &[_] = if planned {
            &[("IMI-SILENCE", thin, 1, 16)]
        } else {
            &[]
        };
        assert_eq!(found(&score(Some([true; 4]), planned)), four);
        assert_eq!(found(&score(None, planned)), four);
    }
}

/// The last chord of a part may have the major third of the final, as the last chord of a piece
/// may (FIC-SCOPE, Fux Fig. 184, 185): a part ends, every voice holds its chord, and the next
/// part begins after a rest. The tenor of a part on D anticipates F♯ over the A of the bass and
/// holds it into the last chord; joined to the part after it, without the mark of the seam the
/// raised note would have to resolve upwards, with it the part ends there. A section that
/// `section_score` joins has its marks (`a_credo_in_three_voices` found such a part, credo/11).
#[test]
fn the_last_chord_of_a_part_may_have_the_major_third() {
    let line = |notes: &str, voice_type: VoiceType| {
        let notes = parse_lilypond(notes, Duration::HALF).unwrap();
        Voice::new(voice_type, Role::Counterpoint(Species::Free), notes)
    };
    let mut joined = Score::new(
        mode("dorian"),
        vec![
            line("a'1 a'1 a'1 a'1 r2 a'2 a'1 a'1 a'1", VoiceType::Alto),
            line(
                "d'1 d'1 e'2 fis'2~ fis'1 r2 f'2 f'1 e'1 d'1",
                VoiceType::Tenor,
            ),
            line("d1 d1 a,1 d1 r2 d2 d1 a,1 d1", VoiceType::Bass),
        ],
    );
    joined.validate().unwrap();
    let scope = |score: &Score| -> Vec<(usize, u32, Level)> {
        Checker::new(&Profile::fux())
            .check(score)
            .violations
            .iter()
            .filter(|v| v.rule == "FIC-SCOPE")
            .map(|v| (v.voice, v.tick, v.level))
            .collect()
    };
    assert_eq!(scope(&joined), [(1, 36, Level::Forbidden)]);
    joined.sections = vec![SectionMark {
        measure: 4,
        kind: SectionKind::Seam,
        title: None,
    }];
    joined.validate().unwrap();
    assert_eq!(scope(&joined), []);
    // A mark elsewhere ends no part there.
    joined.sections[0].measure = 5;
    assert_eq!(scope(&joined), [(1, 36, Level::Forbidden)]);
}

/// A texted part as `assert_texted` of the common tests accepts a setting, under `profile`, the
/// profile the part was set with, where that one takes Fux's: no prohibition, a valid free score
/// whose voices all sing its text, the profile itself finding what the search found, the TXT
/// rules with every case switched on finding no prohibition, the underlay of every voice as the
/// oracle wants it and the cheapest of its notes (the mass changes no level of the TXT rules).
fn assert_part_texted(generated: &Generated, text: &Text, profile: &Profile, label: &str) {
    let score = &generated.score;
    let forbidden: Vec<_> = generated
        .report
        .violations
        .iter()
        .filter(|v| v.is_forbidden())
        .collect();
    assert!(forbidden.is_empty(), "{label}: {forbidden:?}");
    score.validate().unwrap_or_else(|e| panic!("{label}: {e}"));
    assert!(score.is_free(), "{label}");
    let sung = score.text.as_ref().expect("a text");
    assert_eq!(*sung.text, *text, "{label}");
    assert_eq!(sung.voices, vec![true; score.voices.len()], "{label}");
    let report = Checker::new(profile).check(score);
    assert_eq!(report.violations, generated.report.violations, "{label}");
    assert_eq!(common::text_rules().check(score).forbidden(), 0, "{label}");
    let restated = gate_restarts(score, text);
    for (v, voice) in score.voices.iter().enumerate() {
        let label = format!("{label}, voice {v}");
        assert_sung(text, &voice.notes, &restated, &label);
        for (i, note) in voice.notes.iter().enumerate() {
            assert_eq!(
                note.syllable.is_some(),
                note.lyric.is_some(),
                "{label}, note {i}"
            );
        }
    }
    assert_cheapest(generated, label);
}

/// The acceptance of a mass as set (§ 8, of the exports of phase 14b the movements' scores; the
/// files are those of the CLI's and the export's tests), under `profile`, the profile of the mass
/// the parts were set with: it is complete, its parts those of the plan. Every
/// part is a texted setting without a prohibition, the profile reproducing its report; its field
/// fits the plan, and its score as a file gives it back (the text not going on, which only the
/// field says) comes back as set with that report (the file itself, written and read, is the
/// CLI's acceptance, `a_missa_brevis_written`); an imitative part sounds every entry as planned
/// and the detector finds every point; a part that begins together strikes the planned chord in
/// every voice at its start, on the first syllable; no voice sounds before the start; the bass
/// ends on the planned ending, the last part of every movement on the final. The Benedictus sings
/// in three voices, Agnus Dei II in five or in the choir of the mass with a warning. Every section
/// joined breaks no prohibition. Every movement's score (`movement_score`) holds together, and
/// its parts come back from it and its fields (`split_movement`) with their texts and reports.
/// Prints each part and the whole time.
fn accept(request: &MassRequest, mass: &Mass, profile: &Profile, label: &str) {
    let plan = &mass.plan;
    let profile = part_profile(request, profile);
    let checker = Checker::new(&profile);
    assert_eq!(*plan.as_ref(), plan_mass(request).unwrap(), "{label}");
    assert_eq!(mass.results.len(), plan.part_count(), "{label}");
    assert!(mass.complete, "{label}: incomplete");
    let mut seconds = 0.0;
    for (at, part) in plan.parts() {
        let result = mass.result(at).unwrap();
        let label = format!("{label}, {}", plan.address(at));
        let movement = &plan.movements[at.movement];
        let generated = &result.generated;
        let score = &generated.score;
        seconds += result.seconds;
        println!(
            "{label} ({}): attempt {}, seed {}, {} measures, cost {}, {:.1} s{}",
            part.opening.name(Lang::En),
            result.attempt,
            result.seed,
            score.measure_count(),
            generated.cost,
            result.seconds,
            result
                .warnings
                .iter()
                .map(|w| format!("; {}", w.message(Lang::En)))
                .collect::<String>()
        );
        let text = part.sung_text(&request.text);
        assert_part_texted(generated, &text, &profile, &label);
        assert_eq!(checker.check(score).violations, generated.report.violations);
        // As its file keeps it: the field fits the plan (`--resume`), and the score as a file
        // gives it back, its text not going on after it, comes back as set.
        let field: PartField = part_field(mass, at).unwrap().1.parse().unwrap();
        assert!(
            field.fits(plan, at, profile_hash(&profile)),
            "{label}: {field}"
        );
        let mut read = score.clone();
        if let Some(sung) = read.text.as_mut() {
            sung.text = Arc::new(Text {
                continued: false,
                ..(*sung.text).clone()
            });
        }
        let back = part_from_parts(plan, at, read, &field, &profile).unwrap();
        assert_eq!(back.generated.score, *score, "{label}");
        assert_eq!(back.generated.report, generated.report, "{label}");
        assert_eq!(
            back.warnings,
            result.warnings[..back.warnings.len()],
            "{label}"
        );
        let voices = &result.voices;
        assert_eq!(score.voices.len(), voices.len(), "{label}");
        if part.opening == Opening::Imitative {
            assert_imitated(score, &label);
            assert_detected(score, &label);
            assert!(!generated.subjects.is_empty(), "{label}");
        } else {
            assert_eq!(score.imitation, None, "{label}");
            let kinds: Vec<VoiceType> = voices.iter().map(|&v| movement.voices[v]).collect();
            let chord = opening_chord(movement.mode, &kinds, movement.mode.final_step());
            for (v, voice) in score.voices.iter().enumerate() {
                let (start, first) = voice
                    .timed()
                    .find(|(_, note)| !note.is_rest())
                    .unwrap_or_else(|| panic!("{label}, voice {v}: silent"));
                assert_eq!(
                    (start, first.pitch, first.lyric.and_then(Lyric::index)),
                    (part.start, Some(chord[v]), Some(0)),
                    "{label}, voice {v}"
                );
            }
        }
        for (v, voice) in score.voices.iter().enumerate() {
            for (start, note) in voice.timed() {
                assert!(
                    start >= part.start || note.is_rest(),
                    "{label}, voice {v}: a note at {start}"
                );
            }
        }
        let bass = score.voices.last().unwrap().pitches();
        assert_eq!(bass.last().unwrap().step, part.ending, "{label}");
        let last = movement.sections.last().unwrap();
        if last.ut_supra.is_none() && last.parts.last() == Some(part) {
            assert_eq!(part.ending, movement.mode.final_step(), "{label}");
        }
    }
    for (m, movement) in plan.movements.iter().enumerate() {
        for (s, section) in movement.sections.iter().enumerate() {
            let at = PartRef {
                movement: m,
                section: s,
                part: 0,
            };
            let voices = mass.result(at).map(|result| result.voices.len());
            match section.id {
                "benedictus" if request.choir.len() == 4 => assert_eq!(voices, Some(3)),
                "agnus3" if request.choir.len() == 4 => {
                    let fallback = mass
                        .result(at)
                        .unwrap()
                        .warnings
                        .iter()
                        .any(|w| matches!(w, Warning::ScoringFallback { .. }));
                    assert_eq!(voices, Some(if fallback { 4 } else { 5 }), "{label}");
                }
                _ => {}
            }
            let joined = section_score(mass, m, s).unwrap();
            joined.validate().unwrap();
            let report = checker.check(&joined);
            let forbidden: Vec<_> = report
                .violations
                .iter()
                .filter(|v| v.is_forbidden())
                .collect();
            assert!(
                forbidden.is_empty(),
                "{label}, section {}: {forbidden:?}",
                section.id
            );
        }
        // The movement as its file keeps it (14b): its score holds together, and its parts come
        // back from it and its fields as set, so that the check of each repeats its report.
        let score = movement_score(mass, m).unwrap();
        score.validate().unwrap();
        let places: Vec<PartRef> = plan
            .parts()
            .filter(|(at, _)| at.movement == m)
            .map(|(at, _)| at)
            .collect();
        let split = split_movement(&score, &part_fields(mass, m)).unwrap();
        assert_eq!(split.len(), places.len(), "{label}");
        for ((_, part, _), &at) in split.iter().zip(&places) {
            let generated = &mass.result(at).unwrap().generated;
            let label = format!("{label}, {} of the movement", plan.address(at));
            assert_eq!(part.text, generated.score.text, "{label}");
            assert_eq!(
                checker.check(part).violations,
                generated.report.violations,
                "{label}"
            );
        }
    }
    println!("{label}: {seconds:.0} s of search");
}

/// What setting a part took: the wall and CPU seconds of its attempts, and the seconds it would
/// take on an idle machine by the references set before and after its movement ([`reference`]).
#[derive(Clone, Copy, Debug, Default)]
struct Times {
    wall: f64,
    cpu: f64,
    idle: f64,
}

/// A mass set for the acceptance, with what setting it took.
struct Set {
    request: MassRequest,
    mass: Mass,
    /// The wall seconds of the mass, without the references set beside it.
    seconds: f64,
    /// Per part in the order of the plan.
    times: Vec<Times>,
    /// The load average of the last minute before the mass and after each movement.
    loads: Vec<f64>,
}

impl Set {
    /// The seconds the mass would take on an idle machine.
    fn idle(&self) -> f64 {
        self.times.iter().map(|t| t.idle).sum()
    }
}

/// Sets the mass of `request` under the profile of the mass on Fux's, telling each attempt that
/// fails, and times each part: its wall and CPU seconds, and its seconds on an idle machine by a
/// reference set before the mass and after each movement (the method of phase 14a's first
/// measurements; [`reference`]).
fn set_mass(request: &MassRequest, label: &str) -> Set {
    let profile = fux_core::mass::profile(&Profile::fux());
    let plan = plan_mass(request).unwrap();
    let places: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
    // The CPU seconds of the references, the movement each closes, and their wall seconds.
    let (first, first_cpu) = reference();
    let mut references: Vec<(Option<usize>, f64)> = vec![(None, first_cpu)];
    let mut aside = first;
    let mut loads = vec![load()];
    let mut times = vec![Times::default(); places.len()];
    let mut movement: Option<usize> = None;
    let mut started = (Instant::now(), 0.0);
    let clock = Instant::now();
    let mut on = |event: MassEvent| {
        match &event {
            MassEvent::Started { at, attempt: 0, .. } => {
                if movement.is_some_and(|m| m != at.movement) {
                    let (wall, cpu) = reference();
                    aside += wall;
                    references.push((movement, cpu));
                    loads.push(load());
                }
                movement = Some(at.movement);
                started = (Instant::now(), cpu_seconds());
            }
            MassEvent::Finished(result) => {
                let i = places.iter().position(|&at| at == result.at).unwrap();
                times[i].wall = started.0.elapsed().as_secs_f64();
                times[i].cpu = cpu_seconds() - started.1;
            }
            MassEvent::Failed { at, attempt, error } => {
                println!(
                    "{label}: {} attempt {attempt} failed: {error}",
                    plan.address(*at)
                );
            }
            MassEvent::Cancelled => println!("{label}: cancelled"),
            _ => {}
        }
        ControlFlow::Continue(())
    };
    let mass = generate_mass(request, &profile, None, &mut on).unwrap();
    let seconds = clock.elapsed().as_secs_f64() - (aside - first);
    let (_, last_cpu) = reference();
    references.push((movement, last_cpu));
    loads.push(load());
    for (i, at) in places.iter().enumerate() {
        // The references before and after the part's movement.
        let after = references
            .iter()
            .position(|&(m, _)| m == Some(at.movement))
            .unwrap();
        let mean = (references[after - 1].1 + references[after].1) / 2.0;
        times[i].idle = times[i].cpu * REFERENCE_IDLE / mean;
    }
    Set {
        request: request.clone(),
        mass,
        seconds,
        times,
        loads,
    }
}

/// The masses set in this process: a test that needs a mass another test set takes it, so that
/// the Dorian Missa brevis of [`a_missa_brevis`] is the first of [`missa_brevis_in_four_modes`],
/// whose masses [`mass_style`] and [`mass_timings`] read. They are set one at a time.
static MASSES: Mutex<Vec<Arc<Set>>> = Mutex::new(Vec::new());

/// The mass of `request`, set in this process before or now ([`set_mass`]).
fn set(request: &MassRequest, label: &str) -> Arc<Set> {
    let mut masses = MASSES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(set) = masses.iter().find(|set| set.request == *request) {
        return Arc::clone(set);
    }
    let set = Arc::new(set_mass(request, label));
    masses.push(Arc::clone(&set));
    set
}

/// Sets the mass of `request` ([`set`]) and accepts it ([`accept`]); fails where the mass took
/// longer than `limit`, the references aside.
fn set_and_accept(request: &MassRequest, label: &str, limit: Elapsed) -> Arc<Set> {
    let profile = fux_core::mass::profile(&Profile::fux());
    let set = set(request, label);
    accept(request, &set.mass, &profile, label);
    println!(
        "{label}: {:.0} s in all, {:.0} s on an idle machine, load {}",
        set.seconds,
        set.idle(),
        set.loads
            .iter()
            .map(|l| format!("{l:.1}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    assert!(
        set.seconds <= limit.as_secs_f64(),
        "{label}: {} s",
        set.seconds
    );
    set
}

/// The Missa brevis for S A T B in `mode`, seed 1, as the acceptance sets it.
fn brevis(mode_name: &str) -> MassRequest {
    MassRequest {
        preset: Preset::Brevis,
        mode: Some(mode(mode_name)),
        choir: choir(4),
        seed: 1,
        ..MassRequest::default()
    }
}

/// The four modes of [`missa_brevis_in_four_modes`].
const FOUR_MODES: [&str; 4] = [
    "dorisch",
    "phrygisch",
    "mixolydisch",
    "ionisch-transponiert",
];

/// The Kyrie of a mass for three voices (A T B), Dorian, seed 1: Kyrie I, Christe and Kyrie II,
/// each a section of one imitative part, by every oracle of the acceptance; about a minute in
/// release: `cargo test --release -p fux-core --test mass -- --ignored --nocapture kyrie`.
#[test]
#[ignore = "release: sets three parts"]
fn a_kyrie_in_three_voices() {
    let request = MassRequest {
        movements: vec![Movement::Kyrie],
        choir: choir(3),
        seed: 1,
        ..MassRequest::default()
    };
    set_and_accept(&request, "kyrie a 3", Elapsed::from_secs(300));
}

/// The Credo of the Missa brevis for three voices (A T B), Dorian, seed 1: one section of twelve
/// parts joined at seams, after a minim's breath, and at the Breaks of Et incarnatus and the
/// Crucifixus, after a measure's rest, by every oracle of the acceptance; the section joined
/// breaks no prohibition. A few minutes in release: `cargo test --release -p fux-core --test mass
/// -- --ignored --nocapture credo`.
#[test]
#[ignore = "release: sets the twelve parts of a Credo"]
fn a_credo_in_three_voices() {
    let request = MassRequest {
        movements: vec![Movement::Credo],
        choir: choir(3),
        seed: 1,
        ..MassRequest::default()
    };
    let set = set_and_accept(&request, "credo a 3", Elapsed::from_secs(900));
    let credo = section_score(&set.mass, 0, 0).unwrap();
    let kinds: Vec<SectionKind> = credo.sections.iter().map(|mark| mark.kind).collect();
    assert_eq!(kinds.len(), 11);
    assert_eq!(
        kinds
            .iter()
            .filter(|&&kind| kind == SectionKind::Break)
            .count(),
        2
    );
}

/// The acceptance of phase 14 (§ 8; of the exports the movements' scores, whose files the tests
/// of the CLI and of the export write): a Missa brevis for S A T B in the Dorian mode, seed 1,
/// set whole under the profile of the mass on Fux's. Complete, 26 of 26 parts, each by every
/// oracle, every section joined without a prohibition, every movement split into its parts as
/// set; within 20 minutes, the references for the times aside (about 3 minutes on an idle
/// machine since Rang 1; 185 s on 6.11.):
/// `cargo test --release -p fux-core --test mass -- --ignored --nocapture --exact a_missa_brevis`.
#[test]
#[ignore = "release: sets a whole mass, about 3 minutes"]
fn a_missa_brevis() {
    let set = set_and_accept(
        &brevis("dorisch"),
        "Missa brevis",
        Elapsed::from_secs(20 * 60),
    );
    assert_eq!(set.mass.results.iter().flatten().count(), 26);
}

/// The Missa brevis in four modes, seed 1, by the oracles of [`a_missa_brevis`]: the Dorian,
/// the Phrygian, the Mixolydian and the Ionian on F, each complete within 20 minutes:
/// `cargo test --release -p fux-core --test mass -- --ignored --nocapture four_modes`.
#[test]
#[ignore = "release: sets four whole masses, a quarter of an hour"]
fn missa_brevis_in_four_modes() {
    for name in FOUR_MODES {
        let set = set_and_accept(
            &brevis(name),
            &format!("Missa brevis, {name}"),
            Elapsed::from_secs(20 * 60),
        );
        assert_eq!(set.mass.results.iter().flatten().count(), 26);
    }
}

/// Sets the mass of `request` ([`set`]), accepts it ([`accept`]) and checks it with `check`,
/// and only then fails where the mass took longer than `limit`, the references aside: so a mass
/// on a loaded machine is still judged by every oracle.
fn set_accept_and_check(
    request: &MassRequest,
    label: &str,
    limit: Elapsed,
    check: impl FnOnce(&Mass),
) {
    let profile = fux_core::mass::profile(&Profile::fux());
    let set = set(request, label);
    accept(request, &set.mass, &profile, label);
    check(&set.mass);
    println!(
        "{label}: {:.0} s in all, {:.0} s on an idle machine, load {}",
        set.seconds,
        set.idle(),
        set.loads
            .iter()
            .map(|l| format!("{l:.1}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    assert!(
        set.seconds <= limit.as_secs_f64(),
        "{label}: {} s",
        set.seconds
    );
}

/// The Missa brevis of [`a_missa_brevis`] with the head motif (D11, MASS-MOTIF), by its
/// oracles: Kyrie I gives the motif of its first subject, and the first parts of Gloria, Credo,
/// Sanctus and Agnus Dei I take it, each its first subject on the motif's pitches, or a subject
/// from its words with the warning of the fallback; where Kyrie I gives none, they say so. Prints
/// the motif and the warnings. Some minutes in release: `cargo test --release -p fux-core --test
/// mass -- --ignored --nocapture with_the_head_motif`.
#[test]
#[ignore = "release: sets a whole mass, a few minutes"]
fn a_missa_brevis_with_the_head_motif() {
    let request = MassRequest {
        motif: true,
        ..brevis("dorisch")
    };
    let label = "Missa brevis with the head motif";
    set_accept_and_check(
        &request,
        label,
        Elapsed::from_secs(20 * 60),
        head_motif_taken,
    );
}

/// The head motif of `mass` as the acceptance of [`a_missa_brevis_with_the_head_motif`] judges
/// it.
fn head_motif_taken(mass: &Mass) {
    let plan = &mass.plan;
    assert_eq!(mass.results.iter().flatten().count(), 26);
    let kyrie = plan.motif_source().unwrap();
    let given = mass.result(kyrie).unwrap();
    assert_eq!(given.motif, motif_of(&given.generated));
    println!(
        "head motif: {}",
        given
            .motif
            .as_ref()
            .map_or("none".to_string(), ToString::to_string)
    );
    let mut taken = 0;
    for (at, part) in plan.parts().filter(|(_, part)| part.motif) {
        let result = mass.result(at).unwrap();
        let label = plan.address(at);
        let movement = plan.movements[at.movement].movement.title().to_string();
        assert_eq!(result.motif, given.motif, "{label}");
        let origin = result.generated.subjects[0].origin;
        match &given.motif {
            None => assert!(
                result.warnings.contains(&Warning::NoMotif { movement }),
                "{label}"
            ),
            Some(motif) if origin == Origin::Motif => {
                let mode = plan.movements[at.movement].mode;
                let (subject, _) = first_heads(&result.generated, mode);
                assert_eq!(subject, motif.pitches(mode, subject[0].octave), "{label}");
                taken += 1;
            }
            Some(_) => assert!(
                result
                    .warnings
                    .contains(&Warning::MotifFallback { movement }),
                "{label}"
            ),
        }
        println!("{label} ({}): {origin:?}", part.text);
    }
    println!("{taken} of 4 parts on the head motif");
}

/// The Missa brevis on chant IX without a mode of its own (D12, MASS-CHANT), by the oracles of
/// [`a_missa_brevis`]: each movement in the mode of its chant, every imitative part with its
/// subjects from its chant, but where a point's phrase gives none, which says so. Prints the
/// warnings. Some minutes in release: `cargo test --release -p fux-core --test mass -- --ignored
/// --nocapture on_chant_ix`.
#[test]
#[ignore = "release: sets a whole mass, a few minutes"]
fn a_missa_brevis_on_chant_ix() {
    let request = MassRequest {
        chant: Some(ChantMass::IX),
        choir: choir(4),
        seed: 1,
        ..MassRequest::default()
    };
    let label = "Missa brevis on chant IX";
    set_accept_and_check(&request, label, Elapsed::from_secs(20 * 60), on_chant_ix);
}

/// The chant of `mass` as the acceptance of [`a_missa_brevis_on_chant_ix`] judges it.
fn on_chant_ix(mass: &Mass) {
    let plan = &mass.plan;
    assert_eq!(mass.results.iter().flatten().count(), 26);
    let modes: Vec<String> = plan.movements.iter().map(|m| m.mode.id()).collect();
    assert_eq!(
        modes,
        [
            "dorian",
            "mixolydian",
            "hypophrygian",
            "ionian-transposed",
            "ionian-transposed",
            "ionian-transposed"
        ]
    );
    assert_eq!(MassField::of(plan).chant, Some(ChantMass::IX));
    let (mut chanted, mut points) = (0, 0);
    for (at, part) in plan.parts() {
        let result = mass.result(at).unwrap();
        let label = plan.address(at);
        // A point whose phrase gave no head says so, and so a restatement of it.
        let fell_back = result
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::ChantFallback(_)));
        for (p, subject) in result.generated.subjects.iter().enumerate() {
            points += 1;
            match subject.origin {
                Origin::Chant { .. } => chanted += 1,
                _ => assert!(fell_back, "{label} ({}), point {}", part.text, p + 1),
            }
        }
    }
    println!("{chanted} of {points} subjects from the chant");
}

/// The written attacks of voice `v`: its notes that are no rest and continue no tie.
fn attacks(score: &Score, v: usize) -> usize {
    let notes = &score.voices[v].notes;
    notes
        .iter()
        .enumerate()
        .filter(|&(i, note)| !note.is_rest() && !(i > 0 && notes[i - 1].tie))
        .count()
}

/// The pitches sounding at `tick`, one per voice that sounds.
fn sounding_at(score: &Score, tick: u32) -> Vec<Pitch> {
    score
        .voices
        .iter()
        .filter_map(|voice| {
            voice
                .timed()
                .find(|&(start, note)| start <= tick && tick < start + note.duration.ticks())
                .and_then(|(_, note)| note.pitch)
        })
        .collect()
}

/// Whether the chord `pitches` on `root` has its third and, from four voices on, its fifth.
fn complete(pitches: &[Pitch], root: Step) -> bool {
    let has = |steps: i32| {
        pitches
            .iter()
            .any(|p| p.step == Step::from_index(root.index() + steps))
    };
    has(0) && has(2) && (pitches.len() < 4 || has(4))
}

/// The syllable that voice `voice` holds at `tick`: the last it began at or before it, while it
/// sounds; `None` where it rests.
fn held_syllable(voice: &Voice, tick: u32) -> Option<usize> {
    let mut held = None;
    for (start, note) in voice.timed() {
        if start > tick {
            break;
        }
        if note.is_rest() {
            held = None;
        } else if let Some(index) = note.lyric.and_then(Lyric::index) {
            held = Some(index);
        }
        if start + note.duration.ticks() <= tick {
            continue;
        }
        return if note.is_rest() { None } else { held };
    }
    None
}

/// Jeppesen's classes of intelligibility over the syllable onsets of `score` (Style, 1946,
/// pp. 42–43; GC-INTELLIGIBILITY): every tick on which a voice begins a syllable is one case, C
/// where two or more different syllables begin together, B where one syllable begins while
/// another sounding voice holds a different one, A otherwise (one voice alone, or all together).
fn jeppesen_classes(score: &Score) -> [usize; 3] {
    let mut onsets: Vec<(u32, usize, usize)> = Vec::new();
    for (v, voice) in score.voices.iter().enumerate() {
        for (start, note) in voice.timed() {
            if let Some(index) = note
                .lyric
                .and_then(Lyric::index)
                .filter(|_| !note.is_rest())
            {
                onsets.push((start, v, index));
            }
        }
    }
    onsets.sort_unstable();
    let mut classes = [0; 3];
    for group in onsets.chunk_by(|a, b| a.0 == b.0) {
        let tick = group[0].0;
        let syllable = group[0].2;
        if group.iter().any(|&(_, _, s)| s != syllable) {
            classes[2] += 1;
            continue;
        }
        let other = score.voices.iter().enumerate().any(|(v, voice)| {
            !group.iter().any(|&(_, w, _)| w == v)
                && held_syllable(voice, tick).is_some_and(|held| held != syllable)
        });
        classes[usize::from(other)] += 1;
    }
    classes
}

/// The internal cadences of `generated` whose goal falls inside a word: in the bass, and in any
/// voice, the syllable held at the downbeat of the goal is not the last of its word.
fn cadences_in_words(generated: &Generated, text: &Text) -> (usize, usize) {
    let score = &generated.score;
    let per = score.meter.ticks_per_measure();
    let inside = |voice: &Voice, tick: u32| {
        held_syllable(voice, tick).is_some_and(|s| {
            let s = s % text.syllables.len();
            !text.syllables[s]
                .flags
                .contains(fux_core::text::SylFlags::WORD_END)
        })
    };
    let (mut bass, mut any) = (0, 0);
    for cadence in &score.cadences {
        let tick = cadence.measure as u32 * per;
        bass += usize::from(inside(score.voices.last().unwrap(), tick));
        any += usize::from(score.voices.iter().any(|voice| inside(voice, tick)));
    }
    (bass, any)
}

/// The movement lengths of Palestrina's Missa brevis (1570) in measures of 2/2: Kyrie 59
/// breves, Gloria 78, Credo 146, Sanctus 48, Benedictus 38 and its Osanna 12, Agnus Dei I 47
/// and II 57 (research_structure.json).
const BREVIS_LENGTHS: [(Movement, usize); 6] = [
    (Movement::Kyrie, 118),
    (Movement::Gloria, 156),
    (Movement::Credo, 292),
    (Movement::Sanctus, 96),
    (Movement::Benedictus, 100),
    (Movement::Agnus, 208),
];

/// The style of the Missa brevis in the four modes of [`missa_brevis_in_four_modes`] (§ 8,
/// WP15), pooled over the four masses, as the measures of a single mass are too few.
///
/// Asserted:
/// - the internal cadences on the final and on the principal degree P lie within ten points of
///   their shares in the corpus (§ 3.6, weighted by each mass's cadences);
/// - every part that begins together strikes its full chord, its third and from four voices its
///   fifth;
/// - IMI-HELD finds no more cases per 100 voice-measures in the parts without a plan than in the
///   imitative ones.
///
/// Reported (for docs/plan.md, also into the file `MASS_STYLE_OUT` where it is set): movement
/// lengths against the Missa brevis; measures and notes per syllable in Gloria and Credo; held
/// chords per movement; internal cadences by degree; cadences inside words; Jeppesen's classes
/// A/B/C of the syllable onsets of Gloria and Credo; seconds per movement:
/// `MASS_STYLE_OUT=style.md cargo test --release -p fux-core --test mass -- --ignored
/// --nocapture mass_style`.
#[test]
#[ignore = "release: sets four whole masses (or takes them from missa_brevis_in_four_modes)"]
fn mass_style() {
    use std::fmt::Write as _;
    let mut out = String::new();
    let mut line = |text: String| {
        println!("{text}");
        writeln!(out, "{text}").unwrap();
    };
    line("# Style of the Missa brevis in four modes (mass_style)".into());
    line(String::new());
    // Pooled: internal cadences, those on the final and on P, and the corpus shares weighted by
    // each mass's cadences.
    let (mut cadences, mut on_final, mut on_principal) = (0.0, 0.0, 0.0);
    let (mut corpus_final, mut corpus_principal) = (0.0, 0.0);
    let (mut chords, mut full_chords) = (0, 0);
    // IMI-HELD and IMI-SILENCE cases and voice-measures in plain and imitative parts.
    let mut held = [0.0; 2];
    let mut silence = [0.0; 2];
    let mut voice_measures = [0.0; 2];
    let mut pooled_classes = [0; 3];
    for name in FOUR_MODES {
        let set = set(&brevis(name), &format!("Missa brevis, {name}"));
        let plan = &set.mass.plan;
        line(format!(
            "## {name}, principal degree {}",
            plan.principal.letter()
        ));
        line(String::new());
        line("| movement | measures | Missa brevis | held chords | idle s | wall s |".into());
        line("|---|---|---|---|---|---|".into());
        let mut by_degree: Vec<(Step, usize)> = Vec::new();
        let (mut gc_measures, mut gc_syllables, mut gc_attacks, mut gc_voice_syllables) =
            (0, 0, 0, 0);
        let mut classes = [0; 3];
        let (mut in_words_bass, mut in_words_any, mut gc_cadences) = (0, 0, 0);
        for (m, movement) in plan.movements.iter().enumerate() {
            let mut measures = 0;
            let mut held_chords = 0;
            let (mut idle, mut wall) = (0.0, 0.0);
            for (i, (at, part)) in plan.parts().enumerate() {
                if at.movement != m {
                    continue;
                }
                let result = set.mass.results[i].as_ref().unwrap();
                let generated = &result.generated;
                let score = &generated.score;
                let voices = score.voices.len();
                let length = score.measure_count();
                measures += length;
                held_chords += usize::from(matches!(part.join, Join::Seam | Join::Break(_)));
                idle += set.times[i].idle;
                wall += set.times[i].wall;
                let imitative = usize::from(part.opening == Opening::Imitative);
                voice_measures[imitative] += (voices * length) as f64;
                held[imitative] += generated
                    .report
                    .violations
                    .iter()
                    .filter(|v| v.rule == "IMI-HELD")
                    .count() as f64;
                silence[imitative] += generated
                    .report
                    .violations
                    .iter()
                    .filter(|v| v.rule == "IMI-SILENCE")
                    .count() as f64;
                let corpus = tables::corpus_weights(movement.mode, movement.principal);
                let total: f64 = corpus.iter().map(|&(_, w)| f64::from(w)).sum();
                let share = |step: Step| {
                    corpus
                        .iter()
                        .find(|&&(s, _)| s == step)
                        .map_or(0.0, |&(_, w)| f64::from(w) / total)
                };
                for cadence in &score.cadences {
                    cadences += 1.0;
                    on_final += f64::from(u8::from(cadence.target == movement.mode.final_step()));
                    on_principal += f64::from(u8::from(cadence.target == movement.principal));
                    corpus_final += share(movement.mode.final_step());
                    corpus_principal += share(movement.principal);
                    match by_degree.iter_mut().find(|(s, _)| *s == cadence.target) {
                        Some((_, n)) => *n += 1,
                        None => by_degree.push((cadence.target, 1)),
                    }
                }
                if part.opening == Opening::Tutti {
                    chords += 1;
                    let chord = sounding_at(score, part.start);
                    let full =
                        chord.len() == voices && complete(&chord, movement.mode.final_step());
                    full_chords += usize::from(full);
                }
                if matches!(movement.movement, Movement::Gloria | Movement::Credo) {
                    let text = part.sung_text(&set.request.text);
                    gc_measures += length;
                    gc_syllables += part.syllables;
                    gc_attacks += (0..voices).map(|v| attacks(score, v)).sum::<usize>();
                    gc_voice_syllables += voices * part.syllables;
                    for (c, n) in jeppesen_classes(score).into_iter().enumerate() {
                        classes[c] += n;
                    }
                    let (bass, any) = cadences_in_words(generated, &text);
                    in_words_bass += bass;
                    in_words_any += any;
                    gc_cadences += score.cadences.len();
                }
            }
            let model = BREVIS_LENGTHS
                .iter()
                .find(|(movement_of, _)| *movement_of == movement.movement)
                .map_or(0, |&(_, measures)| measures);
            line(format!(
                "| {} | {measures} | {model} | {held_chords} | {idle:.0} | {wall:.0} |",
                movement.movement.title()
            ));
        }
        line(String::new());
        by_degree.sort_by_key(|&(step, n)| (usize::MAX - n, step));
        let all: usize = by_degree.iter().map(|&(_, n)| n).sum();
        line(format!(
            "Internal cadences: {all}, {}",
            by_degree
                .iter()
                .map(|&(step, n)| format!("{} {:.2}", step.letter(), n as f64 / all as f64))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        let total: usize = classes.iter().sum();
        let percent = |n: usize| 100.0 * n as f64 / total.max(1) as f64;
        line(format!(
            "Gloria and Credo: {:.2} measures per syllable (Missa brevis 0.82), {:.2} notes per \
             syllable and voice (Missa brevis 1.04 and 0.98); Jeppesen A/B/C {:.1} / {:.1} / \
             {:.1} % of {total} onsets (Missa brevis 21.9 / 33.5 / 44.6); {in_words_bass} of \
             {gc_cadences} internal cadences inside a word in the bass, {in_words_any} in a voice",
            gc_measures as f64 / gc_syllables as f64,
            gc_attacks as f64 / gc_voice_syllables as f64,
            percent(classes[0]),
            percent(classes[1]),
            percent(classes[2])
        ));
        for (c, n) in classes.into_iter().enumerate() {
            pooled_classes[c] += n;
        }
        line(format!(
            "Time: {:.0} s on an idle machine, {:.0} s wall (load {})",
            set.idle(),
            set.seconds,
            set.loads
                .iter()
                .map(|l| format!("{l:.1}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
        line(String::new());
    }
    let rate = |cases: f64, measures: f64| 100.0 * cases / measures.max(1.0);
    line("## Pooled".into());
    line(String::new());
    line(format!(
        "Internal cadences: {cadences}; on the final {:.3} against the corpus {:.3}, on P {:.3} \
         against {:.3}",
        on_final / cadences,
        corpus_final / cadences,
        on_principal / cadences,
        corpus_principal / cadences
    ));
    line(format!(
        "Opening chords complete: {full_chords} of {chords}"
    ));
    line(format!(
        "IMI-HELD per 100 voice-measures: {:.3} without a plan ({} cases in {} voice-measures), \
         {:.3} imitative ({} in {}); IMI-SILENCE {:.3} and {:.3}",
        rate(held[0], voice_measures[0]),
        held[0],
        voice_measures[0],
        rate(held[1], voice_measures[1]),
        held[1],
        voice_measures[1],
        rate(silence[0], voice_measures[0]),
        rate(silence[1], voice_measures[1])
    ));
    let total: usize = pooled_classes.iter().sum();
    line(format!(
        "Jeppesen A/B/C in Gloria and Credo: {}",
        pooled_classes
            .iter()
            .map(|&n| format!("{:.1} %", 100.0 * n as f64 / total as f64))
            .collect::<Vec<_>>()
            .join(" / ")
    ));
    if let Some(path) = std::env::var_os("MASS_STYLE_OUT") {
        std::fs::write(path, &out).unwrap();
    }
    assert!(
        (on_final - corpus_final).abs() / cadences <= 0.10,
        "final: {:.3} against {:.3}",
        on_final / cadences,
        corpus_final / cadences
    );
    assert!(
        (on_principal - corpus_principal).abs() / cadences <= 0.10,
        "P: {:.3} against {:.3}",
        on_principal / cadences,
        corpus_principal / cadences
    );
    assert_eq!(full_chords, chords);
    assert!(
        rate(held[0], voice_measures[0]) <= rate(held[1], voice_measures[1]),
        "IMI-HELD: {held:?} in {voice_measures:?} voice-measures"
    );
}

/// The times of the Missa brevis in the four modes of [`missa_brevis_in_four_modes`] (§ 8,
/// WP15), natively: per part its wall seconds, its seconds on an idle machine by the references
/// ([`reference`]) and the plan's estimate; per mass and for the four; the parts that would take
/// longest in WebAssembly (× 5.3, the upper Deno factor, against D3's four minutes); and a fit
/// of the estimate's coefficients (§ 3.10, [`tables::estimate_seconds`]) to the idle seconds on
/// the measures each part came out with. Asserted: each mass takes between half and twice its
/// estimate on an idle machine:
/// `cargo test --release -p fux-core --test mass -- --ignored --nocapture mass_timings`.
#[test]
#[ignore = "release: sets four whole masses (or takes them from missa_brevis_in_four_modes)"]
fn mass_timings() {
    // Per part: voices, plain, measures, idle seconds.
    let mut parts: Vec<(usize, bool, f64, f64)> = Vec::new();
    let mut slowest: Vec<(f64, String)> = Vec::new();
    let (mut all_idle, mut all_wall, mut all_estimate) = (0.0, 0.0, 0.0);
    println!(
        "mode\tpart\topening\tvoices\tmeasures\testimated measures\twall\tcpu\tidle\testimate"
    );
    for name in FOUR_MODES {
        let set = set(&brevis(name), &format!("Missa brevis, {name}"));
        let plan = &set.mass.plan;
        let mut estimate = 0.0;
        for (i, (at, part)) in plan.parts().enumerate() {
            let result = set.mass.results[i].as_ref().unwrap();
            let score = &result.generated.score;
            let times = set.times[i];
            let plain = part.opening == Opening::Tutti;
            let measures = score.measure_count();
            estimate += f64::from(part.estimate.seconds);
            println!(
                "{name}\t{}\t{}\t{}\t{measures}\t{}\t{:.1}\t{:.1}\t{:.1}\t{:.1}",
                plan.address(at),
                part.opening.name(Lang::En),
                score.voices.len(),
                part.estimate.measures,
                times.wall,
                times.cpu,
                times.idle,
                part.estimate.seconds
            );
            parts.push((score.voices.len(), plain, measures as f64, times.idle));
            slowest.push((times.idle, format!("{name} {}", plan.address(at))));
        }
        println!(
            "{name}: {:.0} s on an idle machine, estimated {estimate:.0} s; {:.0} s wall",
            set.idle(),
            set.seconds
        );
        all_idle += set.idle();
        all_wall += set.seconds;
        all_estimate += estimate;
        let ratio = set.idle() / estimate;
        assert!((0.5..=2.0).contains(&ratio), "{name}: {ratio:.2}");
    }
    println!(
        "four masses: {:.1} min on an idle machine, estimated {:.1} min; {:.1} min wall",
        all_idle / 60.0,
        all_estimate / 60.0,
        all_wall / 60.0
    );
    slowest.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (idle, part) in slowest.iter().take(6) {
        println!(
            "{part}: {idle:.0} s natively, about {:.0} s in WebAssembly (× 5.3; D3: at most 240 s)",
            idle * 5.3
        );
    }
    // The fit: a and b over a grid, c and the factors of three and five voices as the mean
    // of their logarithmic errors, the least squares of the logarithms.
    let error = |a: f64, b: f64, fit: bool| -> (f64, f64, f64, f64) {
        let base = |n: f64| a * n + b * n * n;
        let mean = |pick: &dyn Fn(usize, bool) -> bool| -> f64 {
            let logs: Vec<f64> = parts
                .iter()
                .filter(|&&(v, plain, _, _)| pick(v, plain))
                .map(|&(_, _, n, t)| (t / base(n)).ln())
                .collect();
            if logs.is_empty() || !fit {
                0.0
            } else {
                logs.iter().sum::<f64>() / logs.len() as f64
            }
        };
        let c = mean(&|v, plain| v == 4 && plain).exp();
        let three = mean(&|v, plain| v == 3 && !plain).exp();
        let five = mean(&|v, plain| v == 5 && !plain).exp();
        let squares: f64 = parts
            .iter()
            .map(|&(v, plain, n, t)| {
                let factor = match v {
                    3 => three,
                    5 => five,
                    _ => 1.0,
                } * if plain { c } else { 1.0 };
                (t / (base(n) * factor)).ln().powi(2)
            })
            .sum();
        ((squares / parts.len() as f64).sqrt().exp(), c, three, five)
    };
    let mut best = (f64::INFINITY, 0.0, 0.0);
    for i in 0..=200 {
        for j in 0..=150 {
            let (a, b) = (0.001 * f64::from(i), 0.0001 * f64::from(j));
            if a + b == 0.0 {
                continue;
            }
            let (rms, ..) = error(a, b, true);
            if rms < best.0 {
                best = (rms, a, b);
            }
        }
    }
    let (rms, c, three, five) = error(best.1, best.2, true);
    println!(
        "fit on the measures set: a {:.3}, b {:.4}, plain × {c:.2}, three voices × {three:.2}, \
         five × {five:.2}; error × {rms:.2} rms",
        best.1, best.2
    );
    let tables_error: f64 = parts
        .iter()
        .map(|&(v, plain, n, t)| (t / tables::estimate_seconds(n, v, plain)).ln().powi(2))
        .sum::<f64>();
    println!(
        "the estimate of tables.rs on the measures set: error × {:.2} rms",
        (tables_error / parts.len() as f64).sqrt().exp()
    );
}

/// The seconds of the reference setting on an idle machine: Kyrie I for S A T B in the Dorian
/// mode without imitation, seed 1, under Fux's profile (release, eight cores). 3.0 s from phase
/// 12 on; since the faster search after the plan (docs/naechste-schritte.md, Rang 1) 1.7 s, from
/// three paired runs of `fux free --text kyrie --voices 4 --mode dorisch --imitation aus --seed
/// 1` on a nearly idle machine (1.67 to 1.70 s, against 2.94 to 2.96 s before).
const REFERENCE_IDLE: f64 = 1.7;

/// The CPU seconds of this process so far, user and system, of all its threads (Linux, in clock
/// ticks of 1/100 s; NaN elsewhere).
fn cpu_seconds() -> f64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let fields: Vec<&str> = stat
        .rsplit_once(')')
        .map_or("", |(_, rest)| rest)
        .split_whitespace()
        .collect();
    let ticks = |i: usize| fields.get(i).and_then(|s| s.parse::<f64>().ok());
    match (ticks(11), ticks(12)) {
        (Some(user), Some(system)) => (user + system) / 100.0,
        _ => f64::NAN,
    }
}

/// The load average of the last minute (Linux; NaN elsewhere).
fn load() -> f64 {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse().ok())
        .unwrap_or(f64::NAN)
}

/// The wall and CPU seconds of the reference setting ([`REFERENCE_IDLE`]). A busy machine
/// inflates the CPU seconds of a search far less than its wall time, and about alike for every
/// search: the CPU seconds of a part times [`REFERENCE_IDLE`] over those of a reference set next
/// to it are the seconds the part takes on an idle machine (the method of phase 14a's first
/// measurements, good to about ±15 %).
fn reference() -> (f64, f64) {
    let text = part_text("kyrie", &TextOptions::default()).unwrap();
    let request = FreeRequest {
        seed: 1,
        text: Some(FreeText::new(Arc::new(text))),
        ..FreeRequest::new(mode("dorian"), choir(4), 0)
    };
    let (clock, cpu) = (Instant::now(), cpu_seconds());
    generate_free(&request, &Profile::fux()).unwrap();
    (clock.elapsed().as_secs_f64(), cpu_seconds() - cpu)
}

/// Whether two pitches sounding together are dissonant: a second, a seventh, a tritone or an
/// altered interval, or a fourth whose lower note is `bass`, the lowest sounding pitch (as
/// tests/palestrina.rs counts the occasions of IMI-HELD).
fn dissonant_pair(a: Pitch, b: Pitch, bass: Pitch) -> bool {
    let (low, high) = if (a.diatonic(), a.midi()) <= (b.diatonic(), b.midi()) {
        (a, b)
    } else {
        (b, a)
    };
    let interval = fux_core::Interval::between(low, high).simple();
    let fourth = interval.steps == 3 && interval.semitones == 5;
    matches!(interval.steps, 1 | 6)
        || interval.is_tritone()
        || (fourth && low == bass)
        || !(interval.is_consonant() || fourth || interval.is_tritone())
}

/// The occasions of IMI-HELD in `score`: attacks dissonant with a note tied over the downbeat
/// before them, on the downbeat (`struck`) and after it (`passing`), as tests/palestrina.rs counts
/// them in Palestrina's movements.
fn held_occasions(score: &Score) -> (usize, usize) {
    let analysis = Analysis::new(score);
    let per = analysis.per_measure;
    let (mut struck, mut later) = (0, 0);
    for slice in &analysis.slices {
        let Some(bass) = slice.lowest() else { continue };
        let downbeat = slice.tick / per * per;
        for (v, note) in slice.sounding.iter().enumerate() {
            let Some(note) = note.filter(|s| s.start == slice.tick && s.end - s.start > 1) else {
                continue;
            };
            for (_, held) in slice.sounding.iter().enumerate().filter(|&(u, _)| u != v) {
                let Some(held) = held.filter(|h| h.start < downbeat) else {
                    continue;
                };
                if dissonant_pair(note.pitch, held.pitch, bass.1.pitch) {
                    if slice.tick == downbeat {
                        struck += 1;
                    } else {
                        later += 1;
                    }
                }
            }
        }
    }
    (struck, later)
}

/// The cases of a parameter of a rule in the report of a setting.
fn cases(generated: &Generated, rule: &str, param: &'static str) -> usize {
    generated
        .report
        .violations
        .iter()
        .filter(|v| v.rule == rule && v.detail == Detail::Param(param))
        .count()
}

/// How strict the profile of a mass is on held dissonances in the parts that begin together
/// (D7, WP15): every such part of the Missa brevis in the four modes of
/// [`missa_brevis_in_four_modes`], set by `generate_part` under the profile of the mass with
/// IMI-HELD `unplanned_passing` at P3, the level of `passing` (D7), and at P2, one after the
/// other. Per part and level: found, the attempt, the idle seconds by the reference set after the
/// part, the cases of `unplanned_struck` and `unplanned_passing` with their occasions (attacks
/// dissonant with a note tied over the downbeat, on it and after it), and the measures. Palestrina
/// without a plan: 12 of 40 381 and 63 of 14 650 occasions in 519 764 voice-measures (0.012
/// cases of `passing` per 100). Rows begin with `row`:
/// `cargo test --release -p fux-core --test mass -- --ignored --nocapture overlay_strictness`.
#[test]
#[ignore = "release: sets the parts that begin together twice, about half an hour"]
fn overlay_strictness() {
    let p3 = fux_core::mass::profile(&Profile::fux());
    let mut p2 = p3.clone();
    p2.apply_override("IMI-HELD.unplanned_passing=P2").unwrap();
    println!(
        "row\tmode\tpart\tlevel\tfound\tattempt\tseconds\tcpu\tref_cpu\tidle\tmeasures\tvoices\tcost\tstruck\tpassing\tstruck_occasions\tpassing_occasions\tload"
    );
    // Per level: parts found, idle seconds, voice-measures, cases, occasions, later attempts.
    let mut totals: Vec<(&str, [f64; 6])> = vec![("P3", [0.0; 6]), ("P2", [0.0; 6])];
    for name in [
        "dorisch",
        "phrygisch",
        "mixolydisch",
        "ionisch-transponiert",
    ] {
        let request = MassRequest {
            mode: Some(mode(name)),
            seed: 1,
            ..MassRequest::default()
        };
        let plan = plan_mass(&request).unwrap();
        for (at, part) in plan.parts() {
            if part.opening != Opening::Tutti {
                continue;
            }
            let mut runs = Vec::new();
            for (level, profile) in [("P3", &p3), ("P2", &p2)] {
                let (clock, cpu) = (Instant::now(), cpu_seconds());
                let result = generate_part(&request, &plan, at, profile, None, None);
                let (seconds, cpu) = (clock.elapsed().as_secs_f64(), cpu_seconds() - cpu);
                runs.push((level, result, seconds, cpu));
            }
            let (_, reference_cpu) = reference();
            for (level, result, seconds, cpu) in runs {
                let idle = cpu * REFERENCE_IDLE / reference_cpu;
                let address = plan.address(at);
                let Ok(result) = result else {
                    println!(
                        "row\t{name}\t{address}\t{level}\t0\t-\t{seconds:.2}\t{cpu:.1}\t{reference_cpu:.1}\t{idle:.2}\t\t\t\t\t\t\t\t{:.1}",
                        load()
                    );
                    continue;
                };
                let generated = &result.generated;
                let score = &generated.score;
                let measures = score.measure_count();
                let (struck_occasions, passing_occasions) = held_occasions(score);
                let struck = cases(generated, "IMI-HELD", "unplanned_struck");
                let passing = cases(generated, "IMI-HELD", "unplanned_passing");
                println!(
                    "row\t{name}\t{address}\t{level}\t1\t{}\t{seconds:.2}\t{cpu:.1}\t{reference_cpu:.1}\t{idle:.2}\t{measures}\t{}\t{}\t{struck}\t{passing}\t{struck_occasions}\t{passing_occasions}\t{:.1}",
                    result.attempt,
                    score.voices.len(),
                    generated.cost,
                    load()
                );
                let total = &mut totals.iter_mut().find(|(l, _)| *l == level).unwrap().1;
                for (sum, value) in total.iter_mut().zip([
                    1.0,
                    idle,
                    (measures * score.voices.len()) as f64,
                    passing as f64,
                    passing_occasions as f64,
                    f64::from(result.attempt),
                ]) {
                    *sum += value;
                }
            }
        }
    }
    for (level, [parts, idle, voice_measures, passing, occasions, attempts]) in totals {
        println!(
            "{level}: {parts} parts found, {idle:.0} s idle, {attempts} attempts beyond the first, \
             {passing} cases of unplanned_passing in {occasions} occasions ({:.2} %), {:.3} per \
             100 voice-measures",
            100.0 * passing / occasions.max(1.0),
            100.0 * passing / voice_measures.max(1.0)
        );
    }
}
