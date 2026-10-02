//! Menu inference must only observe buttons that Elden Ring actually receives.
pub fn infer_menu(current: bool, routed: bool, opener: bool, pressed: bool, right_trigger: u8, cappy: bool) -> bool {
    let pressed = pressed || (right_trigger > 100 && !cappy);
    if opener { true } else if pressed && current == routed { !routed } else { current }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_cap_trigger_does_not_invent_a_popup_or_suspend_the_throw() {
        let mut menu = false;
        for _ in 0..10 {
            // The RT filter removes the Tarnished attack: no game action is routed.
            menu = infer_menu(menu, false, false, false, 255, true);
            assert!(!menu);
        }
        assert!(infer_menu(false, false, false, false, 255, false));
    }
    #[test]
    fn real_menus_dialogue_and_gameplay_keep_their_input_transitions() {
        assert!(infer_menu(false, false, true, false, 255, true));
        assert!(infer_menu(false, false, false, true, 0, true));
        assert!(!infer_menu(true, true, false, true, 0, true));
        assert!(infer_menu(true, false, false, false, 255, true));
    }
}
