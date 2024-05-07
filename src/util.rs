// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

const GIBIBYTE: u64 = 1024 * 1024 * 1024;
const MEBIBYTE: u64 = 1024 * 1024;
const KIBIBYTE: u64 = 1024;

pub fn readable_size(n_bytes: Option<u64>) -> String {
    if let Some(n) = n_bytes {
        if n >= GIBIBYTE {
            format!("{:.1} GiB", (n as f64) / (GIBIBYTE as f64))
        } else if n > MEBIBYTE {
            format!("{:.1} MiB", (n as f64) / (MEBIBYTE as f64))
        } else if n > KIBIBYTE {
            format!("{:.1} KiB", (n as f64) / (KIBIBYTE as f64))
        } else {
            format!("{} byte", n)
        }
    } else {
        "Unknown".to_string()
    }
}

const SECOND: u64 = 1000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;

pub fn readable_length(millis: Option<u64>) -> String {
    if let Some(mut ms) = millis {
        if ms >= MINUTE {
            let mut fragments = Vec::<String>::new();

            if ms >= 2 * HOUR {
                fragments.push(format!("{} hours", ms / HOUR));
                ms = ms % HOUR;
            } else if ms >= HOUR {
                fragments.push(format!("{} hour", ms / HOUR));
                ms = ms % HOUR;
            }

            if ms >= 2 * MINUTE {
                fragments.push(format!("{} minutes", ms / MINUTE));
                ms = ms % MINUTE;
            } else if ms >= MINUTE {
                fragments.push(format!("{} minute", ms / MINUTE));
                ms = ms % MINUTE;
            }

            if ms >= 2 * SECOND {
                fragments.push(format!("{} seconds", ms / SECOND));
            } else if ms >= SECOND {
                fragments.push(format!("{} second", ms / SECOND));
            }

            fragments.join(", ")
        } else {
            format!("{:.1} seconds", (ms as f64) / (SECOND as f64))
        }
    } else {
        "Unknown".to_string()
    }
}
