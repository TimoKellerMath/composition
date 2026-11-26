//! The forms of the mass (MASS-FORM, docs/design/phase-14/spec.md § 3.1): the sections of each
//! movement with their texts, Breaks and titles, and what a book prints under a movement's
//! heading.
//!
//! **brevis** (default) follows the Missa brevis of 1570 and the short four-voice masses: the
//! Gloria in one section with a Break at Qui tollis; the Credo undivided with Breaks at Et
//! incarnatus and at the Crucifixus (50 | 12 | 84 breves in the model); Sanctus, Pleni and Osanna
//! through-composed; a reduced Benedictus followed by a new Osanna in all voices. An undivided
//! Credo occurs only in four-voice masses (8 of 86), and none of the 12 fast four-voice Credos
//! has a separate Crucifixus; an undivided Sanctus is followed by a new Osanna II in 31 of 33
//! masses.
//!
//! **normal** follows the medians of the four-voice masses: Et in terra ‖ Qui tollis (two
//! sections in 52–55 of 85 Glorias, by the study and its verification); Patrem with a Break at
//! Et incarnatus ‖ a reduced Crucifixus up to „non erit finis“ (followed by Et in Spiritum in 40
//! of 69 masses) ‖ Et in Spiritum to the Amen; Sanctus ‖ a reduced Pleni ‖ Osanna, and after the
//! reduced Benedictus „Osanna ut supra“ (25 of 32 three-part Sanctus movements).
//!
//! Both have Kyrie I ‖ Christe ‖ Kyrie II (84 of 86 masses) and Agnus Dei I ‖ an enlarged Agnus
//! Dei II (72 of 86).

use super::{Movement, Preset};
use crate::text::ordinary::{CREDO_INTONATION, GLORIA_INTONATION};

/// How a section is scored against the choir of the mass (MASS-SCORING,
/// [`super::tables::scoring`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scoring {
    /// Every voice of the choir.
    Full,
    /// The Christe: four voices of a six-voice choir, otherwise every voice.
    Christe,
    /// A reduced section (the Benedictus; Crucifixus and Pleni in `normal`): without the bass.
    Reduced,
    /// Agnus Dei II: a voice more in four and five voices.
    Enlarged,
}

/// A text of the Ordinary that a section sings, by its id in `ordinary.txt`, with the label of
/// the Break that begins it inside its section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub id: &'static str,
    pub label: Option<&'static str>,
}

/// A section of a movement as a form has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionForm {
    /// Its id, by which addresses name it.
    pub id: &'static str,
    /// Its title as an edition prints it above the section.
    pub title: &'static str,
    /// The texts it sings, in order.
    pub pieces: &'static [Piece],
    pub scoring: Scoring,
    /// It begins in all voices together, where a section would otherwise begin in imitation:
    /// the Osanna II of the Missa brevis opens chordally (§ 3.7).
    pub tutti: bool,
    /// „Osanna ut supra“: the id of the Sanctus' section it repeats, without a search of its
    /// own.
    pub ut_supra: Option<&'static str>,
}

/// A movement as a form has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MovementForm {
    pub movement: Movement,
    pub sections: &'static [SectionForm],
    /// The priest's intonation that precedes the polyphony, as `text::ordinary` has it.
    pub intonation: Option<&'static str>,
    /// Remarks for the performers, in German and English.
    pub notes: &'static [(&'static str, &'static str)],
}

/// How the Kyrie is sung: each section stands for three invocations.
const KYRIE_NOTE: (&str, &str) = (
    "Kyrie I, Christe und Kyrie II stehen je für drei Anrufungen (Wiederholung oder Wechsel mit \
     dem Choral nach Ortsbrauch).",
    "Kyrie I, Christe and Kyrie II each stand for three invocations (sung again, or alternating \
     with the chant, as local custom has it).",
);

