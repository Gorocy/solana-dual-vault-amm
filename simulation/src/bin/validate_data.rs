use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;
use tracing::{error, info, warn};
use tracing_subscriber;

const TOP_FOLDERS: [u32; 3] = [32, 128, 512];
const SUBFOLDERS: [&str; 4] = ["dual", "dual_no_arb", "single", "single_no_arb"];
const SEED_START: u32 = 42;
const SEED_END: u32 = 1041;

/// Counts total lines and empty lines in a file
fn count_lines(path: &Path) -> Result<(usize, usize), std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut total = 0;
    let mut empty = 0;

    for line in reader.lines() {
        let line = line?;
        total += 1;
        if line.trim().is_empty() {
            empty += 1;
        }
    }

    Ok((total, empty))
}

fn validate_seed(
    top: u32,
    sub_path: &Path,
    seed: u32,
    has_errors: &AtomicBool,
) {
    let base_path = sub_path.join(format!("seed_{}.csv", seed));
    let compute_path = sub_path.join(format!("seed_{}_compute.csv", seed));

    // ---- seed_X.csv ----
    match fs::metadata(&base_path) {
        Ok(_) => match count_lines(&base_path) {
            Ok((total, empty)) => {
                if total != 2003 || empty != 0 {
                    warn!(
                        "Invalid line count in {} (total={}, empty={})",
                          base_path.display(),
                          total,
                          empty
                    );
                    has_errors.store(true, Ordering::Relaxed);
                }
            }
            Err(e) => {
                error!("Cannot read {}: {}", base_path.display(), e);
                has_errors.store(true, Ordering::Relaxed);
            }
        },
        Err(_) => {
            error!("Missing file: {}", base_path.display());
            has_errors.store(true, Ordering::Relaxed);
        }
    }

    // ---- seed_X_compute.csv ----
    match fs::metadata(&compute_path) {
        Ok(_) => match count_lines(&compute_path) {
            Ok((total, _)) => {
                let expected_min = 1000 * (top - 1.max(top.isqrt().isqrt().isqrt())) as usize;
                if total < expected_min {
                    warn!(
                        "Too few lines in {} (total={}, expected at least {})",
                          compute_path.display(),
                          total,
                          expected_min
                    );
                    has_errors.store(true, Ordering::Relaxed);
                }
            }
            Err(e) => {
                error!("Cannot read {}: {}", compute_path.display(), e);
                has_errors.store(true, Ordering::Relaxed);
            }
        },
        Err(_) => {
            error!("Missing file: {}", compute_path.display());
            has_errors.store(true, Ordering::Relaxed);
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let has_errors = AtomicBool::new(false);

    TOP_FOLDERS.par_iter().for_each(|&top| {
        let top_path = PathBuf::from(top.to_string());

        if !top_path.is_dir() {
            error!("Missing top-level folder: {}", top_path.display());
            has_errors.store(true, Ordering::Relaxed);
            return;
        }

        SUBFOLDERS.par_iter().for_each(|sub| {
            let sub_path = top_path.join(sub);

            if !sub_path.is_dir() {
                error!("Missing subfolder: {}", sub_path.display());
                has_errors.store(true, Ordering::Relaxed);
                return;
            }

            (SEED_START..=SEED_END)
            .into_par_iter()
            .for_each(|seed| {
                validate_seed(top, &sub_path, seed, &has_errors);
            });
        });
    });

    if !has_errors.load(Ordering::Relaxed) {
        info!("All folders and files are valid.");
    } else {
        warn!("Validation finished with errors.");
    }
}
