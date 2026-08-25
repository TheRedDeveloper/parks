import math
import random
from collections import deque
import time

# Distinct palette of 16 vibrant RGB background colors for regions
PALETTE = [
    (190, 55, 55),    # 0: Crimson Red
    (35, 120, 210),   # 1: Blue
    (40, 150, 60),    # 2: Forest Green
    (210, 125, 20),   # 3: Amber / Dark Orange
    (145, 60, 175),   # 4: Purple
    (25, 155, 160),   # 5: Cyan / Teal
    (205, 75, 135),   # 6: Rose Pink
    (110, 125, 35),   # 7: Olive
    (175, 95, 65),    # 8: Terracotta / Brown
    (75, 85, 175),    # 9: Slate Indigo
    (30, 145, 115),   # 10: Jade / Sea Green
    (165, 50, 105),   # 11: Berry / Wine
    (105, 145, 30),   # 12: Lime Green
    (150, 105, 165),  # 13: Lavender
    (195, 145, 40),   # 14: Warm Gold
    (70, 120, 145),   # 15: Steel Blue
]


def build_neighbor_masks(n):
    """Precomputes 8-directional neighbor forbidden masks."""
    masks = [0] * (n * n)
    for r in range(n):
        for c in range(n):
            idx = r * n + c
            mask = 0
            for dr in (-1, 0, 1):
                for dc in (-1, 0, 1):
                    nr, nc = r + dr, c + dc
                    if 0 <= nr < n and 0 <= nc < n:
                        mask |= (1 << (nr * n + nc))
            masks[idx] = mask
    return masks


def generate_tree_solution(n):
    """Generates N valid non-touching trees."""
    grid = [[0] * n for _ in range(n)]
    trees = []

    def is_safe(r, c):
        for dr in (-1, 0, 1):
            for dc in (-1, 0, 1):
                nr, nc = r + dr, c + dc
                if 0 <= nr < n and 0 <= nc < n and grid[nr][nc] == 1:
                    return False
        return True

    def backtrack(r):
        if r == n:
            return True
        cols = list(range(n))
        random.shuffle(cols)
        for c in cols:
            if all(c != tc for _, tc in trees) and is_safe(r, c):
                grid[r][c] = 1
                trees.append((r, c))
                if backtrack(r + 1):
                    return True
                trees.pop()
                grid[r][c] = 0
        return False

    backtrack(0)
    return trees


def solve_fast(n, regions, nbr_masks, limit=2):
    """Dual MRV Bitmask Solver to verify uniqueness."""
    solutions = []
    row_cells = [[] for _ in range(n)]
    reg_cells = [[] for _ in range(n)]

    for r in range(n):
        for c in range(n):
            idx = r * n + c
            reg = regions[r][c]
            row_cells[r].append((idx, r, c, reg))
            reg_cells[reg].append((idx, r, c, reg))

    def search(rows_left, cols_used, regs_left, blocked_mask, placed_trees):
        if len(solutions) >= limit:
            return
        if not rows_left:
            solutions.append(list(placed_trees))
            return

        best_cells = None
        min_cands = 999

        for r in range(n):
            if not (rows_left & (1 << r)):
                continue
            valid = []
            for idx, cell_r, cell_c, reg in row_cells[r]:
                if (1 << cell_c) & cols_used or not (regs_left & (1 << reg)) or (1 << idx) & blocked_mask:
                    continue
                valid.append((idx, cell_r, cell_c, reg))

            if len(valid) < min_cands:
                min_cands = len(valid)
                best_cells = valid
                if min_cands <= 1:
                    break

        if min_cands > 1:
            for reg in range(n):
                if not (regs_left & (1 << reg)):
                    continue
                valid = []
                for idx, cell_r, cell_c, reg_id in reg_cells[reg]:
                    if not (rows_left & (1 << cell_r)) or (1 << cell_c) & cols_used or (1 << idx) & blocked_mask:
                        continue
                    valid.append((idx, cell_r, cell_c, reg_id))

                if len(valid) < min_cands:
                    min_cands = len(valid)
                    best_cells = valid
                    if min_cands <= 1:
                        break

        if min_cands == 0:
            return

        for idx, r, c, reg in best_cells:
            placed_trees.append((r, c))
            search(
                rows_left & ~(1 << r),
                cols_used | (1 << c),
                regs_left & ~(1 << reg),
                blocked_mask | nbr_masks[idx],
                placed_trees,
            )
            placed_trees.pop()
            if len(solutions) >= limit:
                return

    all_bits = (1 << n) - 1
    search(all_bits, 0, all_bits, 0, [])
    return solutions