/// How the Agnus Dei is sung: its first section serves for two invocations.
const AGNUS_NOTE: (&str, &str) = (
    "Agnus Dei I gilt für die erste und zweite Anrufung, Agnus Dei II (dona nobis pacem) für \
     die dritte.",
    "Agnus Dei I serves for the first and second invocations, Agnus Dei II (dona nobis pacem) \
     for the third.",
);

/// A text without a Break.
const fn piece(id: &'static str) -> Piece {
    Piece { id, label: None }
}

/// A text that begins with a Break labelled `label`.
const fn after_break(id: &'static str, label: &'static str) -> Piece {
    Piece {
        id,
        label: Some(label),
    }
}

/// A section that begins in imitation and sets its own text.
const fn section(
    id: &'static str,
    title: &'static str,
    pieces: &'static [Piece],
    scoring: Scoring,
) -> SectionForm {
    SectionForm {
        id,
        title,
        pieces,
        scoring,
        tutti: false,
        ut_supra: None,
    }
}

const KYRIE: MovementForm = MovementForm {
    movement: Movement::Kyrie,
    sections: &[
        section("kyrie", "Kyrie I", &[piece("kyrie")], Scoring::Full),
        section("christe", "Christe", &[piece("christe")], Scoring::Christe),
        section("kyrie2", "Kyrie II", &[piece("kyrie2")], Scoring::Full),
    ],
    intonation: None,
    notes: &[KYRIE_NOTE],
};

const AGNUS: MovementForm = MovementForm {
    movement: Movement::Agnus,
    sections: &[
        section("agnus", "Agnus Dei I", &[piece("agnus")], Scoring::Full),
        section(
            "agnus3",
            "Agnus Dei II",
            &[piece("agnus3")],
            Scoring::Enlarged,
        ),
    ],
    intonation: None,
    notes: &[AGNUS_NOTE],
};

const BREVIS: [MovementForm; 6] = [
    KYRIE,
    MovementForm {
        movement: Movement::Gloria,
        sections: &[section(
            "gloria",
            "Et in terra pax",
            &[piece("gloria"), after_break("qui-tollis", "Qui tollis")],
            Scoring::Full,
        )],
        intonation: Some(GLORIA_INTONATION),
        notes: &[],
    },
    MovementForm {
        movement: Movement::Credo,
        sections: &[section(
            "credo",
            "Patrem omnipotentem",
            &[
                piece("credo"),
                after_break("incarnatus", "Et incarnatus est"),
                after_break("crucifixus", "Crucifixus"),
                piece("resurrexit"),
                piece("spiritum"),
            ],
            Scoring::Full,
        )],
        intonation: Some(CREDO_INTONATION),
        notes: &[],
    },
    MovementForm {
        movement: Movement::Sanctus,
        sections: &[section(
            "sanctus",
            "Sanctus",
            &[piece("sanctus"), piece("pleni"), piece("osanna")],
            Scoring::Full,
        )],
        intonation: None,
        notes: &[],
    },
    MovementForm {
        movement: Movement::Benedictus,
        sections: &[
            section(
                "benedictus",
                "Benedictus",
                &[piece("benedictus")],
                Scoring::Reduced,
            ),
            SectionForm {
                tutti: true,
                ..section("osanna2", "Osanna", &[piece("osanna2")], Scoring::Full)
            },
        ],
        intonation: None,
        notes: &[],
    },
    AGNUS,
];

