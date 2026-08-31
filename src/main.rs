use parks::*;
use ply_engine::prelude::*;
use std::{cell::RefCell, time::Duration};
use fxhash::FxHashMap;

static DEFAULT_FONT: FontAsset = font!("assets/fonts/lexend.ttf");
static MONO_FONT: FontAsset = font!("assets/fonts/geist_mono.ttf");

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



#[derive(Clone)]
struct UserLevel {
  level: ParksLevel,
  markings: Vec<Vec<Marking>>,
  toggle_exclude: Vec<(usize, usize)>,
}
impl UserLevel {
  fn new(level: ParksLevel) -> Self {
    Self {
      level: level.clone(),
      markings: vec![vec![Marking::NotMarked; level.size]; level.size],
      toggle_exclude: Vec::new(),
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
  LEVELS.with(|l| l.borrow_mut().insert(difficulty.clone(), level));
}
fn get_level(difficulty: &Difficulty) -> Option<UserLevel> {
  LEVELS.with(|l| l.borrow().get(difficulty).cloned().flatten())
}
fn toggle_cell(difficulty: &Difficulty, row: usize, col: usize, drag_toggle: bool) {
  if let Some(mut level) = get_level(difficulty) {
    if drag_toggle && level.markings[row][col] != Marking::NotMarked {
      return;
    }
    if level.toggle_exclude.contains(&(row, col)) {
      return;
    }
    level.markings[row][col] = level.markings[row][col].next();
    if level.markings[row][col] == Marking::NotMarked {
      level.toggle_exclude.push((row, col));
    }
    set_level(difficulty, Some(level));
  }
}
fn reset_exclude(difficulty: &Difficulty) {
  if let Some(mut level) = get_level(difficulty) {
    level.toggle_exclude.clear();
    set_level(difficulty, Some(level));
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Marking {
  NotMarked,
  MarkedTree,
  MarkedEmpty,
}
impl Marking {
  fn next(&self) -> Self {
    match self {
      Marking::NotMarked => Marking::MarkedEmpty,
      Marking::MarkedEmpty => Marking::MarkedTree,
      Marking::MarkedTree => Marking::NotMarked,
    }
  }
}

fn draw_grid(ui: &mut Ui, level: &UserLevel) {
  ui.element().width(grow!()).height(grow!()).contain(1.0)
    .border(|b| b.all(8).color(LIGHTGRAY).position(Middle))
    .corner_radius(20.0)
    .children(|ui| {
      ui.element().width(grow!()).height(grow!())
        .layout(|l| l.direction(TopToBottom))
        .border(|b| b.between_children(5).color(GRAY).position(Middle))
        .children(|ui| {
          for r in 0..level.size() {
            ui.element().width(grow!()).height(grow!())
              .layout(|l| l.direction(LeftToRight))
              .border(|b| b.between_children(5).color(GRAY))
              .children(|ui| {
                for c in 0..level.size() {
                  let difficulty = level.level.difficulty.clone();
                  let difficulty2 = level.level.difficulty.clone();
                  ui.element().id((format!("cell_{}_{}", r, c).as_str(), 0))
                    .width(grow!()).height(grow!())
                    .layout(|l| l.align(CenterX, CenterY))
                    .accessibility(|a| a.button(&format!("Cell {}:{}", r, c)))
                    .background_color(PALETTE[level.region(r, c).expect("Cell not in any region") % PALETTE.len()])
                    .on_press(move |_, _| {
                      toggle_cell(&difficulty2, r, c, false);
                    })
                    .on_hover(move |_, pointer| {
                      if pointer.pressed() && !pointer.just_pressed() {
                        toggle_cell(&difficulty, r, c, true);
                      }
                    })
                    .children(|ui| {
                      let marking = level.markings[r][c];
                      ui.text(match marking {
                        Marking::NotMarked => "",
                        Marking::MarkedTree => "T",
                        Marking::MarkedEmpty => "X",
                      }, |t| t.font_size(10).color(BLACK));
                    });
                }
              });
          }
        });
    });
}

fn draw_screen(ui: &mut Ui, screen: Screen) {
  match screen {
    Screen::MainMenu => {
      ui.element().width(grow!()).height(grow!())
        .layout(|l| l.align(CenterX, CenterY).direction(TopToBottom).gap(15))
        .children(move |ui| {
          button(ui, "Easy", 400.0, move |_| set_current_screen(Screen::Game(Difficulty::Easy)));
          button(ui, "Medium", 400.0, move |_| set_current_screen(Screen::Game(Difficulty::Medium)));
          button(ui, "Hard", 400.0, move |_| set_current_screen(Screen::Game(Difficulty::Hard)));
        });
    }
    Screen::Game(difficulty) => {
      if is_key_pressed(KeyCode::Escape) {
        set_current_screen(Screen::MainMenu);
      }
      ui.element().width(grow!()).height(grow!())
        .layout(|l| l.align(CenterX, CenterY).direction(TopToBottom))
        .children(|ui| {
          if let Some(level) = get_level(&difficulty) {
            // ui.text(&fmt_board(&level.level), |t| t.font_size(30).color(GREEN).font(&MONO_FONT));
            if is_mouse_button_pressed(MouseButton::Left) {
              reset_exclude(&difficulty);
            }
            draw_grid(ui, &level);
            if (level.is_solved()) {
              ui.text("Congratulations! You solved the puzzle!", |t| t.font_size(30).color(GREEN));
            } else {
              ui.text(&format!("Correct trees: {}/{}", level.correct_tree_count(), level.level.solution_trees.len()), |t| t.font_size(30).color(YELLOW));
            }
          } else {
            ui.text(&format!("Generating level for {:?}...", difficulty), |t| t.font_size(30).color(YELLOW));
          }
        });
    }
  }
}

#[macroquad::main(window_conf)]
async fn main() {
  let mut ply = Ply::new(&DEFAULT_FONT).await;

  for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
    let difficulty_for_task = difficulty.clone();
    if let Err(err) = jobs::spawn(
      format!("levelgenerator_{:?}", difficulty),
      move || async move {
        generate_parks_puzzle(8, &difficulty_for_task, Duration::from_secs(100)).expect("HELP ME I'M STUCK STEPBRO PLEASE STEPBRO HELP :(")
      },
      move |level| {
        let difficulty = level.difficulty.clone();
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
