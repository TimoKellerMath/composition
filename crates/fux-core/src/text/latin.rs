//! Latin words: their syllables, their accent and the plain forms used for lookups.
//!
//! [`syllabify`] divides a word as the Graduale Romanum of 1961 and the Liber Usualis print it
//! and finds its stress, after the rules of the Liber (1961, pp. xxxv–xxxix), of Solesmes
//! ("Distinction des syllabes dans les mots latins", Études grégoriennes, 2016) and of Allen &
//! Greenough (§§ 2–12). A lexicon comes first: every word of the Ordinary with the books'
//! division and accent, and a few words whose accent no rule finds (María, miserére, …). Other
//! words go through four steps.
//!
//! 1. **Letters.** y is a vowel, k counts as c. u is a consonant in qu and in ngu before a vowel
//!    (san-guis), otherwise a vowel (ar-gu-o, su-á-vis). i is a consonant at the start of a word
//!    before a vowel (Je-su, Ie-su), between vowels (cu-jus, cu-ius, al-le-lú-ia) and after a
//!    prefix before a vowel (ád-iu-va, in-iú-ri-a).
//! 2. **Nuclei.** ae, oe, au, ay, æ and œ are one nucleus each, eu only in a few words (Eu-ge,
//!    heu), ei only in hei, ui never; every other pair of vowels is two (De-us, e-lé-i-son, cu-i,
//!    hu-ic). Lists keep ae, oe and au apart in a few words and stems (Is-ra-el, Mi-cha-e-lem,
//!    po-e-tae, a-e-ris), and a diaeresis does so anywhere (po-ë-ta, Mi-cha-ël, I-sa-ï-as).
//! 3. **Boundaries.** Some prefixes keep their consonant (Ad-o-rá-mus, ex-áu-di, in-i-mí-ci,
//!    sús-ci-pe), and so do et in ét-i-am and pot in o-mní-pot-ens. Otherwise a single
//!    consonant goes to the next syllable, doubled consonants are split, x before a consonant
//!    stays, and of a longer group the longest end that can begin a word goes to the next
//!    syllable: a mute and a liquid (Pa-tris, Ec-clé-si-am), gn, sc, sp, st, scr, spl, spr, str,
//!    and the Greek ct, pt, mn, pn, ps, tm, cn, bd (San-ctus, pro-pter, o-mnis, i-pse). ch, ph,
//!    th, rh, qu and a consonantal gu count as one consonant. sm and squ are not among these
//!    groups: the books print ba-ptís-ma, us-que and u-nus-quís-que.
//! 4. **Stress.** A printed acute wins. Words in -que are stressed on the syllable before the
//!    enclitic (Fi-li-ó-que), except a few that keep their own stress (í-ta-que); -ne and -ve
//!    are mostly part of the word (Dó-mi-ne) and count as no enclitic. A disyllable is stressed
//!    on its first syllable, a longer word on a heavy penult, otherwise on the antepenult. The
//!    weight comes from the letters, never from the printed division: a diphthong, x, z or two
//!    consonants make the penult heavy (cae-lé-stis, cru-ci-fí-xus), a vowel or h after it makes
//!    it light (Gló-ri-a). Before a single consonant, or before a mute and a liquid, the letters
//!    do not show its length. Then endings that Latin morphology fixes decide (-ámus, -átus,
//!    -iónem, -ávit, …, and le-vá-vi-mus); otherwise the penult counts as short, and the result
//!    is marked uncertain (Ré-gi-na, which should be Re-gí-na).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::ordinary;

/// The combining acute accent, U+0301.
pub(crate) const COMBINING_ACUTE: char = '\u{301}';

/// The combining diaeresis, U+0308.
pub(crate) const COMBINING_DIAERESIS: char = '\u{308}';

/// Vowels with their precomposed acute forms; Unicode has none for œ.
const ACUTES: [(char, char); 14] = [
    ('a', 'á'),
    ('e', 'é'),
    ('i', 'í'),
    ('o', 'ó'),
    ('u', 'ú'),
    ('y', 'ý'),
    ('A', 'Á'),
    ('E', 'É'),
    ('I', 'Í'),
    ('O', 'Ó'),
    ('U', 'Ú'),
    ('Y', 'Ý'),
    ('æ', 'ǽ'),
    ('Æ', 'Ǽ'),
];

/// Vowels with a diaeresis, which marks a hiatus: the vowel never joins the one before it
/// (po-ë-ta, a-ër, Em-ma-üs).
const DIAERESES: [(char, char); 6] = [
    ('e', 'ë'),
    ('i', 'ï'),
    ('u', 'ü'),
    ('E', 'Ë'),
    ('I', 'Ï'),
    ('U', 'Ü'),
];

/// Lowercase vowel letters without accent.
const VOWELS: &str = "aeiouyæœ";

/// Words in which ae, oe or au are two vowels: the forms of aer (a-e-ris; aerumna has the
/// diphthong), and some names.
const HIATUS_WORDS: [&str; 13] = [
    "aer", "aeris", "aeri", "aerem", "aere", "aera", "aerum", "aeribus", "noe", "noemi", "ioel",
    "emmaus", "esau",
];

/// Beginnings of words in which ae, oe and au are two vowels; an ending keeps its diphthong
/// (Mi-cha-e-lem, Is-ra-e-li-tae, po-e-tae, co-e-git).
const HIATUS_STEMS: [&str; 9] = [
    "israel", "ismael", "michael", "raphael", "poet", "poem", "poes", "coeg", "coem",
];

/// Beginnings that look like a stem of [`HIATUS_STEMS`] but have the diphthong: coe-me-té-ri-um.
const NOT_HIATUS: [&str; 2] = ["coemeter", "coemiter"];

/// Words in which eu is one nucleus.
const EU_DIPHTHONG: [&str; 7] = ["euge", "heu", "heus", "ceu", "seu", "neu", "neuter"];

/// Prefixes after which i is a consonant before a vowel: ád-iu-va, in-iú-ri-a, sub-ié-ctus.
const J_PREFIXES: [&str; 8] = ["ab", "ad", "ob", "sub", "in", "con", "per", "circum"];

/// Prefixes that keep their last consonant, with the letters that must follow them (`*` is any
/// vowel): Ad-o-rá-mus, ex-áu-di, in-i-mí-ci, per-é-git, Red-ém-ptor, sub-í-re, ab-est,
/// ob-oe-dí-re, ét-i-am, sús-ci-pe, dis-cér-ne. Quó-ni-am is no such case.
const PREFIXES: [(&str, &str); 11] = [
    ("ad", "o"),
    ("ex", "*"),
    ("in", "i"),
    ("per", "e"),
    ("red", "e"),
    ("sub", "i"),
    ("ab", "*"),
    ("ob", "*"),
    ("et", "iam"),
    ("sus", "c"),
    ("dis", "c"),
];

/// Beginnings that look like a prefix but are none: di-scí-pu-lus, di-sci-plí-na and dí-sci-te
/// come from disco.
const NO_PREFIX: [&str; 1] = ["disci"];

/// Words in -que without the enclitic, with their own stressed syllable: ítaque, dénique,
/// úndique, útique, ubíque, quóque, néque, átque, úsque, quísque, quicúmque.
const LEXICAL_QUE: [(&str, usize); 11] = [
    ("itaque", 0),
    ("denique", 0),
    ("undique", 0),
    ("utique", 0),
    ("ubique", 1),
    ("quoque", 0),
    ("neque", 0),
    ("atque", 0),
    ("usque", 0),
    ("quisque", 0),
    ("quicumque", 1),
];

