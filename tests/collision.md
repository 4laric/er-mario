# Rock and moving-platform collision

`collision_geometry.rs` verifies the production winding conversion after X reflection:
floors stay floors, ceilings stay ceilings, rock faces keep their exterior, inconsistent convex
face indices become outward, and quantized degenerate triangles are dropped.

`collision_platforms.c` links the production surface loader, collision queries, platform
displacement and matrix routines. It checks a leaning rock wall and its triangle seam across
static refreshes; static-to-object floor ties and rider attachment; the first lift movement;
dynamic-only floor continuity after removing all static surfaces;
100 rising/descending steps with displacements exceeding the floor query's 78-unit buffer;
stopping, translation plus rotation, jumping away, platform deletion and slot reuse.
The workflow also runs the Rust reflected-platform rotation test through `cargo test`.
All fixtures are synthetic and require no game assets, ROM or running game.

The Windows workflow compiles and executes these tests before building the release DLL.

## Adjacent wooden stakes (combined playtest report, 2026-10-02)

The user became trapped among wooden stakes after a regular jump; an enemy hit freed Mario.
The screenshot does not establish which live triangles were involved. A production collision
query regression reproduces a relevant defect: both triangles on a shared edge use the original
probe position, applying the same wall push twice and crossing the opposite obstacle.
Each face now uses the position corrected by the preceding face. The synthetic 70-unit gap
checks both entry sides, all 24 face orders, repeated collision queries and surface refreshes.
The test fails before the change and passes afterward; the earlier rock/elevator checks also
pass. This is a prevention change to the wall solver, not an automatic unstuck adjustment.
Replaying the reported regular jump in-game remains necessary to confirm this incident is fixed.

## Fog gates

Godrick's arena fog was crossed by walking without Interact. Normal SM64 movement now checks
its actual segment at chest/head height against live Havok with the existing player movement
filter before updating the character proxy. A hit clamps horizontal movement on the near side;
vertical simulation continues. This covers blockers missing from the imported mesh, without
guessing a new fog collision layer or making a permanent wall after the blocker is removed.
Actual game-driven entry animations bypass the guard. An unrelated animation change after
Interact can no longer start follow mode and discard wall collision. Body hits are recomputed
at the accepted position so rejected movement cannot damage enemies through the barrier.

Portable tests cover live blockers, legitimate entry, removed blockers, jumping/dashing and
vertical-only movement. Live validation remains required at Godrick: walking/jumping/dashing
must block before Interact, entry must work after Interact, and defeated-boss passage must stay
open. Also check normal stairs, sloped ground and elevators for guard false positives.

## In-game validation still required

The reported rock location was unspecified. No Elden Ring session was launched during this
change, so test success does not establish coverage of every live Havok mesh/layer or the
original clipping incident. The draft DLL should be checked on several stationary boulders
from ground and air, along seams and sloping faces, and on an elevator from rest in both
directions. Check stepping onto/off the lift, jumping while it moves, landing on it, stopping
at both endpoints, and travelling far enough to trigger world-origin rebasing or streaming.

Mesh triangles now retain Havok winding after the coordinate reflection rather than changing
with Mario's location. Convex shapes and synthetic custom-piece boxes explicitly face outward.
Havok accepts two-sided mesh hits; libsm64 uses oriented surfaces, so meshes with intentionally
inconsistent or reversed authoring need particular attention in the in-game check.

This branch starts at `d801f342c1c2ac1f938671ef97dc17a912e66e0c` and changes the collision refresh
portion of `src/lib.rs`. Integrate with `codex/mario-fludd` by reviewing that shared tick loop
and rerunning both branches' tests together; do not overwrite or blindly merge the FLUDD work.
