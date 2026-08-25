use std::collections::{HashSet, VecDeque};
use std::f64::consts::PI;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct SimpleRng(u64);

impl SimpleRng {
    pub fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x853c49e6748fea9b);
        Self(nanos ^ 0xda942042e4dd58b5)
    }

    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    #[inline(always)]
    pub fn gen_range(&mut self, low: usize, high: usize) -> usize {
        if high <= low {
            return low;
        }
        low + (self.next_u64() as usize % (high - low))
    }

    #[inline(always)]
    pub fn gen_f64(&mut self, low: f64, high: f64) -> f64 {
        let unit = (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0);
        low + unit * (high - low)
    }

    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        if slice.is_empty() {
            return;
        }
        for i in (1..slice.len()).rev() {
            let j = self.gen_range(0, i + 1);
            slice.swap(i, j);
        }
    }
}

// 16 distinct RGB palette colors for region backgrounds
const PALETTE: [(u8, u8, u8); 16] = [
    (190, 55, 55),   // 0: Crimson Red
    (35, 120, 210),  // 1: Blue
    (40, 150, 60),   // 2: Forest Green
    (210, 125, 20),  // 3: Amber / Dark Orange
    (145, 60, 175),  // 4: Purple
    (25, 155, 160),  // 5: Cyan / Teal
    (205, 75, 135),  // 6: Rose Pink
    (110, 125, 35),  // 7: Olive
    (175, 95, 65),   // 8: Terracotta / Brown
    (75, 85, 175),   // 9: Slate Indigo
    (30, 145, 115),  // 10: Jade / Sea Green
    (165, 50, 105),  // 11: Berry / Wine
    (105, 145, 30),  // 12: Lime Green
    (150, 105, 165), // 13: Lavender
    (195, 145, 40),  // 14: Warm Gold
    (70, 120, 145),  // 15: Steel Blue
];

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct BitSet256(pub [u64; 4]);

impl BitSet256 {
    #[inline(always)]
    pub fn set_bit(&mut self, idx: usize) {
        self.0[idx / 64] |= 1u64 << (idx % 64);
    }

    #[inline(always)]
    pub fn test_bit(&self, idx: usize) -> bool {
        (self.0[idx / 64] & (1u64 << (idx % 64))) != 0
    }

    #[inline(always)]
    pub fn union_with(&self, other: &Self) -> Self {
        Self([
            self.0[0] | other.0[0],
            self.0[1] | other.0[1],
            self.0[2] | other.0[2],
            self.0[3] | other.0[3],
        ])
    }
}

pub struct ParksLevel {
    pub size: usize,
    pub difficulty: String,
    pub solution_trees: Vec<(usize, usize)>,
    pub regions: Vec<Vec<usize>>,
    pub region_sizes: Vec<usize>,
    pub attempts: usize,
    pub elapsed: Duration,
}

pub fn build_neighbor_masks(n: usize) -> Vec<BitSet256> {
    let mut masks = vec![BitSet256::default(); n * n];
    for r in 0..n {
        for c in 0..n {
            let idx = r * n + c;
            let mut mask = BitSet256::default();
            for dr in -1i32..=1 {
                for dc in -1i32..=1 {
                    let nr = r as i32 + dr;
                    let nc = c as i32 + dc;
                    if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 {
                        mask.set_bit(nr as usize * n + nc as usize);
                    }
                }
            }
            masks[idx] = mask;
        }
    }
    masks
}

pub fn generate_tree_solution(n: usize, rng: &mut SimpleRng) -> Vec<(usize, usize)> {
    let mut grid = vec![vec![0u8; n]; n];
    let mut trees = Vec::with_capacity(n);

    fn is_safe(n: usize, grid: &[Vec<u8>], r: usize, c: usize) -> bool {
        for dr in -1i32..=1 {
            for dc in -1i32..=1 {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 && grid[nr as usize][nc as usize] == 1 {
                    return false;
                }
            }
        }
        true
    }

    fn backtrack(
        n: usize,
        r: usize,
        grid: &mut [Vec<u8>],
        trees: &mut Vec<(usize, usize)>,
        rng: &mut SimpleRng,
    ) -> bool {
        if r == n {
            return true;
        }
        let mut cols: Vec<usize> = (0..n).collect();
        rng.shuffle(&mut cols);

        for &c in &cols {
            if trees.iter().all(|&(_, tc)| tc != c) && is_safe(n, grid, r, c) {
                grid[r][c] = 1;
                trees.push((r, c));
                if backtrack(n, r + 1, grid, trees, rng) {
                    return true;
                }
                trees.pop();
                grid[r][c] = 0;
            }
        }
        false
    }

    backtrack(n, 0, &mut grid, &mut trees, rng);
    trees
}

