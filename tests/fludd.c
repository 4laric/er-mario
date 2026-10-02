#include <assert.h>
#include "er_fludd.h"
int main(void) {
    /* Disabled baseline never consumes/charges or handles native actions. */
    er_fludd_configure(0,0,0); er_fludd_reset(); er_fludd_input(1,1,0,1);
    for (int i=0;i<200;i++) assert(er_fludd_policy(1,1)==0);
    assert(!er_fludd.active && !er_fludd.water && !er_fludd.selected);
    er_fludd_configure(1,7,0); er_fludd_reset();
    assert(er_fludd.water==60 && er_fludd.selected==1);
    er_fludd_input(1,1,1,0);
    for (int i=0;i<60;i++) assert(er_fludd_policy(1,0)==3);
    assert(er_fludd.water==0 && er_fludd_policy(1,0)==0);
    er_fludd_input(1,0,0,0);
    for (int i=0;i<60;i++) assert(er_fludd_policy(1,1)==0);
    assert(er_fludd.water==60); /* one unit per grounded idle tick */
    for (int i=0;i<10;i++) er_fludd_policy(1,1);
    assert(er_fludd.water==60);
    er_fludd_input(1,1,2,0);
    for (int i=0;i<29;i++) assert(er_fludd_policy(1,1)==1);
    assert(er_fludd.water==60 && er_fludd.charge==29);
    assert(er_fludd_policy(1,1)==2 && er_fludd.water==0);
    assert(er_fludd_policy(1,0)==0);
    er_fludd_reset(); er_fludd_input(1,1,4,0);
    for (int i=0;i<19;i++) assert(er_fludd_policy(1,1)==1);
    assert(er_fludd_policy(1,1)==4 && er_fludd.water==59);
    /* Replay and capacity upgrades preserve fuel, selection and ongoing charge. */
    er_fludd_configure(1,7,0);
    assert(er_fludd.water==59 && er_fludd.selected==4 && er_fludd.charge==20);
    er_fludd_configure(1,7,3);
    assert(er_fludd.water==59 && er_fludd.capacity==120 && er_fludd.charge==20);
    assert(er_fludd_policy(1,1)==4 && er_fludd.water==58);
    /* Unsafe states (damage/carry/swim/death) and suspension never refill. */
    assert(er_fludd_policy(0,1)==0 && er_fludd.water==58 && !er_fludd.charge && !er_fludd.active);
    er_fludd_input(0,1,0,0);
    assert(er_fludd_policy(1,1)==0 && er_fludd.water==58);
    /* Release cancels a charge; a nozzle change also cancels it. */
    er_fludd_reset(); er_fludd_input(1,1,2,0); er_fludd_policy(1,1);
    er_fludd_input(1,0,0,0); er_fludd_policy(1,0);
    assert(!er_fludd.charge && er_fludd.water==120);
    er_fludd_input(1,1,2,0); er_fludd_policy(1,0);
    er_fludd_input(1,1,1,0); assert(er_fludd_policy(1,0)==3 && !er_fludd.charge);
    er_fludd_configure(1,1,1); /* shrink clamps water, doesn't refill */
    assert(er_fludd.water==80 && er_fludd.capacity==80);
    er_fludd_input(1,0,0,1); er_fludd_policy(1,1); assert(er_fludd.selected==1);
    er_fludd_configure(1,7,1); er_fludd_input(1,0,0,0); er_fludd_policy(1,1);
    er_fludd_input(1,0,0,1); er_fludd_policy(1,1); assert(er_fludd.selected==2);
    er_fludd_policy(1,1); assert(er_fludd.selected==2); /* held cycle is one edge */
    er_fludd_configure(0,0,0);
    assert(er_fludd.water==0 && !er_fludd.active && !er_fludd.charge && !er_fludd.selected);
    /* Every supported tank level is bounded through depletion and recharge. */
    for (unsigned level=0;level<=3;level++) {
        er_fludd_configure(1,1,level); er_fludd_reset(); er_fludd_input(1,1,0,0);
        for (unsigned t=0;t<200;t++) { er_fludd_policy(1,0); assert(er_fludd.water<=er_fludd.capacity); }
        assert(er_fludd.water==0);
        er_fludd_input(1,0,0,0);
        for (unsigned t=0;t<200;t++) { er_fludd_policy(1,1); assert(er_fludd.water<=er_fludd.capacity); }
        assert(er_fludd.water==60+20*level);
    }
    /* Squirt coexists with the three legacy nozzles and has no charge delay. */
    er_fludd_configure(1,15,0); er_fludd_reset(); er_fludd_input(1,1,8,0);
    for (int t=0;t<60;t++) {
        assert(er_fludd_policy(1,0)==5);
        assert(er_fludd.selected==8 && er_fludd.active==8 && !er_fludd.charge);
    }
    assert(!er_fludd.water && er_fludd_policy(1,0)==0 && !er_fludd.active);
    er_fludd_reset(); er_fludd_input(1,0,8,0); er_fludd_policy(1,1);
    er_fludd_input(1,0,0,1); er_fludd_policy(1,1); assert(er_fludd.selected==1);
    er_fludd_policy(1,1); assert(er_fludd.selected==1);
    for (unsigned expected=2;expected<=8;expected<<=1) {
        er_fludd_input(1,0,0,0); er_fludd_policy(1,1);
        er_fludd_input(1,0,0,1); er_fludd_policy(1,1); assert(er_fludd.selected==expected);
    }
    /* Legacy AP masks cannot select or cycle to Squirt. Revocation relocks it. */
    er_fludd_configure(1,7,0); assert(er_fludd.selected==1);
    er_fludd_input(1,1,8,0); assert(er_fludd_policy(1,0)==3 && er_fludd.selected==1);
    er_fludd_configure(1,8,0); er_fludd_reset(); er_fludd_input(1,1,8,0);
    assert(er_fludd_policy(0,1)==0 && er_fludd.water==60 && !er_fludd.active);
    er_fludd_input(0,1,8,0); assert(er_fludd_policy(1,1)==0 && er_fludd.water==60);
    return 0;
}
