use crate::*;
use std::collections::*;
use sdl3::keyboard;
use sdl3::mouse;
use slotmap::SlotMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button { Key(keyboard::Keycode), Mouse(mouse::MouseButton) }
pub enum ButtonState { Released(i32), Pressed(i32) }

slotmap::new_key_type! { pub struct KeybindKey; }
pub struct Keybind {
    pub button_groups: Vec<HashSet<Button>>,
    pub exclusive_buttons: HashSet<Button>,
    pub attatched_screen: Option<ScreenKey>,

    pub state: ButtonState,
}

pub struct InputManager {
    pub button_states: HashMap<Button, ButtonState>,
    pub keybinds: SlotMap<KeybindKey, Keybind>,
} impl InputManager {
    pub fn new() -> Self {
        Self { button_states: HashMap::new(), keybinds: SlotMap::with_key() }
    }

    pub fn update(&self, app: &App) {
        let mut tracked_buttons = HashSet::new();
        for (_, keybind) in self.keybinds.iter() {
            tracked_buttons.extend(keybind.button_groups.iter().flatten().copied());
            tracked_buttons.extend(keybind.exclusive_buttons.clone());
        }

        for button in tracked_buttons {
            let is_pressed = match button {
                Button::Key(code) => keyboard::.pressed(code),
                Button::Mouse(btn) => mouse.pressed(btn),
            };

            // and_modify().or_insert(): HashMap only, modifies if value exists and inserts if there exists not
            // !!! and_modify returns &mut - dereference before matching !!!
            button_states.0.entry(button)
                .and_modify(|state| {
                    *state = match (is_pressed, *state) {
                        (true, ButtonState::Pressed(frames)) => ButtonState::Pressed(frames + 1),
                        (true, ButtonState::Released(_)) => ButtonState::Pressed(0),
                        (false, ButtonState::Released(frames)) => ButtonState::Released(frames + 1),
                        (false, ButtonState::Pressed(_)) => ButtonState::Released(0),
                    };
                })
                .or_insert(if is_pressed {
                    ButtonState::Pressed(0)
                } else {
                    ButtonState::Released(0)
                });
        }

        for mut keybind in &mut keybinds {
            let screen_ok = match (&keybind.screen_filter, &active_screen.0) {
                (None, _) => true,
                // (Some(filter), Some(current)) => filter == *current,
                _ => false,
            };

            let mut active = screen_ok;

            // exclusive buttons
            for button in &keybind.exclusive_buttons {
                if matches!(button_states.0.get(button), Some(ButtonState::Pressed(_))) {
                    active = false;
                    break;
                }
            }

            for i in 0..keybind.button_groups.len() {
                let group = &keybind.button_groups[i];
                
                // .filter_map(): .map() but with filtering
                let min_dur = group.iter()
                    .filter_map(|btn| match button_states.0.get(btn) {
                        Some(ButtonState::Pressed(frames)) => Some(*frames),
                        _ => None
                    })
                    .min();
                
                if min_dur.is_none() {
                    active = false;
                    break;
                }

                // Compare with next group if exists
                if i + 1 < keybind.button_groups.len() {
                    let next_group = &keybind.button_groups[i + 1];
                    let max_next_dur = next_group.iter()
                        .filter_map(|btn| match button_states.0.get(btn) {
                            Some(ButtonState::Pressed(frames)) => Some(*frames),
                            _ => None
                        })
                        .max();
                    
                    // .map_or()/.map_or_else(): unwraps option with a default value, default value is first
                    if max_next_dur.is_some_and(|max| min_dur.unwrap() < max) {
                        active = false;
                        break;
                    }
                }
            }

            keybind.state = match (active, keybind.state) {
                (true, ButtonState::Pressed(frames)) => ButtonState::Pressed(frames + 1),
                (true, ButtonState::Released(_)) => ButtonState::Pressed(0),
                (false, ButtonState::Released(frames)) => ButtonState::Released(frames + 1),
                (false, ButtonState::Pressed(_)) => ButtonState::Released(0),
            };
        }
    }
}