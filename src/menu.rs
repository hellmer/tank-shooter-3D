//! Title screen, pause menu and options. Works with keyboard, mouse and gamepad.

use crate::settings::{SENSITIVITY_RANGE, SENSITIVITY_STEP, Settings, VOLUME_STEP};
use raylib::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Screen {
    Title,
    Playing,
    Paused,
    /// Options, opened from the title or from the pause menu.
    Options {
        from_pause: bool,
    },
}

/// What the player did in a menu this frame, for `main` to carry out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    None,
    /// Start a new game from the title.
    Play,
    Resume,
    Restart,
    ToTitle,
    Quit,
    /// An option changed: apply it (and save it when leaving the options).
    SettingsChanged,
    /// Leaving the options screen.
    SettingsDone,
}

/// Menu input for one frame, from whatever device.
#[derive(Clone, Copy, Debug, Default)]
pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub back: bool,
    /// Esc or Start: opens and closes the pause menu.
    pub pause: bool,
    pub mouse: Vector2,
    pub mouse_moved: bool,
    pub click: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
    Play,
    Resume,
    Restart,
    Options,
    ToTitle,
    Quit,
    EffectsVolume,
    MusicVolume,
    MouseSensitivity,
    StickSensitivity,
    Back,
}

impl Item {
    fn label(self) -> &'static str {
        match self {
            Item::Play => "Jugar",
            Item::Resume => "Continuar",
            Item::Restart => "Reiniciar",
            Item::Options => "Opciones",
            Item::ToTitle => "Salir al menu",
            Item::Quit => "Salir del juego",
            Item::EffectsVolume => "Efectos",
            Item::MusicVolume => "Musica",
            Item::MouseSensitivity => "Raton",
            Item::StickSensitivity => "Mando",
            Item::Back => "Volver",
        }
    }

    fn is_value(self) -> bool {
        matches!(
            self,
            Item::EffectsVolume
                | Item::MusicVolume
                | Item::MouseSensitivity
                | Item::StickSensitivity
        )
    }
}

fn items(screen: Screen) -> &'static [Item] {
    match screen {
        Screen::Title => &[Item::Play, Item::Options, Item::Quit],
        Screen::Paused => &[
            Item::Resume,
            Item::Restart,
            Item::Options,
            Item::ToTitle,
            Item::Quit,
        ],
        Screen::Options { .. } => &[
            Item::EffectsVolume,
            Item::MusicVolume,
            Item::MouseSensitivity,
            Item::StickSensitivity,
            Item::Back,
        ],
        Screen::Playing => &[],
    }
}

const ROW_W: f32 = 460.0;
const ROW_H: f32 = 46.0;
const ROW_GAP: f32 = 10.0;

/// Screen rectangle of menu row `i`, shared by drawing and mouse picking.
pub fn row_rect(sw: f32, sh: f32, i: usize) -> Rectangle {
    let top = sh * 0.40;
    Rectangle::new(
        (sw - ROW_W) / 2.0,
        top + i as f32 * (ROW_H + ROW_GAP),
        ROW_W,
        ROW_H,
    )
}

fn step_value(settings: &mut Settings, item: Item, dir: f32) -> bool {
    let (lo, hi) = SENSITIVITY_RANGE;
    let before = *settings;
    let vol = |v: f32| ((v + dir * VOLUME_STEP).clamp(0.0, 1.0) * 100.0).round() / 100.0;
    let sens = |v: f32| ((v + dir * SENSITIVITY_STEP).clamp(lo, hi) * 100.0).round() / 100.0;
    match item {
        Item::EffectsVolume => settings.effects_volume = vol(settings.effects_volume),
        Item::MusicVolume => settings.music_volume = vol(settings.music_volume),
        Item::MouseSensitivity => settings.mouse_sensitivity = sens(settings.mouse_sensitivity),
        Item::StickSensitivity => settings.stick_sensitivity = sens(settings.stick_sensitivity),
        _ => {}
    }
    *settings != before
}

fn value_text(settings: &Settings, item: Item) -> String {
    match item {
        Item::EffectsVolume => format!("{:.0}%", settings.effects_volume * 100.0),
        Item::MusicVolume => format!("{:.0}%", settings.music_volume * 100.0),
        Item::MouseSensitivity => format!("x{:.2}", settings.mouse_sensitivity),
        Item::StickSensitivity => format!("x{:.2}", settings.stick_sensitivity),
        _ => String::new(),
    }
}

/// How full the bar under a value is (0..1).
fn value_fill(settings: &Settings, item: Item) -> f32 {
    let (lo, hi) = SENSITIVITY_RANGE;
    match item {
        Item::EffectsVolume => settings.effects_volume,
        Item::MusicVolume => settings.music_volume,
        Item::MouseSensitivity => (settings.mouse_sensitivity - lo) / (hi - lo),
        Item::StickSensitivity => (settings.stick_sensitivity - lo) / (hi - lo),
        _ => 0.0,
    }
}

