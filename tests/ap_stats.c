#include <assert.h>
#include "ap_stats.h"
#include "decomp/include/types.h"

int main(void) {
    struct MarioState m = {0};
    for (unsigned wedges = 4; wedges <= 8; wedges++) {
        er_ap_max_wedges = wedges;
        const int maximum = (wedges << 8) + 0x80;
        assert(er_ap_max_health() == maximum);
        /* Native restore/star/grace values and water healing cannot exceed capacity. */
        assert(er_ap_clamp_health(0x880) == maximum);
        assert(er_ap_clamp_health(0xffff) == maximum);
        for (int health = 0; health <= 0x880; health++) {
            m.health = health; m.healCounter = 31; m.hurtCounter = 0;
            er_ap_enforce_health(&m);
            assert(m.health <= maximum);
            if (health < 0x100) { assert(m.health == health); assert(m.healCounter == 0); }
            else {
                for (int tick = 0; tick < 40; tick++) er_ap_update_heal_hurt(&m);
                assert(m.health == maximum && m.healCounter == 0);
            }
        }
        m.health = maximum - 1; m.healCounter = 255; m.hurtCounter = 0;
        m.health += 0x1a; er_ap_update_heal_hurt(&m); /* water-surface recovery */
        assert(m.health == maximum && m.healCounter == 254);
        m.health = 0x100; m.healCounter = 0; m.hurtCounter = 1;
        er_ap_update_heal_hurt(&m);
        assert(m.health == 0xff); /* drain/hurt death marker preserved */
        m.health = 0xfd; m.healCounter = 0; er_ap_update_heal_hurt(&m);
        assert(m.health == 0xff); /* drowning crosses the death threshold */
        m.health = 0xff; m.healCounter = 31; er_ap_enforce_health(&m);
        assert(m.health == 0xff && m.healCounter == 0);
    }
    er_ap_max_wedges = 4; m.health = 0x380; m.healCounter = 0;
    er_ap_enforce_health(&m); er_ap_max_wedges = 8; er_ap_enforce_health(&m);
    assert(m.health == 0x380); /* receiving/resetting capacity never heals */
    m.health = 0x880; m.healCounter = 31; er_ap_max_wedges = 4; er_ap_enforce_health(&m);
    assert(m.health == 0x480 && m.healCounter == 31); /* preserve native healing buffer */
    m.health = 0xff; er_ap_max_wedges = 8; er_ap_enforce_health(&m);
    assert(m.health == 0xff); /* reset never resurrects */
    /* Exact vanilla parity for simultaneous healing/damage, including full buffers. */
    er_ap_max_wedges = 8;
    for (int health = 0x100; health <= 0x880; health += 0x40) {
        for (unsigned heal = 0; heal <= 31; heal++) {
            for (unsigned hurt = 0; hurt <= 31; hurt++) {
                struct MarioState old = {0};
                old.health = health; old.healCounter = heal; old.hurtCounter = hurt;
                m = old;
                for (int tick = 0; tick < 40; tick++) {
                    if (old.health >= 0x100) {
                        if (old.healCounter > 0) { old.health += 0x40; old.healCounter--; }
                        if (old.hurtCounter > 0) { old.health -= 0x40; old.hurtCounter--; }
                        if (old.health > 0x880) old.health = 0x880;
                        if (old.health < 0x100) old.health = 0xff;
                    }
                    if (m.health >= 0x100) er_ap_update_heal_hurt(&m);
                    assert(m.health == old.health);
                    if (m.health >= 0x100) {
                        assert(m.healCounter == old.healCounter);
                        assert(m.hurtCounter == old.hurtCounter);
                    }
                }
            }
        }
    }
    assert(er_ap_bound_healing(0x880, 510) == 255); /* API accumulation cannot wrap */
    return 0;
}