const NORMAL: [MovementForm; 6] = [
    KYRIE,
    MovementForm {
        movement: Movement::Gloria,
        sections: &[
            section(
                "gloria",
                "Et in terra pax",
                &[piece("gloria")],
                Scoring::Full,
            ),
            section(
                "qui-tollis",
                "Qui tollis",
                &[piece("qui-tollis")],
                Scoring::Full,
            ),
        ],
        intonation: Some(GLORIA_INTONATION),
        notes: &[],
    },
    MovementForm {
        movement: Movement::Credo,
        sections: &[
            section(
                "credo",
                "Patrem omnipotentem",
                &[
                    piece("credo"),
                    after_break("incarnatus", "Et incarnatus est"),
                ],
                Scoring::Full,
            ),
            section(
                "crucifixus",
                "Crucifixus",
                &[piece("crucifixus"), piece("resurrexit")],
                Scoring::Reduced,
            ),
            section(
                "spiritum",
                "Et in Spiritum Sanctum",
                &[piece("spiritum")],
                Scoring::Full,
            ),
        ],
        intonation: Some(CREDO_INTONATION),
        notes: &[],
    },
    MovementForm {
        movement: Movement::Sanctus,
        sections: &[
            section("sanctus", "Sanctus", &[piece("sanctus")], Scoring::Full),
            section(
                "pleni",
                "Pleni sunt caeli",
                &[piece("pleni")],
                Scoring::Reduced,
            ),
            section("osanna", "Osanna", &[piece("osanna")], Scoring::Full),
        ],
        intonation: None,
        notes: &[],
    },
    MovementForm {
        movement: Movement::Benedictus,
        sections: &[
            section(
                "benedictus",
                "Benedictus",
                &[piece("benedictus")],
                Scoring::Reduced,
            ),
            SectionForm {
                ut_supra: Some("osanna"),
                ..section(
                    "osanna2",
                    "Osanna ut supra",
                    &[piece("osanna2")],
                    Scoring::Full,
                )
            },
        ],
        intonation: None,
        notes: &[],
    },
    AGNUS,
];

/// The form of a movement in a preset.
pub fn form(preset: Preset, movement: Movement) -> &'static MovementForm {
    let forms: &'static [MovementForm; 6] = match preset {
        Preset::Brevis => &BREVIS,
        Preset::Normal => &NORMAL,
    };
    &forms[movement as usize]
}

/// The movement whose text a section or a text of the Ordinary belongs to, by its id
/// (`incarnatus` belongs to the Credo); `None` for an unknown id.
pub fn movement_of(id: &str) -> Option<Movement> {
    Some(match id {
        "kyrie" | "christe" | "kyrie2" => Movement::Kyrie,
        "gloria" | "qui-tollis" => Movement::Gloria,
        "credo" | "incarnatus" | "crucifixus" | "resurrexit" | "spiritum" => Movement::Credo,
        "sanctus" | "pleni" | "osanna" => Movement::Sanctus,
        "benedictus" | "osanna2" => Movement::Benedictus,
        "agnus" | "agnus3" => Movement::Agnus,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{TextOptions, ordinary};

    /// Every movement of both forms sings every text of its movement in the order of
    /// `ordinary.txt`, each once; only the Osanna ut supra repeats the Sanctus' Osanna. The
    /// intonation of a movement is the one `ordinary` gives before its first text.
    #[test]
    fn forms_sing_the_whole_ordinary() {
        let options = TextOptions::default();
        for preset in [Preset::Brevis, Preset::Normal] {
            let mut sung: Vec<&str> = Vec::new();
            for movement in Movement::ALL {
                let form = form(preset, movement);
                assert_eq!(form.movement, movement);
                let first = form.sections[0].pieces[0].id;
                assert_eq!(form.intonation, ordinary::intonation(first), "{first}");
                for section in form.sections {
                    assert_eq!(movement_of(section.id), Some(movement), "{}", section.id);
                    if section.ut_supra.is_some() {
                        continue;
                    }
                    for piece in section.pieces {
                        assert_eq!(movement_of(piece.id), Some(movement), "{}", piece.id);
                        sung.push(piece.id);
                    }
                    // A Break never begins a section; it lies inside one.
                    assert_eq!(section.pieces[0].label, None, "{}", section.id);
                }
            }
            let all: Vec<&str> = ordinary::sections(&options)
                .iter()
                .map(|text| text.id.as_str())
                .collect();
            let expected: Vec<&str> = match preset {
                Preset::Brevis => all.clone(),
                Preset::Normal => all.iter().copied().filter(|&id| id != "osanna2").collect(),
            };
            assert_eq!(sung, expected, "{preset:?}");
        }
        assert_eq!(movement_of("eigen"), None);
    }
}
