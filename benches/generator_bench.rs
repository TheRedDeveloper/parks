use parks::{
    all_regions_connected, build_neighbor_masks, construct_gadget_regions,
    generate_parks_puzzle_with_rng, generate_tree_solution, solve_fast, SimpleRng,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[allow(dead_code)]
struct BenchStats {
    name: String,
    iterations: usize,
    total_time: Duration,
    min_time: Duration,
    max_time: Duration,
    mean_time: Duration,
    median_time: Duration,
}

impl BenchStats {
    fn print(&self) {
        let mean_micros = self.mean_time.as_secs_f64() * 1_000_000.0;
        let median_micros = self.median_time.as_secs_f64() * 1_000_000.0;
        let min_micros = self.min_time.as_secs_f64() * 1_000_000.0;
        let max_micros = self.max_time.as_secs_f64() * 1_000_000.0;
        let ops_per_sec = if self.mean_time.as_secs_f64() > 0.0 {
            1.0 / self.mean_time.as_secs_f64()
        } else {
            0.0
        };

        println!(
            "  {:<32} | {:>7} iters | mean: {:>9.2} µs | median: {:>9.2} µs | min: {:>9.2} µs | max: {:>9.2} µs | {:>9.1} ops/s",
            self.name,
            self.iterations,
            mean_micros,
            median_micros,
            min_micros,
            max_micros,
            ops_per_sec,
        );
    }
}

fn bench<F>(name: &str, warmup_dur: Duration, target_dur: Duration, mut f: F) -> BenchStats
where
    F: FnMut(),
{
    // Warmup phase
    let warmup_start = Instant::now();
    while warmup_start.elapsed() < warmup_dur {
        f();
    }

    // Measurement phase
    let mut durations = Vec::new();
    let start = Instant::now();
    while start.elapsed() < target_dur || durations.len() < 10 {
        let t0 = Instant::now();
        f();
        let elapsed = t0.elapsed();
        durations.push(elapsed);
        if durations.len() >= 100_000 {
            break;
        }
    }

    durations.sort();
    let iterations = durations.len();
    let total_time: Duration = durations.iter().copied().sum();
    let min_time = durations[0];
    let max_time = durations[iterations - 1];
    let mean_time = total_time / (iterations as u32);
    let median_time = durations[iterations / 2];

    let stats = BenchStats {
        name: name.to_string(),
        iterations,
        total_time,
        min_time,
        max_time,
        mean_time,
        median_time,
    };
    stats.print();
    stats
}

fn main() {
    println!("=========================================================================================");
    println!("                               PARKS BENCHMARK SUITE                                     ");
    println!("=========================================================================================");

    println!("\n--- 1. Sub-component: generate_tree_solution ---");
    for &n in &[8, 10, 14, 20] {
        let mut rng = SimpleRng::with_seed(12345);
        bench(
            &format!("tree_solution_{}x{}", n, n),
            Duration::from_millis(50),
            Duration::from_millis(200),
            || {
                black_box(generate_tree_solution(black_box(n), &mut rng));
            },
        );
    }

    println!("\n--- 2. Sub-component: construct_gadget_regions ---");
    for &(n, diff) in &[(8, "Easy"), (10, "Medium"), (14, "Hard"), (20, "Hard")] {
        let mut rng = SimpleRng::with_seed(12345);
        let trees = generate_tree_solution(n, &mut rng);
        bench(
            &format!("gadget_regions_{}x{}_{}", n, n, diff),
            Duration::from_millis(50),
            Duration::from_millis(200),
            || {
                black_box(construct_gadget_regions(
                    black_box(n),
                    black_box(&trees),
                    black_box(diff),
                    &mut rng,
                ));
            },
        );
    }

    println!("\n--- 3. Sub-component: all_regions_connected ---");
    for &(n, diff) in &[(8, "Easy"), (10, "Medium"), (14, "Hard"), (20, "Hard")] {
        let mut rng = SimpleRng::with_seed(12345);
        let trees = generate_tree_solution(n, &mut rng);
        let (regions, _, _) = construct_gadget_regions(n, &trees, diff, &mut rng);
        bench(
            &format!("connectivity_{}x{}_{}", n, n, diff),
            Duration::from_millis(50),
            Duration::from_millis(200),
            || {
                black_box(all_regions_connected(black_box(n), black_box(&regions)));
            },
        );
    }

    println!("\n--- 4. Sub-component: solve_fast (MRV bitmask solver) ---");
    for &(n, diff) in &[(8, "Easy"), (10, "Medium"), (14, "Hard")] {
        let mut rng = SimpleRng::with_seed(12345);
        let trees = generate_tree_solution(n, &mut rng);
        let (regions, _, _) = construct_gadget_regions(n, &trees, diff, &mut rng);
        let nbr_masks = build_neighbor_masks(n);
        bench(
            &format!("solve_fast_{}x{}_{}", n, n, diff),
            Duration::from_millis(50),
            Duration::from_millis(200),
            || {
                black_box(solve_fast(
                    black_box(n),
                    black_box(&regions),
                    black_box(&nbr_masks),
                    black_box(2),
                ));
            },
        );
    }

    println!("\n--- 5. End-to-End: generate_parks_puzzle ---");
    for &(n, diff) in &[
        (8, "Easy"),
        (10, "Medium"),
        (12, "Hard"),
        (14, "Hard"),
        (16, "Hard"),
        (20, "Hard"),
    ] {
        let mut rng = SimpleRng::with_seed(42);
        bench(
            &format!("full_gen_{}x{}_{}", n, n, diff),
            Duration::from_millis(100),
            Duration::from_millis(800),
            || {
                black_box(generate_parks_puzzle_with_rng(
                    black_box(n),
                    black_box(diff),
                    Duration::from_secs(10),
                    &mut rng,
                ));
            },
        );
    }
    println!("\n=========================================================================================\n");
}
