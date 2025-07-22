use crate::*;
use std::collections::hash_map::Entry;
use std::collections::*;
use sdl3::keyboard;
use sdl3::mouse;
use slotmap::SlotMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] 
pub enum Button { Key(keyboard::Keycode), Mouse(mouse::MouseButton) }
#[derive(Debug, Clone, Copy)] 
pub enum ButtonState { Up(i32), Down(i32) } // up/down simply means current state, pressed/released now means keystate was also changed that frame

slotmap::new_key_type! { pub struct KeybindKey; }
pub struct Keybind {
    pub button_groups: Vec<HashSet<Button>>,
    pub exclusive_buttons: HashSet<Button>,
    pub attatched_screen: Option<ScreenKey>,

    pub state: ButtonState,
} impl Keybind {
    
}

pub struct InputManager {
    pub keybinds: SlotMap<KeybindKey, Keybind>,
    pub button_states: HashMap<Button, ButtonState>,
    pub mouse_pos_history: VecDeque<(f32, f32)>,
    pub scroll: (f32, f32),
} impl InputManager {
    pub fn new() -> Self {
        Self {
            keybinds: SlotMap::with_key(),
            button_states: HashMap::new(), 
            mouse_pos_history: VecDeque::new(),
            scroll: (0.0, 0.0)
        }
    }

    pub fn reset_states(&mut self) {
        self.button_states.clear();
        self.mouse_pos_history.clear();
        self.scroll = (0.0, 0.0);
    }
    
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::KeyDown {..} | Event::KeyUp {..} | Event::MouseButtonDown {..} | Event::MouseButtonUp {..} => self.handle_button(event),
            Event::MouseMotion { x, y, .. } => {
                self.mouse_pos_history.push_front((*x, *y));
                if self.mouse_pos_history.len() > 10 { self.mouse_pos_history.pop_back(); }
            },
            Event::MouseWheel { x, y, .. } => {
                self.scroll = (*x, *y);
            },
            Event::PenUp {..} => {}
            _ => {}
        }
        
    }

    fn handle_button(&mut self, event: &Event) {
        let (button, is_pressed) = match event {
            Event::KeyDown { keycode: Some(key), .. } => (Button::Key(*key), true),
            Event::KeyUp { keycode: Some(key), .. } => (Button::Key(*key), false),
            Event::MouseButtonDown { mouse_btn, .. } => (Button::Mouse(*mouse_btn), true),
            Event::MouseButtonUp { mouse_btn, .. } => (Button::Mouse(*mouse_btn), false),
            _ => return,
        };

        let new_state = match self.button_states.entry(button) {
            Entry::Occupied(mut entry) => match entry.get() {
                ButtonState::Down(frames) if is_pressed => ButtonState::Down(frames + 1),
                ButtonState::Down(_) => ButtonState::Up(0),
                ButtonState::Up(frames) if !is_pressed => ButtonState::Up(frames + 1),
                ButtonState::Up(_) => ButtonState::Down(0),
            },
            Entry::Vacant(entry) => {
                if is_pressed {
                    ButtonState::Down(0)
                } else {
                    ButtonState::Up(0)
                }
            }
        };

        // HashMap.insert updates kvp or inserts them when non-existent, while returning old states
        self.button_states.insert(button, new_state);
    }

    pub fn is_down(&self, button: &Button) -> bool {
        matches!(self.button_states.get(button), Some(ButtonState::Down(_)))
    }

    pub fn frames(&self, button: &Button) -> i32 {
        match self.button_states.get(button) {
            Some(ButtonState::Down(f)) | Some(ButtonState::Up(f)) => *f,
            _ => i32::max_value()
        }
    }

    pub fn is_pressed(&self, button: &Button) -> bool {
        match self.button_states.get(button) {
            Some(ButtonState::Down(frames)) => *frames == 0,
            _ => false,
        }
    }

    pub fn is_released(&self, button: &Button) -> bool {
        match self.button_states.get(button) {
            Some(ButtonState::Up(frames)) => *frames == 0,
            _ => false,
        }
    }

    pub fn update(&mut self, screens: &ScreenManager) {
        for (_, mut keybind) in &mut self.keybinds {
            let screen_ok = match (&keybind.attatched_screen, screens.active_screen) {
                (None, _) => true,
                // (Some(filter), Some(current)) => filter == *current,
                _ => false,
            };

            let mut active = screen_ok;

            // exclusive buttons
            for button in &keybind.exclusive_buttons {
                if matches!(self.button_states.get(button), Some(ButtonState::Down(_))) {
                    active = false;
                    break;
                }
            }

            for i in 0..keybind.button_groups.len() {
                let group = &keybind.button_groups[i];
                
                // .filter_map(): .map() but with filtering
                let min_dur = group.iter()
                    .filter_map(|btn| match self.button_states.get(btn) {
                        Some(ButtonState::Down(frames)) => Some(*frames),
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
                        .filter_map(|btn| match self.button_states.get(btn) {
                            Some(ButtonState::Down(frames)) => Some(*frames),
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
                (true, ButtonState::Down(frames)) => ButtonState::Down(frames + 1),
                (true, ButtonState::Up(_)) => ButtonState::Down(0),
                (false, ButtonState::Up(frames)) => ButtonState::Up(frames + 1),
                (false, ButtonState::Down(_)) => ButtonState::Up(0),
            };
        }
    }
}