#include "ap_stats.h"
#include "decomp/include/types.h"
uint32_t er_ap_max_wedges = 8;
void er_ap_enforce_health(struct MarioState *m) {
    m->health = er_ap_clamp_health(m->health);
    m->healCounter = er_ap_bound_healing(m->health, m->healCounter);
}
void er_ap_update_heal_hurt(struct MarioState *m) {
    er_ap_enforce_health(m);
    if (m->health < 0x100) { m->health = 0xff; return; }
    if (m->healCounter > 0) { m->health += 0x40; m->healCounter--; }
    if (m->hurtCounter > 0) { m->health -= 0x40; m->hurtCounter--; }
    if (m->health < 0x100) m->health = 0xff;
    er_ap_enforce_health(m);
}
