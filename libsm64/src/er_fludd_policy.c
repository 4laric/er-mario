#include "er_fludd.h"
struct ERFludd er_fludd = {0,0,0,0,0,60,0,0,0,0,0,0,0,0};
static uint32_t first(uint32_t mask) { return mask & (0u-mask); }
void er_fludd_configure(uint32_t enabled, uint32_t mask, uint32_t level) {
    if (enabled > 1 || (mask & ~15u) || level > 3 || (!enabled && (mask || level))) return;
    if (er_fludd.enabled == enabled && er_fludd.mask == mask && er_fludd.level == level) return;
    er_fludd.enabled = enabled; er_fludd.mask = mask; er_fludd.level = level;
    er_fludd.capacity = 60 + 20 * level;
    if (er_fludd.water > er_fludd.capacity) er_fludd.water = er_fludd.capacity;
    if (!(mask & er_fludd.selected)) { er_fludd.selected = first(mask); er_fludd.active = er_fludd.charge = 0; }
    if (!enabled) er_fludd.water = er_fludd.selected = er_fludd.protected_fall = 0;
}
void er_fludd_reset(void) {
    er_fludd.water = er_fludd.enabled ? er_fludd.capacity : 0;
    er_fludd.active = er_fludd.charge = er_fludd.allowed = er_fludd.protected_fall = 0;
    er_fludd.held = er_fludd.cycle = er_fludd.previous_cycle = er_fludd.select = 0;
}
void er_fludd_input(uint32_t allowed, uint32_t held, uint32_t select, uint32_t cycle) {
    er_fludd.allowed = allowed; er_fludd.held = held; er_fludd.select = select; er_fludd.cycle = cycle;
    if (!allowed) er_fludd.active = er_fludd.charge = 0;
}
int er_fludd_policy(int safe, int grounded) {
    uint32_t old = er_fludd.selected;
    if (er_fludd.allowed && er_fludd.enabled) {
        if ((er_fludd.select == 1 || er_fludd.select == 2 || er_fludd.select == 4 || er_fludd.select == 8) && (er_fludd.mask & er_fludd.select)) er_fludd.selected = er_fludd.select;
        if (er_fludd.cycle && !er_fludd.previous_cycle) {
            uint32_t next = er_fludd.selected ? er_fludd.selected << 1 : 1;
            for (int i=0;i<4;i++) { if (next>8) next=1; if (er_fludd.mask & next) break; next <<= 1; }
            if (er_fludd.mask & next) er_fludd.selected = next;
        }
    }
    er_fludd.previous_cycle = er_fludd.cycle;
    if (old != er_fludd.selected) er_fludd.charge = 0;
    er_fludd.active = 0;
    if (!er_fludd.enabled || !er_fludd.allowed || !safe || !er_fludd.selected) {
        er_fludd.charge = 0; return 0;
    }
    if (!er_fludd.held) {
        er_fludd.charge = 0;
        if (grounded && er_fludd.water < er_fludd.capacity) er_fludd.water++;
        return 0;
    }
    if (er_fludd.selected == 2) {
        if (er_fludd.water < 60) { er_fludd.charge = 0; return 0; }
        if (++er_fludd.charge < 30) return 1; /* charging, no water spent yet */
        er_fludd.water -= 60; er_fludd.charge = 0; er_fludd.active = 2; return 2;
    }
    if (er_fludd.selected == 4 && er_fludd.charge < 20) { er_fludd.charge++; if (er_fludd.charge < 20) return 1; }
    if (!er_fludd.water) { er_fludd.charge = 0; return 0; }
    er_fludd.water--; er_fludd.active = er_fludd.selected;
    return er_fludd.selected == 1 ? 3 : (er_fludd.selected == 8 ? 5 : 4);
}
