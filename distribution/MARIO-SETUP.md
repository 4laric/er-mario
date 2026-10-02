# Mario companion for Elden Ring Archipelago 0.6.4

This optional ROM-free companion preserves ER Mario 0.3.3 and adds the Archipelago bridge. Use the normal AP release by itself for ordinary Elden Ring seeds. The companion DLL was built from 4laric/er-mario commit a39033e2e3f71b4fa6010defa4cc8f71b8e7baa5; see manifest.json for the CI run and hashes.

You need Elden Ring on Steam, me3 (https://me3.help), your own US Super Mario 64 ROM (.z64/.n64/.v64), and the normal AP 0.6.4 client bundle. Python 3.11+ and WitchyBND 3.0.1+ are needed only for composing the two icon sets. WitchyBND: https://github.com/ividyon/WitchyBND/releases. This archive supplies neither ROMs nor generated game assets nor the game's proprietary Oodle DLL.

1. Unzip this archive to a writable folder. Put your own US SM64 ROM beside er_mario.dll, or set rom= in er_mario.ini to its full path.
2. Start er-mario-setup.me3 with me3. Mario builds its package from your ROM and installed Elden Ring archives. At Setup complete, follow its prompt to close the game. This setup profile uses a separate ER0000_mario_ap.sl2 save, also used by the paired profile. Do not start the AP seed until setup finishes.
3. Install Python and its local extraction dependency: `py -m pip install cryptography`. Download and extract WitchyBND separately. Save and quit Elden Ring before composing icons.
4. In PowerShell, from this companion folder, run:

   ```powershell
   .\Compose-AP-Flower.ps1 -GameDir 'C:\Program Files (x86)\Steam\steamapps\common\ELDEN RING\Game' -Witchy 'C:\Tools\WitchyBND\WitchyBND.exe'
   ```

   This extracts only the two sprite-layout containers locally, then splices the public AP flower into Mario's generated hi/low atlases. It preserves Mario's icon sheet and all pixels outside the flower rectangle. Original atlases and temporary files remain under .local-build for recovery. It never installs or redistributes Oodle. Repeat this step after Mario regenerates package or game updates invalidate generated assets.
5. Extract the ordinary AP client release normally. Configure that release's me3/apconfig.json, or use its connect overlay. Create the paired profile here, pointing at the folder that contains the AP DLL:

   ```powershell
   .\Create-Paired-Profile.ps1 -APClientDir 'C:\Mods\EldenRing-AP\me3'
   ```

   The generated er-mario-ap.me3 loads this folder's Mario DLL and generated package together with the existing AP DLL by absolute path. AP keeps its configuration beside its own DLL. This does not change the vanilla AP profile. Do not load another full menu atlas override with the paired profile: the composed Mario atlas already contains both icon sets.
6. Launch er-mario-ap.me3 through me3 and connect to your Mario-mode slot. Keep the game offline through me3. Check both the Mario HUD and AP overlay. Mario unlocks Double Jump then Triple Jump from two Progressive Jump copies; other moves have individual items. Basic jump, punch, kick and stomp remain available.

F7 lifts a stuck Mario one metre. Keep AP trap diagnostics disabled because their optional Rune Thief probe also uses F7. Existing nine ability families were tested in game; the new Goldmask Regression interaction passed source/native policy tests but has not been manually tested in game. With Law of Regression in your bag or storage, interact within four metres of the loaded Radagon statue while its native reveal event is waiting. The game owns the reveal/quest flags; the mod only supplies the native Regression effect.

Never share your ROM, package, .local-build, extracted layouts, game archives or generated atlases. A normal AP client update can replace its DLL without copying your private Mario assets anywhere. To restore icons, close the game and copy hi/low originals from the selected .local-build backup back into package/menu.

## Source, licenses and reproducibility

Mario bridge source: https://github.com/4laric/er-mario/tree/a39033e2e3f71b4fa6010defa4cc8f71b8e7baa5. Upstream: https://github.com/deltarooo/er-mario (ER Mario 0.3.3). The mod additions are MIT; libsm64 original contributions are CC0; pinned fromsoftware-rs is MIT/Apache-2.0. See licenses/ for texts and dependency records. The compiled DLL includes texture-stripped Mario mesh code imported from the public n64decomp/sm64 revision 06ec56df7f951f88da05f468cdcacecba496145a. The mod licenses do not license Nintendo or FromSoftware content. Textures, audio, animations and Elden Ring asset wrappers are generated only on the player's machine from their own files.

The composition tool and layout extraction helper are project source under the accompanying MIT notices. Flower artwork is Archipelago's logo, supplied as public source art; the shipped 160x160 BC7 payload contains only that art. The separate normal AP release is the configuration/UI distribution; this archive is optional and carries no AP slot credentials.

The manifest records source revisions, CI artifact and every packaged file hash. tools/package_ap_companion.py in the Mario fork constructs the archive from an explicit public-file allowlist and gathers dependency notices from the locked Cargo metadata. The archive root is ER-Mario-AP; private verification data is never a packaging input. Building from source needs Rust, MSVC tools, clang-cl, Python and the pinned sibling fromsoftware-rs; run libsm64/import-mario-geo.py, then cargo test --locked and cargo build --locked --release with clang-cl selected. No ROM is used for the DLL build.