/// Endings with a penult long by nature: -ámus, -átis, -áre, -ári, -átur, -átus, -áta, -átum,
/// -áto, -árum, -órum, -iónem, -iónis, -ióne, -tórem, -tóris, -tátis, -tátem, -útem, -útis,
/// -útus, -túrus, -túri, -túra, -úras, -álem, -ális, and the perfects -ávi, -ávit, -avérunt,
/// -ívi, -ívit. They decide an open penult whose length the letters do not show (pec-cá-ta,
/// sa-lú-tem, le-vá-vi, au-dí-vit); -ere, -inus, -ilis, -itus and -edit are left out on purpose
/// (mi-se-ré-re, but dí-ce-re).
const LONG_ENDINGS: [&str; 32] = [
    "amus", "atis", "are", "ari", "atur", "atus", "ata", "atum", "ato", "arum", "orum", "ionem",
    "ionis", "ione", "torem", "toris", "tatis", "tatem", "utem", "utis", "utus", "turus", "turi",
    "tura", "uras", "alem", "alis", "avi", "avit", "averunt", "ivi", "ivit",
];

/// Endings with a penult short by nature after a long vowel: the perfect's -imus after -áv-
/// (le-vá-vi-mus).
const SHORT_PENULT_ENDINGS: [&str; 1] = ["avimus"];

/// Words of the lexicon besides those of the Ordinary. The rules miss the accent of most: a long
/// vowel before a vowel (Ma-rí-a, di-é-i), a long open penult (mi-se-ré-re, Pro-phé-tas), or a
/// short penult that an ending takes for long (Je-rú-sa-lem, Sáb-ba-tum). The others are
/// special cases that the rules get right too (ju-di-cá-re, cú-i, hú-ic, Ís-ra-el, Mí-cha-el).
const LEXICON_EXTRA: [&str; 18] = [
    "Ma-rí-a",
    "mi-se-ré-re",
    "Pro-phé-tas",
    "pro-cé-dit",
    "ju-di-cá-re",
    "cú-i",
    "hú-ic",
    "Ís-ra-el",
    "Mí-cha-el",
    "di-é-i",
    "il-lí-us",
    "i-psí-us",
    "u-ní-us",
    "to-tí-us",
    "Je-rú-sa-lem",
    "Sáb-ba-tum",
    "Sáb-ba-to",
    "Sáb-ba-ta",
];

/// A word divided into syllables, with its stress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Syllabified {
    /// The syllables with the letters of the input: case and acutes are kept and no accent is
    /// added, so together they spell the word.
    pub syllables: Vec<String>,
    /// Index of the stressed syllable; `None` for a monosyllable.
    pub stress: Option<usize>,
    /// The stress rests on a guess: the length of the penult is unknown and taken as short.
    pub uncertain: bool,
}

/// Divides a word into syllables and finds its stress, in the style of the Graduale of 1961. The
/// word comes without hyphens and punctuation; it may contain capitals, j, the ligatures æ and
/// œ, and an acute, which marks the stress.
///
/// Words of the lexicon (the Ordinary and a few more) get the books' division and accent,
/// whatever their case and accents, with j read as i and æ as ae. Other words are divided by
/// rule, as the module documentation describes.
pub fn syllabify(word: &str) -> Syllabified {
    divide(word, Some(lexicon()))
}

/// Puts an acute on the nucleus of a syllable: on the e of ae and oe (saé-cu-la), otherwise on
/// its first vowel, passing over a consonantal i or u before another vowel (Quó-ni-am). A
/// syllable that already has an acute stays as it is.
pub fn add_acute(syllable: &str) -> String {
    let target = match nucleus(syllable) {
        Some(target) if !has_acute(syllable) => target,
        _ => return syllable.to_string(),
    };
    let mut out = String::with_capacity(syllable.len() + 2);
    for (i, c) in syllable.chars().enumerate() {
        match precomposed_acute(c) {
            _ if i != target => out.push(c),
            Some(accented) => out.push(accented),
            None => {
                out.push(c);
                out.push(COMBINING_ACUTE);
            }
        }
    }
    out
}

/// Removes every acute: á becomes a, ǽ becomes æ, and a combining acute is dropped.
pub fn strip_acute(s: &str) -> String {
    s.chars()
        .filter(|&c| c != COMBINING_ACUTE)
        .map(without_acute)
        .collect()
}

/// The plain form for lookups: lowercase, without acutes and diaereses, j as i, and the
/// ligatures æ and œ as ae and oe. Other characters are kept.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s
        .chars()
        .filter(|&c| c != COMBINING_ACUTE && c != COMBINING_DIAERESIS)
    {
        push_plain(&mut out, c);
    }
    out
}

/// The vowel of a syllable that carries its accent, as an index of its characters: the e of
/// ae and oe, otherwise the first vowel that is not a consonantal i or u before another vowel.
/// A vowel with a diaeresis is no part of a diphthong and never consonantal.
pub(crate) fn nucleus(syllable: &str) -> Option<usize> {
    let written: Vec<char> = syllable.chars().collect();
    let chars: Vec<char> = written.iter().map(|&c| lower_plain(c)).collect();
    let vowel = |i: usize| chars.get(i).is_some_and(|&c| VOWELS.contains(c));
    let apart = |i: usize| {
        written.get(i).is_some_and(|&c| is_diaeresis(c))
            || written.get(i + 1) == Some(&COMBINING_DIAERESIS)
    };
    let glide = |i: usize| matches!(chars[i], 'i' | 'u') && !apart(i) && vowel(i + 1);
    let first = (0..chars.len()).find(|&i| vowel(i) && !glide(i))?;
    if matches!(chars[first], 'a' | 'o') && chars.get(first + 1) == Some(&'e') && !apart(first + 1)
    {
        Some(first + 1)
    } else {
        Some(first)
    }
}

/// Whether the character is an acute: a precomposed accented vowel or the combining acute.
pub(crate) fn is_acute(c: char) -> bool {
    c == COMBINING_ACUTE || ACUTES.iter().any(|&(_, accented)| accented == c)
}

/// Whether the text contains an acute.
pub(crate) fn has_acute(s: &str) -> bool {
    s.chars().any(is_acute)
}

/// Whether the character is a vowel letter, with or without an acute or a diaeresis: a, e, i,
/// o, u, y, æ or œ in either case, ë, ï or ü.
pub(crate) fn is_vowel(c: char) -> bool {
    VOWELS.contains(lower_plain(c))
}

/// The precomposed acute form of a vowel, if Unicode has one.
pub(crate) fn precomposed_acute(c: char) -> Option<char> {
    ACUTES
        .iter()
        .find(|&&(plain, _)| plain == c)
        .map(|&(_, accented)| accented)
}

/// The precomposed form of e, i or u with a diaeresis.
pub(crate) fn precomposed_diaeresis(c: char) -> Option<char> {
    DIAERESES
        .iter()
        .find(|&&(plain, _)| plain == c)
        .map(|&(_, marked)| marked)
}

/// Whether the character is a vowel with a diaeresis: ë, ï or ü in either case.
pub(crate) fn is_diaeresis(c: char) -> bool {
    DIAERESES.iter().any(|&(_, marked)| marked == c)
}

/// The letter without its precomposed acute.
fn without_acute(c: char) -> char {
    ACUTES
        .iter()
        .find(|&&(_, accented)| accented == c)
        .map_or(c, |&(plain, _)| plain)
}

/// The letter without its precomposed acute or diaeresis.
fn without_marks(c: char) -> char {
    let c = without_acute(c);
    DIAERESES
        .iter()
        .find(|&&(_, marked)| marked == c)
        .map_or(c, |&(plain, _)| plain)
}

/// The lowercase letter without accent or diaeresis, for comparisons.
fn lower_plain(c: char) -> char {
    let plain = without_marks(c);
    plain.to_lowercase().next().unwrap_or(plain)
}

