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
    if (health < 0x100 || health >= er_ap_max_health()) return 0;
    unsigned room = (unsigned)(er_ap_max_health() - health + 0x3f) / 0x40;
    return (uint8_t)(queued < room ? queued : room);
}
void er_ap_enforce_health(struct MarioState *m);
void er_ap_update_heal_hurt(struct MarioState *m);
#endif
