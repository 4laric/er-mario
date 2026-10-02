#ifndef ER_FLUDD_H
#define ER_FLUDD_H
#include <stdint.h>
struct MarioState;
struct ERFludd {
    uint32_t enabled, mask, level, selected, water, capacity;
    uint32_t charge, active, allowed, held, select, cycle, previous_cycle, protected_fall;
};
extern struct ERFludd er_fludd;
void er_fludd_configure(uint32_t enabled, uint32_t mask, uint32_t level);
void er_fludd_reset(void);
void er_fludd_input(uint32_t allowed, uint32_t held, uint32_t select, uint32_t cycle);
/* Pure policy shared by native integration and standalone regression tests. */
int er_fludd_policy(int safe, int grounded);
void er_fludd_after(struct MarioState *m);
int er_fludd_step(struct MarioState *m);
#endif
