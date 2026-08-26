use parks::{
    all_regions_connected, build_neighbor_masks, construct_gadget_regions,
    generate_parks_puzzle_with_rng, generate_tree_solution, is_region_connected, solve_fast,
    SimpleRng,
};
use std::collections::HashSet;
use std::time::Duration;

#[test]
fn test_tree_solution_validity() {
    let mut rng = SimpleRng::with_seed(42);
    for n in [6, 8, 10, 12, 14, 16, 20] {
        for _ in 0..20 {
            let trees = generate_tree_solution(n, &mut rng);
            assert_eq!(trees.len(), n, "Should generate exactly N trees for N={}", n);

            let mut rows = HashSet::new();
            let mut cols = HashSet::new();
            for &(r, c) in &trees {
                assert!(r < n && c < n);
                assert!(rows.insert(r), "Duplicate row {}", r);
                assert!(cols.insert(c), "Duplicate col {}", c);
            }

            // Check no two trees touch (8-way)
            for i in 0..trees.len() {
                for j in (i + 1)..trees.len() {
                    let (r1, c1) = trees[i];
                    let (r2, c2) = trees[j];
                    let dr = (r1 as isize - r2 as isize).abs();
                    let dc = (c1 as isize - c2 as isize).abs();
                    assert!(dr > 1 || dc > 1, "Trees at ({},{}) and ({},{}) touch!", r1, c1, r2, c2);
                }
            }
        }
    }
}

#[test]
fn test_gadget_regions_and_connectivity() {
    let mut rng = SimpleRng::with_seed(100);
    for &(n, diff) in &[
        (8, "Easy"),
        (10, "Medium"),
        (12, "Hard"),
        (14, "Hard"),
    ] {
        let trees = generate_tree_solution(n, &mut rng);
        let (regions, reg_sizes, _min_allowed) = construct_gadget_regions(n, &trees, diff, &mut rng);

        assert_eq!(regions.len(), n);
        assert_eq!(reg_sizes.len(), n);

        for (reg_id, &(tr, tc)) in trees.iter().enumerate() {
            assert_eq!(regions[tr][tc], reg_id, "Tree {} must be in region {}", reg_id, reg_id);
        }

        for reg_id in 0..n {
            assert!(is_region_connected(n, &regions, reg_id), "Region {} must be connected", reg_id);
        }
        assert!(all_regions_connected(n, &regions));
    }
}

#[test]
fn test_solver_finds_tree_solution() {
    let mut rng = SimpleRng::with_seed(2024);
    for n in [8, 10, 12] {
        let trees = generate_tree_solution(n, &mut rng);
        let (regions, _, _) = construct_gadget_regions(n, &trees, "Hard", &mut rng);
        let nbr_masks = build_neighbor_masks(n);

        let sols = solve_fast(n, &regions, &nbr_masks, 10);
        assert!(!sols.is_empty(), "Solver should find at least the constructed solution");

        // Verify that constructed solution is in solutions
        let mut sorted_orig = trees.clone();
        sorted_orig.sort();

        let found = sols.iter().any(|sol| {
            let mut s = sol.clone();
            s.sort();
            s == sorted_orig
        });
        assert!(found, "Constructed tree solution must be among the solver's solutions");
    }
}

#[test]
fn test_end_to_end_generator() {
    let mut rng = SimpleRng::with_seed(777);
    for &(n, diff) in &[(8, "Easy"), (10, "Medium"), (12, "Hard")] {
        let lvl = generate_parks_puzzle_with_rng(n, diff, Duration::from_secs(5), &mut rng);
        assert!(lvl.is_some(), "Should successfully generate a {}x{} {} puzzle", n, n, diff);
        let lvl = lvl.unwrap();
        assert_eq!(lvl.size, n);
        assert_eq!(lvl.solution_trees.len(), n);

        let nbr_masks = build_neighbor_masks(n);
        let sols = solve_fast(n, &lvl.regions, &nbr_masks, 2);
        assert_eq!(sols.len(), 1, "Generated puzzle must have uniquely 1 solution");
    }
}
