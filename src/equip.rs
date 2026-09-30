//! Mario's loadout: in Mario mode the player wears the Mario set (items 9990000-9990300, model 999,
//! added to regulation.bin) with bare fists, and nothing else can be equipped; switching Mario off
//! puts the previous loadout back.
//!
//! Uses the game's own equip and item-give functions, found by byte pattern (patterns and calling
//! conventions from The Grand Archives' Elden Ring cheat table).

use std::sync::OnceLock;

use eldenring::cs::{GameDataMan, MapItemMan, PlayerIns};
use fromsoftware_shared::FromStatic;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::log;

const PROTECTOR: u32 = 0x1000_0000;
/// Mario set: (ChrAsm slot, item id)
pub const MARIO_SET: [(usize, u32); 4] =
    [(12, 660000 | PROTECTOR), (13, 660100 | PROTECTOR), (14, 660200 | PROTECTOR), (15, 660300 | PROTECTOR)];
const UNARMED: u32 = 110000;
const WEAPON_SLOTS: [usize; 6] = [0, 1, 2, 3, 4, 5];

type EquipFn = unsafe extern "C" fn(usize, u32, *const u32, u32, u64, u64, u64) -> u64;
type GiveFn = unsafe extern "C" fn(usize, *const u8, *mut u8, u64) -> u64;

struct Funcs {
    equip: EquipFn,
    give: GiveFn,
}

static FUNCS: OnceLock<Option<Funcs>> = OnceLock::new();

/// The game's image in memory.
fn image() -> &'static [u8] {
    let base = unsafe { GetModuleHandleW(None) }.map(|h| h.0 as usize).unwrap_or(0);
    let nt = base + unsafe { *((base + 0x3c) as *const u32) } as usize;
    let size = unsafe { *((nt + 0x50) as *const u32) } as usize;
    unsafe { std::slice::from_raw_parts(base as *const u8, size) }
}

/// All matches of a pattern like "?? 8b f1 ?? 8b d8" in the executable sections.
fn scan(pattern: &str) -> Vec<usize> {
    let pat: Vec<Option<u8>> = pattern.split_whitespace().map(|t| u8::from_str_radix(t, 16).ok()).collect();
    let img = image();
    let base = img.as_ptr() as usize;
    // only scan committed, readable memory: walk the section table
    let nt = base + unsafe { *((base + 0x3c) as *const u32) } as usize;
    let sections = unsafe { *((nt + 6) as *const u16) } as usize;
    let opt_size = unsafe { *((nt + 20) as *const u16) } as usize;
    let first = nt + 24 + opt_size;
    let mut out = Vec::new();
    for s in 0..sections {
        let h = first + s * 40;
        let characteristics = unsafe { *((h + 36) as *const u32) };
        if characteristics & 0x2000_0000 == 0 {
            continue; // not executable
        }
        let va = unsafe { *((h + 12) as *const u32) } as usize;
        let len = unsafe { *((h + 8) as *const u32) } as usize;
        let sec = &img[va..(va + len).min(img.len())];
        'outer: for i in 0..sec.len().saturating_sub(pat.len()) {
            for (k, p) in pat.iter().enumerate() {
                if let Some(b) = p {
                    if sec[i + k] != *b {
                        continue 'outer;
                    }
                }
            }
            out.push(base + va + i);
            if out.len() > 4 {
                return out;
            }
        }
    }
    out
}

/// Finds the game functions (once, a fraction of a second).
pub fn init() -> bool {
    FUNCS
        .get_or_init(|| {
            let t = std::time::Instant::now();
            let equip = scan("?? 8b f1 ?? 8b d8 ?? 63 ea ?? 8b f9");
            let give = scan("8b 02 83 f8 0a");
            log(format!("equip: pattern scan {:.0} ms, equip {} match(es), give {} match(es)", t.elapsed().as_secs_f32() * 1000.0, equip.len(), give.len()));
            if equip.len() != 1 || give.len() != 1 {
                return None;
            }
            Some(Funcs {
                equip: unsafe { std::mem::transmute::<usize, EquipFn>(equip[0] - 0x17) },
                give: unsafe { std::mem::transmute::<usize, GiveFn>(give[0] - 0x52) },
            })
        })
        .is_some()
}

