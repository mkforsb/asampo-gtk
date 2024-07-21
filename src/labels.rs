// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use libasampo::samplesets::DrumkitLabel;

pub const DRUM_LABELS: [(&str, DrumkitLabel); 16] = [
    ("Rimshot", DrumkitLabel::RimShot),
    ("Clap", DrumkitLabel::Clap),
    ("Closed Hihat", DrumkitLabel::ClosedHihat),
    ("Open Hihat", DrumkitLabel::OpenHihat),
    ("Crash Cymbal", DrumkitLabel::CrashCymbal),
    ("Ride Cymbal", DrumkitLabel::RideCymbal),
    ("Shaker", DrumkitLabel::Shaker),
    ("Percussion #1", DrumkitLabel::Perc1),
    ("Bassdrum", DrumkitLabel::BassDrum),
    ("Snare", DrumkitLabel::SnareDrum),
    ("Low Tom", DrumkitLabel::LowTom),
    ("Middle Tom", DrumkitLabel::MidTom),
    ("High Tom", DrumkitLabel::HighTom),
    ("Percussion #2", DrumkitLabel::Perc2),
    ("Percussion #3", DrumkitLabel::Perc3),
    ("Percussion #4", DrumkitLabel::Perc4),
];