/// Appends the plain form of a character (see [`normalize`]).
fn push_plain(out: &mut String, c: char) {
    for c in without_marks(c).to_lowercase() {
        match c {
            'j' => out.push('i'),
            'æ' => out.push_str("ae"),
            'œ' => out.push_str("oe"),
            c => out.push(c),
        }
    }
}

/// A letter of a word: a character with the acute or diaeresis that may follow it.
#[derive(Clone, Copy, Debug)]
struct Letter {
    /// Where it starts in the word, in bytes.
    start: usize,
    /// Where it starts in the plain form of the word, in characters.
    plain: usize,
    /// The lowercase letter without accent or diaeresis; k is read as c.
    base: char,
    acute: bool,
    /// A diaeresis marks a hiatus: the vowel stands apart from the one before it.
    diaeresis: bool,
}

/// The letters of a word and its plain form. A combining acute or diaeresis belongs to the
/// letter before it.
fn read(word: &str) -> (Vec<Letter>, String) {
    let mut letters: Vec<Letter> = Vec::new();
    let mut plain = String::with_capacity(word.len());
    let mut plain_chars = 0;
    for (start, c) in word.char_indices() {
        if c == COMBINING_ACUTE || c == COMBINING_DIAERESIS {
            if let Some(last) = letters.last_mut() {
                last.acute |= c == COMBINING_ACUTE;
                last.diaeresis |= c == COMBINING_DIAERESIS;
            }
            continue;
        }
        let base = match lower_plain(c) {
            'k' => 'c',
            other => other,
        };
        letters.push(Letter {
            start,
            plain: plain_chars,
            base,
            acute: is_acute(c),
            diaeresis: is_diaeresis(c),
        });
        let before = plain.len();
        push_plain(&mut plain, c);
        plain_chars += plain[before..].chars().count();
    }
    (letters, plain)
}

/// Divides a word as [`syllabify`] does, with the lexicon or by rule alone. A lexicon entry
/// counts only if it fits the letters as written: every syllable needs a vowel (not so the j of
/// "cuj" against cú-i), and a diaeresis must start a syllable.
fn divide(word: &str, lexicon: Option<&Lexicon>) -> Syllabified {
    let (letters, plain) = read(word);
    let acute = letters.iter().position(|letter| letter.acute);
    let base: Vec<char> = letters.iter().map(|letter| letter.base).collect();
    let marked: Vec<bool> = letters.iter().map(|letter| letter.diaeresis).collect();
    let vowel = vowels(&base, &marked);
    if let Some(entry) = lexicon.and_then(|lexicon| lexicon.get(&plain))
        && let Some(starts) = entry.starts(&letters)
        && fits(&starts, &vowel, &marked)
    {
        return Syllabified {
            syllables: split(word, &letters, &starts),
            stress: Some(acute.map_or(entry.stress, |a| syllable_of(&starts, a))),
            uncertain: false,
        };
    }

    let nuclei = nuclei(&letters, &vowel, &plain);
    if nuclei.len() < 2 {
        return Syllabified {
            syllables: vec![word.to_string()],
            stress: None,
            uncertain: false,
        };
    }
    let fixed = morpheme_boundaries(&base, &vowel);
    let mut starts = vec![0];
    for pair in nuclei.windows(2) {
        let (from, to) = (pair[0].1, pair[1].0);
        let start = fixed
            .iter()
            .copied()
            .find(|b| (from..=to).contains(b))
            .unwrap_or_else(|| boundary(&base, &vowel, from, to));
        starts.push(start);
    }
    let (stress, uncertain) = match acute {
        Some(a) => (syllable_of(&starts, a), false),
        None => unmarked_stress(&base, &vowel, &nuclei, &plain),
    };
    Syllabified {
        syllables: split(word, &letters, &starts),
        stress: Some(stress),
        uncertain,
    }
}

/// The syllables of a word, each starting at the letter given in `starts`.
fn split(word: &str, letters: &[Letter], starts: &[usize]) -> Vec<String> {
    starts
        .iter()
        .enumerate()
        .map(|(k, &first)| {
            let from = if k == 0 { 0 } else { letters[first].start };
            let to = starts
                .get(k + 1)
                .map_or(word.len(), |&next| letters[next].start);
            word[from..to].to_string()
        })
        .collect()
}

/// The syllable that holds a letter.
fn syllable_of(starts: &[usize], letter: usize) -> usize {
    starts.iter().rposition(|&s| s <= letter).unwrap_or(0)
}

/// Whether a division of the letters, given by the letters that start its syllables, gives
/// every syllable a vowel and starts a syllable at every vowel that a diaeresis keeps apart from
/// the vowel before it.
fn fits(starts: &[usize], vowel: &[bool], marked: &[bool]) -> bool {
    let ends = starts.iter().skip(1).copied().chain([vowel.len()]);
    starts
        .iter()
        .zip(ends)
        .all(|(&from, to)| vowel.get(from..to).is_some_and(|s| s.contains(&true)))
        && (1..vowel.len()).all(|i| !(marked[i] && vowel[i - 1]) || starts.contains(&i))
}

/// Which letters are vowels: u is a consonant in qu and in ngu before a vowel; i is one before a
/// vowel at the start of the word, after a vowel (not after the u of qu or gu) and after a
/// prefix. Before another i it stays a vowel (ab-i-it, i-i), and after a prefix also before en
/// and er (a-di-ens, ab-i-é-runt from eo). A letter with a diaeresis is always a vowel
/// (I-sa-ï-as).
fn vowels(base: &[char], marked: &[bool]) -> Vec<bool> {
    let letter = |i: usize| base.get(i).copied();
    let vowel_letter = |i: usize| letter(i).is_some_and(|c| VOWELS.contains(c));
    let prefix = J_PREFIXES
        .iter()
        .find(|prefix| starts_with(base, prefix))
        .map(|prefix| prefix.len());
    let mut vowel = vec![false; base.len()];
    for i in 0..base.len() {
        let before_vowel = vowel_letter(i + 1) && letter(i + 1) != Some('i');
        let after_vowel = i > 0
            && vowel[i - 1]
            && !(i > 1 && base[i - 1] == 'u' && matches!(base[i - 2], 'q' | 'g'));
        let after_prefix = prefix == Some(i)
            && !(letter(i + 1) == Some('e') && matches!(letter(i + 2), Some('n' | 'r')));
        vowel[i] = match base[i] {
            _ if marked[i] => true,
            'u' if i > 0 && base[i - 1] == 'q' => false,
            'u' if i > 1 && base[i - 2..i] == ['n', 'g'] && vowel_letter(i + 1) => false,
            'i' if before_vowel && (i == 0 || after_vowel || after_prefix) => false,
            c => VOWELS.contains(c),
        };
    }
    vowel
}

/// The nuclei as ranges of letters: adjacent vowels merge only in the diphthongs, never into a
/// vowel with a diaeresis, and ae, oe and au not where [`hiatus_span`] keeps them apart.
fn nuclei(letters: &[Letter], vowel: &[bool], plain: &str) -> Vec<(usize, usize)> {
    let span = hiatus_span(plain);
    let mut nuclei: Vec<(usize, usize)> = Vec::new();
    for i in (0..letters.len()).filter(|&i| vowel[i]) {
        let second = letters[i];
        match nuclei.last_mut() {
            Some(last)
                if *last == (i - 1, i)
                    && !second.diaeresis
                    && diphthong(letters[i - 1].base, second.base, plain, second.plain < span) =>
            {
                last.1 = i + 1;
            }
            _ => nuclei.push((i, i + 1)),
        }
    }
    nuclei
}