/// Vagabond Knight rows: in Mario mode their model is switched to Mario's (999). New rows injected at
/// runtime aren't accepted by the game's equip code (it resolves armour through another table), and
/// a patched regulation.bin crashes me3, so Mario borrows the Vagabond set.
const VAGABOND: [u32; 4] = [660000, 660100, 660200, 660300];
const VAGABOND_MODEL: u16 = 1280;
const MARIO_MODEL: u16 = 999;
/// "Nothing" armour items per slot (equipping these empties the slot).
const BARE: [(usize, u32); 4] = [(12, 10000 | PROTECTOR), (13, 10100 | PROTECTOR), (14, 10200 | PROTECTOR), (15, 10300 | PROTECTOR)];

/// Menu icons: Vagabond's and Mario's (unused icon slots 13580-13583, packed by tools/build_icons.py).
const VAGABOND_ICONS: [u16; 4] = [14010, 14011, 14012, 14013];
const MARIO_ICONS: [u16; 4] = [13580, 13581, 13582, 13583];

/// Switches the Vagabond set's model and menu icons to Mario's (or back).
fn set_vagabond_model(model: u16) -> bool {
    let Ok(repo) = (unsafe { eldenring::cs::SoloParamRepository::instance_mut() }) else { return false };
    let icons = if model == MARIO_MODEL { MARIO_ICONS } else { VAGABOND_ICONS };
    for (k, id) in VAGABOND.into_iter().enumerate() {
        let Some(row) = repo.get_mut::<eldenring::cs::EquipParamProtector>(id) else { return false };
        row.set_equip_model_id(model);
        row.set_icon_id_m(icons[k]);
        row.set_icon_id_f(icons[k]);
    }
    true
}

/// Set while the armour slots are being emptied (so that equipping the set afterwards loads the
/// model that was just switched); the game applies equips a little later, so this is a phase the
/// lock works through, not a one-off call.
static BARING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn armour_is_bare(player: &PlayerIns) -> bool {
    BARE.iter().all(|&(slot, item)| equipped(player, slot) == item)
}

/// PlayerGameData of the local player.
fn player_game_data() -> Option<usize> {
    let gdm = unsafe { GameDataMan::instance() }.ok()?;
    Some(gdm.main_player_game_data.as_ptr() as usize)
}

/// Inventory index (as the equip function wants it) of an item, if the player has it.
fn inventory_index(item: u32) -> Option<u32> {
    let pgd = player_game_data()?;
    let gdm = unsafe { GameDataMan::instance() }.ok()?;
    let items = &gdm.main_player_game_data.equipment.equip_inventory_data.items_data;
    let head = items.normal_items_head.as_ptr() as usize;
    let len = items.normal_items_len as usize;
    let tail = unsafe { *((pgd + 0x408 + 0x1c) as *const u32) };
    (0..len).find(|i| unsafe { *((head + i * 0x18 + 4) as *const u32) } == item).map(|i| i as u32 + tail)
}

/// Puts `item` (from the inventory) into ChrAsm `slot`. The "nothing" armour items are given first
/// if the inventory lacks them.
fn equip(slot: usize, item: u32) -> bool {
    if inventory_index(item).is_none() && BARE.iter().any(|b| b.1 == item) {
        give(&[item]);
    }
    let (Some(Some(f)), Some(pgd), Some(idx)) = (FUNCS.get(), player_game_data(), inventory_index(item)) else {
        return false;
    };
    let data = [item, 0, 0, 0];
    unsafe { (f.equip)(pgd + 0x2b0, slot as u32, data.as_ptr(), idx, 1, 1, 0) };
    true
}

/// Adds items (quantity 1) to the inventory.
fn give(items: &[u32]) {
    let Some(Some(f)) = FUNCS.get() else { return };
    let Ok(man) = (unsafe { MapItemMan::instance_mut() }) else { return };
    let mut buf = [0u8; 4 + 16 * 10];
    buf[..4].copy_from_slice(&(items.len().min(10) as u32).to_le_bytes());
    for (k, item) in items.iter().take(10).enumerate() {
        let e = 4 + k * 16;
        buf[e..e + 4].copy_from_slice(&item.to_le_bytes());
        buf[e + 4..e + 8].copy_from_slice(&1u32.to_le_bytes());
        buf[e + 8..e + 12].copy_from_slice(&u32::MAX.to_le_bytes());
        buf[e + 12..e + 16].copy_from_slice(&u32::MAX.to_le_bytes());
    }
    let mut scratch = [0u8; 128];
    unsafe { (f.give)(man as *mut MapItemMan as usize, buf.as_ptr(), scratch.as_mut_ptr(), 0) };
}

