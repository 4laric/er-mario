/* Original traversal implementation. No Odyssey/Sonic mod code or assets are used. */
#include "er_addons.h"
#include "er_fludd.h"
#include "decomp/include/sm64.h"
#include "decomp/include/mario_animation_ids.h"
#include "decomp/include/audio_defines.h"
#include "play_sound.h"
#include "decomp/game/mario.h"
#include "decomp/game/mario_step.h"
#include "decomp/engine/surface_collision.h"
#include <math.h>
#include <string.h>
struct ERCappy er_cappy;
struct ERSonic er_sonic;
enum { SONIC_NONE, SONIC_CHARGE, SONIC_ROLL, SONIC_AIR_DASH };
static struct AnimInfo sonic_animation;
static int sonic_animation_saved;
static int sonic_new_attack;
s32 lava_boost_on_wall(struct MarioState *m);
static int safe(struct MarioState *m) {
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
static void cancel_cap(void) { er_cappy.phase = er_cappy.age = 0; }
static void cancel_sonic(void) {
    er_sonic.spin_charge = er_sonic.drop_charge = er_sonic.pending_drop = 0;
    er_sonic.rolling = er_sonic.rolling_unlock = er_sonic.dash_ticks = 0;
    er_sonic.feedback = er_sonic.feedback_ticks = er_sonic.feedback_request = 0;
    sonic_animation_saved = 0;
    er_sonic.attack_state = 0; sonic_new_attack = 0;
}
void er_cappy_configure(uint32_t enabled, uint32_t mask) {
    if (enabled == er_cappy.enabled && mask == er_cappy.mask) return;
    er_cappy.enabled = enabled; er_cappy.mask = mask;
    if (!enabled || !(mask & 1)) cancel_cap();
    /* Unlock receipts/replays never recharge a consumed bounce. */
}
void er_sonic_configure(uint32_t enabled, uint32_t mask) {
    if (enabled == er_sonic.enabled && mask == er_sonic.mask) return;
    er_sonic.enabled = enabled; er_sonic.mask = mask;
    if (!enabled) cancel_sonic();
    if (!(mask & 1)) er_sonic.spin_charge = 0;
    if (!(mask & er_sonic.rolling_unlock)) {
        er_sonic.rolling = er_sonic.rolling_unlock = 0;
        if (er_sonic.attack_state == 8) er_sonic.attack_state = 0;
    }
    if (!(mask & 2)) er_sonic.drop_charge = er_sonic.pending_drop = 0;
    if (!(mask & 4)) {
        er_sonic.dash_ticks = 0;
        if (er_sonic.attack_state == 4) er_sonic.attack_state = 0;
    }
}
void er_addons_reset(void) {
    cancel_cap(); cancel_sonic();
    er_cappy.previous = er_cappy.held = er_cappy.allowed = er_cappy.bounced = er_cappy.needs_release = 0;
    er_sonic.previous_dash = er_sonic.dash = er_sonic.held = er_sonic.allowed = er_sonic.air_used = er_sonic.needs_release = 0;
}
void er_addons_input(uint32_t allowed, uint32_t cap_held, uint32_t spin_held, uint32_t dash) {
    er_cappy.allowed = er_sonic.allowed = allowed;
    er_cappy.held = cap_held; er_sonic.held = spin_held; er_sonic.dash = dash;
    if (!allowed) { cancel_cap(); cancel_sonic();
        /* Neutralization cannot synthesize an edge on return to gameplay. */
        er_cappy.previous = cap_held; er_sonic.previous_dash = dash;
        er_cappy.needs_release = er_sonic.needs_release = 1;
    }
}
void er_addons_landed(struct MarioState *m) {
    er_cappy.bounced = 0; er_sonic.air_used = 0; er_sonic.dash_ticks = 0;
    if (er_sonic.enabled && er_sonic.allowed && (er_sonic.mask & 2)
        && er_sonic.held && er_sonic.drop_charge >= 6 && safe(m))
        er_sonic.pending_drop = er_sonic.drop_charge;
    er_sonic.drop_charge = 0;
}
/* Eight-unit swept samples with a twenty-unit sphere. Unlike a visual teleport,
   every cap segment checks walls, floors and ceilings in the live collision map. */
static int sweep_cap(const float *target) {
    float delta[3], start[3], length = 0;
    for (int i=0;i<3;i++) { start[i]=er_cappy.position[i]; delta[i]=target[i]-start[i]; length+=delta[i]*delta[i]; }
    int steps=(int)ceilf(sqrtf(length)/8.0f); if (steps<1) steps=1;
    for (int n=1;n<=steps;n++) {
        float p[3]; for (int i=0;i<3;i++) p[i]=start[i]+delta[i]*(float)n/steps;
        struct SM64SurfaceCollisionData *floor=0,*ceil=0;
        float fy=find_floor(p[0],p[1],p[2],&floor), cy=find_ceil(p[0],p[1],p[2],&ceil);
        if (f32_find_wall_collision(&p[0],&p[1],&p[2],0,20)
            || (floor && p[1]-20<fy) || (ceil && p[1]+20>cy)) return 0;
        memcpy(er_cappy.position,p,sizeof p);
    }
    return 1;
}
static void advance_cap(struct MarioState *m, int usable) {
    if (!er_cappy.allowed || !usable) er_cappy.needs_release=1;
    else if (!er_cappy.held) er_cappy.needs_release=0;
    int edge=er_cappy.held && !er_cappy.previous && !er_cappy.needs_release;
    er_cappy.previous=er_cappy.held;
    if (!er_cappy.enabled || !er_cappy.allowed || !usable || !(er_cappy.mask&1)) { cancel_cap(); return; }
    if (!er_cappy.phase && edge) {
        float yaw=(float)m->faceAngle[1]*(6.28318530718f/65536.0f);
        er_cappy.direction[0]=sinf(yaw); er_cappy.direction[1]=0; er_cappy.direction[2]=cosf(yaw);
        for (int i=0;i<3;i++) er_cappy.position[i]=m->pos[i];
        er_cappy.position[1]+=140; er_cappy.phase=1; er_cappy.age=0;
    }
    if (!er_cappy.phase) return;
    er_cappy.age++; er_cappy.spin_yaw+=0.45f;
    if (er_cappy.spin_yaw>6.28318530718f) er_cappy.spin_yaw-=6.28318530718f;
    if (er_cappy.age>=90) { cancel_cap(); return; }
    float distance=0; for (int i=0;i<3;i++) { float d=er_cappy.position[i]-m->pos[i]; distance+=d*d; }
    if (distance>800*800) er_cappy.phase=2; /* player can move while the cap hovers */
    if (er_cappy.phase==1) {
        if (er_cappy.age<=12) {
            float p[3]; for (int i=0;i<3;i++) p[i]=er_cappy.position[i]+er_cappy.direction[i]*33;
            if (!sweep_cap(p)) er_cappy.phase=2;
        } else if (!er_cappy.held || er_cappy.age>=60) er_cappy.phase=2;
    }
    if (er_cappy.phase==2) {
        float d[3], length=0; for(int i=0;i<3;i++) { d[i]=m->pos[i]-er_cappy.position[i]; if(i==1)d[i]+=140; length+=d[i]*d[i]; }
        length=sqrtf(length); if(length<=45) { cancel_cap(); return; }
        float p[3]; for(int i=0;i<3;i++)p[i]=er_cappy.position[i]+d[i]*45/length;
        /* An obstructed recall stays at the obstruction until the player comes
           around it or the timeout stows the cap. It never grants player motion. */
        sweep_cap(p);
    }
}
static int move(struct MarioState *m,int ground) {
    if (!er_sonic.feedback_request)
        set_mario_animation(m,ground ? MARIO_ANIM_RUNNING : MARIO_ANIM_GENERAL_FALL);
    if (ground) {
        if (!er_sonic.feedback_request || m->action != ACT_WALKING) set_mario_action(m,ACT_WALKING,0);
        int r=perform_ground_step(m);
        if(r==GROUND_STEP_LEFT_GROUND) { set_mario_action(m,ACT_FREEFALL,0); }
        else if(r==GROUND_STEP_HIT_WALL) { mario_set_forward_vel(m,0); cancel_sonic(); }
    } else {
        if (!er_sonic.feedback_request || m->action != ACT_FREEFALL) set_mario_action(m,ACT_FREEFALL,0);
        int r=perform_air_step(m,0);
        if(r==AIR_STEP_LANDED) { set_mario_action(m,ACT_FREEFALL_LAND,0); }
        else if(r==AIR_STEP_HIT_WALL) { mario_set_forward_vel(m,0); cancel_sonic(); }
        else if(r==AIR_STEP_HIT_LAVA_WALL) { lava_boost_on_wall(m); cancel_sonic(); }
    }
    /* A blocked step cancels attack activity above. Generation changes only
       when a new burst actually resolves a movement step, never during charge. */
    if (er_sonic.attack_state && sonic_new_attack) {
        if (++er_sonic.attack_generation == 0) ++er_sonic.attack_generation;
    }
    sonic_new_attack = 0;
    return 1;
}
int er_addons_step(struct MarioState *m) {
    er_sonic.feedback_request = SONIC_NONE;
    er_sonic.attack_state = 0; sonic_new_attack = 0;
    sonic_animation_saved = er_sonic.feedback == SONIC_CHARGE && m->marioObj
        && m->marioObj->header.gfx.animInfo.animID == MARIO_ANIM_FORWARD_SPINNING;
    if (sonic_animation_saved) sonic_animation = m->marioObj->header.gfx.animInfo;
    int usable=safe(m) && !(m->input&(INPUT_A_PRESSED|INPUT_B_PRESSED|INPUT_Z_PRESSED));
    int air=(m->action&ACT_FLAG_AIR)!=0;
    if (!er_sonic.allowed || !usable) er_sonic.needs_release=1;
    else if (!er_sonic.dash) er_sonic.needs_release=0;
    int dash_edge=er_sonic.dash && !er_sonic.previous_dash && !er_sonic.needs_release;
    er_sonic.previous_dash=er_sonic.dash;
    advance_cap(m,safe(m));
    if(!usable || !er_sonic.allowed || !er_sonic.enabled) cancel_sonic();
    if(!usable) return 0; /* normal jump/punch/crouch and forced actions win */
    if(er_sonic.enabled && er_sonic.allowed) {
        if(air && (er_sonic.mask&4) && dash_edge && !er_sonic.air_used) {
            er_sonic.air_used=1; er_sonic.dash_ticks=6; sonic_new_attack=1;
            if(m->intendedMag>0)m->faceAngle[1]=m->intendedYaw;
            if(m->vel[1]<4)m->vel[1]=4;
        }
        if(air && er_sonic.dash_ticks) {
            er_sonic.dash_ticks--; mario_set_forward_vel(m,80);
            er_sonic.feedback_request = SONIC_AIR_DASH; er_sonic.attack_state=4;
            return move(m,0);
        }
        if(!air && er_sonic.pending_drop && (er_sonic.mask&2)) {
            mario_set_forward_vel(m,50+2*er_sonic.pending_drop);
            er_sonic.pending_drop=0; er_sonic.rolling=14; er_sonic.rolling_unlock=2; sonic_new_attack=1;
            er_sonic.feedback_request = SONIC_ROLL; er_sonic.attack_state=8;
            return move(m,1);
        }
        if(air) {
            er_sonic.spin_charge=0;
            if(er_sonic.held && (er_sonic.mask&2)) { if(er_sonic.drop_charge<30)er_sonic.drop_charge++; }
            else er_sonic.drop_charge=0;
        } else {
            er_sonic.drop_charge=0;
            if(er_sonic.rolling) {
                er_sonic.rolling--; mario_set_forward_vel(m,m->forwardVel>2 ? m->forwardVel-2 : 0);
                er_sonic.feedback_request = SONIC_ROLL; er_sonic.attack_state=8;
                return move(m,1);
            }
            if(er_sonic.held && (er_sonic.mask&1)) {
                if(er_sonic.spin_charge<30)er_sonic.spin_charge++;
                if(m->intendedMag>0)m->faceAngle[1]=m->intendedYaw;
                er_sonic.feedback_request = SONIC_CHARGE;
                mario_set_forward_vel(m,0); return move(m,1);
            }
            if(er_sonic.spin_charge) {
                uint32_t c=er_sonic.spin_charge; er_sonic.spin_charge=0;
                if(c>=6) { mario_set_forward_vel(m,50+2*c); er_sonic.rolling=14; er_sonic.rolling_unlock=1; sonic_new_attack=1; }
            }
            if(er_sonic.rolling) {
                er_sonic.rolling--; mario_set_forward_vel(m,m->forwardVel>2 ? m->forwardVel-2 : 0);
                er_sonic.feedback_request = SONIC_ROLL; er_sonic.attack_state=8;
                return move(m,1);
            }
        }
    }
    if(air && er_cappy.enabled && er_cappy.allowed && (er_cappy.mask&2)
        && er_cappy.phase==1 && !er_cappy.bounced && m->vel[1]<=0) {
        float dx=m->pos[0]-er_cappy.position[0], dz=m->pos[2]-er_cappy.position[2];
        float dy=m->pos[1]-er_cappy.position[1];
        if(dx*dx+dz*dz<=90*90 && dy>=-30 && dy<=70) {
            er_cappy.bounced=1; er_cappy.phase=2; m->vel[1]=60;
            return move(m,0);
        }
    }
    return 0;
}
void er_addons_after(struct MarioState *m) {
    if(!safe(m)) { cancel_cap(); cancel_sonic(); }
    uint32_t visual = er_sonic.feedback_request;
    /* Drop charge does not own physics. Apply its pose after vanilla freefall
       selects an animation, without changing action or consuming another step. */
    if (!visual && er_sonic.enabled && er_sonic.allowed && (er_sonic.mask & 2)
        && er_sonic.held && er_sonic.drop_charge && !er_fludd.active
        && (m->action & ACT_FLAG_AIR) && safe(m)) visual = SONIC_CHARGE;
    if (visual != er_sonic.feedback) er_sonic.feedback_ticks = 0;
    er_sonic.feedback = visual;
    if (!visual) return;
    /* Native forward-spinning animation loops. Re-selecting the same ID leaves
       its frame intact; the renderer advances it normally once per Mario tick. */
    if (sonic_animation_saved && visual == SONIC_CHARGE)
        m->marioObj->header.gfx.animInfo = sonic_animation;
    set_mario_animation(m, MARIO_ANIM_FORWARD_SPINNING);
    if (!er_sonic.feedback_ticks
        || (visual != SONIC_AIR_DASH && er_sonic.feedback_ticks % 12 == 0))
        play_sound(visual == SONIC_CHARGE ? SOUND_ACTION_TWIRL : SOUND_ACTION_SPIN,
                   m->marioObj->header.gfx.cameraToObject);
    er_sonic.feedback_ticks++;
}

/* The production caller and tests share ownership and visual cancellation.
   Pausing a jet for a higher-priority move preserves charge and water. */
int er_addons_dispatch(struct MarioState *m) {
    if (er_addons_step(m)) { er_fludd.active=0; return 1; }
    return er_fludd_step(m);
}
