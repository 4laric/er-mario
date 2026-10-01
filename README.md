# ER Mario

Play Elden Ring as Mario, with Super Mario 64's real movement: triple jumps, wall kicks, long
jumps, ground pounds, punches and kicks that hurt enemies, Bowser's tail swing on staggered
bosses, Bob-omb style throws, SM64's health meter, coins and Lakitu's camera.

Work in progress.

No Nintendo data is in this repository or in the mod. Mario's model, textures, sounds and
icons are built on the player's PC from their own Super Mario 64 ROM (US version). 

## Getting started

You need Elden Ring on Steam and a Super Mario 64 ROM (US version).

1. Install **me3**, the mod loader: [me3.help](https://me3.help) 
2. Download the **ER-Mario** zip from this repository's [Releases page](github.com/deltarooo/er-mario/releases) and unzip it somewhere
   you can write to, like your Documents folder.
3. Put your Super Mario 64 ROM into the **ER-Mario** folder.
4. Start the game by double-clicking **er-mario.me3**.
5. The first time, the mod sets itself up from your ROM: a box on the title screen shows the
   progress (a few seconds), then the game restarts by itself once. On Linux (Proton), or if
   the restart can't happen by itself, the box asks you to quit and start the game again.

That's it, you're Mario. **ER MARIO** and the version in the title screen's bottom left corner
show the mod is loaded. Start a new character, the mod uses its own save file.

**Stuck somewhere?** Press **F7** to lift Mario 1 m. The mod also lifts him by itself when he
can't move for a few seconds.

Only tested with an Xbox One controller and with keyboard and mouse.

**Stay offline.** me3 starts the game offline with anti-cheat off. Never play this mod online.

## Known issues

- Cutscenes show a crumpled Mario with the Tarnished's head.
- Mario's walk can flicker a little while passing fog walls.
- Torrent can't be summoned in Mario mode.
- Some big bosses' ragdolls go wild after a throw; the mod stops them early.
- Mario's shadow can flicker or drop out from some camera angles in sunlight and moonlight.
- Some hills and rocks have no collision in the game itself; Mario walks into them.

## How it works

- A Rust DLL loaded by [me3](https://me3.help), offline only (Easy Anti-Cheat off, separate save).
- [libsm64](https://github.com/libsm64/libsm64) runs Mario (SM64's own physics and animation),
  fed with Elden Ring's live Havok collision.
- The Tarnished stays in the game underneath and follows Mario, so doors, graces, menus, deaths
  and saves keep working.
- The mod builds against a slightly patched libsm64 (ladder climbing, carrying, dive grabs, head
  turning, model part export) in a folder next to this one.

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
