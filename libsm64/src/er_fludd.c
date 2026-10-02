/* Original native jet implementation; no CoopDX Lua or DynOS assets are embedded. */
#include "er_fludd.h"
#include "decomp/include/sm64.h"
#include "decomp/include/mario_animation_ids.h"
#include "decomp/game/mario.h"
#include "decomp/game/mario_step.h"
#include "decomp/game/mario_actions_airborne.h"
#include "decomp/engine/math_util.h"
s32 lava_boost_on_wall(struct MarioState *m);
static int safe_action(struct MarioState *m) {
    if (m->health < 0x100 || m->heldObj || m->hurtCounter || m->invincTimer > 0) return 0;
    switch (m->action) {
        case ACT_IDLE: case ACT_WALKING: case ACT_BRAKING: case ACT_DECELERATING:
        case ACT_JUMP: case ACT_DOUBLE_JUMP: case ACT_TRIPLE_JUMP: case ACT_BACKFLIP:
        case ACT_SIDE_FLIP: case ACT_LONG_JUMP: case ACT_FREEFALL: case ACT_JUMP_LAND:
        case ACT_FREEFALL_LAND: case ACT_DOUBLE_JUMP_LAND: case ACT_TRIPLE_JUMP_LAND:
            return 1;
        default: return 0;
    }
}
int er_fludd_step(struct MarioState *m) {
    int grounded = !(m->action & ACT_FLAG_AIR) && m->pos[1] <= m->floorHeight + 5;
    int safe = safe_action(m) && !(m->input & (INPUT_A_PRESSED | INPUT_B_PRESSED | INPUT_Z_PRESSED));
    int operation = er_fludd_policy(safe, grounded && m->forwardVel > -1 && m->forwardVel < 1);
    if (operation <= 1) return 0; /* native actions keep running during a charge */
    if (operation == 2) {
        set_mario_action(m, ACT_FREEFALL, 0); m->vel[1] = 110.0f; grounded = 0;
    } else if (operation == 3) {
        /* Never lift position through geometry: air-step checks the entire move. */
        set_mario_action(m, ACT_FREEFALL, 0); grounded = 0;
        if (m->vel[1] < -10) m->vel[1] = -10;
        if (m->vel[1] < 10) m->vel[1] += 5;
    }
    if (m->intendedMag > 0) { int turn = (int16_t)(m->intendedYaw - m->faceAngle[1]);
        m->faceAngle[1] += turn > 0x800 ? 0x800 : (turn < -0x800 ? -0x800 : turn); }
    if (operation == 4) mario_set_forward_vel(m, m->forwardVel + 15 > 100 ? 100 : m->forwardVel + 15);
    else mario_set_forward_vel(m, m->forwardVel > 55 ? m->forwardVel - 1 : m->forwardVel);
    er_fludd.protected_fall = 1;
    set_mario_animation(m, grounded ? MARIO_ANIM_RUNNING : MARIO_ANIM_GENERAL_FALL);
    if (grounded) {
        set_mario_action(m, ACT_WALKING, 0);
        int result = perform_ground_step(m);
        if (result == GROUND_STEP_LEFT_GROUND) set_mario_action(m, ACT_FREEFALL, 0);
        else if (result == GROUND_STEP_HIT_WALL) { mario_set_forward_vel(m, 0); er_fludd.active = er_fludd.charge = 0; }
    } else {
        set_mario_action(m, ACT_FREEFALL, 0);
        int result = perform_air_step(m, 0);
        if (result == AIR_STEP_LANDED) { set_mario_action(m, ACT_FREEFALL_LAND, 0); er_fludd.active = er_fludd.charge = 0; }
        else if (result == AIR_STEP_HIT_WALL) { mario_set_forward_vel(m, 0); er_fludd.active = er_fludd.charge = 0; }
        else if (result == AIR_STEP_HIT_LAVA_WALL) { lava_boost_on_wall(m); er_fludd.active = er_fludd.charge = 0; }
    }
    return 1;
}

void er_fludd_after(struct MarioState *m) {
    /* Keep native landing checks safe after our own jet-assisted flight. */
    if (er_fludd.protected_fall) {
        if (!safe_action(m) || !(m->action & ACT_FLAG_AIR)) er_fludd.protected_fall = 0;
        else m->peakHeight = m->floorHeight;
    }
}
