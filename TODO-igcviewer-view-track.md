# TODO: igcviewer / tisot follow-ups

## Task: address Docker Hub image vulnerabilities (critical, found 2026-11-27)

Docker Hub's vulnerability scan on
https://hub.docker.com/repository/docker/amosshapira/igcviewer/general
reports a few CRITICAL CVEs in the pushed images. Amos: "Add a task to
address them later".

Approach when picking this up:
- The findings come from the base images and the toolchain baked into the
  images, not from the app code: the igcviewer runtime is
  `debian:13-slim` (system libraries) and the static Go binary embeds the
  Go stdlib of `golang:1.24-alpine` (Go CVEs are reported for compiled
  binaries too). The tisot image (rust:1-buster builder + debian:13-slim
  runtime) almost certainly has the same issue — check its page too.
- Fix: `docker pull` fresh base images, rebuild, and re-push (a plain
  rebuild usually clears the report); consider `apt-get upgrade` in the
  runtime stage; keep the Go/Rust toolchains on current patch releases.
- Verify: re-scan on Docker Hub after pushing (the report re-runs on
  push); note the tag that was scanned (v1.10 / latest at the time of
  writing).
- Longer term: consider a scheduled rebuild (komodo or cron automation)
  so base-image patches flow into the images regularly instead of only
  on feature builds.

## Future task: 3D sailplane GLB marker (SimpleMeshLayer)

Replace the flat top-down PNG glider marker with a real 3D model via
deck.gl's SimpleMeshLayer, so the replay marker can pitch in climbs,
bank in turns and yaw with the heading (the flat sprite can only rotate
in the map plane — the root of the "reversed orientation" issues).

Model (chosen by Amos, 2026-11-27): Innerscene "Sailplane glider" —
https://www.innerscene.com/tools/library/3d-parts/sailplane-glider-5d42002f
- GLB, 6.8 MB, CC0-1.0 (public domain — no attribution REQUIRED, but
  credit the link in the igcviewer README Credits section anyway, per
  Amos's request)
- Real-world scale: 15.1 m wingspan, 6.8 m length, textured; tagged
  "assembly" (may contain multiple meshes — check it renders as one
  clean sailplane)
- Download: https://www.innerscene.com/api/library/sailplane-glider-5d42002f/download
  (valid until 2030-10; re-download or keep a local copy when
  implementing)

Implementation notes (verified 2026-11-27):
- `deck.SimpleMeshLayer` IS available in the bundled deck.gl 9.4.2
  (checked: typeof deck.SimpleMeshLayer === 'function')
- The GLB downloads fine (valid glTF v2, 6,779,516 bytes)
- Marker: SimpleMeshLayer with mesh '/sailplane.glb' (embed in
  cmd/server/web/), position [lon, lat, altitude], orientation
  [pitch, yaw, roll] computed per replay tick: yaw = track bearing,
  pitch = climb angle from vertical/horizontal speed, optional bank in
  turns. One-time empirical nose-axis alignment (GLB is Y-up; deck has
  its own convention — tune a rotation constant with the harness)
- Scale: real-scale model needs ~20–40x display exaggeration
  (sizeScale constant, tune visually)
- depthTest: false in ground style stays (datum mismatch workaround,
  same as the other deck layers); the marker param options become
  'glider' = the 3D GLB sailplane, 'paraglider' = the SVG sprite
  (unchanged). The flat top-down PNG marker would be retired.
- Verify with the harness (flight 109 at multiple zooms/pitches, both
  markers, replay).

## Future task: hover popup with 2D IGC track in the flights list

Add a new column to the flights list with a "show" button; hovering over
it opens a small popup window rendering the flight's IGC track in 2D.
Interaction model: WeGlide (https://www.weglide.org/flight?airport=Narromine,141284).

Implementation notes (from 2026-11-25 discussion):
- Backend: new session-authenticated endpoint
  `GET /api/v1/flights/<id>/track` returning the parsed IGC track as
  GeoJSON LineString (backend already parses IGC in process_igc.rs; no
  client-side parser needed). Not feature-gated — plain flugbuech
  functionality, upstream-able.
- Frontend: small MapLibre map in a popup near the hovered button
  (flugbuech's maplibre 1.15.1 is fine for 2D LineString). Line +
  start/end markers, fit to bounds. Track renders 2D only; the full 3D
  experience stays with the "View track" igcviewer link.
- Placement: dedicated new column in the flights list (not the actions
  column). Only meaningful for flights with hasIgc.

## Watch: komodo stack environment

Whether a komodo ResourceSync wipes the picton-tisot stack env
(FLUGBUECH_*/IGCVIEWER_TOKEN) on a future push to main: the sync
regenerates the stack from the repo's TOML, which does not contain the
secrets. Recovery values can be read from the running containers
(db: POSTGRES_PASSWORD, api: ROCKET_SECRET_KEY).

## Done (for reference)

- Altitude/ground-profile bottom panel (WeGlide-style) — deployed in
  igcviewer v1.7 (2026-11-26), including the replay marker line + stats.
- 3D marker orientation: the side-view freepngimg icon was replaced by a
  drawn top-down glider silhouette (v1.6); a full 3D GLB model is queued
  above as the final step.
