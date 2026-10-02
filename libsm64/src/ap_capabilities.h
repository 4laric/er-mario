#ifndef ER_AP_CAPABILITIES_H
#define ER_AP_CAPABILITIES_H
#include <stdint.h>
#include "decomp/include/sm64.h"

/* Worker-owned state. Unmanaged abilities remain available. */
extern uint32_t er_ap_managed, er_ap_unlocked;
void sm64_er_ap_set_capabilities(uint32_t managed, uint32_t unlocked);
static inline int er_ap_allows(uint32_t bits) {
    return (bits & er_ap_managed & ~er_ap_unlocked) == 0;
}
static inline uint32_t er_ap_action_capability(uint32_t action) {
    switch (action) {
        case ACT_DOUBLE_JUMP: return 1;
        case ACT_TRIPLE_JUMP: case ACT_FLYING_TRIPLE_JUMP:
        case ACT_SPECIAL_TRIPLE_JUMP: return 128;
        case ACT_BACKFLIP: return 256;
        case ACT_SIDE_FLIP: return 512;
        case ACT_LONG_JUMP: return 2;
        case ACT_WALL_KICK_AIR: return 4;
        case ACT_DIVE: case ACT_DIVE_SLIDE: case ACT_SLIDE_KICK:
        case ACT_SLIDE_KICK_SLIDE: return 8;
        case ACT_GROUND_POUND: case ACT_GROUND_POUND_LAND: return 16;
        case ACT_PICKING_UP: case ACT_DIVE_PICKING_UP: return 32;
        case ACT_PICKING_UP_BOWSER: case ACT_HOLDING_BOWSER:
        case ACT_RELEASING_BOWSER: return 64;
        default: return 0;
    }
}
static inline uint32_t er_ap_attack_capability(uint32_t action) {
    /* Preserve slope-driven slides as movement; only their attacks need Dive. */
    switch (action) {
        case ACT_STOMACH_SLIDE: case ACT_BUTT_SLIDE:
        case ACT_HOLD_STOMACH_SLIDE: case ACT_HOLD_BUTT_SLIDE: return 8;
        default: return er_ap_action_capability(action) & (8 | 16);
    }
}
static inline uint32_t er_ap_filter_action(uint32_t action, uint32_t previous) {
    uint32_t cap = er_ap_action_capability(action);
    /* A diving pickup requires both abilities. */
    if (action == ACT_DIVE_PICKING_UP) cap |= 8;
    if (er_ap_allows(cap)) return action;
    if (cap & (1 | 2 | 128 | 256 | 512)) return ACT_JUMP;
    if (cap & 4) return ACT_FREEFALL;
    return (previous & ACT_FLAG_AIR) ? ACT_FREEFALL : ACT_IDLE;
}
/* Return false when the rejected action would re-enter the current action.
 * SM64 retries true transitions in the same tick with the same pressed input. */
static inline int er_ap_prepare_action(uint32_t action, uint32_t previous, uint32_t *filtered) {
    *filtered = er_ap_filter_action(action, previous);
    return !(*filtered != action && *filtered == previous);
}
#endif
