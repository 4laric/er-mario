#include "ap_capabilities.h"
uint32_t er_ap_managed = 0, er_ap_unlocked = 0;
void sm64_er_ap_set_capabilities(uint32_t managed, uint32_t unlocked) {
    er_ap_managed = managed;
    er_ap_unlocked = unlocked;
}
int sm64_er_ap_attack_allowed(uint32_t action) {
    return er_ap_allows(er_ap_attack_capability(action));
}