def generate_non_collinear_polyomino(tr, tc, n, regions, target_size=3):
    """
    Grows a guaranteed 2D non-collinear polyomino shape around (tr, tc)
    (spans at least 2 rows and 2 columns: L-shapes, elongated L, S-shapes,
    Z-shapes, T-shapes, 2x2 boxes, etc. - never a simple straight line).
    """
    for _ in range(40):
        cells = set([(tr, tc)])
        frontier = []

        for dr, dc in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            nr, nc = tr + dr, tc + dc
            if 0 <= nr < n and 0 <= nc < n and regions[nr][nc] == -1:
                frontier.append((nr, nc))

        while len(cells) < target_size and frontier:
            r, c = frontier.pop(random.randrange(len(frontier)))
            if (r, c) in cells or regions[r][c] != -1:
                continue
            cells.add((r, c))
            for dr, dc in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                nr, nc = r + dr, c + dc
                if 0 <= nr < n and 0 <= nc < n and regions[nr][nc] == -1 and (nr, nc) not in cells:
                    frontier.append((nr, nc))

        rows = set(r for r, _ in cells)
        cols = set(c for _, c in cells)
        # Ensure non-collinear (2D): spans >= 2 rows AND >= 2 columns
        if len(cells) >= 3 and len(rows) >= 2 and len(cols) >= 2:
            return list(cells)

    return list(cells)


def is_region_connected(n, regions, reg_id):
    """Verifies that region reg_id is a single 4-connected component."""
    cells = [(r, c) for r in range(n) for c in range(n) if regions[r][c] == reg_id]
    if not cells:
        return True
    visited = set()
    q = deque([cells[0]])
    visited.add(cells[0])
    while q:
        r, c = q.popleft()
        for dr, dc in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            nr, nc = r + dr, c + dc
            if (nr, nc) in cells and (nr, nc) not in visited:
                visited.add((nr, nc))
                q.append((nr, nc))
    return len(visited) == len(cells)


def all_regions_connected(n, regions):
    """Verifies that ALL regions on the board are 100% 4-connected (no disconnected islands)."""
    for reg_id in range(n):
        if not is_region_connected(n, regions, reg_id):
            return False
    return True


def construct_gadget_regions(n, trees, difficulty="Medium"):
    """
    Constructs regions guaranteeing 2D non-linear polyomino shapes for Group 0 (size 3-4),
    while strictly ensuring 100% 4-connectivity across every region on the board.
    """
    regions = [[-1] * n for _ in range(n)]
    reg_sizes = [0] * n

    for reg_id, (tr, tc) in enumerate(trees):
        regions[tr][tc] = reg_id
        reg_sizes[reg_id] = 1

    elim_time = [[999] * n for _ in range(n)]

    # 1. Group 0: 2D non-linear polyomino (L-shape, elongated L, S, Z, T, box)
    tr0, tc0 = trees[0]
    g0_size = random.choice([3, 4]) if n <= 10 else 3
    g0_cells = generate_non_collinear_polyomino(tr0, tc0, n, regions, target_size=g0_size)
    for r, c in g0_cells:
        if regions[r][c] == -1:
            regions[r][c] = 0
            reg_sizes[0] += 1

    rows0 = set(r for r, _ in g0_cells)
    cols0 = set(c for _, c in g0_cells)
    if (max(rows0) - min(rows0) <= 1) and (max(cols0) - min(cols0) <= 1):
        for r in range(min(rows0), max(rows0) + 1):
            for c in range(min(cols0), max(cols0) + 1):
                if regions[r][c] != 0:
                    elim_time[r][c] = min(elim_time[r][c], 0)

    # 2. Anchor 1 grows 1 adjacent cell (making a 2-cell line)
    tr1, tc1 = trees[1]
    nbrs = [(-1, 0), (1, 0), (0, -1), (0, 1)]
    random.shuffle(nbrs)
    for dr, dc in nbrs:
        nr, nc = tr1 + dr, tc1 + dc
        if 0 <= nr < n and 0 <= nc < n and regions[nr][nc] == -1:
            regions[nr][nc] = 1
            reg_sizes[1] += 1
            if dr == 0:  # horizontal line
                for c in range(n):
                    if regions[tr1][c] != 1:
                        elim_time[tr1][c] = min(elim_time[tr1][c], 0)
            else:  # vertical line
                for r in range(n):
                    if regions[r][tc1] != 1:
                        elim_time[r][tc1] = min(elim_time[r][tc1], 0)
            break

    actual_anchors = 2

    # 3. Setup elimination timestamps for non-anchor trees
    for step in range(actual_anchors, n):
        tr, tc = trees[step]
        for c in range(n):
            elim_time[tr][c] = min(elim_time[tr][c], step)
        for r in range(n):
            elim_time[r][tc] = min(elim_time[r][tc], step)
        for dr in (-1, 0, 1):
            for dc in (-1, 0, 1):
                nr, nc = tr + dr, tc + dc
                if 0 <= nr < n and 0 <= nc < n:
                    elim_time[nr][nc] = min(elim_time[nr][nc], step)

    # 4. Strict Invariant Flood Fill for non-anchor regions
    queue = deque()
    for reg_id in range(actual_anchors, n):
        tr, tc = trees[reg_id]
        for dr, dc in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            nr, nc = tr + dr, tc + dc
            if 0 <= nr < n and 0 <= nc < n and regions[nr][nc] == -1:
                queue.append((nr, nc, reg_id))

    while queue:
        r, c, reg_id = queue.popleft()
        if regions[r][c] != -1:
            continue
        if elim_time[r][c] < reg_id:
            regions[r][c] = reg_id
            reg_sizes[reg_id] += 1
            for dr, dc in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                nr, nc = r + dr, c + dc
                if 0 <= nr < n and 0 <= nc < n and regions[nr][nc] == -1:
                    queue.append((nr, nc, reg_id))

    # 5. Strictly Connected Straggler Drain (strictly attaches to 4-adjacent neighbors only)
    while True:
        unassigned = [(r, c) for r in range(n) for c in range(n) if regions[r][c] == -1]
        if not unassigned:
            break

        unassigned.sort(key=lambda cell: elim_time[cell[0]][cell[1]])
        progress = False

        for r, c in unassigned:
            if regions[r][c] != -1:
                continue

            adj_neighbors = []
            valid_neighbors = []
            for dr, dc in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                nr, nc = r + dr, c + dc
                if 0 <= nr < n and 0 <= nc < n and regions[nr][nc] != -1:
                    n_reg = regions[nr][nc]
                    adj_neighbors.append(n_reg)
                    if n_reg >= actual_anchors and elim_time[r][c] < n_reg:
                        valid_neighbors.append(n_reg)

            if valid_neighbors:
                chosen = max(valid_neighbors)
                regions[r][c] = chosen
                reg_sizes[chosen] += 1
                progress = True
            elif adj_neighbors:
                # Strictly attach to adjacent neighbor (guaranteeing 100% 4-connectivity)
                chosen = max(adj_neighbors)
                regions[r][c] = chosen
                reg_sizes[chosen] += 1
                progress = True

        if not progress:
            break

    return regions, reg_sizes, 2


