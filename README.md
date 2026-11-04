# Game ni Natta yo! Dokuro-chan: Kenkou Shindan Daisakusen — original port

<p align="center">
  <img src="dokuro-chan.webp" alt="ぴぴるぴるぴるぴぴるぴ〜♪" width="200">
</p>

English · [Français](README.fr.md) · [日本語](README.jp.md)

Unofficial Windows and Linux port of *Game ni Natta yo! Dokuro-chan ~Kenkou Shindan Daisakusen~* (PS2), written in Rust from its decompiled code. It aims to play exactly like the original; only genuine mistakes in its code are fixed, and they are all listed below.

The full game is ported: the title screen; Sakura's room (the main menu, with the list of endings seen, the album, the minigame menu and the challenge mode); the story (text, voices, backgrounds, characters, camera, fades, choices, day map, minigames, day results, point distribution, endings) and the bonus scenario; the options; the saves (twelve files, from the game menu; "つづきから" in the room); the movies (the logos and the opening at startup, the opening on the title screen, the story's cutscenes, the album's), decoded by the ps2-video library (pure Rust MPEG-2). Detailed status, in French, is in [docs/moteur.md](docs/moteur.md).

This repository contains **no copyrighted file**: no game assets, no original code. The port reads the disc image you provide, placed next to its executable, and checks it by its CRC32 before starting. The image can also live elsewhere: set its path (or the folder holding it) in `"assets"` → `"path"` of the save file; a relative path starts from the executable. Its CRC32 is checked on the first launch (a few seconds), then the image is recognized by its size and date (`"assets"` → `"checked"`).

The save file goes to `Documents\My Games` on Windows (the Documents folder as Windows knows it, moved or synced included) and `~/.local/share` on Linux. With a `portable.txt` file next to the executable, it goes to a `saves` folder next to it instead.

## The minigames

The minigames as their menu starts them in the game:

