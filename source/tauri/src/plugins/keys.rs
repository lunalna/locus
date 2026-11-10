// Plugin shortcuts are written like "Ctrl+Alt+Space", with W3C key codes, or as modifiers alone like "Alt".

pub const CTRL: u8 = 1;
pub const ALT: u8 = 2;
pub const SHIFT: u8 = 4;
pub const META: u8 = 8;
pub const FN: u8 = 16;
// Set together with ALT while the right Alt or Option key is held.
pub const RIGHT_ALT: u8 = 32;

#[derive(Clone, PartialEq, Debug)]
pub struct Combo {
    pub mods: u8,
    pub code: Option<String>,
}

impl Combo {
    pub fn parse(text: &str) -> Option<Combo> {
        let mut combo = Combo { mods: 0, code: None };
        for token in text.split('+').map(str::trim) {
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => combo.mods |= CTRL,
                "alt" | "option" => combo.mods |= ALT,
                "rightalt" => combo.mods |= ALT | RIGHT_ALT,
                "shift" => combo.mods |= SHIFT,
                "cmd" | "command" | "super" | "meta" => combo.mods |= META,
                "fn" => combo.mods |= FN,
                _ if combo.code.is_none() && !token.is_empty() => combo.code = Some(token.into()),
                _ => return None,
            }
        }
        Some(combo)
    }

    // The right Alt key only has to be the one held when the shortcut asks for it.
    pub fn matches(&self, pressed: &Combo) -> bool {
        self.mods & !RIGHT_ALT == pressed.mods & !RIGHT_ALT
            && (self.mods & RIGHT_ALT == 0 || pressed.mods & RIGHT_ALT != 0)
            && self.code == pressed.code
    }

    #[cfg(desktop)]
    pub fn accelerator(&self) -> String {
        let names = [(CTRL, "Ctrl"), (ALT, "Alt"), (SHIFT, "Shift"), (META, "Super")];
        names
            .iter()
            .filter(|(flag, _)| self.mods & flag != 0)
            .map(|(_, name)| *name)
            .chain(self.code.as_deref())
            .collect::<Vec<_>>()
            .join("+")
    }
}

// Modifier-only shortcuts fire when a modifier goes down, not when another is released.
#[cfg(desktop)]
pub struct Modifiers(u8);

#[cfg(desktop)]
impl Modifiers {
    pub const fn new() -> Self {
        Modifiers(0)
    }

    pub fn set(&mut self, mods: u8) -> Option<Combo> {
        let added = mods & !self.0 != 0;
        self.0 = mods;
        added.then_some(Combo { mods, code: None })
    }

    #[cfg(any(windows, target_os = "linux"))]
    pub fn down(&mut self, flag: u8) -> Option<Combo> {
        self.set(self.0 | flag)
    }

    #[cfg(any(windows, target_os = "linux"))]
    pub fn up(&mut self, flag: u8) {
        self.0 &= !flag;
    }
}
