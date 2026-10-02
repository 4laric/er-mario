#include <assert.h>
#include "ap_capabilities.h"

int main(void) {
    const uint32_t actions[] = {
        ACT_DOUBLE_JUMP, ACT_TRIPLE_JUMP, ACT_BACKFLIP, ACT_SIDE_FLIP,
        ACT_FLYING_TRIPLE_JUMP, ACT_SPECIAL_TRIPLE_JUMP, ACT_LONG_JUMP,
        ACT_WALL_KICK_AIR, ACT_DIVE, ACT_DIVE_SLIDE, ACT_SLIDE_KICK,
        ACT_SLIDE_KICK_SLIDE, ACT_GROUND_POUND, ACT_GROUND_POUND_LAND,
        ACT_PICKING_UP, ACT_DIVE_PICKING_UP, ACT_PICKING_UP_BOWSER,
        ACT_HOLDING_BOWSER, ACT_RELEASING_BOWSER
    };
    const uint32_t expected_bits[] = {
        1, 128, 256, 512, 128, 128, 2, 4, 8, 8, 8, 8,
        16, 16, 32, 32, 64, 64, 64
    };
    const uint32_t free_actions[] = {
        ACT_JUMP, ACT_FREEFALL, ACT_PUNCHING, ACT_JUMP_KICK,
        ACT_CROUCHING, ACT_WALKING, ACT_DEATH_EXIT, ACT_STAR_DANCE_EXIT,
        ACT_CLIMBING_POLE, ACT_ER_LADDER
    };
    for (uint32_t unlocked = 0; unlocked < 1024; ++unlocked) {
        sm64_er_ap_set_capabilities(1023, unlocked);
        for (unsigned i = 0; i < sizeof(actions)/sizeof(actions[0]); ++i) {
            uint32_t a = actions[i], cap = expected_bits[i];
            assert(er_ap_action_capability(a) == cap);
            if (a == ACT_DIVE_PICKING_UP) cap |= 8;
            int allowed = (cap & ~unlocked) == 0;
            assert((er_ap_filter_action(a, ACT_JUMP) == a) == allowed);
            assert((er_ap_filter_action(a, ACT_IDLE) == a) == allowed);
            if (!allowed) {
                assert(er_ap_filter_action(a, ACT_JUMP) == ((cap & (1 | 2 | 128 | 256 | 512)) ? ACT_JUMP : ACT_FREEFALL));
                assert(er_ap_filter_action(a, ACT_IDLE) == ((cap & (1 | 2 | 128 | 256 | 512)) ? ACT_JUMP : ((cap & 4) ? ACT_FREEFALL : ACT_IDLE)));
                /* Exercise the production entry gate with its own fallback as
                 * the current action: no retry loop or fresh jump impulse. */
                uint32_t fallback = er_ap_filter_action(a, ACT_JUMP), filtered;
                assert(!er_ap_prepare_action(a, fallback, &filtered));
                assert(filtered == fallback);
            } else {
                uint32_t filtered;
                assert(er_ap_prepare_action(a, a, &filtered));
                assert(filtered == a);
            }
        }
        for (unsigned i = 0; i < sizeof(free_actions)/sizeof(free_actions[0]); ++i)
            assert(er_ap_filter_action(free_actions[i], ACT_IDLE) == free_actions[i]);
        /* Slopes remain traversable but attacks are disabled without Dive. */
        assert(er_ap_filter_action(ACT_BUTT_SLIDE, ACT_IDLE) == ACT_BUTT_SLIDE);
        assert(er_ap_allows(er_ap_attack_capability(ACT_BUTT_SLIDE)) == !!(unlocked & 8));
        assert(er_ap_allows(er_ap_attack_capability(ACT_GROUND_POUND)) == !!(unlocked & 16));
        assert(er_ap_allows(er_ap_attack_capability(ACT_JUMP)));
    }
    sm64_er_ap_set_capabilities(2, 0);
    assert(er_ap_filter_action(ACT_DIVE, ACT_JUMP) == ACT_DIVE);
    assert(er_ap_filter_action(ACT_LONG_JUMP, ACT_IDLE) == ACT_JUMP);
    sm64_er_ap_set_capabilities(0, 0);
    for (unsigned i = 0; i < sizeof(actions)/sizeof(actions[0]); ++i)
        assert(er_ap_filter_action(actions[i], ACT_JUMP) == actions[i]);
    return 0;
}
