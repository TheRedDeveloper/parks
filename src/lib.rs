use std::collections::HashSet;
use std::mem::MaybeUninit;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const MAX_N: usize = 32;
pub const MAX_CELLS: usize = MAX_N * MAX_N; // 1024

#[derive(Clone, Copy, Debug)]
pub struct SimpleRng(pub u64);

impl SimpleRng {
    pub fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x853c49e6748fea9b);
        Self(nanos ^ 0xda942042e4dd58b5)
    }

    #[inline(always)]
    pub fn with_seed(seed: u64) -> Self {
        Self(seed ^ 0xda942042e4dd58b5)
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
        let range = (high - low) as u64;
        let r = ((self.next_u64() as u128 * range as u128) >> 64) as usize;
        low + r
    }

    #[inline(always)]
    pub fn gen_f64(&mut self, low: f64, high: f64) -> f64 {
        let unit = (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0);
        low + unit * (high - low)
    }

    #[inline(always)]
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        let len = slice.len();
        if len <= 1 {
            return;
        }
        for i in (1..len).rev() {
            let j = self.gen_range(0, i + 1);
            slice.swap(i, j);
        }
    }
}

// 16 distinct RGB palette colors for region backgrounds
pub const PALETTE: [(u8, u8, u8); 16] = [
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
pub struct BitSet1024(pub [u64; 16]);

impl BitSet1024 {
    #[inline(always)]
    pub fn set_bit(&mut self, idx: usize) {
        self.0[idx >> 6] |= 1u64 << (idx & 63);
    }

    #[inline(always)]
    pub fn test_bit(&self, idx: usize) -> bool {
        (self.0[idx >> 6] & (1u64 << (idx & 63))) != 0
    }

    #[inline(always)]
    pub fn union_with(&self, other: &Self) -> Self {
        let mut res = [0u64; 16];
        for i in 0..16 {
            res[i] = self.0[i] | other.0[i];
        }
        Self(res)
    }

    #[inline(always)]
    pub fn union_words(&self, other: &Self, num_words: usize) -> Self {
        let mut res = self.0;
        match num_words {
            1 => res[0] |= other.0[0],
            2 => {
                res[0] |= other.0[0];
                res[1] |= other.0[1];
            }
            3 => {
                res[0] |= other.0[0];
                res[1] |= other.0[1];
                res[2] |= other.0[2];
            }
            4 => {
                res[0] |= other.0[0];
                res[1] |= other.0[1];
                res[2] |= other.0[2];
                res[3] |= other.0[3];
            }
            5..=8 => {
                for i in 0..num_words {
                    res[i] |= other.0[i];
                }
            }
            _ => {
                for i in 0..num_words {
                    res[i] |= other.0[i];
                }
            }
        }
        Self(res)
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

pub fn build_neighbor_masks(n: usize) -> Vec<BitSet1024> {
    let mut masks = vec![BitSet1024::default(); n * n];
    for r in 0..n {
        for c in 0..n {
            let idx = r * n + c;
            let mut mask = BitSet1024::default();
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

#[inline(always)]
pub fn generate_tree_solution(n: usize, rng: &mut SimpleRng) -> Vec<(usize, usize)> {
    let mut tree_cols = [0usize; MAX_N];
    generate_tree_solution_fast(n, rng, &mut tree_cols);
    (0..n).map(|r| (r, tree_cols[r])).collect()
}

#[inline(always)]
pub fn generate_tree_solution_fast(n: usize, rng: &mut SimpleRng, out_cols: &mut [usize; MAX_N]) -> bool {
    let mut col_order = [0usize; MAX_N];
    for i in 0..n {
        col_order[i] = i;
    }

    fn backtrack(
        r: usize,
        n: usize,
        cols_used: u64,
        prev_col: usize,
        out_cols: &mut [usize; MAX_N],
        col_order: &mut [usize; MAX_N],
        rng: &mut SimpleRng,
    ) -> bool {
        if r == n {
            return true;
        }
        rng.shuffle(&mut col_order[..n]);

        for i in 0..n {
            let c = col_order[i];
            if (cols_used & (1u64 << c)) != 0 {
                continue;
            }
            if prev_col != usize::MAX {
                let diff = (c as isize - prev_col as isize).abs();
                if diff <= 1 {
                    continue;
                }
            }

            out_cols[r] = c;
            if backtrack(
                r + 1,
                n,
                cols_used | (1u64 << c),
                c,
                out_cols,
                col_order,
                rng,
            ) {
                return true;
            }
        }
        false
    }

    backtrack(0, n, 0, usize::MAX, out_cols, &mut col_order, rng)
}

pub fn solve_fast(
    n: usize,
    regions: &[Vec<usize>],
    nbr_masks: &[BitSet1024],
    limit: usize,
) -> Vec<Vec<(usize, usize)>> {
    let mut flat_regions = [0u16; MAX_CELLS];
    for r in 0..n {
        for c in 0..n {
            flat_regions[r * n + c] = regions[r][c] as u16;
        }
    }
    solve_fast_internal(n, &flat_regions, nbr_masks, limit)
}

pub fn solve_fast_internal(
    n: usize,
    flat_regions: &[u16; MAX_CELLS],
    nbr_masks: &[BitSet1024],
    limit: usize,
) -> Vec<Vec<(usize, usize)>> {
    let num_words = (n * n + 63) / 64;
    let mut reg_counts = [0u16; MAX_N];
    let total_cells = n * n;
    for idx in 0..total_cells {
        let reg = flat_regions[idx] as usize;
        reg_counts[reg] += 1;
    }

    let mut reg_offsets = [0u16; MAX_N + 1];
    let mut current_offset = 0u16;
    for reg in 0..n {
        reg_offsets[reg] = current_offset;
        current_offset += reg_counts[reg];
    }
    reg_offsets[n] = current_offset;

    let mut reg_write_ptrs = reg_offsets;
    let mut reg_cells: [MaybeUninit<u16>; MAX_CELLS] = [MaybeUninit::uninit(); MAX_CELLS];
    for idx in 0..total_cells {
        let reg = flat_regions[idx] as usize;
        let pos = reg_write_ptrs[reg] as usize;
        reg_cells[pos].write(idx as u16);
        reg_write_ptrs[reg] += 1;
    }

    let mut solutions = Vec::new();
    let mut placed_trees = [(0u8, 0u8); MAX_N];
    let all_bits = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };

    fn search(
        n: usize,
        num_words: usize,
        depth: usize,
        limit: usize,
        rows_left: u64,
        cols_used: u64,
        regs_left: u64,
        blocked_mask: BitSet1024,
        all_bits: u64,
        placed_trees: &mut [(u8, u8); MAX_N],
        flat_regions: &[u16; MAX_CELLS],
        reg_offsets: &[u16; MAX_N + 1],
        reg_cells: &[MaybeUninit<u16>; MAX_CELLS],
        nbr_masks: &[BitSet1024],
        solutions: &mut Vec<Vec<(usize, usize)>>,
    ) {
        if solutions.len() >= limit {
            return;
        }
        if rows_left == 0 {
            let mut sol = Vec::with_capacity(n);
            for i in 0..n {
                let (r, c) = placed_trees[i];
                sol.push((r as usize, c as usize));
            }
            solutions.push(sol);
            return;
        }

        let mut best_cells: [MaybeUninit<u16>; MAX_N] = [MaybeUninit::uninit(); MAX_N];
        let mut best_len = 0usize;
        let mut min_cands = usize::MAX;

        // MRV on remaining rows
        let mut r_left = rows_left;
        while r_left != 0 {
            let r = r_left.trailing_zeros() as usize;
            r_left &= r_left - 1;

            let mut count = 0;
            let mut temp_cells: [MaybeUninit<u16>; MAX_N] = [MaybeUninit::uninit(); MAX_N];
            let row_offset = r * n;
            let mut cols_avail = (!cols_used) & all_bits;

            while cols_avail != 0 {
                let c = cols_avail.trailing_zeros() as usize;
                cols_avail &= cols_avail - 1;
                let idx = row_offset + c;
                let reg = flat_regions[idx] as usize;
                if (regs_left & (1u64 << reg)) == 0 || blocked_mask.test_bit(idx) {
                    continue;
                }
                temp_cells[count].write(idx as u16);
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

        // MRV on remaining regions
        if min_cands > 1 {
            let mut reg_left = regs_left;
            while reg_left != 0 {
                let reg = reg_left.trailing_zeros() as usize;
                reg_left &= reg_left - 1;

                let mut count = 0;
                let mut temp_cells: [MaybeUninit<u16>; MAX_CELLS] = [MaybeUninit::uninit(); MAX_CELLS];
                let start = reg_offsets[reg] as usize;
                let end = reg_offsets[reg + 1] as usize;
                for i in start..end {
                    let idx_u16 = unsafe { reg_cells[i].assume_init() };
                    let idx = idx_u16 as usize;
                    let r = idx / n;
                    let c = idx % n;
                    if (rows_left & (1u64 << r)) == 0
                        || (cols_used & (1u64 << c)) != 0
                        || blocked_mask.test_bit(idx)
                    {
                        continue;
                    }
                    temp_cells[count].write(idx_u16);
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
            let idx = unsafe { best_cells[i].assume_init() } as usize;
            let r = idx / n;
            let c = idx % n;
            let reg = flat_regions[idx] as usize;

            placed_trees[depth] = (r as u8, c as u8);
            search(
                n,
                num_words,
                depth + 1,
                limit,
                rows_left & !(1u64 << r),
                cols_used | (1u64 << c),
                regs_left & !(1u64 << reg),
                blocked_mask.union_words(&nbr_masks[idx], num_words),
                all_bits,
                placed_trees,
                flat_regions,
                reg_offsets,
                reg_cells,
                nbr_masks,
                solutions,
            );
            if solutions.len() >= limit {
                return;
            }
        }
    }

    search(
        n,
        num_words,
        0,
        limit,
        all_bits,
        0,
        all_bits,
        BitSet1024::default(),
        all_bits,
        &mut placed_trees,
        flat_regions,
        &reg_offsets,
        &reg_cells,
        nbr_masks,
        &mut solutions,
    );
    solutions
}

#[inline(always)]
pub fn count_solutions_fast(
    n: usize,
    flat_regions: &[u16; MAX_CELLS],
    nbr_masks: &[BitSet1024],
    limit: usize,
) -> usize {
    let num_words = (n * n + 63) / 64;
    let mut reg_counts = [0u16; MAX_N];
    let total_cells = n * n;
    for idx in 0..total_cells {
        let reg = flat_regions[idx] as usize;
        reg_counts[reg] += 1;
    }

    let mut reg_offsets = [0u16; MAX_N + 1];
    let mut current_offset = 0u16;
    for reg in 0..n {
        reg_offsets[reg] = current_offset;
        current_offset += reg_counts[reg];
    }
    reg_offsets[n] = current_offset;

    let mut reg_write_ptrs = reg_offsets;
    let mut reg_cells: [MaybeUninit<u16>; MAX_CELLS] = [MaybeUninit::uninit(); MAX_CELLS];
    for idx in 0..total_cells {
        let reg = flat_regions[idx] as usize;
        let pos = reg_write_ptrs[reg] as usize;
        reg_cells[pos].write(idx as u16);
        reg_write_ptrs[reg] += 1;
    }

    let mut count = 0usize;
    let all_bits = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };

    fn search(
        n: usize,
        num_words: usize,
        limit: usize,
        rows_left: u64,
        cols_used: u64,
        regs_left: u64,
        blocked_mask: BitSet1024,
        all_bits: u64,
        flat_regions: &[u16; MAX_CELLS],
        reg_offsets: &[u16; MAX_N + 1],
        reg_cells: &[MaybeUninit<u16>; MAX_CELLS],
        nbr_masks: &[BitSet1024],
        count: &mut usize,
    ) {
        if *count >= limit {
            return;
        }
        if rows_left == 0 {
            *count += 1;
            return;
        }

        let mut best_cells: [MaybeUninit<u16>; MAX_N] = [MaybeUninit::uninit(); MAX_N];
        let mut best_len = 0usize;
        let mut min_cands = usize::MAX;

        // MRV on remaining rows
        let mut r_left = rows_left;
        while r_left != 0 {
            let r = r_left.trailing_zeros() as usize;
            r_left &= r_left - 1;

            let mut c_count = 0;
            let mut temp_cells: [MaybeUninit<u16>; MAX_N] = [MaybeUninit::uninit(); MAX_N];
            let row_offset = r * n;
            let mut cols_avail = (!cols_used) & all_bits;

            while cols_avail != 0 {
                let c = cols_avail.trailing_zeros() as usize;
                cols_avail &= cols_avail - 1;
                let idx = row_offset + c;
                let reg = flat_regions[idx] as usize;
                if (regs_left & (1u64 << reg)) == 0 || blocked_mask.test_bit(idx) {
                    continue;
                }
                temp_cells[c_count].write(idx as u16);
                c_count += 1;
            }

            if c_count < min_cands {
                min_cands = c_count;
                best_len = c_count;
                best_cells[..c_count].copy_from_slice(&temp_cells[..c_count]);
                if min_cands <= 1 {
                    break;
                }
            }
        }

        // MRV on remaining regions
        if min_cands > 1 {
            let mut reg_left = regs_left;
            while reg_left != 0 {
                let reg = reg_left.trailing_zeros() as usize;
                reg_left &= reg_left - 1;

                let mut c_count = 0;
                let mut temp_cells: [MaybeUninit<u16>; MAX_CELLS] = [MaybeUninit::uninit(); MAX_CELLS];
                let start = reg_offsets[reg] as usize;
                let end = reg_offsets[reg + 1] as usize;
                for i in start..end {
                    let idx_u16 = unsafe { reg_cells[i].assume_init() };
                    let idx = idx_u16 as usize;
                    let r = idx / n;
                    let c = idx % n;
                    if (rows_left & (1u64 << r)) == 0
                        || (cols_used & (1u64 << c)) != 0
                        || blocked_mask.test_bit(idx)
                    {
                        continue;
                    }
                    temp_cells[c_count].write(idx_u16);
                    c_count += 1;
                }
                if c_count < min_cands {
                    min_cands = c_count;
                    best_len = c_count;
                    best_cells[..c_count].copy_from_slice(&temp_cells[..c_count]);
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
            let idx = unsafe { best_cells[i].assume_init() } as usize;
            let r = idx / n;
            let c = idx % n;
            let reg = flat_regions[idx] as usize;

            search(
                n,
                num_words,
                limit,
                rows_left & !(1u64 << r),
                cols_used | (1u64 << c),
                regs_left & !(1u64 << reg),
                blocked_mask.union_words(&nbr_masks[idx], num_words),
                all_bits,
                flat_regions,
                reg_offsets,
                reg_cells,
                nbr_masks,
                count,
            );
            if *count >= limit {
                return;
            }
        }
    }

    search(
        n,
        num_words,
        limit,
        all_bits,
        0,
        all_bits,
        BitSet1024::default(),
        all_bits,
        flat_regions,
        &reg_offsets,
        &reg_cells,
        nbr_masks,
        &mut count,
    );
    count
}

pub fn generate_non_collinear_polyomino(
    tr: usize,
    tc: usize,
    n: usize,
    regions: &[Vec<i32>],
    target_size: usize,
    rng: &mut SimpleRng,
) -> Vec<(usize, usize)> {
    let mut flat_regions = [0i16; MAX_CELLS];
    for r in 0..n {
        for c in 0..n {
            flat_regions[r * n + c] = regions[r][c] as i16;
        }
    }
    let mut out_cells = [(0u8, 0u8); 64];
    let count = generate_non_collinear_polyomino_fast(tr, tc, n, &flat_regions, target_size, rng, &mut out_cells);
    (0..count).map(|i| (out_cells[i].0 as usize, out_cells[i].1 as usize)).collect()
}

#[inline(always)]
pub fn generate_non_collinear_polyomino_fast(
    tr: usize,
    tc: usize,
    n: usize,
    regions: &[i16; MAX_CELLS],
    target_size: usize,
    rng: &mut SimpleRng,
    out_cells: &mut [(u8, u8); 64],
) -> usize {
    let mut last_len = 0;
    let mut last_cells = [(0u8, 0u8); 64];

    for _ in 0..40 {
        let mut cell_list = [(0u8, 0u8); 64];
        let mut cell_count = 0;

        cell_list[cell_count] = (tr as u8, tc as u8);
        cell_count += 1;

        let mut frontier: [MaybeUninit<(u8, u8)>; 64] = [MaybeUninit::uninit(); 64];
        let mut frontier_len = 0;

        // Add 4-neighbors of start cell
        if tr > 0 && regions[(tr - 1) * n + tc] == -1 {
            frontier[frontier_len].write(((tr - 1) as u8, tc as u8));
            frontier_len += 1;
        }
        if tr + 1 < n && regions[(tr + 1) * n + tc] == -1 {
            frontier[frontier_len].write(((tr + 1) as u8, tc as u8));
            frontier_len += 1;
        }
        if tc > 0 && regions[tr * n + (tc - 1)] == -1 {
            frontier[frontier_len].write((tr as u8, (tc - 1) as u8));
            frontier_len += 1;
        }
        if tc + 1 < n && regions[tr * n + (tc + 1)] == -1 {
            frontier[frontier_len].write((tr as u8, (tc + 1) as u8));
            frontier_len += 1;
        }

        while cell_count < target_size && frontier_len > 0 {
            let idx = rng.gen_range(0, frontier_len);
            let (r_u8, c_u8) = unsafe { frontier[idx].assume_init() };
            frontier[idx] = frontier[frontier_len - 1];
            frontier_len -= 1;

            let r = r_u8 as usize;
            let c = c_u8 as usize;
            let c_idx = r * n + c;
            if regions[c_idx] != -1 || cell_list[..cell_count].contains(&(r_u8, c_u8)) {
                continue;
            }
            cell_list[cell_count] = (r_u8, c_u8);
            cell_count += 1;

            if r > 0 && regions[c_idx - n] == -1 {
                let cand_u = (r_u8 - 1, c_u8);
                if !cell_list[..cell_count].contains(&cand_u) && frontier_len < 64 {
                    frontier[frontier_len].write(cand_u);
                    frontier_len += 1;
                }
            }
            if r + 1 < n && regions[c_idx + n] == -1 {
                let cand_d = (r_u8 + 1, c_u8);
                if !cell_list[..cell_count].contains(&cand_d) && frontier_len < 64 {
                    frontier[frontier_len].write(cand_d);
                    frontier_len += 1;
                }
            }
            if c > 0 && regions[c_idx - 1] == -1 {
                let cand_l = (r_u8, c_u8 - 1);
                if !cell_list[..cell_count].contains(&cand_l) && frontier_len < 64 {
                    frontier[frontier_len].write(cand_l);
                    frontier_len += 1;
                }
            }
            if c + 1 < n && regions[c_idx + 1] == -1 {
                let cand_r = (r_u8, c_u8 + 1);
                if !cell_list[..cell_count].contains(&cand_r) && frontier_len < 64 {
                    frontier[frontier_len].write(cand_r);
                    frontier_len += 1;
                }
            }
        }

        let mut min_r = usize::MAX;
        let mut max_r = 0;
        let mut min_c = usize::MAX;
        let mut max_c = 0;

        for i in 0..cell_count {
            let (r, c) = (cell_list[i].0 as usize, cell_list[i].1 as usize);
            if r < min_r { min_r = r; }
            if r > max_r { max_r = r; }
            if c < min_c { min_c = c; }
            if c > max_c { max_c = c; }
        }

        if cell_count >= 3 && (max_r > min_r) && (max_c > min_c) {
            out_cells[..cell_count].copy_from_slice(&cell_list[..cell_count]);
            return cell_count;
        }

        last_len = cell_count;
        last_cells[..cell_count].copy_from_slice(&cell_list[..cell_count]);
    }

    out_cells[..last_len].copy_from_slice(&last_cells[..last_len]);
    last_len
}

pub fn is_region_connected(n: usize, regions: &[Vec<usize>], reg_id: usize) -> bool {
    let mut flat = [0u16; MAX_CELLS];
    for r in 0..n {
        for c in 0..n {
            flat[r * n + c] = regions[r][c] as u16;
        }
    }
    is_region_connected_fast(n, &flat, reg_id as u16)
}

pub fn is_region_connected_fast(n: usize, regions: &[u16; MAX_CELLS], reg_id: u16) -> bool {
    let total_cells = n * n;
    let mut start_idx = usize::MAX;
    let mut target_count = 0;

    for idx in 0..total_cells {
        if regions[idx] == reg_id {
            if start_idx == usize::MAX {
                start_idx = idx;
            }
            target_count += 1;
        }
    }

    if start_idx == usize::MAX {
        return true;
    }

    let mut visited = [false; MAX_CELLS];
    let mut queue: [MaybeUninit<u16>; MAX_CELLS] = [MaybeUninit::uninit(); MAX_CELLS];
    let mut head = 0;
    let mut tail = 0;

    visited[start_idx] = true;
    queue[tail].write(start_idx as u16);
    tail += 1;
    let mut seen = 0;

    while head < tail {
        let curr = unsafe { queue[head].assume_init() } as usize;
        head += 1;
        seen += 1;

        let r = curr / n;
        let c = curr % n;

        if r > 0 && regions[curr - n] == reg_id && !visited[curr - n] {
            visited[curr - n] = true;
            queue[tail].write((curr - n) as u16);
            tail += 1;
        }
        if r + 1 < n && regions[curr + n] == reg_id && !visited[curr + n] {
            visited[curr + n] = true;
            queue[tail].write((curr + n) as u16);
            tail += 1;
        }
        if c > 0 && regions[curr - 1] == reg_id && !visited[curr - 1] {
            visited[curr - 1] = true;
            queue[tail].write((curr - 1) as u16);
            tail += 1;
        }
        if c + 1 < n && regions[curr + 1] == reg_id && !visited[curr + 1] {
            visited[curr + 1] = true;
            queue[tail].write((curr + 1) as u16);
            tail += 1;
        }
    }

    seen == target_count
}

pub fn all_regions_connected(n: usize, regions: &[Vec<usize>]) -> bool {
    let mut flat = [0u16; MAX_CELLS];
    for r in 0..n {
        for c in 0..n {
            flat[r * n + c] = regions[r][c] as u16;
        }
    }
    all_regions_connected_fast(n, &flat)
}

#[inline(always)]
pub fn all_regions_connected_fast(n: usize, regions: &[u16; MAX_CELLS]) -> bool {
    let total_cells = n * n;
    let mut parent: [MaybeUninit<u16>; MAX_CELLS] = [MaybeUninit::uninit(); MAX_CELLS];
    let mut rank: [MaybeUninit<u8>; MAX_CELLS] = [MaybeUninit::uninit(); MAX_CELLS];
    let mut reg_sizes = [0u16; MAX_N];

    for i in 0..total_cells {
        parent[i].write(i as u16);
        rank[i].write(0);
        reg_sizes[regions[i] as usize] += 1;
    }

    #[inline(always)]
    fn find(i: usize, parent: &mut [MaybeUninit<u16>; MAX_CELLS]) -> usize {
        let mut root = i;
        while unsafe { parent[root].assume_init() } as usize != root {
            root = unsafe { parent[root].assume_init() } as usize;
        }
        let mut curr = i;
        while curr != root {
            let next = unsafe { parent[curr].assume_init() } as usize;
            parent[curr].write(root as u16);
            curr = next;
        }
        root
    }

    #[inline(always)]
    fn union(
        i: usize,
        j: usize,
        parent: &mut [MaybeUninit<u16>; MAX_CELLS],
        rank: &mut [MaybeUninit<u8>; MAX_CELLS],
    ) {
        let root_i = find(i, parent);
        let root_j = find(j, parent);
        if root_i != root_j {
            let rank_i = unsafe { rank[root_i].assume_init() };
            let rank_j = unsafe { rank[root_j].assume_init() };
            if rank_i < rank_j {
                parent[root_i].write(root_j as u16);
            } else if rank_i > rank_j {
                parent[root_j].write(root_i as u16);
            } else {
                parent[root_j].write(root_i as u16);
                rank[root_i].write(rank_i + 1);
            }
        }
    }

    // Connect 4-way adjacent cells with same region
    for r in 0..n {
        let row_offset = r * n;
        for c in 0..n {
            let idx = row_offset + c;
            let reg = regions[idx];
            if c + 1 < n && regions[idx + 1] == reg {
                union(idx, idx + 1, &mut parent, &mut rank);
            }
            if r + 1 < n && regions[idx + n] == reg {
                union(idx, idx + n, &mut parent, &mut rank);
            }
        }
    }

    // Count component sizes
    let mut comp_sizes = [0u16; MAX_CELLS];
    for i in 0..total_cells {
        let r = find(i, &mut parent);
        comp_sizes[r] += 1;
    }

    // For each region, check that the first encountered cell's root comp_size == reg_sizes[reg]
    let mut checked = [false; MAX_N];
    for i in 0..total_cells {
        let reg = regions[i] as usize;
        if !checked[reg] {
            checked[reg] = true;
            let root = find(i, &mut parent);
            if comp_sizes[root] != reg_sizes[reg] {
                return false;
            }
        }
    }

    true
}

pub fn construct_gadget_regions(
    n: usize,
    trees: &[(usize, usize)],
    difficulty: &str,
    rng: &mut SimpleRng,
) -> (Vec<Vec<usize>>, Vec<usize>, usize) {
    let mut tree_arr = [(0usize, 0usize); MAX_N];
    for (i, &t) in trees.iter().enumerate() {
        tree_arr[i] = t;
    }
    let mut flat_regions = [0u16; MAX_CELLS];
    let mut reg_sizes_arr = [0usize; MAX_N];
    let min_allowed = construct_gadget_regions_fast(
        n,
        &tree_arr[..n],
        difficulty,
        rng,
        &mut flat_regions,
        &mut reg_sizes_arr,
    );

    let mut regions = vec![vec![0usize; n]; n];
    for r in 0..n {
        for c in 0..n {
            regions[r][c] = flat_regions[r * n + c] as usize;
        }
    }
    let reg_sizes = reg_sizes_arr[..n].to_vec();
    (regions, reg_sizes, min_allowed)
}

pub fn construct_gadget_regions_fast(
    n: usize,
    trees: &[(usize, usize)],
    difficulty: &str,
    rng: &mut SimpleRng,
    out_regions: &mut [u16; MAX_CELLS],
    out_reg_sizes: &mut [usize; MAX_N],
) -> usize {
    let total_cells = n * n;
    let mut regions = [-1i16; MAX_CELLS];
    let mut elim_time = [999i16; MAX_CELLS];
    out_reg_sizes[..n].fill(0);

    for (reg_id, &(tr, tc)) in trees.iter().enumerate() {
        regions[tr * n + tc] = reg_id as i16;
        out_reg_sizes[reg_id] = 1;
    }

    let actual_anchors = 2;
    let min_allowed = match difficulty {
        "Easy" => 1,
        "Medium" => 3,
        _ => 3,
    };

    // 1. Group 0: 2D non-linear polyomino (or 1-cell singleton for Easy)
    let (tr0, tc0) = trees[0];
    let g0_size = match difficulty {
        "Easy" => 1,
        "Medium" => rng.gen_range(3, 5),
        _ => rng.gen_range(4, 6),
    };

    let mut g0_cells = [(0u8, 0u8); 64];
    let g0_len = if g0_size == 1 {
        g0_cells[0] = (tr0 as u8, tc0 as u8);
        1
    } else {
        generate_non_collinear_polyomino_fast(tr0, tc0, n, &regions, g0_size, rng, &mut g0_cells)
    };

    for i in 0..g0_len {
        let (r, c) = (g0_cells[i].0 as usize, g0_cells[i].1 as usize);
        let idx = r * n + c;
        if regions[idx] == -1 {
            regions[idx] = 0;
            out_reg_sizes[0] += 1;
        }
    }

    if g0_len > 0 {
        let mut min_r = usize::MAX;
        let mut max_r = 0;
        let mut min_c = usize::MAX;
        let mut max_c = 0;
        for i in 0..g0_len {
            let (r, c) = (g0_cells[i].0 as usize, g0_cells[i].1 as usize);
            if r < min_r { min_r = r; }
            if r > max_r { max_r = r; }
            if c < min_c { min_c = c; }
            if c > max_c { max_c = c; }
        }

        let r0_min = if tr0 > 0 { tr0 - 1 } else { 0 };
        let r0_max = if tr0 + 1 < n { tr0 + 1 } else { n - 1 };
        let c0_min = if tc0 > 0 { tc0 - 1 } else { 0 };
        let c0_max = if tc0 + 1 < n { tc0 + 1 } else { n - 1 };
        for r in r0_min..=r0_max {
            let r_off = r * n;
            for c in c0_min..=c0_max {
                let idx = r_off + c;
                if regions[idx] != 0 {
                    elim_time[idx] = elim_time[idx].min(0);
                }
            }
        }

        for r in min_r..=max_r {
            let r_off = r * n;
            for c in min_c..=max_c {
                let idx = r_off + c;
                if regions[idx] != 0 {
                    elim_time[idx] = elim_time[idx].min(0);
                }
            }
        }
    }

    // 2. Anchor 1 grows adjacent cells
    let (tr1, tc1) = trees[1];
    let a1_target = match difficulty {
        "Easy" => 2,
        "Medium" => rng.gen_range(2, 4),
        _ => rng.gen_range(3, 5),
    };

    let mut a1_cells = [(0u8, 0u8); 16];
    a1_cells[0] = (tr1 as u8, tc1 as u8);
    let mut a1_len = 1;

    if a1_target > 1 {
        let nbrs = if tr1 != tr0 {
            [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)]
        } else {
            [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)]
        };
        for &(dr, dc) in &nbrs {
            let nr = tr1 as i32 + dr;
            let nc = tc1 as i32 + dc;
            if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 {
                let ur = nr as usize;
                let uc = nc as usize;
                let idx = ur * n + uc;
                if regions[idx] == -1 {
                    regions[idx] = 1;
                    out_reg_sizes[1] += 1;
                    a1_cells[a1_len] = (ur as u8, uc as u8);
                    a1_len += 1;
                    if out_reg_sizes[1] >= a1_target {
                        break;
                    }
                }
            }
        }
    }

    if a1_len > 0 {
        let mut min_r = usize::MAX;
        let mut max_r = 0;
        let mut min_c = usize::MAX;
        let mut max_c = 0;
        for i in 0..a1_len {
            let (r, c) = (a1_cells[i].0 as usize, a1_cells[i].1 as usize);
            if r < min_r { min_r = r; }
            if r > max_r { max_r = r; }
            if c < min_c { min_c = c; }
            if c > max_c { max_c = c; }
        }

        if a1_len == 1 || min_r == max_r {
            let row_off = min_r * n;
            for c in 0..n {
                let idx = row_off + c;
                if regions[idx] != 1 {
                    elim_time[idx] = elim_time[idx].min(0);
                }
            }
        }
        if a1_len == 1 || min_c == max_c {
            let mut idx = min_c;
            for _ in 0..n {
                if regions[idx] != 1 {
                    elim_time[idx] = elim_time[idx].min(0);
                }
                idx += n;
            }
        }
        if (max_r - min_r <= 1) && (max_c - min_c <= 1) {
            for r in min_r..=max_r {
                let row_off = r * n;
                for c in min_c..=max_c {
                    let idx = row_off + c;
                    if regions[idx] != 1 {
                        elim_time[idx] = elim_time[idx].min(0);
                    }
                }
            }
        }
    }

    // 3. Setup elimination timestamps for non-anchor trees
    for step in actual_anchors..n {
        let (tr, tc) = trees[step];
        let s_i = step as i16;
        let row_offset = tr * n;
        for c in 0..n {
            let idx = row_offset + c;
            if s_i < elim_time[idx] {
                elim_time[idx] = s_i;
            }
        }
        let mut col_idx = tc;
        for _ in 0..n {
            if s_i < elim_time[col_idx] {
                elim_time[col_idx] = s_i;
            }
            col_idx += n;
        }
        let r_min = if tr > 0 { tr - 1 } else { 0 };
        let r_max = if tr + 1 < n { tr + 1 } else { n - 1 };
        let c_min = if tc > 0 { tc - 1 } else { 0 };
        let c_max = if tc + 1 < n { tc + 1 } else { n - 1 };
        for r in r_min..=r_max {
            let r_off = r * n;
            for c in c_min..=c_max {
                let idx = r_off + c;
                if s_i < elim_time[idx] {
                    elim_time[idx] = s_i;
                }
            }
        }
    }

    // 4. Strict Invariant Flood Fill for non-anchor regions
    let mut queue: [MaybeUninit<(u16, u16)>; MAX_CELLS] = [MaybeUninit::uninit(); MAX_CELLS];
    let mut head = 0;
    let mut tail = 0;

    for reg_id in actual_anchors..n {
        let (tr, tc) = trees[reg_id];
        let u_reg = reg_id as u16;
        if tr > 0 && regions[(tr - 1) * n + tc] == -1 {
            queue[tail].write((((tr - 1) * n + tc) as u16, u_reg));
            tail += 1;
        }
        if tr + 1 < n && regions[(tr + 1) * n + tc] == -1 {
            queue[tail].write((((tr + 1) * n + tc) as u16, u_reg));
            tail += 1;
        }
        if tc > 0 && regions[tr * n + (tc - 1)] == -1 {
            queue[tail].write(((tr * n + tc - 1) as u16, u_reg));
            tail += 1;
        }
        if tc + 1 < n && regions[tr * n + (tc + 1)] == -1 {
            queue[tail].write(((tr * n + tc + 1) as u16, u_reg));
            tail += 1;
        }
    }

    while head < tail {
        let (idx_u16, reg_id_u16) = unsafe { queue[head].assume_init() };
        head += 1;
        let idx = idx_u16 as usize;
        let reg_id = reg_id_u16 as usize;

        if regions[idx] != -1 {
            continue;
        }
        if elim_time[idx] < reg_id as i16 {
            regions[idx] = reg_id as i16;
            out_reg_sizes[reg_id] += 1;
            let r = idx / n;
            let c = idx % n;
            if r > 0 && regions[idx - n] == -1 && tail < MAX_CELLS {
                queue[tail].write(((idx - n) as u16, reg_id_u16));
                tail += 1;
            }
            if r + 1 < n && regions[idx + n] == -1 && tail < MAX_CELLS {
                queue[tail].write(((idx + n) as u16, reg_id_u16));
                tail += 1;
            }
            if c > 0 && regions[idx - 1] == -1 && tail < MAX_CELLS {
                queue[tail].write(((idx - 1) as u16, reg_id_u16));
                tail += 1;
            }
            if c + 1 < n && regions[idx + 1] == -1 && tail < MAX_CELLS {
                queue[tail].write(((idx + 1) as u16, reg_id_u16));
                tail += 1;
            }
        }
    }

    // 5. Strictly Connected Straggler Drain
    let mut unassigned: [u16; MAX_CELLS] = [0u16; MAX_CELLS];
    let mut unassigned_len = 0;

    for idx in 0..total_cells {
        if regions[idx] == -1 {
            unassigned[unassigned_len] = idx as u16;
            unassigned_len += 1;
        }
    }

    if unassigned_len > 0 {
        // Sort once by elim_time
        for i in 1..unassigned_len {
            let mut j = i;
            while j > 0 && elim_time[unassigned[j - 1] as usize] > elim_time[unassigned[j] as usize] {
                unassigned.swap(j - 1, j);
                j -= 1;
            }
        }

        loop {
            let mut progress = false;
            let mut write_pos = 0;

            for read_pos in 0..unassigned_len {
                let idx = unassigned[read_pos] as usize;
                if regions[idx] != -1 {
                    continue;
                }
                let r = idx / n;
                let c = idx % n;
                let elim = elim_time[idx];

                let mut max_valid: i16 = -1;
                let mut max_adj: i16 = -1;

                if r > 0 {
                    let n_reg = regions[idx - n];
                    if n_reg != -1 {
                        if n_reg > max_adj { max_adj = n_reg; }
                        if n_reg >= actual_anchors as i16 && elim < n_reg && n_reg > max_valid {
                            max_valid = n_reg;
                        }
                    }
                }
                if r + 1 < n {
                    let n_reg = regions[idx + n];
                    if n_reg != -1 {
                        if n_reg > max_adj { max_adj = n_reg; }
                        if n_reg >= actual_anchors as i16 && elim < n_reg && n_reg > max_valid {
                            max_valid = n_reg;
                        }
                    }
                }
                if c > 0 {
                    let n_reg = regions[idx - 1];
                    if n_reg != -1 {
                        if n_reg > max_adj { max_adj = n_reg; }
                        if n_reg >= actual_anchors as i16 && elim < n_reg && n_reg > max_valid {
                            max_valid = n_reg;
                        }
                    }
                }
                if c + 1 < n {
                    let n_reg = regions[idx + 1];
                    if n_reg != -1 {
                        if n_reg > max_adj { max_adj = n_reg; }
                        if n_reg >= actual_anchors as i16 && elim < n_reg && n_reg > max_valid {
                            max_valid = n_reg;
                        }
                    }
                }

                let chosen = if max_valid != -1 { max_valid } else { max_adj };
                if chosen != -1 {
                    regions[idx] = chosen;
                    out_reg_sizes[chosen as usize] += 1;
                    progress = true;
                } else {
                    unassigned[write_pos] = idx as u16;
                    write_pos += 1;
                }
            }

            unassigned_len = write_pos;
            if !progress || unassigned_len == 0 {
                break;
            }
        }
    }

    for idx in 0..total_cells {
        out_regions[idx] = regions[idx].max(0) as u16;
    }

    min_allowed
}

pub fn generate_parks_puzzle(n: usize, difficulty: &str, timeout: Duration) -> Option<ParksLevel> {
    let mut rng = SimpleRng::new();
    generate_parks_puzzle_with_rng(n, difficulty, timeout, &mut rng)
}

pub fn generate_parks_puzzle_with_rng(
    n: usize,
    difficulty: &str,
    timeout: Duration,
    rng: &mut SimpleRng,
) -> Option<ParksLevel> {
    let start_time = Instant::now();
    let nbr_masks = build_neighbor_masks(n);
    let mut attempts = 0;

    let mut tree_cols = [0usize; MAX_N];
    let mut trees = [(0usize, 0usize); MAX_N];
    let mut flat_regions = [0u16; MAX_CELLS];
    let mut reg_sizes = [0usize; MAX_N];

    while start_time.elapsed() < timeout {
        attempts += 1;
        generate_tree_solution_fast(n, rng, &mut tree_cols);
        for r in 0..n {
            trees[r] = (r, tree_cols[r]);
        }

        let use_center = match difficulty {
            "Easy" => false,
            "Medium" => rng.gen_f64(0.0, 1.0) < 0.50,
            _ => true,
        };

        if use_center {
            // Map trees by row and by column
            let mut tree_by_row = [(0usize, 0usize); MAX_N];
            let mut tree_by_col = [(0usize, 0usize); MAX_N];
            for i in 0..n {
                let (r, c) = trees[i];
                tree_by_row[r] = (r, c);
                tree_by_col[c] = (r, c);
            }

            let sweep_by_row = rng.next_u64() % 2 == 0;

            if sweep_by_row {
                let r_c = (n / 2).clamp(1, n - 2);
                let mut out_idx = 0;
                trees[out_idx] = tree_by_row[r_c];
                out_idx += 1;

                let mut d = 1;
                while out_idx < n {
                    if r_c + d < n {
                        trees[out_idx] = tree_by_row[r_c + d];
                        out_idx += 1;
                    }
                    if out_idx < n && r_c >= d {
                        trees[out_idx] = tree_by_row[r_c - d];
                        out_idx += 1;
                    }
                    d += 1;
                }
            } else {
                let c_c = (n / 2).clamp(1, n - 2);
                let mut out_idx = 0;
                trees[out_idx] = tree_by_col[c_c];
                out_idx += 1;

                let mut d = 1;
                while out_idx < n {
                    if c_c + d < n {
                        trees[out_idx] = tree_by_col[c_c + d];
                        out_idx += 1;
                    }
                    if out_idx < n && c_c >= d {
                        trees[out_idx] = tree_by_col[c_c - d];
                        out_idx += 1;
                    }
                    d += 1;
                }
            }
        } else {
            let edge = rng.gen_range(0, 4);
            let rnd_pos = rng.gen_f64(0.0, (n - 1) as f64);
            let target_pt = match edge {
                0 => (0.0, rnd_pos),
                1 => ((n - 1) as f64, rnd_pos),
                2 => (rnd_pos, 0.0),
                _ => (rnd_pos, (n - 1) as f64),
            };

            // Pick Tree 0 closest to target_pt
            let mut best_idx = 0;
            let mut min_dist_sq = f64::MAX;
            for i in 0..n {
                let (r, c) = trees[i];
                let dr = r as f64 - target_pt.0;
                let dc = c as f64 - target_pt.1;
                let d2 = dr * dr + dc * dc + rng.gen_f64(-0.2, 0.2);
                if d2 < min_dist_sq {
                    min_dist_sq = d2;
                    best_idx = i;
                }
            }
            trees.swap(0, best_idx);

            let (tr0, tc0) = trees[0];
            let angle: f64 = rng.gen_f64(0.0, 2.0 * std::f64::consts::PI);
            let vr = angle.cos();
            let vc = angle.sin();
            let mut rem = [((0usize, 0usize), 0.0f64); MAX_N];
            for i in 1..n {
                let (r, c) = trees[i];
                let dr = r as f64 - tr0 as f64;
                let dc = c as f64 - tc0 as f64;
                let proj = dr * vr + dc * vc + rng.gen_f64(-0.08, 0.08);
                rem[i - 1] = ((r, c), proj);
            }
            let rem_len = n - 1;
            let rem_slice = &mut rem[..rem_len];
            for i in 1..rem_len {
                let mut j = i;
                while j > 0 && rem_slice[j - 1].1 > rem_slice[j].1 {
                    rem_slice.swap(j - 1, j);
                    j -= 1;
                }
            }

            for i in 1..n {
                trees[i] = rem[i - 1].0;
            }
        }

        let min_allowed = construct_gadget_regions_fast(
            n,
            &trees[..n],
            difficulty,
            rng,
            &mut flat_regions,
            &mut reg_sizes,
        );

        let mut size_ok = true;
        for reg in 0..n {
            if reg_sizes[reg] < min_allowed {
                size_ok = false;
                break;
            }
        }
        if !size_ok {
            continue;
        }

        if !all_regions_connected_fast(n, &flat_regions) {
            continue;
        }

        let sol_count = count_solutions_fast(n, &flat_regions, &nbr_masks, 2);
        if sol_count == 1 {
            let mut final_regions = vec![vec![0usize; n]; n];
            for r in 0..n {
                for c in 0..n {
                    final_regions[r][c] = flat_regions[r * n + c] as usize;
                }
            }
            return Some(ParksLevel {
                size: n,
                difficulty: difficulty.to_string(),
                solution_trees: trees[..n].to_vec(),
                regions: final_regions,
                region_sizes: reg_sizes[..n].to_vec(),
                attempts,
                elapsed: start_time.elapsed(),
            });
        }
    }

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
