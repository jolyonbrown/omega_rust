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
| Droid Ship | 1000 | Convoy: orbits the track in loose formation, drops Photon Mines. Doesn't fire. |
| Command Ship | 1500 | Evolved Droid: breaks formation, fires aimed photon bullets (with spread), lays Vapor Mines. |
| Death Ship | 2500 | Evolved Command: fast, hunts the player directly. Doesn't fire — it *is* the bullet. |

- Enemy ships also bounce off borders (flash + zap).
- Enemy bullets die on borders (small flash).
- Escalation: on a wave timer, Droids promote to Command Ships one at a time;
  a long-lived Command Ship promotes to a Death Ship. The last surviving enemy
  ship always escalates quickly — waves must never stalemate.

## 5. Waves, scoring, lives

- Wave *n* spawns `min(4 + n, 10)` Droids as a convoy on the track, moving in a
  randomly chosen direction (clockwise/counter). Brief spawn-in warning so the
  player is never ambushed.
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

Left/Right or A/D rotate · Up or W thrust · Space fire · Enter start ·
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
  `omega_rust --headless --frames N --shot-every M --out DIR [--seed S] [--script FILE]`
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
