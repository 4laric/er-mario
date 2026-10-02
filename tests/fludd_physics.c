#include <assert.h>
#include <string.h>
#include "er_fludd.h"
#include "decomp/include/sm64.h"
#include "decomp/game/mario.h"
/* Engine seams are stubs; the production jet dispatcher and policy are real. */
static int air_calls, ground_calls, collision;
static int sound_calls;
static u32 last_sound;
void play_sound(uint32_t sound,f32 *pos) { assert(pos); sound_calls++; last_sound=sound; }
u32 set_mario_action(struct MarioState *m,u32 action,u32 arg) { m->action=action; m->actionArg=arg; return 1; }
void mario_set_forward_vel(struct MarioState *m,f32 speed) { m->forwardVel=speed; m->vel[0]=speed; }
s16 set_mario_animation(struct MarioState *m,s32 id) { (void)m; (void)id; return 0; }
s32 lava_boost_on_wall(struct MarioState *m) { m->action=ACT_LAVA_BOOST; return 0; }
s32 perform_ground_step(struct MarioState *m) { ground_calls++; if (!collision) m->pos[0]+=m->vel[0]; return collision; }
s32 perform_air_step(struct MarioState *m,u32 arg) {
    (void)arg; air_calls++;
    if (!collision) { m->pos[0]+=m->vel[0]; m->pos[1]+=m->vel[1]; }
    m->vel[1]-=4; return collision;
}
int main(void) {
    struct MarioState m={0}; m.health=0x880; m.action=ACT_IDLE;
    struct MarioState before=m;
    er_fludd_configure(0,0,0); er_fludd_input(1,1,0,0);
    assert(!er_fludd_step(&m) && !memcmp(&m,&before,sizeof m));
    assert(!air_calls && !ground_calls && !sound_calls); /* OFF parity: native state untouched */
    er_fludd_configure(1,7,0); er_fludd_reset(); er_fludd_input(1,1,1,0);
    assert(er_fludd_step(&m)==1 && air_calls==1 && ground_calls==0);
    assert(m.pos[1]==5 && m.vel[1]==1 && er_fludd.water==299);
    assert(sound_calls==1 && last_sound==SOUND_ENV_WATERFALL2);
    er_fludd_after(&m); assert(m.peakHeight==m.floorHeight);
    /* Every refusal leaves movement to the existing native action dispatcher. */
    for (int reason=0;reason<6;reason++) {
        m=before;
        if (reason==0) m.health=0xff;
        if (reason==1) m.heldObj=(struct Object *)1;
        if (reason==2) m.hurtCounter=1;
        if (reason==3) m.action=ACT_WATER_IDLE;
        if (reason==4) m.action=ACT_HOLDING_BOWSER;
        if (reason==5) m.input=INPUT_A_PRESSED;
        before=m; unsigned water=er_fludd.water; int calls=air_calls+ground_calls, sounds=sound_calls;
        assert(!er_fludd_step(&m) && !memcmp(&m,&before,sizeof m));
        assert(er_fludd.water==water && air_calls+ground_calls==calls && !er_fludd.active);
        assert(sound_calls==sounds);
        memset(&before,0,sizeof before); before.health=0x880; before.action=ACT_IDLE;
    }
    m=before; er_fludd_reset(); er_fludd_input(1,1,2,0);
    int sounds=sound_calls;
    for (int t=0;t<29;t++) { assert(!er_fludd_step(&m)); assert(last_sound==SOUND_AIR_BLOW_WIND); }
    assert(sound_calls==sounds+29);
    assert(er_fludd_step(&m) && m.pos[1]==110 && m.vel[1]==106 && er_fludd.water==240);
    assert(last_sound==SOUND_ACTION_FLYING_FAST);
    er_fludd.water=0;
    sounds=sound_calls; assert(!er_fludd_step(&m) && sound_calls==sounds); /* Empty tank is silent. */
    m=before; er_fludd_reset(); er_fludd_input(1,1,4,0);
    for (int t=0;t<19;t++) { assert(!er_fludd_step(&m)); assert(last_sound==SOUND_AIR_BLOW_WIND); }
    assert(er_fludd_step(&m) && ground_calls==1 && m.pos[0]==15);
    assert(last_sound==SOUND_ENV_WATERFALL2);
    for (int t=0;t<10;t++) assert(er_fludd_step(&m));
    assert(m.forwardVel==100);
    collision=GROUND_STEP_HIT_WALL;
    float x=m.pos[0]; assert(er_fludd_step(&m));
    assert(m.pos[0]==x && !m.forwardVel && !er_fludd.active);
    collision=AIR_STEP_HIT_WALL; m=before; er_fludd_reset(); er_fludd_input(1,1,1,0);
    assert(er_fludd_step(&m) && m.pos[1]==0 && !er_fludd.active);
    collision=AIR_STEP_HIT_LAVA_WALL; m=before; er_fludd_reset(); er_fludd_input(1,1,1,0);
    assert(er_fludd_step(&m) && m.action==ACT_LAVA_BOOST);
    er_fludd_input(0,1,0,0); m=before;
    sounds=sound_calls;
    assert(!er_fludd_step(&m) && !er_fludd.active); /* menu/follow suspension */
    assert(sound_calls==sounds);
    er_fludd_reset(); er_fludd_input(1,0,1,0); m=before;
    assert(!er_fludd_step(&m) && sound_calls==sounds); /* Released trigger is silent. */
    /* Squirt leaves native action, animation, velocity and collision dispatch
       untouched, both on the ground and in flight. Its active state feeds the
       rendering/hit seam, without granting the other nozzles' fall protection. */
    er_fludd_configure(1,15,0); er_fludd_reset(); er_fludd_input(1,1,8,0);
    for (int airborne=0;airborne<2;airborne++) {
        m=before; if (airborne) { m.action=ACT_FREEFALL; m.pos[1]=100; m.vel[1]=-20; }
        struct MarioState snapshot=m; int calls=air_calls+ground_calls;
        sounds=sound_calls;
        assert(!er_fludd_step(&m) && !memcmp(&m,&snapshot,sizeof m));
        assert(air_calls+ground_calls==calls && !er_fludd.protected_fall);
        assert(er_fludd.active==8 && er_fludd.water==299u-(uint32_t)airborne);
        assert(sound_calls==sounds+1 && last_sound==SOUND_ENV_WATERFALL2);
    }
    /* Walking recharges without consuming a movement step or changing Mario.
       Airborne, unsafe and suspended states still cannot manufacture water. */
    m=before; m.action=ACT_WALKING; m.forwardVel=25;
    er_fludd.water=290; er_fludd_input(1,0,8,0);
    struct MarioState snapshot=m; int calls=air_calls+ground_calls;
    assert(!er_fludd_step(&m) && er_fludd.water==291 && !memcmp(&m,&snapshot,sizeof m));
    assert(air_calls+ground_calls==calls);
    m.action=ACT_FREEFALL; m.pos[1]=100;
    assert(!er_fludd_step(&m) && er_fludd.water==291);
    m=before; m.hurtCounter=1;
    assert(!er_fludd_step(&m) && er_fludd.water==291);
    m=before; er_fludd_input(0,0,8,0);
    assert(!er_fludd_step(&m) && er_fludd.water==291);
    er_fludd_input(1,1,8,0);
    assert(!er_fludd_step(&m) && er_fludd.water==290); /* Active grounded spray spends fuel. */
    return 0;
}
