#include <assert.h>
#include <string.h>
#include "er_addons.h"
#include "libsm64.h"
#include "er_fludd.h"
#include "decomp/include/sm64.h"
#include "decomp/game/mario.h"
/* Production addon/FLUDD dispatchers linked against explicit engine seams.
   These tests verify ownership, resources, unlock gates and swept queries. */
static int air_calls, ground_calls, collision, wall, queries;
static struct SM64SurfaceCollisionData surface;
static float ceiling=10000, floor_y=0;
u32 set_mario_action(struct MarioState *m,u32 action,u32 arg) {m->action=action;m->actionArg=arg;return 1;}
void mario_set_forward_vel(struct MarioState *m,f32 speed) {m->forwardVel=speed;m->vel[2]=speed;}
s16 set_mario_animation(struct MarioState *m,s32 id) {(void)m;(void)id;return 0;}
s32 lava_boost_on_wall(struct MarioState *m) {m->action=ACT_LAVA_BOOST;return 0;}
s32 perform_ground_step(struct MarioState *m) {ground_calls++;if(!collision)m->pos[2]+=m->vel[2];return collision ? collision : GROUND_STEP_NONE;}
s32 perform_air_step(struct MarioState *m,u32 arg) {
    (void)arg;air_calls++;if(!collision){m->pos[2]+=m->vel[2];m->pos[1]+=m->vel[1];}
    m->vel[1]-=4;if(collision==AIR_STEP_LANDED)er_addons_landed(m);return collision;
}
s32 f32_find_wall_collision(f32 *x,f32 *y,f32 *z,f32 offset,f32 radius) {
    (void)x;(void)y;(void)offset;assert(radius==20);queries++;return wall && *z>=wall;
}
f32 find_floor(f32 x,f32 y,f32 z,struct SM64SurfaceCollisionData **out) {(void)x;(void)y;(void)z;*out=&surface;return floor_y;}
f32 find_ceil(f32 x,f32 y,f32 z,struct SM64SurfaceCollisionData **out) {(void)x;(void)y;(void)z;*out=&surface;return ceiling;}
static struct MarioState mario(void) {struct MarioState m={0};m.health=0x880;m.action=ACT_IDLE;m.floor=&surface;return m;}
static void reset(void) {
    er_cappy_configure(0,0);er_sonic_configure(0,0);er_addons_reset();
    er_fludd_configure(0,0,0);er_fludd_reset();air_calls=ground_calls=collision=wall=queries=0;ceiling=10000;floor_y=0;
}
static int step(struct MarioState *m) {
    int claim=er_addons_step(m)||er_fludd_step(m);
    er_addons_after(m);er_fludd_after(m);return claim;
}
int main(void) {
    reset();struct MarioState m=mario(),before=m;
    er_addons_input(1,1,1,1);
    assert(!step(&m)&&!memcmp(&m,&before,sizeof m));assert(!air_calls&&!ground_calls&&!queries);
    /* Throw requires its own bit; bounce-only cannot throw. */
    for(unsigned mask=0;mask<=3;mask++) {
        reset();m=mario();er_cappy_configure(1,mask);er_addons_input(1,1,0,0);
        assert(!step(&m));assert((er_cappy.phase!=0)==((mask&1)!=0));assert(!air_calls&&!ground_calls);
        if(mask&1){assert(er_cappy.position[2]==33);assert(queries>=5);}
    }
    reset();m=mario();er_cappy_configure(1,3);er_addons_input(1,1,0,0);
    for(int i=0;i<12;i++)assert(!step(&m));
    assert(er_cappy.position[2]==396 && er_cappy.phase==1);
    float z=er_cappy.position[2];for(int i=0;i<4;i++)assert(!step(&m));assert(er_cappy.position[2]==z);
    er_cappy_configure(1,3);assert(er_cappy.phase==1&&er_cappy.age==16); /* replay */
    er_addons_input(1,0,0,0);assert(!step(&m));assert(er_cappy.phase==2&&er_cappy.position[2]<z);
    for(int i=0;i<90;i++) { step(&m); }
    assert(er_cappy.phase==0);
    /* Swept wall/floor/ceiling obstruction: no projectile tunnel. */
    for(int obstacle=0;obstacle<3;obstacle++) {
        reset();m=mario();er_cappy_configure(1,3);er_addons_input(1,1,0,0);
        if(obstacle==0)wall=15;
        if(obstacle==1)floor_y=130;
        if(obstacle==2)ceiling=150;
        assert(!step(&m));assert(er_cappy.position[2]<15);assert(!air_calls&&!ground_calls);
    }
    /* Bounce consumes once. Forced freefall/receipts/FLUDD do not restore it. */
    reset();m=mario();er_cappy_configure(1,3);er_addons_input(1,1,0,0);step(&m);
    m.action=ACT_FREEFALL;m.pos[1]=er_cappy.position[1];m.pos[2]=er_cappy.position[2];m.vel[1]=-5;
    assert(step(&m)&&air_calls==1&&m.vel[1]==56&&er_cappy.bounced);
    er_cappy_configure(1,1);er_cappy_configure(1,3);assert(er_cappy.bounced);
    er_cappy.phase=1;m.vel[1]=-5;m.pos[1]=er_cappy.position[1];m.pos[2]=er_cappy.position[2];
    assert(!step(&m)&&air_calls==1); /* no second bounce */
    er_addons_landed(&m);assert(!er_cappy.bounced);
    /* Relocking bounce leaves cap flight but denies the extra jump. */
    er_cappy_configure(1,1);m.pos[1]=er_cappy.position[1];m.pos[2]=er_cappy.position[2];assert(!step(&m));
    er_cappy_configure(1,0);assert(!er_cappy.phase);
    /* Sonic locks are independent across every accepted mask. */
    for(unsigned mask=0;mask<=7;mask++) {
        reset();m=mario();er_sonic_configure(1,mask);er_addons_input(1,0,1,0);
        for(int i=0;i<6;i++)step(&m);
        assert((er_sonic.spin_charge==6)==((mask&1)!=0));
        er_addons_input(1,0,0,0);step(&m);assert((m.forwardVel>0)==((mask&1)!=0));
        m.action=ACT_FREEFALL;er_addons_input(1,0,1,0);
        for(int i=0;i<6;i++)step(&m);
        assert((er_sonic.drop_charge==6)==((mask&2)!=0));
        er_addons_landed(&m);assert((er_sonic.pending_drop==6)==((mask&2)!=0));
        er_addons_input(1,0,0,1);step(&m);assert((er_sonic.air_used!=0)==((mask&4)!=0));
    }
    reset();m=mario();er_sonic_configure(1,7);er_addons_input(1,0,1,0);
    for(int i=0;i<30;i++) { assert(step(&m)); }
    assert(m.pos[2]==0&&er_sonic.spin_charge==30);
    er_sonic_configure(1,7);assert(er_sonic.spin_charge==30);
    er_addons_input(1,0,0,0);assert(step(&m)&&m.forwardVel==108);
    collision=GROUND_STEP_HIT_WALL;assert(step(&m)&&!m.forwardVel&&!er_sonic.rolling);
    er_sonic.rolling=10;er_sonic.rolling_unlock=1;er_sonic_configure(1,6);assert(!er_sonic.rolling);
    er_sonic.rolling=10;er_sonic.rolling_unlock=2;er_sonic_configure(1,7);assert(er_sonic.rolling==10);
    er_sonic_configure(1,1);assert(!er_sonic.rolling); /* relock the actual originating family */
    /* Real landing carries charged Drop Dash to the ground without needing Spin. */
    reset();m=mario();m.action=ACT_FREEFALL;er_sonic_configure(1,2);er_addons_input(1,0,1,0);
    for(int i=0;i<12;i++)assert(!step(&m));
    collision=AIR_STEP_LANDED;perform_air_step(&m,0);m.action=ACT_FREEFALL_LAND;collision=0;
    assert(step(&m)&&m.forwardVel==74);assert(step(&m)&&m.forwardVel==72);
    /* Air Dash consumes once and uses the native collision step exactly once. */
    reset();m=mario();m.action=ACT_FREEFALL;er_sonic_configure(1,4);er_addons_input(1,0,0,1);
    assert(step(&m)&&air_calls==1&&m.forwardVel==80&&er_sonic.air_used);
    for(int i=0;i<5;i++) { assert(step(&m)); }
    assert(!step(&m));
    er_addons_input(1,0,0,0);step(&m);er_addons_input(1,0,0,1);assert(!step(&m));
    er_sonic_configure(1,0);er_sonic_configure(1,4);assert(er_sonic.air_used);
    er_addons_landed(&m);er_addons_input(1,0,0,0);step(&m);er_addons_input(1,0,0,1);assert(step(&m));
    /* Priority: Sonic movement prevents a second FLUDD air step or fuel consumption. */
    reset();m=mario();m.action=ACT_FREEFALL;er_sonic_configure(1,4);er_fludd_configure(1,7,0);er_fludd_reset();
    er_addons_input(1,0,0,1);er_fludd_input(1,1,1,0);assert(step(&m)&&air_calls==1&&er_fludd.water==60);
    /* Normal buttons and unsafe conditions cancel traversal without granting resources. */
    for(int reason=0;reason<9;reason++) {
        reset();m=mario();er_cappy_configure(1,3);er_sonic_configure(1,7);er_addons_input(1,1,1,1);
        if(reason==0)m.health=0xff;
        if(reason==1)m.heldObj=(struct Object*)1;
        if(reason==2)m.hurtCounter=1;
        if(reason==3)m.invincTimer=1;
        if(reason==4)m.action=ACT_WATER_IDLE;
        if(reason==5)m.action=ACT_HOLDING_BOWSER;
        if(reason==6)m.input=INPUT_A_PRESSED;
        if(reason==7)m.input=INPUT_B_PRESSED;
        if(reason==8)m.input=INPUT_Z_PRESSED;
        before=m;assert(!step(&m)&&!memcmp(&m,&before,sizeof m));assert(!air_calls&&!ground_calls);
        assert(!er_sonic.spin_charge&&!er_sonic.drop_charge&&!er_sonic.dash_ticks);
    }
    reset();m=mario();er_cappy_configure(1,3);er_sonic_configure(1,7);er_addons_input(1,1,1,0);step(&m);
    er_addons_input(0,1,1,1);assert(!er_cappy.phase&&!er_sonic.spin_charge);assert(!step(&m));
    er_addons_input(1,1,1,1);step(&m);assert(!er_cappy.phase); /* held menu key isn't a fresh throw */
    reset();m=mario();m.action=ACT_FREEFALL;er_cappy_configure(1,3);er_sonic_configure(1,4);
    er_addons_input(0,0,0,0);step(&m);er_addons_input(1,1,0,1);assert(!step(&m));
    assert(!er_cappy.phase&&!er_sonic.air_used); /* neutralized pause cannot synthesize edges */
    er_addons_input(1,0,0,0);step(&m);er_addons_input(1,1,0,1);assert(step(&m));
    assert(er_cappy.phase&&er_sonic.air_used);
    return 0;
}
