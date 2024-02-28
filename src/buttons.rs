#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ButtonTypes {
    None,
    VolUp,
    VolDown,
    Play,
    Menu,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Buttons {
    pub menu: bool,
    pub play: bool,
    pub vol_up: bool,
    pub vol_dn: bool,
}

// impl Deref for Buttons {
//     type Target = Buttons;

//     fn deref(&self) -> &Self::Target {

//     }
// }
impl Buttons {
    pub fn set_state(&mut self, menu: bool, play: bool, vol_up: bool, vol_dn: bool) {
        self.menu = menu;
        self.play = play;
        self.vol_up = vol_up;
        self.vol_dn = vol_dn;
    }

    pub fn reset_state(&mut self, value: bool) {
        self.set_state(value, value, value, value);
    }

    pub fn set_button_state(&mut self, button: ButtonTypes, value: bool) {
        match button {
            ButtonTypes::None => todo!(),
            ButtonTypes::VolUp => self.vol_up = value,
            ButtonTypes::VolDown => self.vol_dn = value,
            ButtonTypes::Play => self.play = value,
            ButtonTypes::Menu => self.menu = value,
        }
    }
}

#[cfg(test)]
mod test_super {
    use super::*;

    #[test]
    fn test_button_state() {
        let mut buttons = Buttons {
            menu: false,
            play: false,
            vol_up: false,
            vol_dn: false,
        };
        buttons.set_state(true, false, false, false);
        assert_eq!(buttons.menu, true);
    }
}
