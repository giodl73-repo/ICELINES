# Animated hockey moment schema

Use this contract for a goal, celly, save, hit, penalty, faceoff, bench reaction,
or linked sequence. YAML is preferred.

```yaml
schema: icelines.animated-hockey-moment.v0.1
moment:
  id: stable-customer-id
  kind: goal|celly|save|hit|penalty|faceoff|bench|linked
  target_duration_seconds: 10.0
  game_id: null
  event_id: null
  period: null
  clock: null
  score_before: null
  score_after: null
sources:
  - id: primary-video
    type: nhl-event|team-social|collection-media|other
    locator: URL-or-repo-relative-path
    status: verified|visible|inferred|unknown
facts:
  scorer: null
  assists: []
  strength: null
  participants: []
  attacking_direction: unknown
observations:
  - id: obs-01
    status: verified|visible|inferred|unknown
    text: concise falsifiable observation
    source_ids: [primary-video]
beats:
  - id: beat-01
    phase: establish|anticipation|action|impact|recognition|reaction|release
    duration_frames: 12
    camera: rink-wide|medium|close|insert|puck-camera|reaction
    hockey_action: observable action only
    participants: []
    puck_state: controlled|passed|released|airborne|deflected|in-net|not-visible
    pose: concise key-pose description
    evidence: [obs-01]
    uncertainty: null
style:
  grammar: original kinetic cel animation
  devices: [bold-contours, angular-shadows, diagonal-speed-lines]
  prohibited: [copied-characters, copied-frames, readable-unlicensed-logos]
reel:
  timing_fps: 24
  shot_id: stable-shot-id
  frame_root: repo-relative-directory
  mode: sprite-animation|animation
  camera_delivery_aspects: ["16:9", "9:16"]
```

## Beat recipes

Goal: establish formation → possession/setup → pass or carry → release → puck
travel/deflection → net response → scorer recognition → celly → bench/crowd.

Celly: recognition → first gesture → teammate arrival → group shape → bench or
crowd reaction → release pose.

Hit/penalty: approach → legal/illegal contact as observed → impact → loose-puck
or player response → whistle → referee signal → escort → penalty-box reaction.

Save: shot setup → release → goalie read → lateral/vertical move → contact →
rebound control → whistle or continuation → reaction.

## Timing budget

At 24 timing frames per second, 5/10/15 seconds contain 120/240/360 delivery
frames. Limited animation does not require that many unique drawings. Start with
8–12, 12–24, or 18–30 authored cels respectively, then assign holds. Add unique
one-to-three-frame drawings only where motion readability or impact requires
them.

The sum of all `duration_frames` must equal
`target_duration_seconds * reel.timing_fps`.

## REEL mapping

Prefer sprite animation for a stable camera and rink:

```yaml
media_kind: sprite-animation
motion: pan-right
sprite_animation:
  background: backgrounds/rink.png
  timing_fps: 24
  sprites:
    - id: puck
      z_index: 40
      keyframes:
        - { frame: 0, asset: sprites/puck.png, x: 0.25, y: 0.65, width: 0.025 }
        - { frame: 48, asset: sprites/puck.png, x: 0.70, y: 0.52, width: 0.025 }
```

Use camera keyframes for `hold`, `follow`, `whip`, and `settle`, bound to the
same beats as the verified action. Keep player and puck tracks independent of
camera motion, and render every delivery aspect ratio to verify crop safety.

Use full cels when the composition itself changes:

Map each beat or cel to:

```yaml
media_kind: animation
animation:
  timing_fps: 24
  frames:
    - asset: frames/001-establish.png
      hold_frames: 18
      pose: establish
```

The frame asset is production material, not a factual source. Keep the original
source locator and evidence pack alongside the production manifest.
