use parks::*;
use ply_engine::prelude::*;
use std::{cell::RefCell, time::Duration};
use fxhash::FxHashMap;

static DEFAULT_FONT: FontAsset = font!("assets/fonts/lexend.ttf");
#[allow(dead_code)]
static MONO_FONT: FontAsset = font!("assets/fonts/geist_mono.ttf");

static CROSS_PLACEHOLDER: GraphicAsset = graphic!("assets/images/cross_placeholder.png");
static TREE_PLACEHOLDER: GraphicAsset = graphic!("assets/images/tree_placeholder.png");

static UNDO_PLACEHOLDER: GraphicAsset = graphic!("assets/images/undo_placeholder.png");
static EYE_PLACEHOLDER: GraphicAsset = graphic!("assets/images/eye_placeholder.png");
static QUESTION_PLACEHOLDER: GraphicAsset = graphic!("assets/images/question_placeholder.png");
static PENCIL_PLACEHOLDER: GraphicAsset = graphic!("assets/images/pencil_placeholder.png");

fn window_conf() -> macroquad::conf::Conf {
  macroquad::conf::Conf {
    miniquad_conf: miniquad::conf::Conf {
      window_title: "Hello Ply!".to_owned(),
      window_width: 800,
      window_height: 600,
      high_dpi: true,
      sample_count: 4,
      platform: miniquad::conf::Platform {
        webgl_version: miniquad::conf::WebGLVersion::WebGL2,
        ..Default::default()
      },
      ..Default::default()
    },
    draw_call_vertex_capacity: 100000,
    draw_call_index_capacity: 100000,
    ..Default::default()
  }
}

fn button(ui: &mut Ui, text: &'static str, width: f32, action: impl Fn(Id) + 'static) {
  ui.element()
    .id(text)
    .accessibility(|a| a.button(text))
    .width(fixed!(width))
    .with(|ui, el| {
      el.background_color(if ui.pressed() { DARKBLUE } else if ui.hovered() { BLUE } else { GRAY })
    })
    .layout(|l| l.padding((20, 0, 20, 0)).align(CenterX, CenterY))
    .corner_radius(5.0)
    .on_release(move |id, _, hovered| {
      if hovered {
        action(id);
      }
    })
    .children(|ui| {
      ui.text(text, |t| t.font_size(50).color(WHITE));
    });
}

#[derive(Clone, Debug)]
enum Screen {
  MainMenu,
  Game(Difficulty),
  Options,
}
thread_local! {
  static CURRENT_SCREEN: RefCell<Screen> = RefCell::new(Screen::MainMenu);
}
fn set_current_screen(screen: Screen) {
  CURRENT_SCREEN.with(|s| *s.borrow_mut() = screen);
}
fn get_current_screen() -> Screen {
  CURRENT_SCREEN.with(|s| s.borrow().clone())
}



#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HighlightMode {
  None,
  Section,
  Row,
  Column,
}

thread_local! {
  static HIGHLIGHT_MODE: RefCell<HighlightMode> = RefCell::new(HighlightMode::Section);
  static EYE_HOLDING: RefCell<bool> = RefCell::new(false);
  static EYE_INITIAL_MODE: RefCell<HighlightMode> = RefCell::new(HighlightMode::Section);
  static EYE_HOVERED_OPTION: RefCell<Option<HighlightMode>> = RefCell::new(None);
}

fn get_highlight_mode() -> HighlightMode {
  HIGHLIGHT_MODE.with(|m| *m.borrow())
}

fn set_highlight_mode(mode: HighlightMode) {
  HIGHLIGHT_MODE.with(|m| *m.borrow_mut() = mode);
}

fn get_eye_holding() -> bool {
  EYE_HOLDING.with(|h| *h.borrow())
}

fn set_eye_holding(holding: bool) {
  EYE_HOLDING.with(|h| *h.borrow_mut() = holding);
}