/// Item id currently in a ChrAsm slot.
fn equipped(player: &PlayerIns, slot: usize) -> u32 {
    let id = player.chr_asm.equipment_param_ids[slot] as u32;
    if (12..=15).contains(&slot) { id | PROTECTOR } else { id }
}

/// What the player wore before Mario mode.
pub struct Loadout(Vec<(usize, u32)>);

/// Mario mode on: remember the loadout, make sure the Mario set is in the inventory, wear it.
pub fn enter(player: &PlayerIns) -> Option<Loadout> {
    if !init() {
        return None;
    }
    crate::names::apply(true);
    crate::voice::mute(true);
    if !set_vagabond_model(MARIO_MODEL) {
        log("equip: Vagabond rows not found, leaving the equipment alone");
        return None;
    }
    let saved = Loadout(WEAPON_SLOTS.iter().chain([12, 13, 14, 15].iter()).map(|&s| (s, equipped(player, s))).collect());
    // calibration log: our index for the equipped chest vs the game's own record
    if let Some(gdm) = unsafe { GameDataMan::instance() }.ok() {
        let game_idx = gdm.main_player_game_data.equipment.equipment_item_idx_list[13];
        log(format!("equip: chest {:#x}: our index {:?}, game's {game_idx}", equipped(player, 13), inventory_index(equipped(player, 13))));
    }
    let missing: Vec<u32> = MARIO_SET.iter().map(|&(_, i)| i).filter(|&i| inventory_index(i).is_none()).collect();
    if !missing.is_empty() {
        log(format!("equip: giving the Mario set {missing:x?}"));
        give(&missing);
    }
    // empty the armour slots first; the lock then equips the set, which loads the Mario model
    BARING.store(true, std::sync::atomic::Ordering::Relaxed);
    enforce(player);
    Some(saved)
}

/// Mario mode: re-equip the Mario set / fists wherever something else is worn. Returns true if it had to.
pub fn enforce(player: &PlayerIns) -> bool {
    if !matches!(FUNCS.get(), Some(Some(_))) {
        return false;
    }
    let mut changed = false;
    static LOGGED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if BARING.load(std::sync::atomic::Ordering::Relaxed) && armour_is_bare(player) {
        BARING.store(false, std::sync::atomic::Ordering::Relaxed);
        log("equip: armour emptied, putting the Mario set on");
    }
    let armour = if BARING.load(std::sync::atomic::Ordering::Relaxed) { BARE } else { MARIO_SET };
    let wanted = armour.iter().copied().chain(WEAPON_SLOTS.iter().map(|&s| (s, UNARMED)));
    for (slot, item) in wanted {
        let now = equipped(player, slot);
        if now != item {
            let ok = equip(slot, item);
            if LOGGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 20 {
                log(format!("equip: slot {slot} has {now:#x}, want {item:#x}: {}", if ok { "equipping" } else { "not in inventory" }));
            }
            changed |= ok;
        }
    }
    changed
}

/// Mario mode off, step 1: Vagabond model back, armour slots emptied (so the model reloads).
pub fn leave(_saved: &Loadout) {
    set_vagabond_model(VAGABOND_MODEL);
    crate::names::apply(false);
    crate::voice::mute(false);
}

/// Mario mode off, step 2 (called every frame until it returns true): empty the armour slots, then
/// put the previous loadout back.
pub fn restore(player: &PlayerIns, saved: &Loadout) -> bool {
    if !armour_is_bare(player) {
        for (slot, item) in BARE {
            if equipped(player, slot) != item {
                equip(slot, item);
            }
        }
        return false;
    }
    for &(slot, item) in &saved.0 {
        equip(slot, item);
    }
    true
}


