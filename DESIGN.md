# OMEGA RUST — Design Bible

A loving homage to *Omega Race* (Midway, 1981) — Midway's only vector arcade game —
written in Rust. Not an emulation: a faithful reconstruction of the feel, tuned for
modern hardware. Wordplay intended.

## 1. Creative vision

**The fantasy:** you are flying a lone Omegan fighter around a race track in space,
harried by a convoy of droid ships that seed the track with mines and evolve into
hunters. The game is a knife fight in a corridor.

**The look:** a 1981 monochrome vector monitor. Pure black void. Luminous stroked
lines — no fills, no sprites, no textures, ever. Brightness is the only "colour":
a hot white core with a cool blue-white phosphor glow (think X-Y monitor beam,
`#E6F0FF` core). Explosions are the object's own line segments shattering apart and
tumbling. UI text is a stroke font (segmented vector characters), never a TTF.
Restraint is the aesthetic: if a modern effect wouldn't read as "this is what a
vector monitor wishes it could do", cut it.

**The feel:** the original used a spinner, so rotation must be *fast, smooth,
frictionless* — nothing like Asteroids' sluggish turn. Thrust is Newtonian with
gentle drag. The walls are the signature mechanic: the ship *bounces* off invisible
force-field borders that flash into view on impact. Skilled play uses the bounce
deliberately.

## 2. Playfield

- Virtual space: **1024 × 768** units, 4:3, letterboxed into any window/fullscreen.
  All game logic in virtual units. Origin top-left, +y down.
- **Outer border:** rectangle inset 12 units from the virtual edge. Invisible during
  play except a faint idle shimmer; the struck *segment* flashes bright on any
  bounce (ship, enemy, or bullet impact), decaying over ~300 ms.
- **Center console:** rectangle ~520 × 240 centered at (512, 408). Same bounce
  behaviour, same flash. Inside it, in stroke font: `HIGH SCORE` + value (top),
  player score (large, centre), remaining ships as small ship glyphs (bottom row).
- **The track** is the space between outer border and console. Enemies orbit it;
  the player may fly anywhere in it (never inside the console).
- Player spawns bottom-centre of the track, facing up.

## 3. Player ship

- Shape: slim dart / arrowhead, ~26 units long, drawn from ~6 segments.
- Rotate: left/right at ~330°/s, slight ease-in (~60 ms to full rate) — spinner feel.
- Thrust: acceleration ~420 u/s²; max speed ~520 u/s; linear drag such that
  coasting halves speed in ~2.5 s. Thrust shows a flickering exhaust flame
  (random 2-frame jitter, authentic vector twinkle).
- Bounce: elastic reflection off borders/console, ~0.9 restitution, plus border
  flash and zap sound. No damage from walls.
- Fire: one shot per press, max **4** live shots. Shot = short line dash, speed
  ~900 u/s, dies on border/console/target impact. No player-shot bounce.
- Death: contact with any enemy, mine, or enemy bullet. Line-shatter explosion,
  ~1.5 s pause, respawn at spawn point (cleared of nearby threats), facing up.

## 4. Enemy roster (canonical point values)

| Enemy | Points | Behaviour |
|---|---|---|
| Photon Mine | 350 | Small static diamond, dropped behind Droids. Deadly to touch. |
| Vapor Mine | 500 | Larger pulsing X, laid by Command Ships. Deadly to touch. |
| Droid Ship | 1000 | Convoy: orbits the track in loose formation, drops Photon Mines. Returns fire only in overdrive. |
| Command Ship | 1500 | Evolved Droid: breaks formation, fires aimed photon bullets (with spread), lays Vapor Mines. |
| Death Ship | 2500 | Evolved Command: fast, hunts the player directly. Doesn't fire — it *is* the bullet. |

- Enemy ships also bounce off borders (flash + zap).
- Enemy bullets die on borders (small flash).
- Escalation: on a wave timer, Droids promote to Command Ships one at a time;
  a long-lived Command Ship promotes to a Death Ship. The last surviving enemy
  ship always escalates quickly — waves must never stalemate.

**Difficulty ramp:** Wave *n* uses `t = min((n - 1) / 12, 1)`, with enemy tuning
lerped from wave 1 to full heat at wave 13: convoy speed 140→205; Command wander
145–205→190–260, fire 1.6–2.6→0.9–1.6 s, aim error ±10°→±4°, and Vapor Mines
5.5–8.5→3.8–6.0 s; Death speed/steering 300/230→360/300; enemy bullets
340→430; Droid Photon Mines 3.2–5.0→2.0–3.2 s; escalation first/repeat
10/8→6/5 s; and Command-to-Death age 12→8 s. Full heat holds through wave 13.

### Overdrive (waves 14+)

After full heat, `u = clamp((wave - 13) / 12, 0, 1)` drives a second ramp which
reaches its cap at wave 25. The wave 1–13 simulation and RNG stream remain exactly
the full-heat design above.

| Parameter | Wave 13 (`u=0`) | Wave 25+ (`u=1`) |
|---|---:|---:|
| Convoy orbit speed | 205 | 220 |
| Command fire interval | 0.9–1.6 s | 0.62–1.15 s |
| Command aim error | ±4° | ±2.5° |
| Enemy bullet speed | 430 | 470 |
| Death Ship max speed / steering | 360 / 300 | 385 / 340 |
| Escalation first / repeat | 6 / 5 s | 4 / 3.5 s |
| Command-to-Death age | 8 s | 6 s |

