# Archipelago capability bridge

This fork exposes a versioned C ABI for the Elden Ring Archipelago client. It
does not connect to an Archipelago server itself. The client reconstructs a
complete capability snapshot from received items and submits it after reconnect
or seed changes. No capabilities are restricted until a caller manages them.

```c
struct ErMarioApState {
    uint32_t abi_version;
    uint32_t flags;
    uint32_t managed;
    uint32_t unlocked;
};
uint32_t er_mario_ap_abi_version(void); // 1
uint32_t er_mario_ap_set_capabilities(uint32_t managed, uint32_t unlocked);
uint32_t er_mario_ap_get_state(struct ErMarioApState *out);
```

The setter returns 1 when accepted and 0 for unknown bits or unlocks outside the
managed mask. It publishes an atomic request; the libsm64 worker applies the
latest request before its next job, including before a Mario tick. The getter
returns the **applied** snapshot, never a pending request. `out` must point to
16 writable bytes; null returns 0. Successful queries return 1.

Flags: bit 0 means libsm64 initialized, generated assets available, and worker
not hung; bit 1 means Mario enabled; bit 2 means the applied snapshot matches the
latest requested snapshot. An accepted setter call is not an acknowledgement of
application. Wait for all three flags and matching masks before admitting the
player to a Mario seed.

| Bit | Capability |
| --- | --- |
| 1 | Double Jump |
| 2 | Long Jump |
| 4 | Wall Kick |
| 8 | Dive, slide kick, and slide attacks |
| 16 | Ground Pound |
| 32 | Enemy Grab and Throw |
| 64 | Boss Swing and Throw |
| 128 | Triple Jump (including flying/special triple jump variants) |
| 256 | Backflip |
| 512 | Side Flip |

All known bits total 1023. A capability is available when unmanaged or unlocked.
The client handles Progressive Jump items: one grants Double Jump, two grant
Triple Jump too. This bridge receives individual bits, not item counts.

Actions are filtered before their initialization changes jump velocity or dive
speed. Locked jumps fall back to ordinary jump; locked wall kicks fall back to
freefall. Dive, slide kick, ground pound and pickup attempts fall back to a safe
idle/freefall state. Existing actions are cancelled on revocation without a new
jump impulse. Death, forced warps, ladders, ordinary jumping, punches, kicks,
sweeps and stomps remain available. Slope-driven sliding remains possible, but
its attacks require Dive. This avoids repeatedly cancelling terrain-driven
slides and preserves traversal of slippery slopes.

Enemy and boss pickup initiation is also gated on the game thread. Their active
carry/throw updates release enemies and restore gravity on revocation. Damage
checks run before libsm64's attack query, which can bounce Mario as a side effect.
Direct action assignments in libsm64 were audited: they only transition to moving
punch, throw landing, or ledge climb; none initiate a locked move.

## Tests without a game or ROM

Rust ABI test (only rustc required):

```powershell
rustc --edition=2024 --test tests/ap_bridge.rs -o ap_bridge_tests.exe
.\ap_bridge_tests.exe
```

C action tests (clang-cl in a Visual Studio developer environment):

```powershell
clang-cl /Ilibsm64/src /Ilibsm64/src/decomp/include /Ishim /DGBI_FLOATS /DVERSION_US tests/ap_actions.c libsm64/src/ap_capabilities.c /Fe:ap_actions_tests.exe
.\ap_actions_tests.exe
```

Both tests enumerate every unlock mask. Full DLL compilation requires the
existing pinned sibling fromsoftware-rs source and imported Mario geometry code;
it does not require a ROM. Runtime assets remain player-supplied. Live game
compatibility, move controls, region warps, death synchronization, and required
boss encounters still need Windows gameplay validation.
