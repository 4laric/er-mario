# Independent DLL hook ownership regression

`hook_owner.c` is compiled twice as separate DLLs, each linking its own copy of
the actual MinHook sources vendored in hudhook. This reproduces how the Mario
and Archipelago DLLs hook the same DXGI function with independent MinHook state.
The test uses a plain exported function so it needs no game or graphics device.

Build owner A with hudhook 0.9.0's `vendor/minhook` and `ADDEND=1`; build owner B
with hudhook 0.9.3's `vendor/minhook` and `ADDEND=2`. Link `hook.c`, `buffer.c`,
`trampoline.c`, and `hde/hde64.c` into **each** DLL. Build `hook_host.c` as an exe
beside `owner_a.dll` and `owner_b.dll`, then run:

```powershell
./hook_host.exe
./hook_host.exe serialized
```

The first run creates both hooks before applying either. Both installations
return success, but only the last applied hook receives calls:

```
overlapped result=26 A_calls=0 B_calls=1 expected_both=27
```

The second run creates and applies A before creating B. B's trampoline then
captures A's installed prologue, preserving both callbacks:

```
serialized result=27 A_calls=1 B_calls=1 expected_both=27
```

Both assertions passed on native Windows using gcc and the exact crate sources.
This proves the installation race, without claiming that a particular live
failure was caused by that interleaving. Both cooperating mods must hold the
named `Local\ERArchipelagoHudhookInstall.v1` mutex around the **entire** hudhook
construction and application. Protecting only `apply()` is insufficient.
