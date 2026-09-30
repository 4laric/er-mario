# ER Mario

Play Elden Ring as Mario, with Super Mario 64's real movement: triple jumps, wall kicks, long
jumps, ground pounds, punches and kicks that hurt enemies, Bowser's tail swing on staggered
bosses, Bob-omb style throws, SM64's health meter, coins, stars and Lakitu's camera.

Work in progress.

No Nintendo data is in this repository or in the mod. Mario's model, textures, sounds and
icons are built on the player's PC from their own Super Mario 64 ROM (US version).

## How it works

- A Rust DLL loaded by [me3](https://me3.help), offline only (Easy Anti-Cheat off, separate save).
- [libsm64](https://github.com/libsm64/libsm64) runs Mario (SM64's own physics and animation),
  fed with Elden Ring's live Havok collision.
- The Tarnished stays in the game underneath and follows Mario, so doors, graces, menus, deaths
  and saves keep working.
- The mod builds against a slightly patched libsm64 (ladder climbing, carrying, head turning,
  model part export) in a folder next to this one.

## Credits

- [libsm64](https://github.com/libsm64/libsm64) (CC0) by the libsm64 contributors, built on the
  [Super Mario 64 decompilation](https://github.com/n64decomp/sm64) by the n64decomp team
- [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs) (MIT / Apache-2.0) by Vincent
  Swarte and contributors
- [me3](https://me3.help) by the me3 team
- [hudhook](https://github.com/veeenu/hudhook) (MIT) by veeenu, for the overlay HUD
- Item and inventory function patterns: The Grand Archives' Elden Ring cheat table

Super Mario 64 and Mario are Nintendo's. Elden Ring is FromSoftware's. This is a free fan mod,
not affiliated with either.
