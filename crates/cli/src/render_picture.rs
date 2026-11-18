//! Statistics rendering for picture comparison.

use autocompare_compare::PictureComparison;

use crate::term;

/// Whether the comparison found any difference.
pub fn outcome(cmp: &PictureComparison) -> crate::Outcome {
    if cmp.diff.identical {
        crate::Outcome::Same
    } else {
        crate::Outcome::Different
    }
}

/// Prints sizes, the differing-pixel share over the overlap, the
/// bounding box of differences, and where the heatmap went.
pub fn print(cmp: &PictureComparison, color: bool, heatmap: Option<&str>) {
    let d = &cmp.diff;
    println!("left:  {}x{}", d.width_l, d.height_l);
    println!("right: {}x{}", d.width_r, d.height_r);
    if d.identical {
        println!("no differences");
    } else {
        let pct = d.differing as f64 / d.total as f64 * 100.0;
        println!(
            "overlap: {}x{} ({} pixels)",
            d.overlap_w, d.overlap_h, d.total
        );
        println!(
            "{}",
            term::paint(
                color,
                term::YELLOW,
                &format!("differing pixels: {} ({pct:.2}%)", d.differing)
            )
        );
        if let Some((x0, y0, x1, y1)) = d.bbox {
            println!("difference bounds: ({x0},{y0})..({x1},{y1})");
        }
        if d.width_l != d.width_r || d.height_l != d.height_r {
            eprintln!("autocompare: note: image sizes differ; only the overlap was compared");
        }
    }
    if let Some(path) = heatmap {
        println!(
            "{}",
            term::paint(color, term::DIM, &format!("heatmap written to {path}"))
        );
    }
}
