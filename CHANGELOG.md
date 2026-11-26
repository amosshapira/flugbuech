# Changelog

All notable changes in this fork (`amosshapira/flugbuech`) since it diverged
from upstream (`dbrgn/flugbuech`) at `b153211` (toolchain bump, #250), newest
first. Written for the upstream maintainer to decide what (if anything) to
integrate.

Highlights in one paragraph: the fork extends Flugbuech for **sailplane /
glider pilots** (glider metadata, winch/aerotow flight fields, IGC-derived
crew and altitudes) while keeping paraglider workflows unchanged; adds an
**IGC back-fill** endpoint to retro-populate existing flights from their
stored IGC files; integrates an optional external **3D track viewer**
behind a compile-time feature flag; and adds a few **per-user preferences**
(display units, post-login landing page, viewer behaviour) plus small UI
niceties (optional P2 column, favicon).

## Added

### Sailplane / gliding support (44a6a1c, 20cc2cf, 366026e, 4907a90)

- **Glider metadata**: registration, competition ID, two-seater flag, FAI
  class (Club/Standard/15m/18m/20m/Open/Two-Seater), motorglider flag.
  Registration-aware uniqueness (two club ASK-21s can coexist); gliders
  without registration keep the old per-user model uniqueness. Gliders
  table shows a dedicated Registration column, with Manufacturer/Since/
  Until behind a remembered "Show more columns" toggle.
- **Flight fields for winch/aerotow flying**: pilot in command (P1),
  second pilot (P2, marked as instructor/student/passenger via a
  flight-capacity field: PIC/Dual/Instructor/ICUS), launch type
  (Aerotow/Winch/Car/Self-Launch), release altitude, tow height, tug
  registration, barometric max altitude, height gain, outlanding flag and
  engine run time. All optional; existing paraglider flights default to
  PIC and need no migration attention.
- **IGC parsing for the new fields**: crew names (P1/P2) and barometric
  altitudes (max altitude, height gain) are extracted from uploaded IGC
  files. Negative barometric altitudes (field elevation below ISA datum)
  are handled. `NIL` header values (the IGC convention for "not
  available", written e.g. by the Naviter Oudie) are treated as missing
  instead of being stored as crew names.
- **IGC glider auto-fill**: when uploading an IGC file, the flight form
  pre-selects the glider by registration (`HFGIDGLIDERID`) or model
  (`HFGTYGLIDERTYPE`) — only on an unambiguous match and only while the
  glider selection is still on its initial default.
- **IGC back-fill endpoint** (`POST /api/v1/flights/backfill_igc` +
  "Back-fill from IGC files" button on the My Flights page): re-parses the
  user's stored IGC files and fills in EMPTY fields of existing flights
  (glider, launch/landing site by nearest location within 1 km, crew,
  barometric altitudes). Never overwrites existing values; returns a
  summary of what changed. Useful for importing a logbook first and
  enriching it from IGC files afterwards.
- **Display impl change**: `Glider` renders as `Registration (Model)` when
  registered, `Model` otherwise (was `Manufacturer Model (Registration)`).

### Per-user preferences (44a6a1c, 4278bc9, 0a1f29d, 131af63)

- **Display units**: altitude (m/ft) and distance (km/mi/nm) stored on the
  user record, editable in the profile and via a navbar switcher (session
  override for guests). Data stays metric in the DB and API; the frontend
  converts for display and input.
- **Home page after login**: `default_page` (default `/`), CHECK-constrained
  to the main navigation destinations. The login redirects there unless an
  explicit `?redirect=` target was requested.
- Per-user preferences follow a common pattern: column with CHECK
  constraint on `users`, exposed through the profile API
  (`GET`/`POST /api/v1/profile`), settings block on the profile page, tests
  included.

### Optional igcviewer integration (f68ca8a, 4278bc9, 131af63)

Deep-links flights into a self-hosted 3D IGC viewer instance
([amosshapira/igcviewer](https://github.com/amosshapira/igcviewer), a fork
of [abbbi/igcviewer](https://github.com/abbbi/igcviewer)) next to the
Flugbuech container. **Disabled unless built with the `igcviewer` cargo
feature + `VITE_FEATURE_IGCVIEWER=1` frontend flag** (`FEATURE_IGCVIEWER=1`
build arg); upstream builds are unaffected.

- `GET /api/v1/flights/<id>/igc/view`: session-authenticated route that
  redirects the browser to the configured viewer
  (`IGCVIEWER_PUBLIC_URL`), falling back to the regular IGC download when
  unconfigured.
- `GET /api/v1/flights/<id>/igc?t=<token>`: the IGC download endpoint also
  accepts a shared access token (constant-time compared, `IGCVIEWER_TOKEN`)
  so the viewer can fetch IGC files server-side over the docker network —
  no private data is exposed to unauthenticated sessions.
- **"View track" buttons** (flight detail page + flights list rows) with
  per-user preferences: open in a new tab or same tab
  (`open_viewer_in_new_tab`, default new tab), choice of replay marker
  (`track_marker`: glider or paraglider), and optional reuse of the viewer
  tab (`reuse_viewer_tab`, default on): with "open in new tab" and reuse
  enabled, clicking "View track" navigates the already-open viewer tab
  (identified by a fixed window name) to the new flight instead of opening
  another tab each time.
- The fork's viewer README documents the container side.

### Small niceties (baae87f, 1a6f108)

- **Optional P2 column in the flights list**: header "Co-Pilot", sits
  between glider and launch; the column only appears when at least one
  flight in the list has a second pilot recorded (no setting needed), and
  cells are empty for flights without one.

## Changed

- **Flights list**: removed the XContest column (tracktype/distance/link).
  The XContest summary on the flight detail page is unaffected.
- **Favicon**: the site previously had no favicon (browsers 404'd
  `/favicon.ico`). Adds a glider-silhouette favicon — ⚠️ note for
  integration: the icon is [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/)
  (FreePNGimg), which is not compatible with redistribution in an AGPL
  project; upstream would want to swap in their own artwork (the code
  change itself is trivial: two files in `frontend/static/` + two
  `<link>` lines in `app.html`).

## Test coverage

All new backend functionality ships with API tests (62 → 66 tests,
`cargo test --features igcviewer`; 61 without the feature): profile
preference round-trips and validation, back-fill behaviour (idempotency,
never-overwrite), view route authorization matrix (401/403/404, redirect
targets, token auth), and parser regressions.
