use crate::*;

pub struct ScreenManager {
    pub screens: SlotMap<ScreenKey, Screen>,
    pub active_screen: Option<ScreenKey>,
}
impl ScreenManager {
    pub fn new() -> Self {
        Self {
            screens: SlotMap::with_key(),
            active_screen: None,
        }
    }
}

slotmap::new_key_type! { pub struct ScreenKey; }
pub struct Screen {}