fn get_eye_initial_mode() -> HighlightMode {
  EYE_INITIAL_MODE.with(|m| *m.borrow())
}

fn set_eye_initial_mode(mode: HighlightMode) {
  EYE_INITIAL_MODE.with(|m| *m.borrow_mut() = mode);
}

fn get_eye_hovered_option() -> Option<HighlightMode> {
  EYE_HOVERED_OPTION.with(|o| *o.borrow())
}

fn set_eye_hovered_option(option: Option<HighlightMode>) {
  EYE_HOVERED_OPTION.with(|o| *o.borrow_mut() = option);
}

fn get_active_highlight_mode() -> HighlightMode {
  if get_eye_holding() {
    get_eye_hovered_option().unwrap_or_else(get_eye_initial_mode)
  } else {
    get_highlight_mode()
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Marking {
  NotMarked,
  MarkedTree,
  MarkedEmpty,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct CellChange {
  row: usize,
  col: usize,
  from: Marking,
  to: Marking,
}

#[derive(Clone, Debug)]
struct BoardAction {
  changes: Vec<CellChange>,
}

#[derive(Clone)]
struct UserLevel {
  level: ParksLevel,
  markings: Vec<Vec<Marking>>,
  undo_stack: Vec<BoardAction>,
  redo_stack: Vec<BoardAction>,
}
impl UserLevel {
  fn new(level: ParksLevel) -> Self {
    Self {
      level: level.clone(),
      markings: vec![vec![Marking::NotMarked; level.size]; level.size],
      undo_stack: Vec::new(),
      redo_stack: Vec::new(),
    }
  }

  fn correct_tree_count(&self) -> usize {
    let mut count = 0;
    for r in 0..self.level.size {
      for c in 0..self.level.size {
        if self.markings[r][c] == Marking::MarkedTree && self.level.solution_trees.contains(&(r, c)) {
          count += 1;
        }
      }
    }
    count
  }

  fn is_solved(&self) -> bool {
    self.correct_tree_count() == self.level.solution_trees.len()
  }

  fn region(&self, r: usize, c: usize) -> Option<usize> {
    self.level.regions.get(r).and_then(|row| row.get(c)).copied()
  }

  fn size(&self) -> usize {
    self.level.size
  }
}
thread_local! {
  static LEVELS: RefCell<FxHashMap<Difficulty, Option<UserLevel>>> = RefCell::new(FxHashMap::default());
}
fn set_level(difficulty: &Difficulty, level: Option<UserLevel>) {
  LEVELS.with(|l| l.borrow_mut().insert(*difficulty, level));
}
fn get_level(difficulty: &Difficulty) -> Option<UserLevel> {
  LEVELS.with(|l| l.borrow().get(difficulty).cloned().flatten())
}

#[derive(Clone)]
struct Options {
  pub automark: bool,
}
thread_local! {
  static OPTIONS: RefCell<Options> = RefCell::new(Options { automark: false });
}
fn get_options() -> Options {
  OPTIONS.with(|o| o.borrow().clone())
}
fn set_options(options: Options) {
  OPTIONS.with(|o| *o.borrow_mut() = options);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragMode {
  Cross,
  Erase,
}

#[derive(Clone, Debug)]
struct ActiveInteraction {
  difficulty: Difficulty,
  start_pos: (usize, usize),
  drag_mode: Option<DragMode>,
  changes: Vec<CellChange>,
  visited: Vec<(usize, usize)>,
}

thread_local! {
  static ACTIVE_INTERACTION: RefCell<Option<ActiveInteraction>> = RefCell::new(None);
}

fn cell_press(difficulty: &Difficulty, row: usize, col: usize) {
  if get_eye_holding() {
    return;
  }
  end_interaction();

  if let Some(mut level) = get_level(difficulty) {
    let current_marking = level.markings[row][col];
    let mut changes = Vec::new();
    let drag_mode = if current_marking == Marking::NotMarked {
      level.markings[row][col] = Marking::MarkedEmpty;
      changes.push(CellChange {
        row,
        col,
        from: Marking::NotMarked,
        to: Marking::MarkedEmpty,
      });
      set_level(difficulty, Some(level));
      Some(DragMode::Cross)
    } else {
      None
    };

    ACTIVE_INTERACTION.with(|ai| {
      *ai.borrow_mut() = Some(ActiveInteraction {
        difficulty: *difficulty,
        start_pos: (row, col),
        drag_mode,
        changes,
        visited: vec![(row, col)],
      });
    });
  }
}

fn cell_drag(difficulty: &Difficulty, row: usize, col: usize) {
  if get_eye_holding() {
    return;
  }
  ACTIVE_INTERACTION.with(|ai| {
    let mut ai_borrow = ai.borrow_mut();
    let interaction = match ai_borrow.as_mut() {
      Some(i) if &i.difficulty == difficulty => i,
      _ => return,
    };

    if interaction.visited.contains(&(row, col)) {
      return;
    }
    interaction.visited.push((row, col));

    let Some(mut level) = get_level(difficulty) else { return; };

    match interaction.drag_mode {
      Some(DragMode::Cross) => {
        if level.markings[row][col] == Marking::NotMarked {
          level.markings[row][col] = Marking::MarkedEmpty;
          interaction.changes.push(CellChange {
            row,
            col,
            from: Marking::NotMarked,
            to: Marking::MarkedEmpty,
          });
          set_level(difficulty, Some(level));
        }
      }
      None => {
        interaction.drag_mode = Some(DragMode::Erase);
        let (sr, sc) = interaction.start_pos;
        let start_marking = level.markings[sr][sc];
        if start_marking != Marking::NotMarked {
          level.markings[sr][sc] = Marking::NotMarked;
          interaction.changes.push(CellChange {
            row: sr,
            col: sc,
            from: start_marking,
            to: Marking::NotMarked,
          });
        }
        let cur_marking = level.markings[row][col];
        if cur_marking != Marking::NotMarked {
          level.markings[row][col] = Marking::NotMarked;
          interaction.changes.push(CellChange {
            row,
            col,
            from: cur_marking,
            to: Marking::NotMarked,
          });
        }
        set_level(difficulty, Some(level));
      }
      Some(DragMode::Erase) => {
        let cur_marking = level.markings[row][col];
        if cur_marking != Marking::NotMarked {
          level.markings[row][col] = Marking::NotMarked;
          interaction.changes.push(CellChange {
            row,
            col,
            from: cur_marking,
            to: Marking::NotMarked,
          });
          set_level(difficulty, Some(level));
        }
      }
    }
  });
}

fn end_interaction() {
  let interaction = ACTIVE_INTERACTION.with(|ai| ai.borrow_mut().take());
  let Some(mut interaction) = interaction else { return; };

  if let Some(mut level) = get_level(&interaction.difficulty) {
    if interaction.drag_mode.is_none() {
      let (sr, sc) = interaction.start_pos;
      let current = level.markings[sr][sc];
      let new_marking = match current {
        Marking::MarkedEmpty => Marking::MarkedTree,
        Marking::MarkedTree => Marking::NotMarked,
        Marking::NotMarked => Marking::NotMarked,
      };
      if new_marking != current {
        level.markings[sr][sc] = new_marking;
        interaction.changes.push(CellChange {
          row: sr,
          col: sc,
          from: current,
          to: new_marking,
        });

        if new_marking == Marking::MarkedTree && get_options().automark {
          let target_region = level.region(sr, sc);
          let n = level.size();
          for r in 0..n {
            for c in 0..n {
              if (r, c) == (sr, sc) {
                continue;
              }
              let is_same_row = r == sr;
              let is_same_col = c == sc;
              let is_same_region = target_region.is_some() && level.region(r, c) == target_region;
              let is_adjacent = (r as isize - sr as isize).abs() <= 1 && (c as isize - sc as isize).abs() <= 1;

              if (is_same_row || is_same_col || is_same_region || is_adjacent)
                && level.markings[r][c] == Marking::NotMarked
              {
                level.markings[r][c] = Marking::MarkedEmpty;
                interaction.changes.push(CellChange {
                  row: r,
                  col: c,
                  from: Marking::NotMarked,
                  to: Marking::MarkedEmpty,
                });
              }
            }
          }
        }
      }
    }

    if !interaction.changes.is_empty() {
      level.undo_stack.push(BoardAction {
        changes: interaction.changes,
      });
      level.redo_stack.clear();
      set_level(&interaction.difficulty, Some(level));
    }
  }
}

fn undo(difficulty: &Difficulty) {
  end_interaction();

  if let Some(mut level) = get_level(difficulty) {
    if let Some(action) = level.undo_stack.pop() {
      for change in action.changes.iter().rev() {
        level.markings[change.row][change.col] = change.from;
      }
      level.redo_stack.push(action);
      set_level(difficulty, Some(level));
    }
  }
}

fn redo(difficulty: &Difficulty) {
  end_interaction();

  if let Some(mut level) = get_level(difficulty) {
    if let Some(action) = level.redo_stack.pop() {
      for change in action.changes.iter() {
        level.markings[change.row][change.col] = change.to;
      }
      level.undo_stack.push(action);
      set_level(difficulty, Some(level));
    }
  }
}

fn draw_top_bar(ui: &mut Ui, level: &UserLevel, scaling_factor: f32) {
  ui.element()
    .width(grow!(max: screen_height()-280.0*scaling_factor))
    .layout(|l| l.padding((20.0 * scaling_factor) as u16))
    .children(|ui| {
      ui.element().height(grow!()).width(fixed!(100.0 * scaling_factor))
        .image(&graphic!("assets/images/back_placeholder.png"))
        .on_press(move |_, _| {
          if get_eye_holding() {
            set_highlight_mode(get_eye_initial_mode());
            set_eye_holding(false);
            set_eye_hovered_option(None);
          }
          set_current_screen(Screen::MainMenu);
        }).empty();
      ui.element().width(grow!())
        .layout(|l| l.align(CenterX, CenterY))
        .children(|ui| {
          ui.text(&format!("{:?}", level.level.difficulty), |t| t.font_size((100.0 * scaling_factor) as u16).color(WHITE));
        });
      ui.element().height(grow!()).width(fixed!(100.0 * scaling_factor)).empty();
    });
}

fn bottom_bar_button<'ui, 'ply>(ui: &'ui mut Ui<'_, 'ply>, image: &'static GraphicAsset, height: f32, action: impl Fn() + 'static) -> ElementBuilder<'ply, NoId<'ui>> {
  ui.element().height(fixed!(height)).width(grow!()).contain(1.0)
    .image(image)
    .on_release(move |_, _, hovered| {
      if hovered {
        action();
      }
    })
}

fn draw_bottom_bar(ui: &mut Ui, level: &UserLevel, scaling_factor: f32) {
  let difficulty = level.level.difficulty;
  ui.element()
    .width(grow!(max: screen_height()-280.0*scaling_factor))
    .height(fixed!(140.0 * scaling_factor))
    .layout(|l| l.padding((20.0 * scaling_factor) as u16))
    .children(|ui| {
      bottom_bar_button(ui, &UNDO_PLACEHOLDER, 100.0 * scaling_factor, move || {
        undo(&difficulty);
      }).empty();
      bottom_bar_button(ui, &UNDO_PLACEHOLDER, 100.0 * scaling_factor, move || {
        redo(&difficulty);
      }).rotate_visual(|r| r.flip_x()).empty();
      bottom_bar_button(ui, &QUESTION_PLACEHOLDER, 100.0 * scaling_factor, move || { todo!() }).empty();
      bottom_bar_button(ui, &PENCIL_PLACEHOLDER, 100.0 * scaling_factor, move || { todo!() }).empty();

      let is_holding = get_eye_holding();
      let eye_el = ui.element().id("eye_button")
        .height(fixed!(100.0 * scaling_factor))
        .width(grow!())
        .contain(1.0)
        .image(&EYE_PLACEHOLDER)
        .on_press(move |_, _| {
          end_interaction();
          set_eye_holding(true);
          set_eye_initial_mode(get_highlight_mode());
          set_eye_hovered_option(None);
        });

      if is_holding {
        eye_el.children(|ui| {
          ui.element().id("eye_popup_menu")
            .width(fit!())
            .height(fit!())
            .floating(|f| f
              .attach_parent()
              .anchor((Right, Bottom), (Right, Top))
              .z_index(100)
            )
            .capture()
            .background_color(0x1C1C24)
            .border(|b| b.all((2.0 * scaling_factor).max(1.0) as u16).color(0x3A3A4A).position(Outside))
            .corner_radius(8.0 * scaling_factor)
            .layout(|l| l
              .direction(TopToBottom)
              .gap((4.0 * scaling_factor) as u16)
              .padding((6.0 * scaling_factor) as u16)
            )
            .children(|ui| {
              let options = [
                ("None", HighlightMode::None, "eye_opt_none"),
                ("Section", HighlightMode::Section, "eye_opt_section"),
                ("Row", HighlightMode::Row, "eye_opt_row"),
                ("Column", HighlightMode::Column, "eye_opt_column"),
              ];
              let active_mode = get_active_highlight_mode();
              for (label, mode, id_str) in options {
                let is_hovered = get_eye_hovered_option() == Some(mode);
                let is_active = active_mode == mode;
                let bg_color = if is_hovered {
                  0x2A62E8
                } else if is_active {
                  0x303042
                } else {
                  0x1C1C24
                };
                let text_color = if is_hovered || is_active {
                  WHITE
                } else {
                  LIGHTGRAY
                };
                ui.element().id(id_str)
                  .width(grow!())
                  .capture()
                  .corner_radius(6.0 * scaling_factor)
                  .background_color(bg_color)
                  .layout(|l| l
                    .align(CenterX, CenterY)
                    .padding((
                      (8.0 * scaling_factor) as u16,
                      (14.0 * scaling_factor) as u16,
                      (8.0 * scaling_factor) as u16,
                      (14.0 * scaling_factor) as u16,
                    ))
                  )
                  .on_hover(move |_, _| {
                    set_eye_hovered_option(Some(mode));
                  })
                  .children(|ui| {
                    ui.text(label, |t| t
                      .font_size((70.0 * scaling_factor) as u16)
                      .color(text_color)
                    );
                  });
              }
            });
        });
      } else {
        eye_el.empty();
      }
    });
}

fn draw_grid(ui: &mut Ui, level: &UserLevel, scaling_factor: f32) {
  let corner_radius = 20.0 * scaling_factor;
  let border_surround = (8.0 * scaling_factor) as u16;
  let border_large = (10.0 * scaling_factor) as u16;
  let border_small = (5.0 * scaling_factor) as u16;
  let active_mode = get_active_highlight_mode();

  ui.element().width(grow!()).height(grow!()).contain(1.0).id("grid_container")
    .layout(|l| l.padding(20))
    .children(|ui| {
      ui.element().width(grow!()).height(grow!())
        .border(|b| b.all(border_surround).color(LIGHTGRAY).position(Middle))
        .corner_radius(corner_radius)
        .layout(|l| l.direction(TopToBottom))
        .children(|ui| {
          for r in 0..level.size() {
            ui.element().width(grow!()).height(grow!())
              .layout(|l| l.direction(LeftToRight))
              .children(|ui| {
                for c in 0..level.size() {
                  let difficulty = level.level.difficulty;
                  let difficulty2 = level.level.difficulty;
                  let left_border = if c == 0 {
                    0
                  } else {
                    match active_mode {
                      HighlightMode::None => border_small,
                      HighlightMode::Section => if level.region(r, c) != level.region(r, c - 1) { border_large } else { border_small },
                      HighlightMode::Row => border_small,
                      HighlightMode::Column => border_large,
                    }
                  };
                  let top_border = if r == 0 {
                    0
                  } else {
                    match active_mode {
                      HighlightMode::None => border_small,
                      HighlightMode::Section => if level.region(r, c) != level.region(r - 1, c) { border_large } else { border_small },
                      HighlightMode::Row => border_large,
                      HighlightMode::Column => border_small,
                    }
                  };
                  ui.element().id((format!("cell_{}_{}", r, c).as_str(), 0))
                    .width(grow!()).height(grow!())
                    .layout(|l| l.align(CenterX, CenterY))
                    .accessibility(|a| a.button(&format!("Cell {}:{}", r, c)))
                    .background_color(PALETTE[level.region(r, c).expect("Cell not in any region") % PALETTE.len()])
                    .corner_radius(match (r, c) {
                      (0, 0) => (corner_radius, 0.0, 0.0, 0.0),
                      (r, 0) if r == level.size() - 1 => (0.0, 0.0, 0.0, corner_radius),
                      (0, c) if c == level.size() - 1 => (0.0, corner_radius, 0.0, 0.0),
                      (r, c) if r == level.size() - 1 && c == level.size() - 1 => {
                        (0.0, 0.0, corner_radius, 0.0)
                      },
                      _ => (0.0, 0.0, 0.0, 0.0),
                    })
                    .border(|b| b
                      .left(left_border)
                      .top(top_border)
                      .color(DARKGRAY)
                      .position(Middle)
                    )
                    .on_press(move |_, _| {
                      cell_press(&difficulty2, r, c);
                    })
                    .on_hover(move |_, pointer| {
                      if pointer.pressed() && !pointer.just_pressed() {
                        cell_drag(&difficulty, r, c);
                      }
                    })
                    .children(|ui| {
                      let marking = level.markings[r][c];
                      if marking != Marking::NotMarked {
                        ui.element().height(grow!()).width(grow!())
                          .image(match marking {
                            Marking::MarkedEmpty => &CROSS_PLACEHOLDER,
                            Marking::MarkedTree => &TREE_PLACEHOLDER,
                            _ => unreachable!(),
                          })
                          .empty();
                      }
                    });
                }
              });
          }
        });
    });
}

fn draw_screen(ui: &mut Ui, screen: Screen) {
  let scaling_factor = ((screen_width() / 1080.0).min(screen_height() / 1920.0) * 0.7).max(0.5);
  match screen {
    Screen::MainMenu => {
      ui.element().width(grow!()).height(grow!())
        .layout(|l| l.align(CenterX, CenterY).direction(TopToBottom).gap(15))
        .children(move |ui| {
          button(ui, "Easy", 400.0, move |_| set_current_screen(Screen::Game(Difficulty::Easy)));
          button(ui, "Medium", 400.0, move |_| set_current_screen(Screen::Game(Difficulty::Medium)));
          button(ui, "Hard", 400.0, move |_| set_current_screen(Screen::Game(Difficulty::Hard)));
          button(ui, "Options", 400.0, move |_| set_current_screen(Screen::Options));
        });
    }
    Screen::Game(difficulty) => {
      if is_key_pressed(KeyCode::Escape) {
        end_interaction();
        if get_eye_holding() {
          set_highlight_mode(get_eye_initial_mode());
          set_eye_holding(false);
          set_eye_hovered_option(None);
        }
        set_current_screen(Screen::MainMenu);
      }

      let ctrl_down = is_key_down(KeyCode::LeftControl)
        || is_key_down(KeyCode::RightControl)
        || is_key_down(KeyCode::LeftSuper)
        || is_key_down(KeyCode::RightSuper);

      if ctrl_down && is_key_pressed(KeyCode::Z) {
        end_interaction();
        undo(&difficulty);
      } else if (ctrl_down && is_key_pressed(KeyCode::Y))
        || (ctrl_down && is_key_down(KeyCode::LeftShift) && is_key_pressed(KeyCode::Z))
      {
        end_interaction();
        redo(&difficulty);
      }

      if get_eye_holding() {
        let hovered = if ui.pointer_over("eye_opt_none") {
          Some(HighlightMode::None)
        } else if ui.pointer_over("eye_opt_section") {
          Some(HighlightMode::Section)
        } else if ui.pointer_over("eye_opt_row") {
          Some(HighlightMode::Row)
        } else if ui.pointer_over("eye_opt_column") {
          Some(HighlightMode::Column)
        } else {
          None
        };
        set_eye_hovered_option(hovered);

        if is_mouse_button_released(MouseButton::Left) || !is_mouse_button_down(MouseButton::Left) {
          let final_mode = hovered.unwrap_or_else(get_eye_initial_mode);
          set_highlight_mode(final_mode);
          set_eye_holding(false);
          set_eye_hovered_option(None);
        }
      } else if is_mouse_button_released(MouseButton::Left) {
        end_interaction();
      }

      ui.element().width(grow!()).height(grow!())
        .layout(|l| l.align(CenterX, CenterY).direction(TopToBottom))
        .children(|ui| {
          if let Some(level) = get_level(&difficulty) {
            // ui.text(&fmt_board(&level.level), |t| t.font_size(30).color(GREEN).font(&MONO_FONT));
            draw_top_bar(ui, &level, scaling_factor);
            draw_grid(ui, &level, scaling_factor);
            draw_bottom_bar(ui, &level, scaling_factor);
            if level.is_solved() {
              ui.element().width(grow!()).height(grow!())
                .floating(|f| f.attach_root())
                .background_color((0, 0, 0, 200))
                .layout(|l| l.align(CenterX, CenterY))
                .children(|ui| {
                  ui.text("Solved!", |t| t.font_size(50).color(GREEN));
                });
            }
          } else {
            ui.text(&format!("Generating level for {:?}...", difficulty), |t| t.font_size(30).color(YELLOW));
          }
        });
    },
    Screen::Options => {
      if is_key_pressed(KeyCode::Escape) {
        set_current_screen(Screen::MainMenu);
      }

      let options = get_options();

      ui.element().width(grow!()).height(grow!())
        .layout(|l| l.align(CenterX, CenterY).direction(TopToBottom).gap(15))
        .children(move |ui| {
          let automark_label = if options.automark { "Automark: On" } else { "Automark: Off" };
          button(ui, automark_label, 400.0, move |_| {
            let mut new_options = get_options();
            new_options.automark = !new_options.automark;
            set_options(new_options);
          });
        });
      }
  }
}

#[macroquad::main(window_conf)]
async fn main() {
  let mut ply = Ply::new(&DEFAULT_FONT).await;

  for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
    if let Err(err) = jobs::spawn(
      format!("levelgenerator_{:?}", difficulty),
      move || async move {
        generate_parks_puzzle(8, &difficulty, Duration::from_secs(100)).expect("HELP ME I'M STUCK STEPBRO PLEASE STEPBRO HELP :(")
      },
      move |level| {
        let difficulty = level.difficulty;
        set_level(&difficulty, Some(UserLevel::new(level)));
        println!("Level generated for {:?}!", difficulty);
      },
    ) {
      eprintln!("Failed to spawn level generator for {:?}: {:?}", difficulty, err);
    }
  }
  let mut is_debug_mode = false;

  loop {
    clear_background(BLACK);

    let mut ui = ply.begin();

    if is_key_pressed(KeyCode::D) {
      is_debug_mode = !is_debug_mode;
      ui.set_debug_mode(is_debug_mode);
    }

    draw_screen(&mut ui, get_current_screen());

    ui.show().await;
    
    next_frame().await;
  }
}
