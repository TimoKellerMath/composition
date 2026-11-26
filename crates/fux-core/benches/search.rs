//! Benchmarks of the search: a cantus firmus, counterpoint in all five species over Fux's
//! Dorian cantus, and free counterpoint in four voices with and without a text
//! (`cargo bench -p fux-core`).

use std::sync::Arc;

use criterion::{Criterion, SamplingMode, criterion_group, criterion_main};
use fux_core::corpus::fux_cantus;
use fux_core::generate::{
    CantusRequest, CounterpointRequest, FreeRequest, FreeText, NewVoice, generate_cantus,
    generate_counterpoint, generate_free,
};
use fux_core::rules::Profile;
use fux_core::text::plan::plan;
use fux_core::text::{TextOptions, resolve};
use fux_core::{Duration, ModeSpec, Note, Role, Score, Species, Voice, VoiceType};

fn dorian_cantus() -> Score {
    let cantus = fux_cantus()
        .into_iter()
        .find(|c| c.id == "dorian")
        .expect("Fux's Dorian cantus");
    let notes = cantus
        .notes
        .iter()
        .map(|&p| Note::new(p, Duration::WHOLE))
        .collect();
    Score::new(
        cantus.mode,
        vec![Voice::new(VoiceType::Alto, Role::CantusFirmus, notes)],
    )
}

fn search(c: &mut Criterion) {
    let profile = Profile::fux();
    let mut group = c.benchmark_group("search");
    group.sample_size(10);
    group.bench_function("cantus", |b| {
        let mut request = CantusRequest::new("dorian".parse::<ModeSpec>().unwrap(), 11);
        request.count = 1;
        b.iter(|| generate_cantus(&request, &profile).unwrap());
    });
    let base = dorian_cantus();
    for species in [
        Species::First,
        Species::Second,
        Species::Third,
        Species::Fourth,
        Species::Fifth,
    ] {
        let request = CounterpointRequest::new(
            base.clone(),
            vec![NewVoice {
                voice_type: VoiceType::Soprano,
                species,
            }],
        );
        group.bench_function(format!("species {}", species.number()), |b| {
            b.iter(|| generate_counterpoint(&request, &profile).unwrap());
        });
    }
    group.finish();
}

/// Free counterpoint in four voices, Dorian: Kyrie I sung by every voice at the length and with
/// the cadences its text plans (`free_text`, 16 measures and a cadence in measure 7), and the
/// same measures and cadences without a text (`free`). A run takes seconds, so each of the ten
/// samples is one run.
fn free(c: &mut Criterion) {
    let profile = Profile::fux();
    let dorian: ModeSpec = "dorian".parse().unwrap();
    let choir = vec![
        VoiceType::Soprano,
        VoiceType::Alto,
        VoiceType::Tenor,
        VoiceType::Bass,
    ];
    let kyrie = Arc::new(resolve("kyrie", &TextOptions::default()).unwrap().0);
    let planned = plan(&kyrie, dorian, choir.len(), None, None, true).unwrap();
    let untexted = FreeRequest {
        cadences: planned.cadences,
        ..FreeRequest::new(dorian, choir.clone(), planned.measures)
    };
    let texted = FreeRequest {
        text: Some(FreeText::new(kyrie)),
        ..FreeRequest::new(dorian, choir, 0)
    };
    let mut group = c.benchmark_group("free");
    group
        .sample_size(10)
        .sampling_mode(SamplingMode::Flat)
        .measurement_time(std::time::Duration::from_secs(30));
    group.bench_function("free", |b| {
        b.iter(|| generate_free(&untexted, &profile).unwrap());
    });
    group.bench_function("free_text", |b| {
        b.iter(|| generate_free(&texted, &profile).unwrap());
    });
    group.finish();
}

criterion_group!(benches, search, free);
criterion_main!(benches);
