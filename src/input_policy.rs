//! Menu inference must only observe buttons that Elden Ring actually receives.
/// R3 belongs to native lock-on; Sonic's air dash uses LT.
pub fn air_dash_trigger(sonic: bool, left_trigger: u8) -> bool {
    sonic && left_trigger > 100
}

/// A native lock keeps ownership even when Lakitu is the selected free camera.
pub fn lakitu_owns_camera(selected: bool, target_locked: bool) -> bool {
    selected && !target_locked
}

pub fn infer_menu(current: bool, routed: bool, opener: bool, pressed: bool, right_trigger: u8, cappy: bool) -> bool {
    let pressed = pressed || (right_trigger > 100 && !cappy);
    if opener { true } else if pressed && current == routed { !routed } else { current }
}

/// Opening a guessed menu requires a fresh game-owned press. Returning to
/// gameplay accepts held input, so a stick held through unpause still works.
pub fn infer_menu_fresh(current: bool, routed: bool, opener: bool, pressed: bool, right_trigger: u8, cappy: bool, held: &mut bool) -> bool {
    let native_pressed = pressed || (right_trigger > 100 && !cappy);
    let fresh = native_pressed && !*held;
    *held = native_pressed;
    infer_menu(current, routed, opener, if current { native_pressed } else { fresh }, 0, cappy)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crouch_routing_gaps_do_not_invent_menus_and_held_input_unpauses() {
        let mut held = false;
        assert!(!infer_menu_fresh(false, true, false, true, 0, false, &mut held));
        for routed in [false, true, false, true] {
            assert!(!infer_menu_fresh(false, routed, false, true, 0, false, &mut held));
        }
        assert!(!infer_menu_fresh(false, false, false, false, 0, false, &mut held));
        assert!(infer_menu_fresh(false, false, false, true, 0, false, &mut held));
        assert!(!infer_menu_fresh(true, true, false, true, 0, false, &mut held));
        assert!(infer_menu_fresh(false, true, true, true, 0, false, &mut held));
        held = false;
        for _ in 0..5 { assert!(!infer_menu_fresh(false, false, false, false, 255, true, &mut held)); }
        assert!(infer_menu_fresh(false, false, false, false, 255, false, &mut held));
    }
    #[test]
    fn dash_trigger_requires_sonic_and_a_pressed_trigger() {
        for (sonic, trigger, dash) in [
            (true, 0, false), (true, 100, false), (true, 101, true),
            (true, 255, true), (false, 255, false),
        ] { assert_eq!(air_dash_trigger(sonic, trigger), dash); }
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
