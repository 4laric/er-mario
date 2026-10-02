//! Menu inference must only observe buttons that Elden Ring actually receives.
/// R3 belongs to native lock-on; Sonic takes it only with the dash modifier.
pub fn air_dash_chord(sonic: bool, left_shoulder: bool, right_click: bool) -> bool {
    sonic && left_shoulder && right_click
}

/// A native lock keeps ownership even when Lakitu is the selected free camera.
pub fn lakitu_owns_camera(selected: bool, target_locked: bool) -> bool {
    selected && !target_locked
}

pub fn infer_menu(current: bool, routed: bool, opener: bool, pressed: bool, right_trigger: u8, cappy: bool) -> bool {
    let pressed = pressed || (right_trigger > 100 && !cappy);
    if opener { true } else if pressed && current == routed { !routed } else { current }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_lock_click_and_modified_dash_have_distinct_owners() {
        for (sonic, modifier, click, dash) in [
            (true, false, true, false), (true, true, true, true),
            (false, true, true, false), (true, true, false, false),
        ] { assert_eq!(air_dash_chord(sonic, modifier, click), dash); }
    }
    #[test]
    fn native_lock_yields_camera_and_unlock_restores_selected_mode() {
        assert!(lakitu_owns_camera(true, false));
        assert!(!lakitu_owns_camera(true, true));
        assert!(!lakitu_owns_camera(false, true));
        assert!(!lakitu_owns_camera(false, false));
        assert!(lakitu_owns_camera(true, false));
    }
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
