#include <assert.h>
#include <string.h>
#include <math.h>
#include "er_skate.h"
#include "libsm64.h"
#include "er_fludd.h"
#include "decomp/include/sm64.h"
#include "decomp/game/mario.h"
/* Production momentum/controller code, with explicit native collision seams. */
struct ERFludd er_fludd;
static struct SM64SurfaceCollisionData floor_surface;
static struct Object object;
static int ground_calls, air_calls, collision, fall_checks, injury, sounds, cancelled;
u32 set_mario_action(struct MarioState *m,u32 a,u32 arg) {m->action=a;m->actionArg=arg;return 1;}
void mario_set_forward_vel(struct MarioState *m,f32 speed) {
    float yaw=m->faceAngle[1]*3.14159265358979323846f/32768;
    m->forwardVel=speed;m->vel[0]=sinf(yaw)*speed;m->vel[2]=cosf(yaw)*speed;
}
s16 set_mario_animation(struct MarioState *m,s32 id) {(void)m;(void)id;return 0;}
void play_sound(uint32_t id,f32 *pos) {(void)id;assert(pos);sounds++;}
void er_addons_input(uint32_t allowed,uint32_t c,uint32_t s,uint32_t d) {
    assert(!allowed&&!c&&!s&&!d);cancelled++;
}
void er_fludd_input(uint32_t allowed,uint32_t h,uint32_t s,uint32_t c) {
    assert(!allowed&&!h&&!s&&!c);
}
s32 perform_ground_step(struct MarioState *m) {
    ground_calls++;if(!collision){m->pos[0]+=m->vel[0];m->pos[2]+=m->vel[2];}return collision ? collision : GROUND_STEP_NONE;
}
s32 perform_air_step(struct MarioState *m,u32 arg) {
    (void)arg;air_calls++;
    if(!collision){m->pos[0]+=m->vel[0];m->pos[2]+=m->vel[2];m->pos[1]+=m->vel[1];}
    m->vel[1]-=4;if(collision==AIR_STEP_LANDED)m->pos[1]=m->floorHeight;return collision;
}
s32 check_fall_damage_or_get_stuck(struct MarioState *m,u32 a) {
    fall_checks++;if(injury){m->hurtCounter=8;m->action=a;}return injury;
}
s32 lava_boost_on_wall(struct MarioState *m) {m->action=ACT_LAVA_BOOST;return 0;}
static struct MarioState mario(void) {
    struct MarioState m={0};m.health=0x880;m.action=ACT_IDLE;m.floor=&floor_surface;m.marioObj=&object;
    floor_surface.normal.x=floor_surface.normal.z=0;floor_surface.normal.y=1;return m;
}
static void mount(struct MarioState *m) {
    er_skate_configure(1);er_skate_reset();
    er_skate_input(1,0,0,0,0,0);assert(!er_skate_step(m));
    er_skate_input(1,1,0,0,0,0);assert(er_skate_step(m));assert(er_skate.mounted);
    er_skate_input(1,0,0,0,0,0);
}
int main(void) {
    struct MarioState m=mario(),before=m;
    er_skate_configure(0);er_skate_input(1,1,1,1,1,1);
    assert(!er_skate_step(&m)&&!memcmp(&m,&before,sizeof m));
    assert(!ground_calls&&!air_calls&&!sounds&&!cancelled);
    mount(&m);er_fludd.active=er_fludd.protected_fall=1;er_fludd.water=123;
    er_skate_input(1,0,1,0,0,0);
    int g=ground_calls;
    for(int i=0;i<100;i++)assert(er_skate_step(&m));
    assert(ground_calls-g==100);assert(!air_calls);assert(er_skate.speed==90);assert(!er_fludd.active);
    assert(!er_fludd.protected_fall&&er_fludd.water==123);
    float fast=er_skate.speed;
    er_skate_configure(1);assert(er_skate.mounted&&er_skate.speed==fast); /* replay */
    er_skate_input(1,0,0,0,0,0);assert(er_skate_step(&m));
    assert(er_skate.speed<fast&&er_skate.speed>85); /* coast, not walk acceleration */
    er_skate.speed=20;floor_surface.normal.z=0.5f;
    er_skate_step(&m);assert(er_skate.speed>20); /* downhill */
    er_skate.speed=20;floor_surface.normal.z=-0.5f;
    er_skate_step(&m);assert(er_skate.speed<20); /* uphill */
    floor_surface.normal.z=0;
    er_skate_input(1,0,1,1,0,0);
    for(int i=0;i<40;i++)er_skate_step(&m);
    assert(er_skate.speed==0); /* brake wins over push, never reverse */
    er_skate_input(1,0,1,0,0,1);er_skate_step(&m);
    assert(m.faceAngle[1]>0&&er_skate.lean>0&&m.vel[0]>0);
    er_skate_input(1,0,0,0,0,NAN);assert(er_skate.steer==0);
    collision=GROUND_STEP_HIT_WALL;float z=m.pos[2];er_skate_step(&m);
    assert(er_skate.speed==0&&m.pos[2]==z);collision=0;
    er_skate_input(1,0,1,0,1,0);g=ground_calls;int a=air_calls;
    assert(er_skate_step(&m)&&ground_calls==g&&air_calls==a+1);
    assert(er_skate.airborne&&er_skate.trick==1&&m.vel[1]==40);
    float vy=m.vel[1];er_skate_step(&m);assert(m.vel[1]==vy-4); /* held X doesn't relaunch */
    er_skate_input(1,1,0,0,0,0);er_skate_step(&m);assert(er_skate.mounted); /* no air dismount */
    collision=AIR_STEP_LANDED;er_skate_step(&m);
    assert(fall_checks==1&&er_skate.mounted&&!er_skate.airborne&&!er_skate.trick);
    collision=0;er_skate_input(1,0,0,0,0,0);er_skate_step(&m);
    er_skate_input(1,1,0,0,0,0);assert(!er_skate_step(&m)&&!er_skate.mounted);
    mount(&m);er_skate_input(0,0,0,0,0,0);assert(!er_skate.mounted);
    er_skate_input(1,1,0,0,1,0);assert(!er_skate_step(&m));
    er_skate_input(1,0,0,0,0,0);er_skate_step(&m);
    er_skate_input(1,1,0,0,0,0);assert(er_skate_step(&m)&&er_skate.mounted);
    m.action=ACT_GROUND_POUND;assert(!er_skate_step(&m)&&!er_skate.mounted);
    m=mario();mount(&m);m.health=0xff;assert(!er_skate_step(&m)&&!er_skate.mounted);
    m=mario();mount(&m);er_skate_input(1,0,0,0,1,0);er_skate_step(&m);
    injury=1;collision=AIR_STEP_LANDED;er_skate_step(&m);
    assert(fall_checks==2&&!er_skate.mounted&&er_skate.bail_ticks==15);
    collision=injury=0;m=mario();mount(&m);er_skate_reset();assert(!er_skate.mounted);
    er_skate_input(1,1,0,0,0,0);assert(!er_skate_step(&m)); /* lifecycle held latch */
    m=mario();mount(&m);m.heldObj=&object;assert(!er_skate_step(&m)&&!er_skate.mounted);
    m=mario();mount(&m);m.floor=0;assert(!er_skate_step(&m)&&!er_skate.mounted);
    m=mario();mount(&m);er_skate_configure(0);before=m;
    assert(!er_skate_step(&m)&&!memcmp(&m,&before,sizeof m));
    return 0;
}
