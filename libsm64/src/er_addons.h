#ifndef ER_ADDONS_H
#define ER_ADDONS_H
#include <stdint.h>
struct MarioState;
struct ERCappy {
    uint32_t enabled, mask, allowed, held, previous, needs_release, phase, age, bounced;
    float position[3], direction[3], spin_yaw;
    /* Snapshot at throw, bounded before every segment is swept. */
    float velocity[3];
};
struct ERSonic {
    uint32_t enabled, mask, allowed, held, dash, previous_dash, needs_release;
    uint32_t spin_charge, drop_charge, pending_drop, rolling, rolling_unlock, dash_ticks, air_used;
    /* Presentation only: no extra movement step or resource consumption. */
    uint32_t feedback, feedback_ticks, feedback_request;
    /* Completed collision step: Air Dash=4, rolling dash=8, charge=0. */
    uint32_t attack_state, attack_generation;
};
extern struct ERCappy er_cappy;
extern struct ERSonic er_sonic;
void er_cappy_configure(uint32_t enabled, uint32_t mask);
void er_sonic_configure(uint32_t enabled, uint32_t mask);
void er_addons_input(uint32_t allowed, uint32_t cap_held, uint32_t spin_held, uint32_t dash);
void er_addons_reset(void);
/* Called only for an actual AIR_STEP_LANDED result, never on action assignment. */
void er_addons_landed(struct MarioState *m);
/* Advances the cap, then claims at most one collision-resolved Mario physics step. */
int er_addons_step(struct MarioState *m);
int er_addons_dispatch(struct MarioState *m);
void er_addons_after(struct MarioState *m);
#endif
