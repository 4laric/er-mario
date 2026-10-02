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

This upstream contribution starts at `bf558b37449a7a1a73e870a0ec89c3fd8785f574`
and contains only collision changes and their regression coverage. Synthetic tests and a
Windows build do not replace the live-game checks above; keep the PR in draft until those pass.
