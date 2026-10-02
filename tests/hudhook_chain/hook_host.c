#include <stdio.h>
#include <windows.h>
#include <assert.h>
#ifdef _MSC_VER
__declspec(noinline)
#else
__attribute__((noinline))
#endif
int target(int value) { volatile int input = value; return input * 13 + 11; }
typedef int (*Create)(void*);
typedef int (*NoArg)(void);
int main(int argc, char** argv) {
 HMODULE a = LoadLibraryA("owner_a.dll"), b = LoadLibraryA("owner_b.dll");
 assert(a && b);
 Create create_a=(Create)GetProcAddress(a,"create"),create_b=(Create)GetProcAddress(b,"create");
 NoArg enable_a=(NoArg)GetProcAddress(a,"enable"),enable_b=(NoArg)GetProcAddress(b,"enable");
 NoArg calls_a=(NoArg)GetProcAddress(a,"calls"),calls_b=(NoArg)GetProcAddress(b,"calls");
 assert(create_a(target)==0);
 if (argc>1) { assert(enable_a()==0); assert(create_b(target)==0); assert(enable_b()==0); }
 else { assert(create_b(target)==0); assert(enable_a()==0); assert(enable_b()==0); }
 int value=target(1);
 printf("%s result=%d A_calls=%d B_calls=%d expected_both=27\n",argc>1?"serialized":"overlapped",value,calls_a(),calls_b());
 assert(value==(argc>1?27:26));
 assert(calls_a()==(argc>1?1:0));
 assert(calls_b()==1);
 return 0;
}
