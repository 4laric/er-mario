//! Observe native flask use; ER owns consumption, refills and AP flask upgrades.
use eldenring::cs::{EquipParamGoods, GameDataMan, PlayerIns, SoloParamRepository, SpEffectParam};
use fromsoftware_shared::FromStatic;

use crate::flask_policy::{self, Snapshot};

pub fn snapshot() -> Option<Snapshot> {
    let gdm = unsafe { GameDataMan::instance() }.ok()?;
    let pgd = &gdm.main_player_game_data;
    let items = &pgd.equipment.equip_inventory_data.items_data;
    let mut total = 0u32;
    let mut tiers = 0u16;
    // Flasks live in the single-player key list even when a multiplayer key
    // accessor is selected. Do not use InventoryItemsData::items() here.
    for entry in items
        .key_entries()
        .iter()
        .chain(items.normal_entries())
        .filter_map(|e| e.as_option())
    {
        if let Some(tier) = flask_policy::crimson_tier(entry.item_id.into_inner()) {
            total = total.checked_add(entry.quantity)?;
            tiers |= 1 << tier;
        }
    }
    (total <= 20).then_some(Snapshot {
        player: pgd.as_ptr() as usize,
        allocation: pgd.max_hp_flask,
        total,
        tiers,
    })
}

pub fn queued(player: &PlayerIns) -> Option<u32> {
    let id = player.chr_ins.tae_queued_use_item.as_valid()?.into_inner();
    flask_policy::crimson_tier(id).map(|_| id)
}

pub fn healing(player: &PlayerIns, item: u32) -> u8 {
    let Some(tier) = flask_policy::crimson_tier(item) else {
        return 0;
    };
    let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
        return 0;
    };
    let Some(goods) = repo.get::<EquipParamGoods>(1001 + 2 * tier) else {
        return 0;
    };
    // Validate the consumed goods, then resolve its actual live effect instead
    // of reading AP's desired potency or modifying ER's mirrored flask IDs.
    if goods.ref_category() != 2 || goods.goods_use_anim() != 10 || goods.ref_id_default() < 0 {
        return 0;
    }
    let Some(effect) = repo.get::<SpEffectParam>(goods.ref_id_default() as u32) else {
        return 0;
    };
    let mut correction = 1.0f32;
    for entry in player.chr_ins.special_effect.entries() {
        let Some(row) = entry.param_data.map(|p| unsafe { p.as_ref() }) else {
            continue;
        };
        let rate = row.change_hp_estus_flask_correct_rate();
        if !rate.is_finite() || rate < 0.0 {
            return 0;
        }
        correction *= rate;
    }
    flask_policy::healing(effect.change_hp_estus_flask_point(), correction)
}
