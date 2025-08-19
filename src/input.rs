use crate::*;
use std::collections::hash_map::Entry;
use std::collections::*;
use sdl3::keyboard;
use sdl3::mouse;
use sdl3::mouse::MouseButton;
use slotmap::SlotMap;
use smart_default::SmartDefault;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] 
pub enum Button { 
    Key(keyboard::Keycode), 
    Mouse(mouse::MouseButton), 
    Pen(u8),
}
#[derive(Debug, Clone, Copy)] 
pub enum ButtonState { Up(i32), Down(i32) } // up/down simply means current state, pressed/released now means keystate was also changed that frame

#[derive(Debug, Clone, Copy, SmartDefault)]
pub struct PenState {
    pub pressure: f32, // 0.0 ~ 1.0
    #[default(Vector2::new(0.0, 0.0))] pub tilt: Vector2<f32>, // -90.0 ~ 90.0, left-right, top-down
    pub distance: f32, // 0.0 ~ 1.0, distance from pen to tablet
    pub rotation: f32, // -180.0 ~ 179.9, clockwise, barrel rotation
    pub slider: f32, // 0.0 ~ 1.0, pen finger wheel or slider whatever it is
    pub tangential_pressure: f32, // 0.0 ~ 1.0, barrel pressure
    pub proximity: bool,
    pub is_down: bool,
}

type Callback = Option<Box<dyn FnMut() + Send + Sync + 'static>>;

slotmap::new_key_type! { pub struct KeybindKey; }
pub struct Keybind {
    pub button_groups: Vec<HashSet<Button>>,
    pub exclusive_buttons: HashSet<Button>,
    pub attatched_screen: Option<ScreenKey>,

    pub state: ButtonState,
    pub callbacks: (Callback, Callback, Callback),
} impl Keybind {
    
}

pub struct InputManager {
    pub keybinds: SlotMap<KeybindKey, Keybind>,
    pub button_states: HashMap<Button, ButtonState>,
    pub mouse_pos_history: VecDeque<(f32, f32)>,
    pub scroll: (f32, f32),
    pub pen: PenState,
    physical_left_button_down: bool,
} impl InputManager {
    pub fn new() -> Self {
        Self {
            keybinds: SlotMap::with_key(),
            button_states: HashMap::new(), 
            mouse_pos_history: VecDeque::new(),
            scroll: (0.0, 0.0),
            pen: PenState::default(),
            physical_left_button_down: false
        }
    }

    pub fn reset_states(&mut self) {
        self.button_states.clear();
        self.mouse_pos_history.clear();
        self.scroll = (0.0, 0.0);
        self.pen = PenState::default();
        self.physical_left_button_down = false;
    }
    
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::KeyDown {..} 
            | Event::KeyUp {..} 
            | Event::MouseButtonDown {..} 
            | Event::MouseButtonUp {..} 
            | Event::PenDown {..} 
            | Event::PenUp {..} 
            | Event::PenButtonDown {..} 
            | Event::PenButtonUp {..} => self.handle_button(event),

            Event::MouseMotion { x, y, .. } => {
                self.mouse_pos_history.push_front((*x, *y));
                if self.mouse_pos_history.len() > 10 { self.mouse_pos_history.pop_back(); }
            },
            Event::MouseWheel { x, y, .. } => {
                self.scroll = (*x, *y);
            },
            Event::PenMotion { x, y, .. } => {}
            Event::PenAxis { axis, value, .. } => {
                match axis {
                    PenAxis::Pressure => self.pen.pressure = *value,
                    PenAxis::XTilt => self.pen.tilt.x = *value,
                    PenAxis::YTilt => self.pen.tilt.y = *value,
                    PenAxis::Distance => self.pen.distance = *value,
                    PenAxis::Rotation => self.pen.rotation = *value,
                    PenAxis::Slider => self.pen.slider = *value,
                    PenAxis::TangentialPressure => self.pen.tangential_pressure = *value,
                    PenAxis::Unknown => {},
                    PenAxis::Count => {},
                    _ => todo!(),
                }
            }
            Event::PenProximityIn {..} => self.pen.proximity = true,
            Event::PenProximityOut {..} => self.pen.proximity = false,
            _ => {}
        }
        
    }

    fn handle_button(&mut self, event: &Event) {
        let (button, mut is_pressed) = match event {
            Event::KeyDown { keycode: Some(key), .. } => (Button::Key(*key), true),
            Event::KeyUp { keycode: Some(key), .. } => (Button::Key(*key), false),
            Event::MouseButtonDown { mouse_btn: MouseButton::Left, .. } => {
                self.physical_left_button_down = true;
                (Button::Mouse(mouse::MouseButton::Left), true)
            },
            Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => {
                self.physical_left_button_down = false;
                (Button::Mouse(mouse::MouseButton::Left), self.pen.is_down)
            },
            Event::MouseButtonDown { mouse_btn, .. } => (Button::Mouse(*mouse_btn), true),
            Event::MouseButtonUp { mouse_btn, .. } => (Button::Mouse(*mouse_btn), false),
            Event::PenDown { .. } => {
                self.pen.is_down = true;
                (Button::Mouse(mouse::MouseButton::Left), true)
            }
            Event::PenUp { .. } => {
                self.pen = PenState {
                    proximity: self.pen.proximity,
                    ..Default::default()
                };
                (Button::Mouse(mouse::MouseButton::Left), self.physical_left_button_down)
            }
            Event::PenButtonDown { button, .. } => (Button::Pen(*button), true),
            Event::PenButtonUp { button, .. } => (Button::Pen(*button), false),
            _ => return,
        };

        if (button == Button::Mouse(mouse::MouseButton::Left)) {
            is_pressed = is_pressed && self.pen.is_down;
        }

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