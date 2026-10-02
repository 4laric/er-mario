//! Supply the native Regression effect on an interact edge; EMEVD owns the reveal and quest flags.
use crate::goldmask_policy as policy;
use eldenring::cs::{
    CSEventFlagMan, ChrInsExt, EquipGameData, EquipInventoryData, EquipInventoryDataListEntry,
    GameDataMan, InventoryItemsData, PlayerGameData, PlayerIns, WorldChrMan,
};
use fromsoftware_shared::FromStatic;
use std::mem::{MaybeUninit, offset_of, size_of};
use windows::Win32::System::{Diagnostics::Debug::ReadProcessMemory, Threading::GetCurrentProcess};

// Read scalar inventory metadata/entries through Windows: unavailable memory is never possession.
fn read<T: Copy>(address: usize) -> Option<T> {
    let mut out = MaybeUninit::<T>::uninit();
    let mut count = 0;
    unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const _,
            out.as_mut_ptr().cast(),
            size_of::<T>(),
            Some(&mut count),
        )
        .ok()?;
        (count == size_of::<T>()).then(|| out.assume_init())
    }
}

fn inventory_has_law(items: *const InventoryItemsData) -> bool {
    let base = items as usize;
    let Some(head) = read::<usize>(base + offset_of!(InventoryItemsData, normal_items_head)) else {
        return false;
    };
    let Some(len) = read::<u32>(base + offset_of!(InventoryItemsData, normal_items_len)) else {
        return false;
    };
    let Some(capacity) = read::<u32>(base + offset_of!(InventoryItemsData, normal_items_capacity))
    else {
        return false;
    };
    if head == 0 || len > capacity || capacity > 100_000 {
        return false;
    }
    for i in 0..len as usize {
        let Some(entry) = head.checked_add(i * size_of::<EquipInventoryDataListEntry>()) else {
            return false;
        };
        let id = read::<u32>(entry + offset_of!(EquipInventoryDataListEntry, item_id));
        let quantity = read::<u32>(entry + offset_of!(EquipInventoryDataListEntry, quantity));
        if id.is_none() || quantity.is_none() {
            return false;
        }
        if id == Some(policy::LAW_OF_REGRESSION) && quantity.is_some_and(|q| q > 0) {
            return true;
        }
    }
    false
}

fn owns_law() -> bool {
    let Ok(gdm) = (unsafe { GameDataMan::instance() }) else {
        return false;
    };
    let pgd = gdm.main_player_game_data.as_ptr() as usize;
    let bag = pgd
        + offset_of!(PlayerGameData, equipment)
        + offset_of!(EquipGameData, equip_inventory_data)
        + offset_of!(EquipInventoryData, items_data);
    if inventory_has_law(bag as *const InventoryItemsData) {
        return true;
    }
    // Option<OwnedPtr<_>> uses the nullable-pointer niche of its NonNull wrapper.
    let Some(storage) = read::<usize>(pgd + offset_of!(PlayerGameData, storage)) else {
        return false;
    };
    storage != 0
        && inventory_has_law(
            (storage + offset_of!(EquipInventoryData, items_data)) as *const InventoryItemsData,
        )
}

pub fn interact(player: &PlayerIns, edge: bool) {
    let mut state = crate::ap_capabilities::State {
        abi_version: 0,
        flags: 0,
        managed: 0,
        unlocked: 0,
    };
    unsafe { crate::ap_capabilities::er_mario_ap_get_state(&mut state) };
    let live = state.managed != 0 && state.flags & 7 == 7;
    let alive = player.chr_ins.modules.data.hp > 0;
    let menu = crate::MENU_OPEN.load(std::sync::atomic::Ordering::Relaxed);
    if !edge || !live || !alive || menu {
        return;
    }
    let Ok(flags) = (unsafe { CSEventFlagMan::instance() }) else {
        return;
    };
    let revealed = flags.virtual_memory_flag.get_flag(policy::REVEALED);
    let waiting = flags.virtual_memory_flag.get_flag(policy::WAITING);
    if revealed || !waiting || !owns_law() {
        return;
    }
    let Ok(world) = (unsafe { WorldChrMan::instance() }) else {
        return;
    };
    let here = player.chr_ins.modules.physics.position;
    let distance = world
        .chr_sets
        .iter()
        .flatten()
        .flat_map(|set| set.characters())
        .filter(|chr| chr.event_entity_id == policy::STATUE)
        .map(|chr| {
            let p = chr.modules.physics.position;
            (p.0 - here.0).powi(2) + (p.1 - here.1).powi(2) + (p.2 - here.2).powi(2)
        })
        .find(|d| d.is_finite() && *d <= 16.0);
    if policy::permitted(policy::Context {
        interact: edge,
        managed_live: live,
        alive,
        menu,
        revealed,
        waiting,
        owns_law: true,
        distance_squared: distance,
    }) {
        let Ok(world) = (unsafe { WorldChrMan::instance_mut() }) else {
            return;
        };
        let Some(main) = world.main_player.as_mut() else {
            return;
        };
        if !std::ptr::eq::<PlayerIns>(&**main, player) {
            return;
        }
        main.chr_ins
            .apply_speffect(policy::REGRESSION_EFFECT, false);
        crate::log("Goldmask: interact supplied native Law of Regression effect");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_reads_fail_closed_and_require_full_id_and_positive_quantity() {
        fn put<const N: usize>(bytes: &mut [u8], offset: usize, value: [u8; N]) {
            bytes[offset..offset + N].copy_from_slice(&value);
        }
        let mut entries = vec![0u8; 2 * size_of::<EquipInventoryDataListEntry>()];
        let mut header = vec![0u8; size_of::<InventoryItemsData>()];
        put(
            &mut header,
            offset_of!(InventoryItemsData, normal_items_head),
            (entries.as_ptr() as usize).to_ne_bytes(),
        );
        put(
            &mut header,
            offset_of!(InventoryItemsData, normal_items_len),
            2u32.to_ne_bytes(),
        );
        put(
            &mut header,
            offset_of!(InventoryItemsData, normal_items_capacity),
            2u32.to_ne_bytes(),
        );
        let inventory = header.as_ptr().cast::<InventoryItemsData>();
        assert!(!inventory_has_law(
            std::ptr::dangling::<InventoryItemsData>()
        ));
        assert!(!inventory_has_law(inventory));
        // Entry zero is an empty gap; a stored spell can be farther down the list.
        let id_offset = size_of::<EquipInventoryDataListEntry>()
            + offset_of!(EquipInventoryDataListEntry, item_id);
        let quantity_offset = size_of::<EquipInventoryDataListEntry>()
            + offset_of!(EquipInventoryDataListEntry, quantity);
        put(
            &mut entries,
            id_offset,
            policy::LAW_OF_REGRESSION.to_ne_bytes(),
        );
        assert!(!inventory_has_law(inventory));
        put(&mut entries, quantity_offset, 1u32.to_ne_bytes());
        assert!(inventory_has_law(inventory));
        put(&mut entries, id_offset, 0x1a4au32.to_ne_bytes());
        assert!(!inventory_has_law(inventory));
        put(
            &mut entries,
            id_offset,
            policy::LAW_OF_REGRESSION.to_ne_bytes(),
        );
        put(
            &mut header,
            offset_of!(InventoryItemsData, normal_items_len),
            3u32.to_ne_bytes(),
        );
        assert!(!inventory_has_law(inventory));
    }
}
