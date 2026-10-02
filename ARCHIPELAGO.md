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

Flags: bit 0 means libsm64 initialized, generated assets available, worker
not hung, and a live Mario instance with a successful tick and usable pose;
bit 1 means Mario enabled; bit 2 means the applied snapshot matches the
latest requested snapshot. An accepted setter call is not an acknowledgement of
application. Wait for all three flags and matching masks before admitting the
player to a Mario seed.

Flags bit 3 (value 8) is immutable `SUPPORTS_REGRESSION_INTERACT`, available
before initialization. ABI version remains 1. Seeds declaring
`mario_regression_v1` require this support flag. While a managed Mario is live,
interacting within 4 metres of the loaded Radagon statue with Law of Regression
in the bag or storage supplies the native reveal effect. The game's event owns
the reveal and Goldmask quest flags; this bridge never writes them.

Instance readiness is cleared before creation failures, deletion, respawn,
loading, death, missing poses, safety shutdown and caught game-thread panics.
It is published by the game thread through an atomic; the ABI query never locks
the Mario state mutex.

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

## Coexisting overlays

The AP client and this fork each statically link hudhook/MinHook. Their hook
installation must share `Local\ERArchipelagoHudhookInstall.v1`, covering both
trampoline creation and application. Otherwise overlapping installations can
both report success while the last installed DLL bypasses the other's overlay.
The Mario guard uses a 30-second bounded wait and releases its ownership and
handle on exit. The client must use the same guard. See the independently
linked [native regression](tests/hudhook_chain/README.md).

## Optional health and power extension

Base ABI v1 and the 16-byte capability state remain unchanged. Base state flag
16 advertises stats support, including before Mario is initialized. The additional
exports are `er_mario_ap_set_stats(max_wedges: u32, power_basis_points: u32) -> u32`
and `er_mario_ap_get_stats_state(out: *mut StatState) -> u32`. `StatState` is four
u32 fields: `abi_version` (1), `flags`, `max_wedges`, and `power_basis_points`.
Flags 1/2/4 mean live ready, Mario enabled, and the requested stats snapshot
actually applied. Null output and invalid setter arguments return 0; success is 1.

Capacity accepts 4 through 8 wedges, power accepts 7500/10000/12500/15000 basis
points. Defaults and reset are 8/10000. The setter only queues an atomic snapshot;
the SM64 worker applies it before jobs and ticks. Clients must wait for both the
capability and stats acknowledgments when changing both. Health capacity clamps
current health and pending healing without healing or resurrecting Mario when
capacity increases. Initialization and explicit healing respect the applied
maximum. All attack shares and enemy/boss throw impacts use the applied power
multiplier, preserving finisher and minimum damage rules. The meter treats the
applied capacity as full and displays current/max below eight wedges.

`tests/ap_bridge.rs` exercises the stats ABI, queued/applied state, all stat
combinations, damage policy and meter ratio. `tests/ap_stats.c` runs the production
health policy across every capacity and health value, including death, drowning,
healing, draining and capacity reset. Windows CI runs these before the full DLL
compile. New stats behavior still requires live gameplay validation.