/// How many characters of a plain form, from its start, keep ae, oe and au apart: all of a word
/// of [`HIATUS_WORDS`], the stem of one of [`HIATUS_STEMS`], otherwise none.
fn hiatus_span(plain: &str) -> usize {
    if HIATUS_WORDS.contains(&plain) {
        return usize::MAX;
    }
    if NOT_HIATUS.iter().any(|stem| plain.starts_with(stem)) {
        return 0;
    }
    HIATUS_STEMS
        .iter()
        .find(|stem| plain.starts_with(*stem))
        .map_or(0, |stem| stem.len())
}

/// Whether two adjacent vowels of a word form one nucleus; `apart` keeps ae, oe and au apart.
fn diphthong(first: char, second: char, word: &str, apart: bool) -> bool {
    match (first, second) {
        ('a' | 'o', 'e') | ('a', 'u') => !apart,
        ('a', 'y') => true,
        ('e', 'u') => EU_DIPHTHONG.contains(&word),
        ('e', 'i') => word == "hei",
        _ => false,
    }
}

/// Whether the letters of a word begin with `prefix`, and more follow.
fn starts_with(base: &[char], prefix: &str) -> bool {
    base.len() > prefix.len() && prefix.chars().zip(base).all(|(p, &b)| p == b)
}

/// The syllable boundaries that the parts of a word fix, as the letters that start a syllable:
/// after a prefix of [`PREFIXES`], and after pot before ens and est (o-mní-pot-ens, pot-é-stas).
fn morpheme_boundaries(base: &[char], vowel: &[bool]) -> Vec<usize> {
    let follows = |at: usize, next: &str| {
        next.chars().enumerate().all(|(k, c)| {
            let i = at + k;
            match c {
                '*' => vowel.get(i) == Some(&true),
                c => base.get(i) == Some(&c) && (!VOWELS.contains(c) || vowel[i]),
            }
        })
    };
    let mut fixed: Vec<usize> = PREFIXES
        .iter()
        .filter(|&&(prefix, next)| starts_with(base, prefix) && follows(prefix.len(), next))
        .filter(|_| !NO_PREFIX.iter().any(|word| starts_with(base, word)))
        .map(|&(prefix, _)| prefix.len())
        .collect();
    fixed.extend(
        base.windows(5)
            .enumerate()
            .filter(|(_, w)| w[..4] == ['p', 'o', 't', 'e'] && matches!(w[4], 'n' | 's'))
            .map(|(i, _)| i + 3),
    );
    fixed
}

/// Where the syllable after the consonants `from..to` starts: doubled consonants are split, and
/// otherwise the longest end of the group that can begin a word goes to it, but never one that
/// starts with x before a consonant.
fn boundary(base: &[char], vowel: &[bool], from: usize, to: usize) -> usize {
    let units = consonant_units(base, vowel, from, to);
    if let Some(pair) = units
        .windows(2)
        .find(|pair| pair[0].1 == 1 && pair[1].1 == 1 && base[pair[0].0] == base[pair[1].0])
    {
        return pair[1].0;
    }
    (0..units.len())
        .find(|&m| onset(base, &units[m..]) && !(m + 1 < units.len() && base[units[m].0] == 'x'))
        .map_or(to, |m| units[m].0)
}

/// The consonants between two nuclei as units `(first letter, letters)`: ch, ph, th and rh are
/// one unit, and so are qu and a consonantal gu.
fn consonant_units(base: &[char], vowel: &[bool], from: usize, to: usize) -> Vec<(usize, usize)> {
    let mut units = Vec::new();
    let mut i = from;
    while i < to {
        let pair = i + 1 < to
            && (matches!((base[i], base[i + 1]), ('c' | 'p' | 't' | 'r', 'h'))
                || matches!(base[i], 'q' | 'g') && base[i + 1] == 'u' && !vowel[i + 1]);
        let len = if pair { 2 } else { 1 };
        units.push((i, len));
        i += len;
    }
    units
}

/// The letters of a consonant unit.
fn unit(base: &[char], (start, len): (usize, usize)) -> &[char] {
    &base[start..start + len]
}

/// Whether consonant units can begin a word, so that they start a syllable together (the
/// grammarians' rule of the Liber): one consonant, a mute and a liquid, gn, sc, sp, st, scr,
/// spl, spr, str, and the Greek ct, pt, mn, pn, ps, tm, cn, bd.
fn onset(base: &[char], units: &[(usize, usize)]) -> bool {
    match *units {
        [_] => true,
        [first, second] => {
            let (first, second) = (unit(base, first), unit(base, second));
            mute_and_liquid(first, second)
                || matches!(
                    (first, second),
                    (['g'], ['n'])
                        | (['s'], ['c' | 'p' | 't'])
                        | (['c' | 'p'], ['t'])
                        | (['m' | 'p'], ['n'])
                        | (['p'], ['s'])
                        | (['t'], ['m'])
                        | (['c'], ['n'])
                        | (['b'], ['d'])
                )
        }
        [first, second, third] => matches!(
            (unit(base, first), unit(base, second), unit(base, third)),
            (['s'], ['c'], ['r']) | (['s'], ['p'], ['l' | 'r']) | (['s'], ['t'], ['r'])
        ),
        _ => false,
    }
}

/// Whether two consonant units are a mute and a liquid, which begin a syllable together
/// (Pa-tris, Chri-ste, Ec-clé-si-am). tl, dl and thl are none (Beth-le-hem).
fn mute_and_liquid(mute: &[char], liquid: &[char]) -> bool {
    match liquid {
        ['r'] => matches!(
            mute,
            ['b' | 'c' | 'd' | 'f' | 'g' | 'p' | 't'] | ['c' | 'p' | 't', 'h']
        ),
        ['l'] => matches!(mute, ['b' | 'c' | 'f' | 'g' | 'p'] | ['c' | 'p', 'h']),
        _ => false,
    }
}

/// What the letters say about the length of a penult.
enum Weight {
    /// A vowel or h follows its vowel.
    Light,
    /// A diphthong, or two consonants, x or z after its vowel; a consonantal i between vowels
    /// counts double (al-le-lú-ia).
    Heavy,
    /// One consonant, or a mute and a liquid, follows: the length of the vowel decides.
    Unknown,
}

/// The weight of the penult of a word of three or more syllables.
fn penult_weight(base: &[char], vowel: &[bool], nuclei: &[(usize, usize)]) -> Weight {
    let (start, end) = nuclei[nuclei.len() - 2];
    if end - start > 1 || matches!(base[start], 'æ' | 'œ') {
        return Weight::Heavy;
    }
    let units = consonant_units(base, vowel, end, nuclei[nuclei.len() - 1].0);
    let weight: usize = units
        .iter()
        .map(|&(i, len)| match base[i] {
            'x' | 'z' => 2,
            'h' if len == 1 => 0,
            'i' | 'j' if units.len() == 1 => 2,
            _ => 1,
        })
        .sum();
    match (weight, units.as_slice()) {
        (0, _) => Weight::Light,
        (1, _) => Weight::Unknown,
        (_, &[mute, liquid]) if mute_and_liquid(unit(base, mute), unit(base, liquid)) => {
            Weight::Unknown
        }
        _ => Weight::Heavy,
    }
}

/// The stressed syllable of a word of two or more syllables without an acute, and whether it
/// rests on a guess.
fn unmarked_stress(
    base: &[char],
    vowel: &[bool],
    nuclei: &[(usize, usize)],
    plain: &str,
) -> (usize, bool) {
    let count = nuclei.len();
    if plain.ends_with("que") {
        if let Some(&(_, own)) = LEXICAL_QUE.iter().find(|&&(word, _)| word == plain) {
            return (own, false);
        }
        if count >= 3 {
            return (count - 2, false);
        }
    }
    if count == 2 {
        return (0, false);
    }
    match penult_weight(base, vowel, nuclei) {
        Weight::Heavy => (count - 2, false),
        Weight::Light => (count - 3, false),
        Weight::Unknown
            if SHORT_PENULT_ENDINGS
                .iter()
                .any(|ending| plain.ends_with(ending)) =>
        {
            (count - 3, false)
        }
        Weight::Unknown if LONG_ENDINGS.iter().any(|ending| plain.ends_with(ending)) => {
            (count - 2, false)
        }
        Weight::Unknown => (count - 3, true),
    }
}

