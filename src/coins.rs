//! SM64's yellow coins, dropped by the enemies Mario defeats: they spin where the enemy fell, and
//! touching one heals a wedge (and counts on the HUD). They blink out after 20 s, like SM64's.

use std::sync::Mutex;
use std::time::Instant;

use glam::Vec3;


const LIFETIME: f32 = 20.0;
const BLINK: f32 = 3.0;
/// Mario picks it up within this distance (m)
const REACH: f32 = 0.7;

struct Coin {
    pos: Vec3,
    born: Instant,
    /// the camera can see it (no map geometry in the way)
    visible: bool,
}

static COINS: Mutex<Vec<Coin>> = Mutex::new(Vec::new());

/// A coin where an enemy fell (its feet, in the game's physics coordinates).
pub fn spawn(pos: Vec3) {
    COINS.lock().unwrap_or_else(|e| e.into_inner()).push(Coin { pos: pos + Vec3::Y * 0.05, born: Instant::now(), visible: true });
}

/// Coins Mario (feet at `mario`) touches now; they're gone (as are expired ones).
pub fn collect(mario: Vec3) -> u32 {
    let mut coins = COINS.lock().unwrap_or_else(|e| e.into_inner());
    let mut taken = 0;
    coins.retain(|c| {
        let d = c.pos - mario;
        let touched = Vec3::new(d.x, 0.0, d.z).length() < REACH && d.y > -0.8 && d.y < 1.7;
        taken += touched as u32;
        !touched && c.born.elapsed().as_secs_f32() < LIFETIME
    });
    taken
}

/// The game re-based its physics coordinates by `d`.
pub fn shift(d: Vec3) {
    for c in COINS.lock().unwrap_or_else(|e| e.into_inner()).iter_mut() {
        c.pos += d;
    }
}

pub fn clear() {
    COINS.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// Game thread: which coins the camera at `cam` can see; `blocked(from, to)` says whether map
/// geometry is between two points. A coin shows if its middle or its top is in view.
pub fn update_visibility(cam: Vec3, blocked: impl Fn(Vec3, Vec3) -> bool) {
    for c in COINS.lock().unwrap_or_else(|e| e.into_inner()).iter_mut() {
        c.visible = [0.32, 0.6].iter().any(|&h| !blocked(cam, c.pos + Vec3::Y * h));
    }
}

/// The coins to draw: position (bottom centre) and age in seconds (blinking ones left out).
pub fn visible() -> Vec<(Vec3, f32)> {
    COINS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .filter(|c| c.visible)
        .map(|c| (c.pos, c.born.elapsed().as_secs_f32()))
        .filter(|&(_, age)| age < LIFETIME && !(age > LIFETIME - BLINK && (age * 10.0) as u32 % 2 == 0))
        .collect()
}
