# ER Mario

Play Elden Ring as Mario, with Super Mario 64's real movement. Triple jumps, wall kicks, long
jumps, ground pounds, punches and kicks that hurt enemies, Bowser's tail swing on staggered
bosses, Bob-omb style enemy throws, SM64's health meter, coins and Lakitu's camera.

Work in progress.

You need your own Super Mario 64 ROM (US version): Mario's textures, sounds, animations and the
menu icons come from it, built on your PC the first time you play. Like other
[libsm64](https://github.com/libsm64/libsm64) projects, the mod itself includes Mario's 3D
model (his mesh) from the public [SM64 decompilation](https://github.com/n64decomp/sm64). This
repository contains no Nintendo data.

## Getting started

You need Elden Ring on Steam and a Super Mario 64 ROM (US version).

1. Install **me3**, the mod loader: [me3.help](https://me3.help)
2. Download **ER-Mario-x.y.z.zip** from the
   [latest release](https://github.com/deltarooo/er-mario/releases/latest) (under Assets) and
   unzip it somewhere you can write to, like your Documents folder.
3. Put your Super Mario 64 ROM into the **ER-Mario** folder.
4. Start the game by double-clicking **er-mario.me3**.
5. The first time, the mod sets itself up from your ROM: a box on the title screen shows the
   progress (a few seconds). When it says **Setup complete**, press any button to close the
   game, then start it again with **er-mario.me3**. That's only needed once.

**ER MARIO** and the version in the title screen's bottom left corner
show the mod is loaded. Start a new character, the mod uses its own save file.

**Stuck somewhere?** Press **F7** to lift Mario 1 m. The mod also lifts him by itself when he
can't move for a few seconds. If that does not work, you might have to fast travel to a grace.

Tested with an Xbox One controller, a PS5 controller (through Steam) and keyboard and mouse.
The mod reads controllers the way Xbox pads report them: PlayStation, Switch and other pads
work through Steam Input, which Steam turns on for them by default.

**Stay offline.** me3 starts the game offline with anti-cheat off. Never play this mod online.

## FLUDD (experimental AP fork builds)

This fork includes an original FLUDD backpack with Hover, Rocket, Turbo and Squirt nozzles.
Set `fludd = on` in `er_mario.ini` to enable all four for standalone play. Archipelago
controls FLUDD and its unlocks when connected; enable `mario_fludd` in your player YAML.
These changes are development builds, not part of the upstream release linked above.

Hold **RB** to use the selected nozzle. While holding RB, **D-pad Up** selects Hover,
**Down** selects Rocket, **Right** selects Turbo, and **Left** selects Squirt.
On keyboard, hold **J** to use FLUDD and press **I** to cycle nozzles. Rocket and Turbo
charge before firing; ordinary jump, attack and crouch inputs take priority.

The HUD shows the selected nozzle and water remaining. Release FLUDD while safely
grounded to refill gradually, including while walking; resting at a grace or respawning
refills the tank. Airborne movement and active nozzles do not refill water. The base tank
holds 300 water; AP tank upgrades add 100 each, reaching 400, 500 and 600 without refilling water.
Squirt sprays forward while ordinary movement continues; its short stream damages nearby
targets and stops at map geometry. Hover jets can damage targets below Mario. The current AP item pool unlocks the three traversal
nozzles; Squirt is available in standalone play. Jets, charging and launches use SM64 audio.

The backpack adds generated model parts. On the first start with this build, allow
asset setup to finish, then restart when prompted, as during initial installation.

## Cappy and Sonic movement (experimental AP fork builds)

Set `cappy = on` or `sonic_movement = on` in `er_mario.ini` for standalone play.
Archipelago seeds use `mario_cappy` and `mario_sonic_movement`; all options default off
and can coexist with FLUDD. These development builds keep Mario's appearance and add
original movement implementations inspired by Odyssey and Sonic.

**Cappy:** tap **RT** or **O** to throw the cap; tap again to recall it early.
It stays deployed on its own timer and then returns automatically. Throws retain Mario's
movement momentum. Land on the deployed cap while descending
to bounce. One cap bounce is available per real landing on the ground. AP seeds unlock
Cap Throw and Cap Bounce separately. Mario keeps his original SM64 capped head at rest
and switches to his native uncapped head while the original cap flies, without added eyes.
Its swept hitbox damages each target once per throw. Enemy capture is not included.

**Sonic movement:** hold **Left Stick Click** or **U** while stationary on the ground
to charge Spin Dash, then release. Its spinning charge animation ramps from 1x to a
maximum of 3x speed; Mario does not drift during charging. Hold it in the air to charge
Drop Dash for the next landing. Press
**Right Stick Click** or **P** to Air Dash once per airtime; landing on the ground
restores it. Cap bounces and FLUDD do not restore the aerial moves. AP seeds unlock
the three moves separately. Charge and dash use a spinning animation and SM64 sounds.
Only actual dash movement damages targets, once per target per burst; charging does not.
Homing attack is not included.

Addon damage follows ordinary Mario combat, including boss scaling, AP damage upgrades,
enemy hit reactions and kill credit. Defaults are 20% for the cap, 12% per Squirt hit,
6% per Hover-jet hit (water hits repeat
at most once every 12 simulation ticks), and 34% for Sonic dashes, relative to a normal enemy's
maximum HP. `damage_cap`, `damage_squirt`, `damage_hover` and `damage_sonic` in `er_mario.ini` override these
percentages. Addon hits never initiate enemy or boss grabs.

Menus, loss of focus, death and incompatible actions cancel addon input. The AP console
captures these keyboard keys while typing. Ordinary Mario actions retain priority, and
only one movement system advances physics in a tick. Cappy's new model parts trigger
the same asset setup and restart described above. Appearance, controls and balance
still need a live playtest.

## Known issues

- Cutscenes show a crumpled Mario with the Tarnished's head.
- Mario's walk can flicker a little while passing fog walls.
- Torrent can't be summoned in Mario mode.
- Some big bosses' ragdolls go wild after a throw; the mod stops them early.
- Mario's shadow can flicker or drop out from some camera angles in sunlight and moonlight.
- Some hills and rocks have no collision in the game itself; Mario walks into them.
- With a controller, Mario can sometimes keep flicking between two directions while you
  move, as if two sticks were steering him. Restart the game and it's gone. It seems to happen
  when the controller connects or reconnects while the game is already running.

## Reporting problems

Open an [issue](https://github.com/deltarooo/er-mario/issues) and say what happened and
when. Attach both files from the **logs** folder inside your ER-Mario folder:
`er_mario.log` (the last session) and `er_mario.prev.log` (the one before). If the game didn't
start at all, me3's own log helps too: paste `%LOCALAPPDATA%\garyttierney\me3\data\logs` into
the Explorer address bar, open the **er-mario** folder and attach the newest file.

The logs contain your mod folder's path, which can include your Windows user name; feel free
to blank it out.

## How it works

- A Rust DLL loaded by [me3](https://me3.help), offline only (Easy Anti-Cheat off, separate save).
- [libsm64](https://github.com/libsm64/libsm64) runs Mario (SM64's own physics and animation),
  fed with Elden Ring's live Havok collision.
- The Tarnished stays in the game underneath and follows Mario, so doors, graces, menus, deaths
  and saves keep working.
- The `libsm64` folder is libsm64 with a few patches for the mod (ladder climbing, carrying,
  dive grabs, head turning, model part export), compiled into the DLL.
- Once per launch the mod asks GitHub for the latest release's version, and the title screen says
  when there's a newer one. Nothing is downloaded or installed automatically. `update_check = off`
  in er_mario.ini turns the check off.

## Building

Windows, with [Rust](https://rustup.rs), Visual Studio Build Tools (C++),
[LLVM](https://github.com/llvm/llvm-project/releases) (clang-cl compiles libsm64's C code) and
Python. Cargo fetches fromsoftware-rs itself (pinned to a commit in `Cargo.toml`):

```
git clone https://github.com/deltarooo/er-mario
cd er-mario/libsm64
python import-mario-geo.py
cd ..
.\build.ps1
```

`import-mario-geo.py` (libsm64's own setup script) downloads Mario's model code, two files, from
the SM64 decompilation once and strips their texture data (textures come from the player's ROM).
The model code isn't part of this repository; it gets compiled into the DLL.

`build.ps1 -Dist <ER-Mario folder>` also copies the DLL into an ER-Mario folder (it renames
the old DLL first, so it works while the game is running; the next start loads the new one).
The release zip is that folder without the generated `package` and `logs` folders.

## License

[MIT](LICENSE). This covers the mod's own code only, not Super Mario 64's or Elden Ring's
content: Mario's textures, sounds and animations come from the player's ROM, his mesh from the
SM64 decompilation (see above), and nothing of FromSoftware's is included. The `libsm64` folder
keeps libsm64's own license, CC0 ([libsm64/LICENSE.md](libsm64/LICENSE.md)).

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