/// The lexicon: plain forms with their division and stress.
type Lexicon = BTreeMap<String, Entry>;

/// A word of the lexicon.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    /// Where the second and later syllables start in the plain form, in characters.
    starts: Vec<usize>,
    /// Index of the stressed syllable.
    stress: usize,
}

impl Entry {
    /// Reads a word of two or more syllables as the books write it: with hyphens and, from three
    /// syllables on, an acute (`Ma-rí-a`, `cu-jus`). Returns its plain form and the entry, or
    /// `None` for a word that is not written so.
    fn read(written: &str) -> Option<(String, Entry)> {
        let pieces: Vec<&str> = written.split('-').collect();
        let stress = pieces
            .iter()
            .position(|piece| has_acute(piece))
            .or((pieces.len() == 2).then_some(0))?;
        let mut starts = Vec::with_capacity(pieces.len() - 1);
        let mut offset = 0;
        for piece in &pieces[..pieces.len() - 1] {
            offset += normalize(piece).chars().count();
            starts.push(offset);
        }
        Some((normalize(&pieces.concat()), Entry { starts, stress }))
    }

    /// The letters of a word with this plain form at which its syllables start; `None` if a
    /// boundary falls inside a letter, as in a ligature.
    fn starts(&self, letters: &[Letter]) -> Option<Vec<usize>> {
        let mut starts = Vec::with_capacity(self.starts.len() + 1);
        starts.push(0);
        for &offset in &self.starts {
            starts.push(letters.iter().position(|letter| letter.plain == offset)?);
        }
        Some(starts)
    }
}

