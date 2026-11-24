# TODO: igcviewer / tisot follow-ups

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