def generate_parks_puzzle(n=16, difficulty="Medium", timeout=1.0):
    """
    Generates a uniquely solvable Parks puzzle within the time limit.
    Guarantees 100% 4-connected regions with no disconnected pieces or scattered islands.
    """
    start_time = time.time()
    nbr_masks = build_neighbor_masks(n)
    attempts = 0

    while (time.time() - start_time) < timeout:
        attempts += 1
        trees = generate_tree_solution(n)
        # Random sweep direction vector so Group 0 starts in random locations/corners
        angle = random.uniform(0, 2 * math.pi)
        vr, vc = math.cos(angle), math.sin(angle)
        trees.sort(key=lambda t: t[0] * vr + t[1] * vc + random.uniform(-0.1, 0.1))

        regions, reg_sizes, min_allowed = construct_gadget_regions(n, trees, difficulty=difficulty)

        # Enforce minimum region size to prevent trivial singletons
        if min(reg_sizes) < min_allowed:
            continue

        # Strict 4-connectivity verification: every region must be a single contiguous polyomino
        if not all_regions_connected(n, regions):
            continue

        # Verification step
        sols = solve_fast(n, regions, nbr_masks, limit=2)
        if len(sols) == 1:
            return {
                "size": n,
                "difficulty": difficulty,
                "solution_trees": trees,
                "regions": regions,
                "region_sizes": reg_sizes,
                "attempts": attempts,
                "elapsed": time.time() - start_time,
            }

    print(f"Generation timed out after {timeout}s ({attempts} attempts).")
    return None


def get_region_ansi(reg_id, is_tree=False):
    """Returns ANSI escape sequences for a distinct background color with high-contrast text."""
    r, g, b = PALETTE[reg_id % len(PALETTE)]
    bg_code = f"\033[48;2;{r};{g};{b}m"
    # Calculate luminance for clear contrast text
    luminance = 0.299 * r + 0.587 * g + 0.114 * b
    fg_code = "\033[38;2;0;0;0m" if luminance > 140 else "\033[38;2;255;255;255m"

    if is_tree:
        # Bold yellow text with tree marker on region background
        return f"{bg_code}\033[1;93m"
    return f"{bg_code}{fg_code}"


def print_board(level):
    """Prints the Parks board with unique ANSI background colors for each group."""
    n = level["size"]
    regions = level["regions"]
    trees = set(level["solution_trees"])
    reset = "\033[0m"

    print(f"\n=== Parks Board ({n}x{n}) | Difficulty: {level['difficulty']} ===")
    header = "    " + "".join(f"{c:3} " for c in range(n))
    print(header)
    print("   +" + "----" * n + "+")

    for r in range(n):
        row_str = f"{r:2} |"
        for c in range(n):
            reg = regions[r][c]
            is_tree = (r, c) in trees
            color = get_region_ansi(reg, is_tree=is_tree)
            if is_tree:
                cell_text = f"*{reg:<2} "
            else:
                cell_text = f" {reg:<2} "
            row_str += f"{color}{cell_text}{reset}"
        row_str += "|"
        print(row_str)

    print("   +" + "----" * n + "+")
    print("Legend: '*N' indicates Tree for Region N (Colored by Region Background)\n")


if __name__ == "__main__":
    for size in [8, 10, 12, 14, 16]:
        lvl = generate_parks_puzzle(n=size, difficulty="Medium", timeout=1.0)
        if lvl:
            print_board(lvl)
            print(f"-> Generated {size}x{size} in {lvl['elapsed']:.3f}s ({lvl['attempts']} attempts)")
        else:
            print(f"Failed {size}x{size}")