/// The lexicon, built on first use from the words of `ordinary.txt` and [`LEXICON_EXTRA`]. It
/// reads the raw text of the Ordinary, not the parsed sections: the parser calls [`syllabify`].
fn lexicon() -> &'static Lexicon {
    static LEXICON: OnceLock<Lexicon> = OnceLock::new();
    LEXICON.get_or_init(|| {
        ordinary::words()
            .chain(LEXICON_EXTRA)
            .filter(|word| word.contains('-'))
            .filter_map(Entry::read)
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{SylFlags, Text, TextOptions, TextWarning};

    fn split(word: &str) -> (String, Option<usize>, bool) {
        let s = syllabify(word);
        (s.syllables.join("-"), s.stress, s.uncertain)
    }

    /// The division with an acute on the stressed syllable of a word of two or more syllables:
    /// `Ký-ri-e`, `Dé-us`.
    fn marked(found: &Syllabified) -> String {
        let mut syllables = found.syllables.clone();
        if let Some(k) = found.stress {
            syllables[k] = add_acute(&syllables[k]);
        }
        syllables.join("-")
    }

    /// The word tokens of the Ordinary with two or more syllables, as written.
    fn tokens() -> Vec<&'static str> {
        ordinary::words()
            .filter(|word| word.contains('-'))
            .collect()
    }

    /// A word as the Ordinary writes it: its division without acutes and its stressed syllable.
    fn book_form(written: &str) -> (String, usize) {
        let stress = written.split('-').position(has_acute).unwrap_or(0);
        (strip_acute(written), stress)
    }

    #[test]
    fn acute_goes_on_the_nucleus() {
        assert_eq!(add_acute("le"), "lé");
        assert_eq!(add_acute("Ky"), "Ký");
        assert_eq!(add_acute("sae"), "saé");
        assert_eq!(add_acute("Sae"), "Saé");
        assert_eq!(add_acute("coe"), "coé");
        assert_eq!(add_acute("au"), "áu");
        assert_eq!(add_acute("Quo"), "Quó");
        assert_eq!(add_acute("qui"), "quí");
        assert_eq!(add_acute("Je"), "Jé");
        assert_eq!(add_acute("Ie"), "Ié");
        assert_eq!(add_acute("lei"), "léi");
        assert_eq!(add_acute("sæ"), "sǽ");
        assert_eq!(add_acute("Æ"), "Ǽ");
        assert_eq!(add_acute("cœ"), "cœ\u{301}");
        assert_eq!(add_acute("A"), "Á");
        // A diaeresis keeps its vowel out of the diphthong and makes it no consonant.
        assert_eq!(add_acute("ë"), "ë\u{301}");
        assert_eq!(add_acute("aë"), "áë");
        assert_eq!(add_acute("ae\u{308}"), "áe\u{308}");
        assert_eq!(add_acute("ïa"), "ï\u{301}a");
        // Already accented, or without a vowel: unchanged.
        assert_eq!(add_acute("lé"), "lé");
        assert_eq!(add_acute("saé"), "saé");
        assert_eq!(add_acute("le\u{301}"), "le\u{301}");
        assert_eq!(add_acute("st"), "st");
    }

    #[test]
    fn acutes_are_stripped() {
        assert_eq!(strip_acute("Ký-ri-e e-lé-i-son."), "Ky-ri-e e-le-i-son.");
        assert_eq!(strip_acute("saé-cu-la"), "sae-cu-la");
        assert_eq!(strip_acute("sǽ-cu-la Ǽ"), "sæ-cu-la Æ");
        assert_eq!(strip_acute("le\u{301}"), "le");
        assert_eq!(strip_acute("cœ\u{301}"), "cœ");
        assert_eq!(strip_acute("ÁÉÍÓÚÝ áéíóúý"), "AEIOUY aeiouy");
        // A diaeresis is no accent and stays.
        assert_eq!(strip_acute("po-ë-ta I-sa-ï-as"), "po-ë-ta I-sa-ï-as");
    }

    #[test]
    fn normal_forms() {
        assert_eq!(normalize("Ký"), "ky");
        assert_eq!(normalize("Jesu"), "iesu");
        assert_eq!(normalize("cu-jus"), "cu-ius");
        assert_eq!(normalize("cæli"), "caeli");
        assert_eq!(normalize("Cœli"), "coeli");
        assert_eq!(normalize("Sǽcula"), "saecula");
        assert_eq!(normalize("SAÉ"), "sae");
        assert_eq!(normalize("e\u{301}"), "e");
        assert_eq!(normalize("ÁÉÍÓÚÝ"), "aeiouy");
        assert_eq!(normalize("ŒÆ"), "oeae");
        assert_eq!(normalize("Poëta"), "poeta");
        assert_eq!(normalize("ISAÏAS Emmaüs"), "isaias emmaus");
        assert_eq!(normalize("poe\u{308}ta"), "poeta");
    }

    #[test]
    fn vowels() {
        for c in "aeiouyAEIOUYáéíóúýÁÉÍÓÚÝæœÆŒǽǼëïüËÏÜ".chars() {
            assert!(is_vowel(c), "{c}");
        }
        for c in "bcdjqvwxzBJ-.,\u{301}".chars() {
            assert!(!is_vowel(c), "{c}");
        }
    }

    #[test]
    fn monosyllables_of_the_ordinary() {
        let monosyllables: Vec<&str> = ordinary::words().filter(|w| !w.contains('-')).collect();
        assert_eq!(monosyllables.len(), 78);
        for word in monosyllables {
            assert_eq!(split(word), (word.to_string(), None, false), "{word}");
        }
        for word in ["iam", "Jam", "quae", "heu", "hei", "Æ", "cœ\u{301}l"] {
            assert_eq!(split(word), (word.to_string(), None, false), "{word}");
        }
    }

    #[test]
    fn simple_words() {
        assert_eq!(split("Kyrie"), ("Ky-ri-e".into(), Some(0), false));
        assert_eq!(split("Deus"), ("De-us".into(), Some(0), false));
        assert_eq!(split("Jesu"), ("Je-su".into(), Some(0), false));
        assert_eq!(split("cujus"), ("cu-jus".into(), Some(0), false));
        assert_eq!(split("Patrem"), ("Pa-trem".into(), Some(0), false));
        assert_eq!(split("Hosanna"), ("Ho-san-na".into(), Some(1), false));
        assert_eq!(split("gloria"), ("glo-ri-a".into(), Some(0), false));
        // An acute wins, and the letters are kept.
        assert_eq!(split("Ecclésiam"), ("Ec-clé-si-am".into(), Some(1), false));
        assert_eq!(split("Dóminus"), ("Dó-mi-nus".into(), Some(0), false));
        assert_eq!(split("ele\u{301}ison").0, "e-le\u{301}-i-son");
        // An open penult before one consonant: its length is unknown, unless the ending shows
        // it.
        assert_eq!(split("animam"), ("a-ni-mam".into(), Some(0), true));
        assert_eq!(split("levavi"), ("le-va-vi".into(), Some(1), false));
    }

    #[test]
    fn syllabifier_without_lexicon() {
        // The 209 word tokens of two or more syllables in ordinary.txt (149 distinct words). The
        // design's 224 tokens come from the GregoBase transcriptions of Kyrie I, Gloria I, Credo I,
        // Sanctus I and Agnus I, which add the two intonations (6 tokens), the second Agnus Dei
        // (7) and one more Kyrie eleison (2).
        let tokens = tokens();
        assert_eq!(tokens.len(), 209);
        let mut misses = Vec::new();
        for &written in &tokens {
            let (division, stress) = book_form(written);
            let found = divide(&division.replace('-', ""), None);
            // Every syllable count and every division of the books.
            assert_eq!(found.syllables.join("-"), division, "{written}");
            if found.stress != Some(stress) {
                misses.push((marked(&found), found.uncertain));
            }
        }
        // 203 of 209 stresses (217 of the design's 224). Rules cannot know a long open penult
        // (miserére, procédit, Prophétas, flagged uncertain) or a long vowel before a vowel
        // (María).
        assert_eq!(
            misses,
            [
                ("mi-sé-re-re".to_string(), true),
                ("mi-sé-re-re".to_string(), true),
                ("Má-ri-a".to_string(), false),
                ("pró-ce-dit".to_string(), true),
                ("Pró-phe-tas".to_string(), true),
                ("mi-sé-re-re".to_string(), true),
            ]
        );
    }

    #[test]
    fn syllabifier_with_lexicon() {
        for written in tokens() {
            let (division, stress) = book_form(written);
            let found = syllabify(&division.replace('-', ""));
            assert_eq!(
                (found.syllables.join("-"), found.stress, found.uncertain),
                (division, Some(stress), false),
                "{written}"
            );
        }
        // Lookups ignore case and accents, and read j as i and æ as ae.
        assert_eq!(split("MISERERE"), ("MI-SE-RE-RE".into(), Some(2), false));
        assert_eq!(split("Prophetas"), ("Pro-phe-tas".into(), Some(1), false));
        assert_eq!(split("iudicare"), ("iu-di-ca-re".into(), Some(2), false));
        assert_eq!(split("Cuius"), ("Cu-ius".into(), Some(0), false));
        assert_eq!(split("sæcula"), ("sæ-cu-la".into(), Some(0), false));
        assert_eq!(split("Sǽculi"), ("Sǽ-cu-li".into(), Some(0), false));
        assert_eq!(split("cæli"), ("cæ-li".into(), Some(0), false));
        // A printed acute still wins.
        assert_eq!(split("Dominús"), ("Do-mi-nús".into(), Some(2), false));
    }

    #[test]
    fn special_words() {
        let words = [
            ("Kyrie", "Ký-ri-e"),
            ("eleison", "e-lé-i-son"),
            ("Deus", "Dé-us"),
            ("cujus", "cú-jus"),
            ("cuius", "cú-ius"),
            ("Jesu", "Jé-su"),
            ("Iesu", "Ié-su"),
            ("Maria", "Ma-rí-a"),
            ("Filioque", "Fi-li-ó-que"),
            ("quoniam", "quó-ni-am"),
            ("etiam", "ét-i-am"),
            ("omnipotens", "o-mní-pot-ens"),
            ("baptisma", "ba-ptís-ma"),
            ("suscipe", "sús-ci-pe"),
            ("Sabaoth", "Sá-ba-oth"),
            ("huic", "hú-ic"),
            ("cui", "cú-i"),
        ];
        for (word, expected) in words {
            let found = syllabify(word);
            assert_eq!(marked(&found), expected, "{word}");
            assert!(!found.uncertain, "{word}");
        }
        // The rules alone get all but María right; where the penult is open before one
        // consonant they are unsure.
        for (word, expected) in words {
            let found = divide(word, None);
            let unsure = ["eleison", "omnipotens", "suscipe"].contains(&word);
            if word == "Maria" {
                assert_eq!(marked(&found), "Má-ri-a");
            } else {
                assert_eq!(marked(&found), expected, "{word}");
                assert_eq!(found.uncertain, unsure, "{word}");
            }
        }
    }

    #[test]
    fn unknown_words_get_uncertain_accents() {
        assert_eq!(split("animam"), ("a-ni-mam".into(), Some(0), true));
        assert_eq!(split("Regina"), ("Re-gi-na".into(), Some(0), true));
        assert_eq!(split("Dominus"), ("Do-mi-nus".into(), Some(0), false));
        assert_eq!(
            divide("Dominus", None),
            Syllabified {
                syllables: vec!["Do".into(), "mi".into(), "nus".into()],
                stress: Some(0),
                uncertain: true,
            }
        );
        // The parser warns about them and marks their syllables; levavi has a known ending.
        let options = TextOptions::default();
        let (text, warnings) = Text::own("Ad te levavi animam meam.", &options).unwrap();
        let uncertain = |word: &str, result: &str| TextWarning::UncertainAccent {
            word: word.into(),
            result: result.into(),
        };
        assert_eq!(warnings, [uncertain("animam", "á-ni-mam")]);
        let flagged: Vec<&str> = text
            .syllables
            .iter()
            .filter(|s| s.flags.contains(SylFlags::UNCERTAIN))
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(flagged, ["á", "ni", "mam"]);
        assert_eq!(text.render(true, false), "Ad te le-vá-vi á-ni-mam me-am.");
        // Words of the lexicon, and words whose letters decide, give none.
        let (_, warnings) = Text::own(
            "Kyrie eleison. Dominus Deus Sabaoth, miserere nobis. Laudamus te, benedictus.",
            &options,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// Checks divisions by rule alone, without the lexicon.
    fn assert_divisions(cases: &[(&str, &str)]) {
        for &(word, expected) in cases {
            assert_eq!(divide(word, None).syllables.join("-"), expected, "{word}");
        }
    }

    #[test]
    fn consonant_groups() {
        assert_divisions(&[
            // A mute and a liquid, gn, s before c, p, t and the Greek groups begin a syllable.
            ("patris", "pa-tris"),
            ("Ecclesiam", "Ec-cle-si-am"),
            ("magnam", "ma-gnam"),
            ("benignus", "be-ni-gnus"),
            ("nostram", "no-stram"),
            ("caelestis", "cae-le-stis"),
            ("descendit", "de-scen-dit"),
            ("Sanctus", "San-ctus"),
            ("propter", "pro-pter"),
            ("omnium", "o-mni-um"),
            ("ipse", "i-pse"),
            ("Redemptor", "Red-em-ptor"),
            ("instruxit", "in-stru-xit"),
            ("conglorificatur", "con-glo-ri-fi-ca-tur"),
            ("consubstantialem", "con-sub-stan-ti-a-lem"),
            // sm, squ, s and ch, thl begin no syllable together.
            ("baptisma", "ba-ptis-ma"),
            ("usque", "us-que"),
            ("unusquisque", "u-nus-quis-que"),
            ("Pascha", "Pas-cha"),
            ("Bethlehem", "Beth-le-hem"),
            ("athleta", "ath-le-ta"),
            // Doubled consonants are split, and x before a consonant stays.
            ("Hosanna", "Ho-san-na"),
            ("Altissimus", "Al-tis-si-mus"),
            ("passus", "pas-sus"),
            ("exspecto", "ex-spe-cto"),
            ("dexteram", "dex-te-ram"),
            ("excelsis", "ex-cel-sis"),
            ("crucifixus", "cru-ci-fi-xus"),
            // ch, ph, th, qu and gu after n are single consonants, and so is h.
            ("Christe", "Chri-ste"),
            ("catholicam", "ca-tho-li-cam"),
            ("Philippus", "Phi-lip-pus"),
            ("relinquo", "re-lin-quo"),
            ("sanguis", "san-guis"),
            ("lingua", "lin-gua"),
            ("Abraham", "A-bra-ham"),
            ("mihi", "mi-hi"),
        ]);
    }

    #[test]
    fn nuclei_and_hiatus() {
        assert_divisions(&[
            ("Laudamus", "Lau-da-mus"),
            ("exaudi", "ex-au-di"),
            ("aeternum", "ae-ter-num"),
            ("caelum", "cae-lum"),
            ("cælum", "cæ-lum"),
            ("coepit", "coe-pit"),
            ("Raymundus", "Ray-mun-dus"),
            ("Euge", "Eu-ge"),
            ("eleison", "e-le-i-son"),
            ("meus", "me-us"),
            ("eunt", "e-unt"),
            ("Dei", "De-i"),
            ("diei", "di-e-i"),
            ("fuit", "fu-it"),
            ("huic", "hu-ic"),
            ("tuae", "tu-ae"),
            ("Moyses", "Mo-y-ses"),
            ("Kyrie", "Ky-ri-e"),
            ("Israel", "Is-ra-el"),
            ("Michael", "Mi-cha-el"),
            ("Raphael", "Ra-pha-el"),
            ("aer", "a-er"),
            ("poeta", "po-e-ta"),
            ("Noe", "No-e"),
            ("Emmaus", "Em-ma-us"),
            // The stems keep the hiatus in every form; an ending keeps its diphthong.
            ("Michaelem", "Mi-cha-e-lem"),
            ("Michaeli", "Mi-cha-e-li"),
            ("Michaelis", "Mi-cha-e-lis"),
            ("Raphaelem", "Ra-pha-e-lem"),
            ("Israelita", "Is-ra-e-li-ta"),
            ("Israelitae", "Is-ra-e-li-tae"),
            ("Ismaelitae", "Is-ma-e-li-tae"),
            ("poetae", "po-e-tae"),
            ("poemata", "po-e-ma-ta"),
            ("poesis", "po-e-sis"),
            ("coegit", "co-e-git"),
            ("coemo", "co-e-mo"),
            ("Joel", "Jo-el"),
            ("Noemi", "No-e-mi"),
            ("aeris", "a-e-ris"),
            ("aeri", "a-e-ri"),
            ("aerem", "a-e-rem"),
            ("aere", "a-e-re"),
            ("aera", "a-e-ra"),
            ("aerum", "a-e-rum"),
            ("aeribus", "a-e-ri-bus"),
            // Other words with these letters have the diphthong.
            ("aerumna", "ae-ru-mna"),
            ("coemeterium", "coe-me-te-ri-um"),
            ("poena", "poe-na"),
            ("coetus", "coe-tus"),
            // A diaeresis keeps a vowel apart anywhere and makes it no consonant.
            ("poëta", "po-ë-ta"),
            ("aër", "a-ër"),
            ("Michaël", "Mi-cha-ël"),
            ("Israël", "Is-ra-ël"),
            ("Emmaüs", "Em-ma-üs"),
            ("Isaïas", "I-sa-ï-as"),
            ("POËTA", "PO-Ë-TA"),
            ("poe\u{308}ta", "po-e\u{308}-ta"),
            ("Laüdamus", "La-ü-da-mus"),
        ]);
    }

    #[test]
    fn consonantal_i_and_u() {
        assert_divisions(&[
            ("Jesum", "Je-sum"),
            ("Iesum", "Ie-sum"),
            ("iudicare", "iu-di-ca-re"),
            ("Ioannes", "Io-an-nes"),
            ("eius", "e-ius"),
            ("maiestatis", "ma-ie-sta-tis"),
            ("alleluia", "al-le-lu-ia"),
            ("alleluja", "al-le-lu-ja"),
            ("requiem", "re-qui-em"),
            ("sanguine", "san-gui-ne"),
            ("arguo", "ar-gu-o"),
            ("suavis", "su-a-vis"),
            ("adiuva", "ad-iu-va"),
            ("adjuva", "ad-ju-va"),
            ("iniuria", "in-iu-ri-a"),
            ("subiectus", "sub-ie-ctus"),
            ("coniunctio", "con-iun-cti-o"),
            // In forms of eo, i stays a vowel after a prefix.
            ("abiit", "ab-i-it"),
            ("adiens", "a-di-ens"),
            ("abierunt", "ab-i-e-runt"),
            ("inimicus", "in-i-mi-cus"),
            ("ii", "i-i"),
        ]);
    }

    #[test]
    fn morpheme_table() {
        assert_divisions(&[
            ("adoramus", "ad-o-ra-mus"),
            ("adoratur", "ad-o-ra-tur"),
            ("exaudi", "ex-au-di"),
            ("exercitus", "ex-er-ci-tus"),
            ("inimici", "in-i-mi-ci"),
            ("iniquitatem", "in-i-qui-ta-tem"),
            ("peregit", "per-e-git"),
            ("peregrinus", "per-e-gri-nus"),
            ("redemptor", "red-em-ptor"),
            ("subito", "sub-i-to"),
            ("abest", "ab-est"),
            ("abundantia", "ab-un-dan-ti-a"),
            ("oboedire", "ob-oe-di-re"),
            ("etiam", "et-i-am"),
            ("omnipotens", "o-mni-pot-ens"),
            ("omnipotentem", "o-mni-pot-en-tem"),
            ("potestas", "pot-e-stas"),
            ("potentiam", "pot-en-ti-am"),
            ("suscipe", "sus-ci-pe"),
            ("suscepit", "sus-ce-pit"),
            ("discerne", "dis-cer-ne"),
            // in keeps its n only before i, sus its s only before c, as the design's table has it
            // (GR 1961 prints in-á-nem and sus-ti-né-bit).
            ("inanis", "i-na-nis"),
            ("inopem", "i-no-pem"),
            ("sustinebit", "su-sti-ne-bit"),
            // No prefix in disco and its family, and never quo|niam.
            ("discipulis", "di-sci-pu-lis"),
            ("quoniam", "quo-ni-am"),
        ]);
    }

    #[test]
    fn enclitics() {
        for (word, expected) in [
            ("Filioque", "Fi-li-ó-que"),
            ("Patrique", "Pa-trí-que"),
            ("Genitoque", "Ge-ni-tó-que"),
            ("sanguinisque", "san-gui-nís-que"),
            ("terraque", "ter-rá-que"),
            ("itaque", "í-ta-que"),
            ("denique", "dé-ni-que"),
            ("undique", "ún-di-que"),
            ("utique", "ú-ti-que"),
            ("ubique", "u-bí-que"),
            ("quicumque", "qui-cúm-que"),
            ("quoque", "quó-que"),
            ("neque", "né-que"),
            ("atque", "át-que"),
            // -ne is no enclitic here.
            ("Domine", "Dó-mi-ne"),
            ("Virgine", "Vír-gi-ne"),
        ] {
            let found = divide(word, None);
            assert_eq!(marked(&found), expected, "{word}");
            assert!(!found.uncertain || word.ends_with("ne"), "{word}");
        }
    }

    #[test]
    fn weights_endings_and_guesses() {
        for (word, expected, uncertain) in [
            // A diphthong, x, z, two consonants or a consonantal i between vowels: heavy.
            ("exaudi", "ex-áu-di", false),
            ("caelestis", "cae-lé-stis", false),
            ("crucifixus", "cru-ci-fí-xus", false),
            ("resurrexit", "re-sur-ré-xit", false),
            ("baptizo", "ba-ptí-zo", false),
            ("secundum", "se-cún-dum", false),
            ("alleluia", "al-le-lú-ia", false),
            // A vowel or h after it: light.
            ("gloria", "gló-ri-a", false),
            ("Ecclesiam", "Ec-clé-si-am", false),
            ("Abraham", "Á-bra-ham", false),
            // One consonant: endings long by nature decide.
            ("cantamus", "can-tá-mus", false),
            ("civitatis", "ci-vi-tá-tis", false),
            ("laudare", "lau-dá-re", false),
            ("laudari", "lau-dá-ri", false),
            ("laudatur", "lau-dá-tur", false),
            ("beatus", "be-á-tus", false),
            ("animarum", "a-ni-má-rum", false),
            ("saeculorum", "sae-cu-ló-rum", false),
            ("ratione", "ra-ti-ó-ne", false),
            ("salvatoris", "sal-va-tó-ris", false),
            ("veritatem", "ve-ri-tá-tem", false),
            ("virtutis", "vir-tú-tis", false),
            ("futurus", "fu-tú-rus", false),
            ("creatura", "cre-a-tú-ra", false),
            ("spiritualis", "spi-ri-tu-á-lis", false),
            // The perfects in -ávi and -ívi, and the short -imus after -áv-.
            ("levavi", "le-vá-vi", false),
            ("speravi", "spe-rá-vi", false),
            ("clamavi", "cla-má-vi", false),
            ("levavit", "le-vá-vit", false),
            ("exaltavit", "ex-al-tá-vit", false),
            ("levaverunt", "le-va-vé-runt", false),
            ("levavimus", "le-vá-vi-mus", false),
            ("exaltavimus", "ex-al-tá-vi-mus", false),
            ("audivi", "au-dí-vi", false),
            ("audivit", "au-dí-vit", false),
            // Otherwise, and before a mute and a liquid, the penult counts as short: a guess.
            ("Dominus", "Dó-mi-nus", true),
            ("dicere", "dí-ce-re", true),
            ("tenebrae", "té-ne-brae", true),
            ("cathedra", "cá-the-dra", true),
            ("animam", "á-ni-mam", true),
            ("Regina", "Ré-gi-na", true),
            ("audivimus", "au-dí-vi-mus", true),
        ] {
            let found = divide(word, None);
            assert_eq!(
                (marked(&found), found.uncertain),
                (expected.into(), uncertain),
                "{word}"
            );
        }
    }

    #[test]
    fn lexicon_entries() {
        let lexicon = lexicon();
        // Every word of two or more syllables of the Ordinary and every extra word is in it with
        // the books' division and stress; words written alike agree.
        for written in tokens().into_iter().chain(LEXICON_EXTRA) {
            let (key, entry) = Entry::read(written).unwrap_or_else(|| panic!("{written}"));
            assert_eq!(lexicon.get(&key), Some(&entry), "{written}");
            let found = syllabify(&strip_acute(&written.replace('-', "")));
            assert_eq!(marked(&found), add_acute_at(written), "{written}");
        }
        // 148 words of the Ordinary (San-ctus and san-ctus are one) and 13 more.
        assert_eq!(lexicon.len(), 161);
        // Most extras differ from the rules in their stress.
        for (word, rules) in [
            ("diei", "dí-e-i"),
            ("illius", "íl-li-us"),
            ("unius", "ú-ni-us"),
            ("totius", "tó-ti-us"),
            ("ipsius", "í-psi-us"),
            ("Jerusalem", "Je-ru-sá-lem"),
            ("Sabbatum", "Sab-bá-tum"),
        ] {
            assert_eq!(marked(&divide(word, None)), rules, "{word}");
        }
        assert_eq!(split("Ierusalem"), ("Ie-ru-sa-lem".into(), Some(1), false));
        assert_eq!(Entry::read("Ky-ri-e"), None);
        assert_eq!(Entry::read("Ky"), None);
    }

    #[test]
    fn lexicon_entries_fit_the_letters() {
        // A j has the plain form of i but is no vowel: De-i and cú-i do not fit.
        assert_eq!(split("Dej"), ("Dej".into(), None, false));
        assert_eq!(split("cuj"), ("cuj".into(), None, false));
        assert_eq!(split("CUJ"), ("CUJ".into(), None, false));
        assert_eq!(split("dJei"), ("dJe-i".into(), Some(0), false));
        // Every syllable the parser gets has a vowel, so the text renders and parses again.
        let options = TextOptions::default();
        for src in ["Dej", "cuj", "déJ", "dJei"] {
            let (text, _) = Text::own(src, &options).unwrap();
            let rendered = text.render(true, true);
            let (again, _) = Text::own(&rendered, &options).unwrap();
            assert_eq!(again.syllables, text.syllables, "{src}");
            for syllable in &text.syllables {
                assert!(
                    syllable.text.chars().any(is_vowel),
                    "{src}: {}",
                    syllable.text
                );
            }
        }
        // An entry must split a hiatus that a diaeresis marks: Ís-ra-ël fits, Lau-dá-mus does
        // not fit La-ü-da-mus.
        assert_eq!(split("Israël"), ("Is-ra-ël".into(), Some(0), false));
        assert_eq!(split("Laüdamus"), ("La-ü-da-mus".into(), Some(2), false));
        assert_eq!(split("Kyrië"), ("Ky-ri-ë".into(), Some(0), false));
    }

    /// A word as written in the books, with the acute added on a disyllable: `Dé-us`.
    fn add_acute_at(written: &str) -> String {
        let mut pieces: Vec<String> = written.split('-').map(String::from).collect();
        if pieces.len() == 2 && !has_acute(written) {
            pieces[0] = add_acute(&pieces[0]);
        }
        pieces.join("-")
    }

    #[test]
    fn letters_are_kept() {
        for word in [
            "",
            "st",
            "Ǽ",
            "AETERNUS",
            "cœ\u{301}lum",
            "le\u{301}vavi",
            "\u{301}amen",
            "Sǽcula",
            "QUONIAM",
            "x7y",
            "aeiou",
            "poëta",
            "poe\u{308}ta",
            "\u{308}a",
            "ë\u{301}",
            "Dej",
        ] {
            let found = syllabify(word);
            assert_eq!(found.syllables.concat(), word, "{word}");
            assert!(found.syllables.iter().all(|s| !s.is_empty()) || word.is_empty());
        }
        assert_eq!(
            split("cœ\u{301}lum"),
            ("cœ\u{301}-lum".into(), Some(0), false)
        );
        assert_eq!(split("AETERNUS"), ("AE-TER-NUS".into(), Some(1), false));
        assert_eq!(
            split("le\u{301}vavi"),
            ("le\u{301}-va-vi".into(), Some(0), false)
        );
        assert_eq!(split("Kyrié"), ("Ky-ri-é".into(), Some(2), false));
        assert_eq!(split(""), (String::new(), None, false));
    }
}
