use std::time::Duration;
use parks::{generate_parks_puzzle, print_board};

fn main() {
    println!("===============================================================");
    println!("             PARKS PUZZLE GENERATOR (HIGH COMPLEXITY)          ");
    println!("===============================================================");

    for &(size, difficulty) in &[
        (8, "Easy"),
        (10, "Medium"),
        (12, "Hard"),
        (14, "Hard"),
        (16, "Hard"),
        (20, "Hard"),
    ] {
        let timeout = Duration::from_secs(5);
        if let Some(lvl) = generate_parks_puzzle(size, difficulty, timeout) {
            print_board(&lvl);
            let min_s = lvl.region_sizes.iter().min().copied().unwrap_or(0);
            let max_s = lvl.region_sizes.iter().max().copied().unwrap_or(0);
            let avg_s = lvl.region_sizes.iter().sum::<usize>() as f64 / size as f64;
            println!(
                "-> Generated {}x{} [{}] in {:.4}s ({} attempts) | Region sizes: min={}, max={}, avg={:.1}",
                size,
                size,
                difficulty,
                lvl.elapsed.as_secs_f64(),
                lvl.attempts,
                min_s,
                max_s,
                avg_s
            );
        } else {
            println!("Failed {}x{} [{}]", size, size, difficulty);
        }
    }
}