#[derive(Clone, Copy)]
struct CellInfo {
    idx: usize,
    r: usize,
    c: usize,
    reg: usize,
}

pub fn solve_fast(n: usize, regions: &[Vec<usize>], nbr_masks: &[BitSet256], limit: usize) -> Vec<Vec<(usize, usize)>> {
    let mut solutions = Vec::new();
    let mut row_cells: Vec<Vec<CellInfo>> = vec![Vec::with_capacity(n); n];
    let mut reg_cells: Vec<Vec<CellInfo>> = vec![Vec::with_capacity(n); n];

    for r in 0..n {
        for c in 0..n {
            let idx = r * n + c;
            let reg = regions[r][c];
            let info = CellInfo { idx, r, c, reg };
            row_cells[r].push(info);
            reg_cells[reg].push(info);
        }
    }

    fn search(
        n: usize,
        limit: usize,
        rows_left: u32,
        cols_used: u32,
        regs_left: u32,
        blocked_mask: BitSet256,
        placed_trees: &mut Vec<(usize, usize)>,
        row_cells: &[Vec<CellInfo>],
        reg_cells: &[Vec<CellInfo>],
        nbr_masks: &[BitSet256],
        solutions: &mut Vec<Vec<(usize, usize)>>,
    ) {
        if solutions.len() >= limit {
            return;
        }
        if rows_left == 0 {
            solutions.push(placed_trees.clone());
            return;
        }

        let mut best_cells = [CellInfo { idx: 0, r: 0, c: 0, reg: 0 }; 256];
        let mut best_len = 0;
        let mut min_cands = 999usize;

        // MRV on rows
        for r in 0..n {
            if (rows_left & (1 << r)) == 0 {
                continue;
            }
            let mut count = 0;
            let mut temp_cells = [CellInfo { idx: 0, r: 0, c: 0, reg: 0 }; 256];
            for &info in &row_cells[r] {
                if (cols_used & (1 << info.c)) != 0
                    || (regs_left & (1 << info.reg)) == 0
                    || blocked_mask.test_bit(info.idx)
                {
                    continue;
                }
                temp_cells[count] = info;
                count += 1;
            }
            if count < min_cands {
                min_cands = count;
                best_len = count;
                best_cells[..count].copy_from_slice(&temp_cells[..count]);
                if min_cands <= 1 {
                    break;
                }
            }
        }

        // MRV on regions
        if min_cands > 1 {
            for reg in 0..n {
                if (regs_left & (1 << reg)) == 0 {
                    continue;
                }
                let mut count = 0;
                let mut temp_cells = [CellInfo { idx: 0, r: 0, c: 0, reg: 0 }; 256];
                for &info in &reg_cells[reg] {
                    if (rows_left & (1 << info.r)) == 0
                        || (cols_used & (1 << info.c)) != 0
                        || blocked_mask.test_bit(info.idx)
                    {
                        continue;
                    }
                    temp_cells[count] = info;
                    count += 1;
                }
                if count < min_cands {
                    min_cands = count;
                    best_len = count;
                    best_cells[..count].copy_from_slice(&temp_cells[..count]);
                    if min_cands <= 1 {
                        break;
                    }
                }
            }
        }

        if min_cands == 0 {
            return;
        }

        for i in 0..best_len {
            let info = best_cells[i];
            placed_trees.push((info.r, info.c));
            search(
                n,
                limit,
                rows_left & !(1 << info.r),
                cols_used | (1 << info.c),
                regs_left & !(1 << info.reg),
                blocked_mask.union_with(&nbr_masks[info.idx]),
                placed_trees,
                row_cells,
                reg_cells,
                nbr_masks,
                solutions,
            );
            placed_trees.pop();
            if solutions.len() >= limit {
                return;
            }
        }
    }

    let all_bits = (1u32 << n) - 1;
    let mut placed = Vec::with_capacity(n);
    search(
        n,
        limit,
        all_bits,
        0,
        all_bits,
        BitSet256::default(),
        &mut placed,
        &row_cells,
        &reg_cells,
        nbr_masks,
        &mut solutions,
    );
    solutions
}

