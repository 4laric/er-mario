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
            m.health = health; m.healCounter = 255; m.hurtCounter = 0;
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
        assert(m.health == maximum && m.healCounter == 0);
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
    assert(m.health == 0x480 && m.healCounter == 0); /* shrink discards excess healing */
    m.health = 0xff; er_ap_max_wedges = 8; er_ap_enforce_health(&m);
    assert(m.health == 0xff); /* reset never resurrects */
    return 0;
}
