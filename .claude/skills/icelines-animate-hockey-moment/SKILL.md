---
name: icelines-animate-hockey-moment
description: "Reconstruct a verified hockey goal, celebration, save, hit, penalty, faceoff, or bench sequence as a 5–15 second stylized animation for a REEL manifest. Use when a user wants to animate a real play, replace live game footage with an evidence-backed recreation, derive hockey-correct key poses from NHL/IceLines data and source video, or create goal/celly animation beats without inventing the play."
---

# IceLines Animate Hockey Moment

Turn a real hockey moment into a hockey-correct evidence pack, pose plan, and
REEL limited-animation shot. IceLines owns what happened; REEL owns how the
authored cels are timed and rendered.

Read [references/moment-schema.md](references/moment-schema.md) before making
the beat sheet or REEL manifest.

## Workflow

1. Identify the moment precisely. Prefer NHL game id plus play/event id. If only
   a clip is supplied, record its URL or collection id, visible clock, teams,
   period, and claimed players before interpreting it.
2. Query IceLines and available NHL play-by-play/boxscore data for the game,
   scorer, assists, strength state, time, score state, and on-ice participants.
   Treat the event feed as fact and video observations as geometry.
3. Watch the source from at least two seconds before the decisive action through
   the reaction. Record only observable puck movement, handedness, player lanes,
   contact, referee signal, bench reaction, and celebration order.
4. Write an evidence pack using the reference schema. Mark each claim
   `verified`, `visible`, `inferred`, or `unknown`. Do not turn an inference into
   a documentary fact.
5. Decompose the moment into hockey beats. Use anticipation, decisive action,
   impact, recognition, and reaction. For a goal, preserve possession and pass
   order through release, puck travel, net response, and celly. For a penalty,
   preserve the infraction, whistle, signal, escort, and box reaction.
6. Choose the economical animation form. Use REEL `sprite-animation` when the
   rink/camera can stay stable and players, goalie, and puck can move as separate
   layers; draw two-to-four pose sprites per player and keyframe their paths. Use
   stepped player motion with smooth puck motion when the intended language is
   kinetic limited animation rather than continuous cutout puppetry. Use
   full `animation` cels when camera angle, contact geometry, crowd composition,
   or perspective changes materially from beat to beat. For full cels, design
   8–30 authored poses for a 5–15 second vignette. In both forms, use helmets for
   on-ice play and preserve handedness, bench/box geometry, and attacking direction.
7. Translate requested influences into general visual grammar. For classic
   kinetic racing-anime energy, use bold ink contours, angular cel shadows,
   diagonal speed lines, snap zooms, reaction close-ups, mechanical inserts, and
   graphic impact frames. Do not reproduce a named show's characters, frames,
   logos, or signature compositions.
8. Create or select the assets, then emit either a REEL `sprite-animation` shot
   with background and sprite keyframes or an `animation` shot with ordered cels
   and frame holds. Full-cel holds must sum to the shot duration; sprite tracks
   must begin at frame zero and remain within it.
9. For sprite work, express camera `hold`, `follow`, `whip`, and `settle` as
   keyframes compiled from the same action beats. Render crop-safe proofs at
   every intended aspect ratio rather than relying on a shot-level background
   pan.
10. Render a silent visual proof first, then add sourced or newly created audio.
   Never imply recreated animation is original game footage.
11. Bind the final manifest, choreography, render artifact report, video, and
   review evidence into a REEL production-package receipt. Byte integrity is
   not hockey-fact, editorial, rights, or publication approval.

## Output ownership

- IceLines/customer repo: event evidence, observations, beat sheet, player and
  rink geometry, source references, and animation frame assets.
- REEL: animation sequence schema, timing validation, frame hashing, assembly,
  audio events, rendering, and artifact verification.
- Customer production: editorial selection, visual direction, and final proof.

## Required review

Reject or revise a recreation when:

- the puck path, pass order, scorer, assist, or result conflicts with evidence;
- an unknown player is silently assigned an identity;
- skaters lack helmets during live play or use impossible rink/bench geometry;
- the animation changes a routine play into a different spectacular play;
- a real broadcast frame or named animation property is copied too closely;
- the REEL frame holds do not equal the declared shot duration;
- provenance omits the source event or source clip.

## Deliverable

Return the evidence-pack path, beat-sheet path, REEL manifest path, rendered
proof path when requested, and a concise list of unresolved observations. A
useful result is reproducible without relying on chat history.