pub fn generate_non_collinear_polyomino(
    tr: usize,
    tc: usize,
    n: usize,
    regions: &[Vec<i32>],
    target_size: usize,
    rng: &mut SimpleRng,
) -> Vec<(usize, usize)> {
    let dirs = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)];
    let mut last_cells = Vec::new();

    for _ in 0..40 {
        let mut cells = HashSet::new();
        cells.insert((tr, tc));
        let mut frontier = Vec::new();

        for &(dr, dc) in &dirs {
            let nr = tr as i32 + dr;
            let nc = tc as i32 + dc;
            if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 && regions[nr as usize][nc as usize] == -1 {
                frontier.push((nr as usize, nc as usize));
            }
        }

        while cells.len() < target_size && !frontier.is_empty() {
            let idx = rng.gen_range(0, frontier.len());
            let (r, c) = frontier.swap_remove(idx);
            if cells.contains(&(r, c)) || regions[r][c] != -1 {
                continue;
            }
            cells.insert((r, c));
            for &(dr, dc) in &dirs {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if nr >= 0
                    && nr < n as i32
                    && nc >= 0
                    && nc < n as i32
                    && regions[nr as usize][nc as usize] == -1
                    && !cells.contains(&(nr as usize, nc as usize))
                {
                    frontier.push((nr as usize, nc as usize));
                }
            }
        }

        let min_r = cells.iter().map(|&(r, _)| r).min().unwrap();
        let max_r = cells.iter().map(|&(r, _)| r).max().unwrap();
        let min_c = cells.iter().map(|&(_, c)| c).min().unwrap();
        let max_c = cells.iter().map(|&(_, c)| c).max().unwrap();

        let res: Vec<(usize, usize)> = cells.into_iter().collect();
        if res.len() >= 3 && (max_r > min_r) && (max_c > min_c) {
            return res;
        }
        last_cells = res;
    }
    last_cells
}

pub fn is_region_connected(n: usize, regions: &[Vec<usize>], reg_id: usize) -> bool {
    let mut start = None;
    let mut count = 0;
    for r in 0..n {
        for c in 0..n {
            if regions[r][c] == reg_id {
                if start.is_none() {
                    start = Some((r, c));
                }
                count += 1;
            }
        }
    }
    let (sr, sc) = match start {
        Some(s) => s,
        None => return true,
    };

    let mut visited = vec![vec![false; n]; n];
    let mut q = VecDeque::new();
    visited[sr][sc] = true;
    q.push_back((sr, sc));
    let mut seen = 0;

    while let Some((r, c)) = q.pop_front() {
        seen += 1;
        for &(dr, dc) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 {
                let ur = nr as usize;
                let uc = nc as usize;
                if regions[ur][uc] == reg_id && !visited[ur][uc] {
                    visited[ur][uc] = true;
                    q.push_back((ur, uc));
                }
            }
        }
    }
    seen == count
}

pub fn all_regions_connected(n: usize, regions: &[Vec<usize>]) -> bool {
    for reg_id in 0..n {
        if !is_region_connected(n, regions, reg_id) {
            return false;
        }
    }
    true
}