pub struct Menu {
    pub screen: Screen,
    selected: usize,
}

impl Menu {
    pub fn new(screen: Screen) -> Menu {
        Menu {
            screen,
            selected: 0,
        }
    }

    fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.selected = 0;
    }

    /// Opens the pause menu (called while playing).
    pub fn pause(&mut self) {
        self.go(Screen::Paused);
    }

    /// Handles one frame of menu input. `sw`/`sh` are the screen size, for the mouse.
    pub fn update(
        &mut self,
        input: &MenuInput,
        sw: f32,
        sh: f32,
        settings: &mut Settings,
    ) -> Action {
        let list = items(self.screen);
        if list.is_empty() {
            return Action::None;
        }
        let n = list.len();

        // Leaving a screen.
        let back = input.back || input.pause;
        match self.screen {
            Screen::Paused if back => {
                self.go(Screen::Playing);
                return Action::Resume;
            }
            Screen::Options { from_pause } if back => {
                self.go(if from_pause {
                    Screen::Paused
                } else {
                    Screen::Title
                });
                return Action::SettingsDone;
            }
            _ => {}
        }

        if input.up {
            self.selected = (self.selected + n - 1) % n;
        }
        if input.down {
            self.selected = (self.selected + 1) % n;
        }
        // The mouse picks the row it is over when it moves or clicks.
        let hovered = (0..n).find(|&i| row_rect(sw, sh, i).check_collision_point_rec(input.mouse));
        if let Some(i) = hovered
            && (input.mouse_moved || input.click)
        {
            self.selected = i;
        }
        let item = list[self.selected.min(n - 1)];

        if item.is_value() {
            let mut dir = 0.0;
            if input.left {
                dir -= 1.0;
            }
            if input.right {
                dir += 1.0;
            }
            // Clicking a value row: left half lowers it, right half raises it.
            if input.click && hovered.is_some() {
                let r = row_rect(sw, sh, self.selected);
                dir = if input.mouse.x < r.x + r.width / 2.0 {
                    -1.0
                } else {
                    1.0
                };
            }
            if dir != 0.0 && step_value(settings, item, dir) {
                return Action::SettingsChanged;
            }
            return Action::None;
        }

        let chosen = input.select || (input.click && hovered.is_some());
        if !chosen {
            return Action::None;
        }
        match item {
            Item::Play => {
                self.go(Screen::Playing);
                Action::Play
            }
            Item::Resume => {
                self.go(Screen::Playing);
                Action::Resume
            }
            Item::Restart => {
                self.go(Screen::Playing);
                Action::Restart
            }
            Item::Options => {
                let from_pause = self.screen == Screen::Paused;
                self.go(Screen::Options { from_pause });
                Action::None
            }
            Item::ToTitle => {
                self.go(Screen::Title);
                Action::ToTitle
            }
            Item::Quit => Action::Quit,
            Item::Back => {
                let Screen::Options { from_pause } = self.screen else {
                    return Action::None;
                };
                self.go(if from_pause {
                    Screen::Paused
                } else {
                    Screen::Title
                });
                Action::SettingsDone
            }
            _ => Action::None,
        }
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle, settings: &Settings, gamepad: bool) {
        let list = items(self.screen);
        if list.is_empty() {
            return;
        }
        let sw = d.get_screen_width();
        let sh = d.get_screen_height();
        let shade = if self.screen == Screen::Title {
            90
        } else {
            150
        };
        d.draw_rectangle(0, 0, sw, sh, Color::new(5, 8, 12, shade));

        let cyan = Color::new(70, 235, 255, 255);
        let (title, size) = match self.screen {
            Screen::Title => ("TANK SHOOTER 3D", 64),
            Screen::Paused => ("PAUSA", 52),
            _ => ("OPCIONES", 52),
        };
        let tw = d.measure_text(title, size);
        let ty = (sh as f32 * 0.40) as i32 - size - 50;
        d.draw_text(
            title,
            (sw - tw) / 2 + 3,
            ty + 3,
            size,
            Color::new(0, 0, 0, 160),
        );
        d.draw_text(title, (sw - tw) / 2, ty, size, cyan);

        for (i, item) in list.iter().enumerate() {
            let r = row_rect(sw as f32, sh as f32, i);
            let on = i == self.selected;
            let bg = if on {
                Color::new(70, 235, 255, 60)
            } else {
                Color::new(0, 0, 0, 120)
            };
            d.draw_rectangle_rec(r, bg);
            if on {
                d.draw_rectangle_lines_ex(r, 2.0, cyan);
            }
            let fg = if on {
                Color::WHITE
            } else {
                Color::new(200, 205, 210, 255)
            };
            let (x, y) = (r.x as i32, r.y as i32);
            if item.is_value() {
                d.draw_text(item.label(), x + 18, y + 12, 24, fg);
                let value = value_text(settings, *item);
                let arrows = format!("<  {value}  >");
                let vw = d.measure_text(&arrows, 24);
                d.draw_text(&arrows, x + r.width as i32 - vw - 18, y + 12, 24, fg);
                let fill = value_fill(settings, *item).clamp(0.0, 1.0);
                let bar_w = (r.width - 36.0) as i32;
                d.draw_rectangle(
                    x + 18,
                    y + r.height as i32 - 7,
                    bar_w,
                    3,
                    Color::new(255, 255, 255, 40),
                );
                d.draw_rectangle(
                    x + 18,
                    y + r.height as i32 - 7,
                    (bar_w as f32 * fill) as i32,
                    3,
                    cyan,
                );
            } else {
                let lw = d.measure_text(item.label(), 26);
                d.draw_text(item.label(), x + (r.width as i32 - lw) / 2, y + 10, 26, fg);
            }
        }

        let help = match (self.screen, gamepad) {
            (Screen::Options { .. }, true) => "Cruceta: elegir y cambiar | B: volver",
            (Screen::Options { .. }, false) => {
                "Flechas o WASD: elegir y cambiar | clic | Esc: volver"
            }
            (Screen::Title, true) => "Cruceta: elegir | A: aceptar",
            (Screen::Title, false) => "Flechas o WASD: elegir | Enter o clic: aceptar",
            (_, true) => "Cruceta: elegir | A: aceptar | B: volver",
            (_, false) => "Flechas o WASD: elegir | Enter o clic: aceptar | Esc: volver",
        };
        let hw = d.measure_text(help, 20);
        d.draw_text(
            help,
            (sw - hw) / 2,
            sh - 44,
            20,
            Color::new(220, 220, 220, 200),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 1280.0;
    const H: f32 = 720.0;

    fn press(f: impl FnOnce(&mut MenuInput)) -> MenuInput {
        let mut i = MenuInput {
            mouse: Vector2::new(-100.0, -100.0),
            ..Default::default()
        };
        f(&mut i);
        i
    }

    #[test]
    fn title_play_starts_the_game() {
        let mut m = Menu::new(Screen::Title);
        let mut s = Settings::default();
        assert_eq!(
            m.update(&press(|i| i.select = true), W, H, &mut s),
            Action::Play
        );
        assert_eq!(m.screen, Screen::Playing);
    }

    #[test]
    fn pause_key_opens_and_closes_options_back_to_pause() {
        let mut m = Menu::new(Screen::Playing);
        let mut s = Settings::default();
        m.pause();
        // Down twice to Options, select, then Esc goes back to the pause menu, and Esc resumes.
        m.update(&press(|i| i.down = true), W, H, &mut s);
        m.update(&press(|i| i.down = true), W, H, &mut s);
        m.update(&press(|i| i.select = true), W, H, &mut s);
        assert_eq!(m.screen, Screen::Options { from_pause: true });
        assert_eq!(
            m.update(&press(|i| i.pause = true), W, H, &mut s),
            Action::SettingsDone
        );
        assert_eq!(m.screen, Screen::Paused);
        assert_eq!(
            m.update(&press(|i| i.pause = true), W, H, &mut s),
            Action::Resume
        );
        assert_eq!(m.screen, Screen::Playing);
    }

    #[test]
    fn left_and_right_change_values_within_limits() {
        let mut m = Menu::new(Screen::Options { from_pause: false });
        let mut s = Settings::default();
        for _ in 0..20 {
            m.update(&press(|i| i.right = true), W, H, &mut s);
        }
        assert_eq!(s.effects_volume, 1.0);
        for _ in 0..20 {
            m.update(&press(|i| i.left = true), W, H, &mut s);
        }
        assert_eq!(s.effects_volume, 0.0);
        m.update(&press(|i| i.down = true), W, H, &mut s);
        m.update(&press(|i| i.down = true), W, H, &mut s);
        assert_eq!(
            m.update(&press(|i| i.right = true), W, H, &mut s),
            Action::SettingsChanged
        );
        assert_eq!(s.mouse_sensitivity, 1.25);
    }

    #[test]
    fn mouse_clicks_the_row_under_it() {
        let mut m = Menu::new(Screen::Title);
        let mut s = Settings::default();
        let r = row_rect(W, H, 2);
        let at = Vector2::new(r.x + 10.0, r.y + 10.0);
        let a = m.update(
            &press(|i| {
                i.mouse = at;
                i.click = true;
            }),
            W,
            H,
            &mut s,
        );
        assert_eq!(a, Action::Quit);
    }
}