Overdrive convoys grow from 10 ships at wave 14 to 16 at wave 25. A fleet-level
timer lets a random Droid return fire: 3.6–5.4 s tightening to 1.5–2.6 s, with
±16°→±9° spread and bullets at 85% of standard enemy-bullet speed. Command wander,
mine-drop intervals and cap, lone-survivor timing, player physics, scoring, and
fleet-bonus rhythm never ramp beyond their wave-13 values.

Vapor Mines become proximity weapons throughout overdrive, including mines carried
from wave 13. At 70→100 units they arm with a 0.9→0.65 s fuse, ratcheting sound,
fast spin, shake, and bright flicker. Shooting one still safely defuses it for 500
points. Detonation produces a 60→110-unit blast: the player is killed through the
normal death path, ships and Photon Mines shatter for their normal points, and
nearby Vapor Mines chain-arm on a random 0.12–0.30 s stagger. The blast is drawn as
expanding rough vector rings plus a particle burst. From `u >= 0.3`, it also throws
3→6 evenly spaced, randomly rotated shrapnel streaks which kill the player and
expire after 0.7 s or on wall contact. Newly laid Vapor Mines drift at 12→40 u/s,
reflecting perfectly from the arena and console; older Vapor Mines and all Photon
Mines remain static. Mine fuses freeze outside active play and during ShipDeath.
The attract-screen `W` selector cycles practice starts through waves 1, 14, 17,
21, and 25; practice runs are labelled and never write the high score. Headless
captures can start directly at a chosen wave with `--wave N`.

## 5. Waves, scoring, lives

- Through wave 13, wave *n* spawns `min(4 + n, 10)` Droids. From wave 14 onward it
  spawns `10 + min((n - 13) / 2, 6)` using integer division, capped at 16. The
  convoy moves in a randomly chosen direction (clockwise/counter), with a brief
  spawn-in warning so the player is never ambushed.
- Wave cleared when all *ships* are destroyed. Mines persist across waves
  (the track gets meaner) — but every 4th wave cleared awards the
  **FLEET BONUS: 5000** and sweeps all mines clean. That's the game's breathing
  rhythm.
- Extra ship at **40,000**, at **100,000**, then every further 100,000.
- 3 ships to start. High score persisted to disk (`~/.local/share/omega_rust/hiscore`).

## 6. Game states

`Attract → Playing ⇄ ShipDeath → GameOver → Attract`

- **Attract:** OMEGA RUST title in large stroke letters, short original story crawl
  (riff on the 1981 "The time: 2003…" copy — ours is 2081), enemy roster with point
  values, blinking `PRESS ENTER`. High score shown.
- **GameOver:** `GAME OVER` + final score; new high score gets a small ceremony
  (flashing value). Auto-return to Attract.
- Pause on `P`. Quit on `Esc` (from Attract only; in-game Esc → Attract).

## 7. Controls

Left/Right or A/D rotate · Up or W thrust · W cycles practice wave on Attract ·
Space fire · Enter start ·
P pause · M mute · F fullscreen · Esc back/quit.

## 8. Audio direction

All SFX synthesized at startup (44.1 kHz mono WAV in memory) in the flavour of the
original's AY-3-8912 PSGs: square waves + noise, fast pitch sweeps, no reverb.
Events: fire (descending square zap, ~90 ms), wall bounce (bright metallic blip),
enemy explosion (noise burst + falling tone), player explosion (longer, layered),
mine pop (short blip), thrust (looping filtered noise rumble, gated by input),
convoy hum (slow pulsing drone whose rate/pitch climbs as the fleet shrinks —
the game's heartbeat), extra ship (rising arpeggio), fleet bonus fanfare.
If the user later supplies a legally-owned MAME ROM set, authentic samples can be
recorded and swapped in via `assets/sfx/` overrides; the synth remains the default.

## 9. Architecture (the display-list rule)

The game emits **vectors, not pixels** — exactly like the original hardware, where
the CPU wrote a display list and the X-Y monitor drew it.

- `DisplayList`: `Vec<Seg { a: Vec2, b: Vec2, intensity: f32 }>` rebuilt every frame
  by game logic. **All** rendering goes through it.
- Sink A — **screen** (macroquad): draws segments as lines; M3 adds the glow
  post-pass (render-to-texture, blurred additive re-composite) and beam variance.
- Sink B — **headless** (pure CPU): rasterizes the display list to PNG with no
  window, no GPU, no X server. CLI:
  `omega_rust --headless --frames N --shot-every M --out DIR [--seed S] [--wave N] [--script FILE]`
  where the script feeds deterministic per-frame inputs. This is the automated
  visual-verification harness — agents check their own work by generating frames
  and reading the PNGs.
- Fixed timestep: logic at 60 Hz; render interpolation unnecessary at these speeds.
- Determinism: seedable RNG; headless defaults to a fixed seed.
- Stroke font: `0-9 A-Z . : -` as segment lists, used for every glyph in the game.

## 10. Milestones

1. **M1 — the vector core:** display list + both sinks, stroke font, arena
   (borders + console + score layout), ship flight/bounce/fire, fixed timestep,
   headless harness. *Proof: PNGs of ship coasting, bouncing, firing.*
2. **M2 — the game:** full roster, convoy AI, mines, escalation, collisions,
   shatter explosions, waves/scoring/lives, attract/game-over, hiscore file.
3. **M3 — the glow:** post-process bloom, border flash polish, exhaust flicker,
   beam intensity variance, attract-mode ceremony, feel tuning.
4. **M4 — the sound:** synth engine + all events above, mixed and ducked.

Non-goals: multiplayer, gamepad (stretch), online scores, exact ROM behaviour.
