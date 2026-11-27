# Bulk IGC upload / flight deletion for Flugbuech

Two small tools for a [Flugbuech](https://github.com/dbrgn/flugbuech) instance
such as `https://flights.home/flights/`:

| File | Purpose |
| --- | --- |
| `upload_igc.py` | Upload many `.igc` files in bulk |
| `delete_all_flights.py` | Delete all (or a filtered subset of) flights, with optional IGC backup |
| `flugbuech_api.py` | Shared HTTP client + login used by both |

Flugbuech has no API token, no import-from-folder feature and no bulk endpoint — the
web UI simply makes a few HTTP calls per flight. These scripts replay exactly those
calls, so they need nothing but your normal login and the Python standard library.


---

## How the upload works in Flugbuech

Everything below is from the upstream source (`main` branch).

### 1. Log in — `POST /api/v1/auth/login`

`src/auth.rs` → `login()`:

```jsonc
// request body
{"username": "amos", "password": "…"}

// 204 No Content + Set-Cookie: user_id=<encrypted>, user_name=<username>
// 403 Forbidden      {"error":{"code":403,"reason":"BadCredentials", …}}
```

The session is a **private Rocket cookie** (`user_id`, encrypted server-side with
`ROCKET_SECRET_KEY`) plus a plain `user_name` cookie. There is no bearer token, so a
HTTP client has to keep the cookies — which is what this script's cookie jar does.

### 2. Parse the IGC file — `POST /api/v1/flights/add/process_igc`

`src/process_igc.rs` → `process_igc()`:

```
Content-Type: application/octet-stream
<body = raw .igc bytes, max 50 MiB (src/main.rs: MAX_IGC_UPLOAD_BYTES)>
```

```jsonc
{
  "type": "success",
  "pilot": "Amos",              // HFPLT
  "glidertype": "Epsilon 8",    // HFGTY
  "site": "Hitzeggen",          // HFSIT
  "dateYmd": [2019, 7, 22],     // HFDTE
  "launch":  {"pos": {"lat": 46.7, "lng": 9.1}, "alt": 1568, "timeHms": [13, 42, 26], "locationId": 3},
  "landing": {"pos": {"lat": 46.7, "lng": 9.2}, "alt": 1300, "timeHms": [13, 46, 7], "locationId": 4},
  "trackDistance": 1.98988      // km
}
// or {"type": "error", "msg": "…"} with HTTP 200
```

`locationId` is only present when one of **your** launch/landing locations is within
1000 m of the first/last track point — that is how the web form pre-selects them.

### 3. Create the flight — `POST /api/v1/flights`

`src/flights.rs` → `add()` / `FlightAddUpdateForm` + `frontend/src/routes/flights/FlightForm.svelte`:

```jsonc
{
  "launchDate": "2019-07-22",        // NaiveDate, or omit together with the times
  "launchTime": "13:42:26",          // NaiveTime, UTC, no timezone conversion by the server
  "landingTime": "13:46:07",
  "launchSite": 3,                   // optional location ID, must belong to you
  "landingSite": 4,
  "trackDistance": 1.99,             // km, f32, must be finite and >= 0
  "glider": 2,                       // optional glider ID, must belong to you
  "hikeandfly": false,
  "comment": "…", "videoUrl": "…", "xcontestUrl": "…",   // optional
  "igcData": "QStYWFhBQkMgRkxJR0hUOjE…"   // URL-safe base64, NO padding, of the .igc bytes
}
// 201 Created
```

Rules that matter for automation (all enforced server-side):

* `launchDate`, `launchTime` and `landingTime` must be **all three present or all
  three absent**; a partial triple returns `400 InvalidData`.
* `igcData` must be URL-safe base64 **without** `=` padding
  (`base64::engine::general_purpose::URL_SAFE_NO_PAD`), not standard base64.
* The request body is JSON, so the base64 blob counts against Rocket's
  `json = "10 MiB"` limit (`Rocket.toml`). ~7.5 MiB of IGC data is the practical
  maximum; bigger files need that limit raised on the server.
* Times are stored as-is (naive → UTC), so IGC timestamps (which are UTC) are
  correct without conversion.
* Duplicate detection does not exist — uploading the same file twice creates two
  flights. That is why this script checks `GET /api/v1/flights` first and skips
  flights whose launch time already exists.

There is **no batch endpoint**. The CSV import (`POST /api/v1/flights/import/csv`,
`src/import_csv.rs`) can create several flights at once, but it cannot attach IGC
files, so it is only useful for flights without a track.

---

## Requirements

* Python 3.8+ (standard library only — no `pip install`)
* Network access to your instance, and a normal account on it

## Uploading flights

```bash
cd ~/Documents/amos-flugbuech/scripts

# Everything under a folder (recursive), passwords prompted for:
./upload_igc.py ~/Documents/IGC

# Point it at your instance explicitly, several folders + globs:
./upload_igc.py ~/igc/2023 ~/igc/2024 "~/Downloads/*.igc" \
  --url https://flights.home

# See what would happen, without creating anything:
./upload_igc.py ~/igc --url https://flights.home --dry-run

# Self-signed certificate on the LAN:
./upload_igc.py ~/igc --url https://flights.home --insecure
```

The password prompt does not echo. For scripts you can instead use
`FLUGBUECH_USERNAME` / `FLUGBUECH_PASSWORD` (or `-u` / `-p`, but then it lands in
your shell history).

Example output:

```
Server:   https://flights.home/api/v1
User:     amos
Files:    42 IGC file(s)

Already logged: 17 flight(s) with a launch time; those will be skipped.

[ 1/42] OK   2019-07-22_13-42_Hitzeggen.igc  2019-07-22 13:42:26-13:46:07, 1.99 km, launch#3, landing#4
[ 2/42] SKIP 2019-07-22_13-42_Hitzeggen_copy.igc  already logged (…)
[ 3/42] FAIL 0007.igc  HTTP 400: 400 InvalidData: Invalid flight data: Invalid launch location
...
Summary: 24 uploaded, 17 existing, 1 failed  (38.4s)
```

Exit codes: `0` everything uploaded/skipped, `1` some files failed, `2` setup
problem (bad URL, wrong credentials, unreachable server).

### Options

| Option | Meaning |
| --- | --- |
| `PATH …` | Files, directories (searched recursively for `*.igc`) or globs |
| `--url URL` | Flight book URL, default `https://flights.home`; `/flights/` and `/api/v1` suffixes are normalised away |
| `-u`, `--username` | Username (prompted if omitted) |
| `-p`, `--password` | Password (prompted if omitted) |
| `--glider ID` | Attach a glider ID to every uploaded flight (IDs are in `GET /api/v1/gliders` or the `/gliders/` page) |
| `--hikeandfly` | Mark every uploaded flight as hike-and-fly |
| `--dry-run` | Parse + validate on the server, print the planned flights, create nothing |
| `--no-skip-existing` | Upload even when a flight with the same launch time already exists |
| `--force` | Try files that do not start with an IGC `A` record |
| `--insecure` | Skip TLS verification (self-signed certificate) |
| `--ca-bundle FILE` | Verify with a custom CA bundle instead |
| `--timeout SECONDS` | Per-request timeout, default 180 |
| `--retries N` | Retries for network errors and 5xx, default 2 |
| `--report FILE` | Write a JSON report (per file status, warnings, submitted fields; `igcData` is omitted) |
| `-v`, `--verbose` | Print every HTTP request |

## Deleting flights

`delete_all_flights.py` removes flights via `DELETE /api/v1/flights/<id>`
(`src/flights.rs` → `delete()`, `204` on success, `404` if it is already gone,
`403` if the flight belongs to someone else). The server has **no bulk delete and
no undo**, so the script is deliberately cautious:

* it is a **dry run unless you pass `--yes`**;
* it prints every flight it selected;
* with `--yes` it asks you to type `delete <count>` (skipped automatically when
  stdin is not a terminal, e.g. cron — use `--confirm-count N` there as a guard);
* `--backup DIR` downloads a `flights.json` manifest plus every stored IGC file
  (`GET /api/v1/flights/<id>/igc`) **before** deleting anything, and a flight whose
  IGC could not be saved is **not** deleted;
* filters let you keep the newest flights or a date range instead of nuking everything.

```bash
# Show what would be deleted (safe, creates nothing / deletes nothing):
./delete_all_flights.py

# Back up IGC files, then delete everything:
./delete_all_flights.py --backup ~/flugbuech-backup --yes

# Keep the 10 newest flights, delete the rest:
./delete_all_flights.py --keep-newest 10 --yes

# Only old flights, and only after a backup:
./delete_all_flights.py --before 2020-01-01 --backup ~/old-flights --yes

# Specific flights by ID:
./delete_all_flights.py --ids 17,42,99 --yes
```

Example output:

```
Flights on the account: 42
Selected for deletion:  32

  #7   2019-07-22 13:42  no. 1  1.99 km  Advance Epsilon 8
  #8   2019-07-28 10:05  34.10 km  Ozone Zeno 2
  …

You are logged in as 'amos' and are about to PERMANENTLY delete 32 flight(s).
There is no undo in Flugbuech. IGC files are gone unless you backed them up.
Type "delete 32" to continue, anything else to abort: delete 32

[ 1/32] DEL  #7  2019-07-22 13:42
…
Deleted 32 of 32 selected flight(s).
```

### Options

| Option | Meaning |
| --- | --- |
| `--yes`, `-y` | Actually delete (default is a dry run) |
| `--keep-newest N` | Keep the N newest flights of the selection |
| `--before DATE` | Only flights launched before `YYYY-MM-DD` |
| `--after DATE` | Only flights launched after `YYYY-MM-DD` |
| `--ids 1,2,3` | Only these flight IDs (see the `/flights/` page) |
| `--backup DIR` | Save manifest + IGC files to `DIR` before deleting |
| `--confirm-count N` | Abort unless exactly N flights are selected (good for scripts) |
| `--pause SECONDS` | Sleep between deletions |
| `--report FILE` | Write a JSON report (selected/deleted/failed IDs, filters) |
| `--url`, `-u`, `-p`, `--insecure`, `--ca-bundle`, `--timeout`, `--retries`, `-v` | Same as the uploader |

Notes:

* Flights without a launch date cannot be matched by `--before`/`--after`; they are
  only included when you give no date filter. They sort as "oldest" for `--keep-newest`.
* The backup manifest (`flights.json`) holds metadata only. Re-uploading the saved
  `flight<id>.igc` files with `upload_igc.py` restores the flights (without comments
  or XContest URLs, which are not part of the IGC file).
* The flight list endpoint is the same one the web UI uses, so it reflects exactly
  what you see at `/flights/`.

---

## Notes and gotchas

* **Cookies without `Path`.** `src/auth.rs` creates the session cookies without a
  `Path` attribute, so a strict RFC 6265 client would only send them back to
  `/api/v1/auth/…` and every upload would return `401`. The script deliberately
  ignores cookie path scoping (it only talks to one host) — that is the
  `_PermissiveCookiePolicy` class.
* **Wrong password** → `403 BadCredentials`, exit code 2, nothing uploaded.
* **`401 MissingAuthentication`** mid-run means the server restarted with a new
  `ROCKET_SECRET_KEY` or the cookies were dropped; the script stops and tells you.
* **`400 InvalidData: Invalid launch location`** means the location ID derived from
  the IGC track is not (or no longer) one of your locations. Create the location
  first, or upload once and set it in the UI.
* **No signal / no GPS fix** files (`HFDTE` present but no `B` records) upload with
  the IGC attached but without date or times; the script prints a `note:` about it.
* **Landing after midnight**: the server computes duration from the same calendar
  day, so such a flight shows no duration. The script warns about it.
* **Nothing is deleted or modified** by this script: it only logs in, reads the
  flight list, parses IGC files and creates flights. `--dry-run` skips the last step.

## Quick check without a real instance

Any HTTP endpoint will do for a smoke test; the API base is derived from `--url`:

```bash
./upload_igc.py ~/igc --url http://127.0.0.1:8731 -u test --dry-run -v
```
