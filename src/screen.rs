use slotmap::SlotMap;

pub struct ScreenManager {
    pub screens: SlotMap<ScreenKey, Screen>,
} impl ScreenManager {
    pub fn new() -> Self {
        Self { screens: SlotMap::with_key() }
    }
}

slotmap::new_key_type! { pub struct ScreenKey; }
pub struct Screen {

}