pub fn construct_gadget_regions(
    n: usize,
    trees: &[(usize, usize)],
    _difficulty: &str,
    rng: &mut SimpleRng,
) -> (Vec<Vec<usize>>, Vec<usize>, usize) {
    let mut regions = vec![vec![-1i32; n]; n];
    let mut reg_sizes = vec![0usize; n];

    for (reg_id, &(tr, tc)) in trees.iter().enumerate() {
        regions[tr][tc] = reg_id as i32;
        reg_sizes[reg_id] = 1;
    }

    let mut elim_time = vec![vec![999i32; n]; n];

    // 1. Group 0: 2D non-linear polyomino (L-shape, elongated L, S, Z, T, box)
    let (tr0, tc0) = trees[0];
    let g0_size = if n <= 10 { rng.gen_range(3, 5) } else { 3 };
    let g0_cells = generate_non_collinear_polyomino(tr0, tc0, n, &regions, g0_size, rng);
    for &(r, c) in &g0_cells {
        if regions[r][c] == -1 {
            regions[r][c] = 0;
            reg_sizes[0] += 1;
        }
    }

    if !g0_cells.is_empty() {
        let min_r = g0_cells.iter().map(|&(r, _)| r).min().unwrap();
        let max_r = g0_cells.iter().map(|&(r, _)| r).max().unwrap();
        let min_c = g0_cells.iter().map(|&(_, c)| c).min().unwrap();
        let max_c = g0_cells.iter().map(|&(_, c)| c).max().unwrap();
        if (max_r - min_r <= 1) && (max_c - min_c <= 1) {
            for r in min_r..=max_r {
                for c in min_c..=max_c {
                    if regions[r][c] != 0 {
                        elim_time[r][c] = elim_time[r][c].min(0);
                    }
                }
            }
        }
    }

    // 2. Anchor 1 grows 1 adjacent cell
    let (tr1, tc1) = trees[1];
    let mut nbrs = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)];
    rng.shuffle(&mut nbrs);
    for &(dr, dc) in &nbrs {
        let nr = tr1 as i32 + dr;
        let nc = tc1 as i32 + dc;
        if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 && regions[nr as usize][nc as usize] == -1 {
            regions[nr as usize][nc as usize] = 1;
            reg_sizes[1] += 1;
            if dr == 0 {
                for c in 0..n {
                    if regions[tr1][c] != 1 {
                        elim_time[tr1][c] = elim_time[tr1][c].min(0);
                    }
                }
            } else {
                for r in 0..n {
                    if regions[r][tc1] != 1 {
                        elim_time[r][tc1] = elim_time[r][tc1].min(0);
                    }
                }
            }
            break;
        }
    }

    let actual_anchors = 2;

    // 3. Setup elimination timestamps for non-anchor trees
    for step in actual_anchors..n {
        let (tr, tc) = trees[step];
        let s_i = step as i32;
        for c in 0..n {
            elim_time[tr][c] = elim_time[tr][c].min(s_i);
        }
        for r in 0..n {
            elim_time[r][tc] = elim_time[r][tc].min(s_i);
        }
        for dr in -1i32..=1 {
            for dc in -1i32..=1 {
                let nr = tr as i32 + dr;
                let nc = tc as i32 + dc;
                if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 {
                    elim_time[nr as usize][nc as usize] = elim_time[nr as usize][nc as usize].min(s_i);
                }
            }
        }
    }

    // 4. Strict Invariant Flood Fill for non-anchor regions
    let mut queue = VecDeque::new();
    for reg_id in actual_anchors..n {
        let (tr, tc) = trees[reg_id];
        for &(dr, dc) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nr = tr as i32 + dr;
            let nc = tc as i32 + dc;
            if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 && regions[nr as usize][nc as usize] == -1 {
                queue.push_back((nr as usize, nc as usize, reg_id));
            }
        }
    }

    while let Some((r, c, reg_id)) = queue.pop_front() {
        if regions[r][c] != -1 {
            continue;
        }
        if elim_time[r][c] < reg_id as i32 {
            regions[r][c] = reg_id as i32;
            reg_sizes[reg_id] += 1;
            for &(dr, dc) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 && regions[nr as usize][nc as usize] == -1 {
                    queue.push_back((nr as usize, nc as usize, reg_id));
                }
            }
        }
    }

    // 5. Strictly Connected Straggler Drain
    loop {
        let mut unassigned = Vec::new();
        for r in 0..n {
            for c in 0..n {
                if regions[r][c] == -1 {
                    unassigned.push((r, c));
                }
            }
        }
        if unassigned.is_empty() {
            break;
        }
        unassigned.sort_by_key(|&(r, c)| elim_time[r][c]);
        let mut progress = false;

        for &(r, c) in &unassigned {
            if regions[r][c] != -1 {
                continue;
            }
            let mut adj_neighbors = Vec::with_capacity(4);
            let mut valid_neighbors = Vec::with_capacity(4);

            for &(dr, dc) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 {
                    let n_reg = regions[nr as usize][nc as usize];
                    if n_reg != -1 {
                        let u_reg = n_reg as usize;
                        adj_neighbors.push(u_reg);
                        if u_reg >= actual_anchors && elim_time[r][c] < n_reg {
                            valid_neighbors.push(u_reg);
                        }
                    }
                }
            }

            if let Some(&chosen) = valid_neighbors.iter().max() {
                regions[r][c] = chosen as i32;
                reg_sizes[chosen] += 1;
                progress = true;
            } else if let Some(&chosen) = adj_neighbors.iter().max() {
                regions[r][c] = chosen as i32;
                reg_sizes[chosen] += 1;
                progress = true;
            }
        }

        if !progress {
            break;
        }
    }

    let final_regions: Vec<Vec<usize>> = regions
        .into_iter()
        .map(|row| row.into_iter().map(|val| val as usize).collect())
        .collect();

    (final_regions, reg_sizes, 2)
}

