#ifndef ER_AP_STATS_H
#define ER_AP_STATS_H
#include <stdint.h>
struct MarioState;
extern uint32_t er_ap_max_wedges;
static inline int16_t er_ap_max_health(void) { return (int16_t)((er_ap_max_wedges << 8) + 0x80); }
static inline int16_t er_ap_clamp_health(int health) {
    return (int16_t)(health > er_ap_max_health() ? er_ap_max_health() : health);
}
static inline uint8_t er_ap_bound_healing(int health, unsigned queued) {
    if (health < 0x100) return 0;
    /* Native healing buffers offset pending damage even while already full. */
    return (uint8_t)(queued < 255 ? queued : 255);
}
void er_ap_enforce_health(struct MarioState *m);
void er_ap_update_heal_hurt(struct MarioState *m);
#endif
