#include <windows.h>
#include "MinHook.h"
typedef int (*Target)(int);
static Target original;
static void *address;
static int hits;
#ifndef ADDEND
#define ADDEND 1
#endif
static int detour(int value) { hits++; return original(value) + ADDEND; }
__declspec(dllexport) int create(void *target) {
 address = target;
 int status = MH_Initialize();
 if (status != MH_OK) return status;
 return MH_CreateHook(target, detour, (void**)&original);
}
__declspec(dllexport) int enable(void) { return MH_EnableHook(address); }
__declspec(dllexport) int calls(void) { return hits; }