- **ドクロちゃん捕縛アミ** (*Dokuro-chan Capture Net*): Dokuro pops out of nine bushes; catching her with the net drains her gauge, a miss refills it. Won if the gauge is empty within 40 seconds.
- **アドバンスド徒競走** (*Advanced Footrace*): a race against Dokuro. Mash ○ to run, ↑ jumps over hurdles, ↓ slides; obstacles stun Sakura, a hurdle stops him. Won by reaching the goal first.
- **水着で神経衰弱** (*Swimsuit Memory*): a memory game against Dokuro, who remembers the cards she has seen. A pair is worth 2 points and plays again; the joker is worth 1 point and shows every card for a moment. Won at 8 points.
- **桜くん私物キャッチ** (*Catch Sakura-kun's Belongings*): Dokuro throws Sakura's belongings out of the window. An item hitting the ground or a caught bomb costs one of three hearts. Won by lasting 50 seconds.
- **バッティングエスカリボルグ** (*Batting Escalibolg*): Dokuro hits Sakura with her bat by stopping three gauges (power, timing, angle). Won by going past 600 m in three tries.

They are played along the story, then from the minigame menu in Sakura's room once played in the story. That menu also offers the challenge mode (the five games in a row, harder, each one earning points, counted in skulls; cleared at 100), the choice of the character cheering Sakura on, and the tutorials. The challenge menu opens once an ending has been seen; clearing it opens the bonus scenario there, "12歳の世界！", whose ending leads back to the room.

## Playing

- Required file: the disc image (ISO) of the Japanese disc (SLPM-66186, CRC32 `B9F5F0C5`), with any name ending in `.iso`.
- Put it next to `dokuro_chan_ps2.exe` (`dokuro_chan_ps2` on Linux) and run the executable.
- Played with a gamepad like on PS2: A for ○, B for ×, X for □, Y for △, LB and RB for L1 and R1, the triggers for L2 and R2, the D-pad or the left stick; Start + Select quits. In the story, △ starts or stops skipping, START the auto mode, R1 switches between skipping read pages (既読) and skipping everything (強制), and ×, □ or L1 stop them; their sign shows in the top right. The gamepad vibrates like the DualShock 2 when the camera shakes: the big motor through the low-frequency motor, the small one through the high-frequency motor (on Linux, the gamepad's `/dev/input/event*` device must be writable).
- Keyboard: Enter or Z for ○, Backspace or X for ×, Escape for × (for □, the menu, in the story), C for □, V for △, Q, W, A and S for L1, R1, L2 and R2 (A is the auto mode in the story), the arrow keys. During the race, Space jumps and Ctrl slides; elsewhere, Space is START.
- In the story, as in a PC visual novel: left click, Enter or the wheel down for the next page; right click (×) hides the windows; the wheel up (L1) opens the history, where the wheel scrolls (△ takes the story back to the chosen page, □ opens the menu); Escape opens the menu (also during a choice and on the map, as in the game); holding Ctrl skips for as long as it is held; A toggles the auto mode.
- During a movie: START (Space) stops it, as in the game; so do Escape and a click. The game only listens after the first half second.
- Mouse, with the same rule everywhere: hovering a menu entry, an answer, an option value, a memory card file, an album thumbnail, tab or sheet, a point-sharing cell, a map place or the portrait of the character cheering Sakura on moves the cursor there; a click on it is ○, a click beside it does nothing. Where the game waits for any key, a click anywhere will do: on the title screen it is START; in the story it moves the text on. The right button is ×. A click on an option gauge (volumes, opacities, speeds) sets it directly. In the minigames, the net aims at the hovered bush, the memory game picks the hovered card, Sakura follows the mouse in the belongings game. The pointer is Dokuro's Escalibolg (an unused image from the game); it hides as soon as the keyboard or gamepad is used, or after 3 seconds without touching the mouse.
- The PS2 screen (640 × 448) is shown at 4:3, 60 frames per second.
- Alt+Enter or F11: borderless fullscreen.
- Save file (the display mode, what the game keeps from one playthrough to the next — endings seen, challenge cleared, minigames played, pages read, option settings, album images seen —, the twelve saved games, and the disc image's path and check): `Documents\My Games\Game ni Natta yo! Dokuro-chan Kenkou Shindan Daisakusen\save.json` on Windows, `~/.local/share/Game ni Natta yo! Dokuro-chan Kenkou Shindan Daisakusen/save.json` on Linux.

### Game bugs fixed

The port fixes the mistakes that the game's code proves (never what could be intended). For each one: what the game does, why it is a mistake, what the port does.

- Failed challenge, the screen does not darken: on success, the game darkens plane 5 with `AD_BlackOutStart(10, 5, 0, 0x50)` (layer 0, opacity 0x50); on failure, it calls `AD_BlackOutStart(10, 5, 0x50, 0x7f)`, that is layer 0x50, while a plane only has two. Nothing visible darkens (the game writes outside its fade table), and closing the message, which clears layer 0, turns the screen dark at once before brightening it. The port darkens the screen as on success (0x50 over 10 frames).
- Information messages do not shrink when closing: `AD_InformationWinDel` chains two steps (10 frames, then a shrink down to 0 × 0), but the window engine (`AD_WindowProc`) does not reset its frame count from one step to the next. The second step therefore starts already finished and the message vanishes at once; the same oversight makes the step given to a window that is still opening end at once. The port resets the count at each step: messages (from the challenge as well as the story) shrink.
- Race: `MiniRunHitDown`, shared by both runners, compares their height with 294, the ground of Sakura's lane. Dokuro, on her own ground, counts as airborne and, level with a hurdle, briefly goes back to her jump pose. The port compares each runner with her own ground.
- Options: when leaving the "スキップ・オートプレイ設定" window, the game puts the cursor on entry 4 of the game window then, in the same frame, wraps it with the four entries of the window it leaves: it goes back to the top, on "スキップモード", and the move sound follows the back sound. The same check adds that sound after the confirm sound when opening a skip mode's details. The port leaves the cursor on entry 4, with only the back (or confirm) sound.
- Camera shake: when switching to type 0x1b (all three axes, over 8 frames), the game forgets the transition factor on the zoom, which jumps at once to the new shake instead of blending into it as for the other types. The port applies the factor.
- Windows: when an action replaces another one in the middle of its animation (a window closing while it is still opening, for example), `AD_WinActSet` computes how far the step got with `(float)(count / duration)`, an integer division done before the conversion: it is 0 until the step is over, while the computation that follows (start + progress × (target − start)) expects a fraction. The window therefore jumps back to the start of its step at once. The port divides as decimals: the window carries on from where it was.
- Album, bonus image: after the bonus image won by completing the album, the game should wait 0x14 frames for the black to fall before showing the album, but its test is inverted (it only counts down when the count is already below 1, and moves on otherwise): the album appears at once, before the black has had time to fall. The port waits the 0x14 frames.
- Auto mode, the sign in the top right corner: the "オート", "強制スキップ" and "既読スキップ" labels are 119 × 21 pixels in the `win2` image, at rows 2, 26 and 50. The game cuts the last two correctly, but takes "オート" from row 0: two empty rows at the top and the two rows of its bottom edge missing, so the label looks cut off. The port cuts it from row 2.
- Looping sound effects: the game keeps the number of the loop in each of its two slots, and only resets them at startup (`AD_SoundInit`). Returning to the room during a scene where a loop is playing cuts the sound but keeps its number; in the next game, opening the options or the memory card from the game menu restarts it on the way back (`AD_LoopSERePlay`), and it plays until a script stops that slot. The port forgets the loops in place when leaving the story.

### What the game contains but never uses

Studying the executable and the disc's 35 scripts (the `Tools/coverage.py` report) shows code that is present but unreachable, probably abandoned during development. The port does not bring it back, since the game cannot reach it.

- A battle mode (`AD_Dcb*` functions, about 40 KB of code). The player picks a character, then presses at the right moment: a marker turns around a circle holding targets, and each target hit earns points. Around it: hit points, a gauge, full-screen cut-ins, support, turns, a gun in the background (`dcbgun`), win and lose screens (`dcb_win`, `dcb_lose`). It would be played on the story's backgrounds with the story's characters. The story would start it through a batch of kind 6, but no script instruction produces that kind; neither its fifteen images nor its table of characters are on the disc.
- A number pad (`AD_TenKey*` functions, batch kind 5), laid out like a phone keypad (1 to 9, *, 0, #), to have the player dial a number, with its right and wrong answer texts: its instructions (`tenkey_mode`, `tenkey_ok_text_set`, `tenkey_ng_text_set`) appear in no script, and its two images (`tenkey`, `tenkeybg`) are not on the disc.
- A sprite and 3D model engine (`SP_SYSTEM`, `MGP2`, particles, morphing), which runs every frame but stays empty: no script uses its instructions.
- Album bonus images, shown at the end of the game depending on a flag or when the album is complete (`AD_CgBonusGetJudg`): no script registers any (`bonus_cg_flag_entry`, `bonus_cg_spflag_entry`, `bonus_cg_comp_entry`). Only the minigames' bonus images are used. The port keeps the mechanism, which stays unused as in the game.
- On the title screen, a yes / no question that nothing opens.
- In the room, an entry that would start a separate prologue (`AD_AdvPartStartProc(0)`): no script registers it (`prologue_stcm`), and the menu has no image for it; the cursor skips it.
- In script 3, every movie registered a second time at development paths (`host0:pss/…`, files read from the development PC), in a section the game never runs.

### What differs from the PS2

- Disc reading waits do not exist: where the game waits for its data to load (`AD_DataReadStateGet`) or for music to fall silent, the port moves on. A movie thus starts sooner after the page before it, and the story resumes sooner after it.
- After a story movie, the restarted music is only heard after a quarter of a second. The game restarts it and then cuts it right away when the next scene is silent: on PS2, the disc reading time kept it from being heard, whereas the port, which reads instantly, brought it back for an instant. All other music starts at once.
- Movies are decoded exactly, but their colour, stored at half resolution, is interpolated between its samples instead of being repeated in 2 × 2 blocks as the console's IPU does: colour edges are sharper, with nothing added to the picture. Their sound plays at full volume, as on PS2.
- Where the game writes its system data to the memory card (when leaving changed options), the port writes its save file, without the memory card screen. Kept flags go there as soon as they change, pages read at each wait (a page, a choice, the map), when the game copies them with its system data; the skip mode chosen with R1, when a game is saved.
- Saved games go into the port's save file, through the game's own memory card screen, messages and questions (the first time, it offers to create the game data, as on a fresh card). Errors of a missing, full or damaged card cannot happen. Operations are immediate: their message stays half a second.

## Checking your file

The CRC32 tells whether the disc image is the right version. Check it with 7-Zip (right click > CRC SHA > CRC-32) or with Python:

```
python Tools/crc32.py <file>
```

## Building

Requirements: [Rust](https://rustup.rs) (stable).

### Windows

```
build.bat
```

builds the port in release and writes the ready-to-share archive (executable only, no game file): `target\Dokuro-chan.PS2.Windows.zip`.

### Linux

`pkg-config` and the ALSA headers are also needed (`sudo apt install pkg-config libasound2-dev` on Ubuntu). On Linux:

```
./build.sh
```

From Windows, `build_linux.bat` does the same in WSL (Ubuntu). The archive is written to `target/Dokuro-chan.PS2.Linux.tar.gz`.

## Layout

| Folder | Contents |
|---|---|
| `src/` | the port |
| `Tools/` | tools used to study the game: checks, disc extraction, decompilation with Ghidra, image and sound dumps (see `Tools/README.md`) |
| `docs/` | the study of the game, in French: the script format ([scripts.md](docs/scripts.md)), the engine and the port status ([moteur.md](docs/moteur.md)) |
| `decompiled/` | the decompiled game code, a reading reference for the port (ignored by git, made by `Tools/decompile.bat`) |

### Redoing the study

Everything is redone from the ISO placed at the repository root (details in `Tools/README.md`, in French):

1. `Tools\decompile.bat <Ghidra folder>`: the executable taken out of the ISO, its function names, the pseudo-C of the whole game in `decompiled/`.
2. `python -I Tools/uni_extract.py <iso> UNION/SCRIPT.UNI Tools/work/script`: the story scripts.
3. `python -I Tools/stcm2_table.py Tools/work/SLPM_661.86 Tools/work/symbols.txt > Tools/work/stcm2_opcodes.txt`, then `python -I Tools/stcm2_dump.py <script>.bin Tools/work/stcm2_opcodes.txt`: the scripts as readable text.
4. `python -I Tools/uni_png.py <iso> <sector> <folder>`: the images of a package (sectors given by `Tools/iso_list.py`); `python -I Tools/sprites_png.py <iso> <folder>` exports every image of the game at once.

### Testing

`cargo test`: the tests (those reading the real scripts only run when the ISO is at the repository root). In debug builds, environment variables drive the game without watching a window: `DOKURO_SNAPSHOT=<frames>:<file.png>` (one capture, then the game quits), `DOKURO_EVERY=<n>[@<frame>]` (a capture every n frames, from that frame on), `DOKURO_INPUT=<frame>[-<frame>]:<button>;…` (presses, or a button held from one frame to another), `DOKURO_STORY=<script>[:<entry point>]` (the story at that place, for example `102:sure2`), `DOKURO_TRACE=1` (every entry point played, written to standard error: enough to restart the story just before what is being tested), `DOKURO_BOOT=1` (the startup movies, which a test otherwise skips: it starts on the title screen) and `DOKURO_ZONES=1` (a red outline around the screen's mouse zones). `cargo test audit -- --ignored --nocapture` walks through the story on the real scripts, without a window (the first answer to each choice, the first place on the map), and lists the scenes where a zoom, a camera offset, a shake, a fade plane or an off-screen character lasts ten pages or more, with the entry point and batch where it starts, as well as the movies met on the way; `DOKURO_AUDIT=<script>:<entry point>` starts it elsewhere (an ending, for example `121:sure1`).

On Windows, `Tools\Tester.bat` sets these variables from a menu, then starts the game, the audit or the tests.

In debug builds, the port also looks for the disc image at the root of this repository (`*.iso` is ignored by git).

## Credits

*Game ni Natta yo! Dokuro-chan* (2005) is based on the novels by Masaki Okayu (MediaWorks), published by Idea Factory and developed by Design Factory. All rights to the game, its characters and assets belong to their respective owners. This port is unofficial and not affiliated with them.

## License

Copyright (C) 2026 Nyaldee. Licensed under the [GNU General Public License v3.0](LICENSE) — see the `LICENSE` file for the full text.

[Claude](https://claude.com), Anthropic's AI, contributed to the code of this project.