pub fn generate_parks_puzzle(n: usize, difficulty: &str, timeout: Duration) -> Option<ParksLevel> {
    let start_time = Instant::now();
    let nbr_masks = build_neighbor_masks(n);
    let mut rng = SimpleRng::new();
    let mut attempts = 0;

    while start_time.elapsed() < timeout {
        attempts += 1;
        let mut trees = generate_tree_solution(n, &mut rng);

        // Random sweep direction vector so Group 0 starts in random locations/corners
        let angle: f64 = rng.gen_f64(0.0, 2.0 * PI);
        let vr = angle.cos();
        let vc = angle.sin();
        trees.sort_by(|a, b| {
            let val_a = a.0 as f64 * vr + a.1 as f64 * vc + rng.gen_f64(-0.1, 0.1);
            let val_b = b.0 as f64 * vr + b.1 as f64 * vc + rng.gen_f64(-0.1, 0.1);
            val_a.partial_cmp(&val_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let (regions, reg_sizes, min_allowed) = construct_gadget_regions(n, &trees, difficulty, &mut rng);

        if reg_sizes.iter().any(|&s| s < min_allowed) {
            continue;
        }
        if !all_regions_connected(n, &regions) {
            continue;
        }

        let sols = solve_fast(n, &regions, &nbr_masks, 2);
        if sols.len() == 1 {
            return Some(ParksLevel {
                size: n,
                difficulty: difficulty.to_string(),
                solution_trees: trees,
                regions,
                region_sizes: reg_sizes,
                attempts,
                elapsed: start_time.elapsed(),
            });
        }
    }

    println!("Generation timed out after {:?} ({} attempts).", timeout, attempts);
    None
}

pub fn get_region_ansi(reg_id: usize, is_tree: bool) -> String {
    let (r, g, b) = PALETTE[reg_id % PALETTE.len()];
    let bg_code = format!("\x1b[48;2;{};{};{}m", r, g, b);
    let luminance = 0.299 * (r as f64) + 0.587 * (g as f64) + 0.114 * (b as f64);
    let fg_code = if luminance > 140.0 {
        "\x1b[38;2;0;0;0m"
    } else {
        "\x1b[38;2;255;255;255m"
    };

    if is_tree {
        format!("{}\x1b[1;93m", bg_code)
    } else {
        format!("{}{}", bg_code, fg_code)
    }
}

pub fn print_board(level: &ParksLevel) {
    let n = level.size;
    let regions = &level.regions;
    let trees: HashSet<(usize, usize)> = level.solution_trees.iter().cloned().collect();
    let reset = "\x1b[0m";

    println!("\n=== Parks Board ({}x{}) | Difficulty: {} ===", n, n, level.difficulty);
    print!("    ");
    for c in 0..n {
        print!("{:3} ", c);
    }
    println!();
    println!("   +{}", "----".repeat(n) + "+");

    for r in 0..n {
        print!("{:2} |", r);
        for c in 0..n {
            let reg = regions[r][c];
            let is_tree = trees.contains(&(r, c));
            let color = get_region_ansi(reg, is_tree);
            let cell_text = if is_tree {
                format!("*{:<2} ", reg)
            } else {
                format!(" {:<2} ", reg)
            };
            print!("{}{}{}", color, cell_text, reset);
        }
        println!("|");
    }

    println!("   +{}", "----".repeat(n) + "+");
    println!("Legend: '*N' indicates Tree for Region N (Colored by Region Background)\n");
}

fn main() {
    for &size in &[8, 10, 12, 14, 16] {
        let timeout = Duration::from_secs(1);
        if let Some(lvl) = generate_parks_puzzle(size, "Medium", timeout) {
            print_board(&lvl);
            println!(
                "-> Generated {}x{} in {:.4}s ({} attempts)",
                size,
                size,
                lvl.elapsed.as_secs_f64(),
                lvl.attempts
            );
        } else {
            println!("Failed {}x{}", size, size);
        }
    }
}

