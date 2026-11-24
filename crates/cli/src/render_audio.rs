//! Statistics rendering for audio comparison.

use autocompare_compare::AudioComparison;

use crate::term;

/// Whether the comparison found any difference.
pub fn outcome(cmp: &AudioComparison) -> crate::Outcome {
    if cmp.diff.identical {
        crate::Outcome::Same
    } else {
        crate::Outcome::Different
    }
}

/// Prints durations, stream configuration, and the differing share of
/// 10 ms windows over the common duration.
pub fn print(cmp: &AudioComparison, color: bool) {
    let d = &cmp.diff;
    println!(
        "left:  {:.3}s, {} Hz, {} channel(s)",
        d.duration_l, d.sample_rate_l, d.channels_l
    );
    println!(
        "right: {:.3}s, {} Hz, {} channel(s)",
        d.duration_r, d.sample_rate_r, d.channels_r
    );
    if d.config_mismatch {
        eprintln!(
            "autocompare: note: stream configurations differ; sample comparison is not meaningful and was skipped"
        );
    } else if d.identical {
        println!("no differences");
    } else {
        match d.first_diff_at {
            Some(at) => {
                println!(
                    "{}",
                    term::paint(
                        color,
                        term::YELLOW,
                        &format!("first difference at {at:.3}s")
                    )
                );
            }
            None => println!("first difference: -"),
        }
        let pct = if d.windows_total > 0 {
            d.windows_differing as f64 / d.windows_total as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "differing 10ms windows: {}/{} ({pct:.2}%)",
            d.windows_differing, d.windows_total
        );
    }
}
