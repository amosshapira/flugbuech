#!/usr/bin/env python3
"""Bulk-upload IGC flight files to a Flugbuech instance.

Flugbuech (https://github.com/dbrgn/flugbuech) has no upload token and no bulk
endpoint, but its web UI is a thin layer over three ordinary JSON/HTTP calls.
This script replays exactly what the web UI does, once per IGC file:

  0. POST /api/v1/auth/login                 JSON {username, password} -> 204 + session cookie
  1. POST /api/v1/flights/add/process_igc    body = raw IGC bytes (application/octet-stream)
                                             -> parsed metadata: date, launch/landing
                                                time+position, track distance
  2. POST /api/v1/flights                    JSON, includes igcData = URL-safe,
                                             unpadded base64 of the IGC file -> 201

Sources (upstream, main branch):
  src/auth.rs               -> login route, cookie names, status codes
  src/process_igc.rs        -> /flights/add/process_igc, response fields
  src/flights.rs            -> /flights add/list, FlightAddUpdateForm fields
  frontend/src/routes/flights/FlightForm.svelte -> how the UI builds the JSON body

Requirements: Python 3.8+ and nothing outside the standard library.
"""

from __future__ import annotations

import argparse
import base64
import glob
import json
import os
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Sequence

from flugbuech_api import (
    JSON_LIMIT_BYTES,
    MAX_IGC_UPLOAD_BYTES,
    Client,
    FlugbuechError,
    InvalidIgc,
    NotAuthenticated,
    RequestRejected,
    add_connection_arguments,
    api_base_from_url,
    client_from_args,
    normalize_timestamp,
    prompt_credentials,
)


# --------------------------------------------------------------------------- #
# File handling and payload construction
# --------------------------------------------------------------------------- #


def expand_paths(entries: Sequence[str]) -> list[Path]:
    """Expand files, directories (recursive .igc) and shell-style globs."""
    files: list[Path] = []
    for entry in entries:
        expanded = os.path.expanduser(entry)
        if any(char in expanded for char in "*?["):
            matches = sorted(glob.glob(expanded, recursive=True))
            if not matches:
                raise FlugbuechError(f"No files match {entry!r}")
        else:
            if not os.path.exists(expanded):
                raise FlugbuechError(f"No such file or directory: {entry}")
            matches = [expanded]

        for match in matches:
            path = Path(match)
            if path.is_dir():
                for candidate in sorted(path.rglob("*")):
                    if (
                        candidate.is_file()
                        and candidate.suffix.lower() == ".igc"
                        and not any(part.startswith(".") for part in candidate.relative_to(path).parts)
                    ):
                        files.append(candidate)
            elif path.is_file():
                files.append(path)
            else:
                raise FlugbuechError(f"Not a regular file: {match}")

    seen: set[Path] = set()
    unique: list[Path] = []
    for path in files:
        key = path.resolve()
        if key not in seen:
            seen.add(key)
            unique.append(path)
    return unique


def looks_like_igc(raw: bytes) -> bool:
    """Every IGC file starts with an A record (a line beginning with 'A')."""
    head = raw.lstrip(b"\xef\xbb\xbf")[:4096].decode("latin-1", "replace")
    for line in head.splitlines():
        line = line.strip()
        if line:
            return line.startswith("A") and len(line) >= 3
    return False


def hms_to_time(value: Iterable[int]) -> str:
    hours, minutes, seconds = value
    return f"{int(hours):02d}:{int(minutes):02d}:{int(seconds):02d}"


@dataclass
class FlightDraft:
    payload: dict[str, Any]
    launch_stamp: str | None
    warnings: list[str] = field(default_factory=list)


def build_flight(
    metadata: dict[str, Any],
    raw: bytes,
    *,
    glider_id: int | None = None,
    hikeandfly: bool = False,
) -> FlightDraft:
    """Build the POST /api/v1/flights body, mirroring FlightForm.svelte.

    The backend requires launchDate, launchTime and landingTime to be either all
    present or all absent (src/flights.rs, `into_new_flight`), so the triple is
    only emitted when the whole triple is known.
    """
    warnings: list[str] = []
    payload: dict[str, Any] = {}

    date_ymd = metadata.get("dateYmd")
    launch = metadata.get("launch") or {}
    landing = metadata.get("landing") or {}
    launch_hms = launch.get("timeHms")
    landing_hms = landing.get("timeHms")

    launch_date: str | None = None
    if date_ymd and launch_hms and landing_hms:
        year, month, day = date_ymd
        launch_date = f"{int(year):04d}-{int(month):02d}-{int(day):02d}"
        payload["launchDate"] = launch_date
        payload["launchTime"] = hms_to_time(launch_hms)
        payload["landingTime"] = hms_to_time(landing_hms)
        if payload["landingTime"] < payload["launchTime"]:
            warnings.append("landing time is before launch time (overnight flight?); duration will be empty")
    else:
        missing = [
            name
            for name, value in (
                ("flight date (HFDTE)", date_ymd),
                ("launch time", launch_hms),
                ("landing time", landing_hms),
            )
            if not value
        ]
        warnings.append("no date/time stored because " + ", ".join(missing) + " is missing from the IGC file")

    if launch.get("locationId") is not None:
        payload["launchSite"] = int(launch["locationId"])
    if landing.get("locationId") is not None:
        payload["landingSite"] = int(landing["locationId"])

    distance = metadata.get("trackDistance")
    if isinstance(distance, (int, float)) and distance > 0:
        payload["trackDistance"] = round(float(distance), 2)

    if glider_id is not None:
        payload["glider"] = glider_id
    payload["hikeandfly"] = bool(hikeandfly)

    # URL-safe base64 without padding, exactly like u8aToBase64(..., {urlSafe, noPad}).
    payload["igcData"] = base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")

    launch_stamp = None
    if launch_date and launch_hms:
        launch_stamp = normalize_timestamp(f"{launch_date}T{hms_to_time(launch_hms)}")
    return FlightDraft(payload=payload, launch_stamp=launch_stamp, warnings=warnings)


# --------------------------------------------------------------------------- #
# Reporting
# --------------------------------------------------------------------------- #


@dataclass
class Result:
    path: Path
    status: str  # uploaded | existing | dry-run | invalid | failed
    detail: str = ""
    flight: dict[str, Any] | None = None
    warnings: list[str] = field(default_factory=list)


def describe(meta: dict[str, Any], draft: FlightDraft) -> str:
    parts: list[str] = []
    if draft.payload.get("launchDate"):
        parts.append(
            f'{draft.payload["launchDate"]} '
            f'{draft.payload["launchTime"]}-{draft.payload["landingTime"]}'
        )
    if "trackDistance" in draft.payload:
        parts.append(f'{draft.payload["trackDistance"]:.2f} km')
    sites = []
    if "launchSite" in draft.payload:
        sites.append(f'launch#{draft.payload["launchSite"]}')
    if "landingSite" in draft.payload:
        sites.append(f'landing#{draft.payload["landingSite"]}')
    if sites:
        parts.append(", ".join(sites))
    pilot = meta.get("pilot")
    glider = meta.get("glidertype")
    if pilot or glider:
        parts.append(" / ".join(x for x in (pilot, glider) if x))
    return ", ".join(parts)


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #


def parse_args(argv: Sequence[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="upload_igc.py",
        description="Bulk-upload .igc files to a Flugbuech flight book.",
        epilog=(
            "examples:\n"
            "  ./upload_igc.py ~/Documents/IGC\n"
            "  ./upload_igc.py ~/igc --url https://flugbue.ch --username amos --dry-run\n"
            "  FLUGBUECH_PASSWORD=secret ./upload_igc.py ~/igc/2023 ~/igc/2024 --insecure\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("paths", nargs="+", metavar="PATH", help="IGC files, directories (searched recursively) or globs")
    add_connection_arguments(parser)
    parser.add_argument("--glider", type=int, metavar="ID", help="glider ID to attach to the flights (optional)")
    parser.add_argument("--hikeandfly", action="store_true", help="mark all uploaded flights as hike-and-fly")
    parser.add_argument("--dry-run", action="store_true", help="parse and validate only, do not create flights")
    parser.add_argument("--no-skip-existing", action="store_true", help="upload even if a flight with the same launch time exists")
    parser.add_argument("--force", action="store_true", help="upload files that do not start with an IGC A record")
    parser.add_argument("--report", metavar="FILE", help="write a JSON report of the run to FILE")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)

    try:
        api_base = api_base_from_url(args.url)
        files = expand_paths(args.paths)
        client: Client = client_from_args(args)
    except FlugbuechError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if not files:
        print("error: no .igc files found", file=sys.stderr)
        return 2

    try:
        username, password = prompt_credentials(args.username, args.password)
    except FlugbuechError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    print(f"Server:   {api_base}")
    print(f"User:     {username}")
    print(f"Files:    {len(files)} IGC file(s)")
    if args.dry_run:
        print("Mode:     dry run (nothing will be created)")
    print()

    try:
        client.login(username, password)
    except FlugbuechError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    existing: set[str] = set()
    if not args.no_skip_existing:
        try:
            known = client.list_flights()
        except FlugbuechError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 2
        existing = {
            stamp
            for stamp in (normalize_timestamp(item.get("launchTime")) for item in known.get("flights", []))
            if stamp is not None
        }
        print(f"Already logged: {len(existing)} flight(s) with a launch time; those will be skipped.")
        print()

    results: list[Result] = []
    started = time.monotonic()
    started_wall = time.time()
    width = len(str(len(files)))

    try:
        for index, path in enumerate(files, start=1):
            prefix = f"[{index:>{width}}/{len(files)}]"
            label = path.name

            try:
                raw = path.read_bytes()
            except OSError as exc:
                results.append(Result(path, "failed", f"could not read file: {exc}"))
                print(f"{prefix} FAIL {label}  cannot read file: {exc}")
                continue

            if not raw:
                results.append(Result(path, "invalid", "file is empty"))
                print(f"{prefix} FAIL {label}  file is empty")
                continue

            if len(raw) > MAX_IGC_UPLOAD_BYTES:
                results.append(Result(path, "invalid", f"file is larger than {MAX_IGC_UPLOAD_BYTES} bytes"))
                print(f"{prefix} FAIL {label}  larger than the server's 50 MiB limit")
                continue

            if not looks_like_igc(raw) and not args.force:
                results.append(Result(path, "invalid", "does not start with an IGC A record"))
                print(f"{prefix} FAIL {label}  not an IGC file (use --force to try anyway)")
                continue

            try:
                metadata = client.process_igc(raw)
            except InvalidIgc as exc:
                results.append(Result(path, "invalid", str(exc)))
                print(f"{prefix} FAIL {label}  {exc}")
                continue

            draft = build_flight(metadata, raw, glider_id=args.glider, hikeandfly=args.hikeandfly)
            summary = describe(metadata, draft)

            if draft.launch_stamp and draft.launch_stamp in existing:
                results.append(Result(path, "existing", "flight with this launch time already logged", draft.payload))
                print(f"{prefix} SKIP {label}  already logged ({summary})")
                continue

            encoded_size = len(draft.payload["igcData"])
            if encoded_size > JSON_LIMIT_BYTES:
                draft.warnings.append(
                    f"base64 payload is {encoded_size / 1024 / 1024:.1f} MiB, above the default "
                    "10 MiB JSON limit; the upload will probably fail"
                )

            if args.dry_run:
                results.append(Result(path, "dry-run", summary, draft.payload, draft.warnings))
                print(f"{prefix} DRY  {label}  would create: {summary}")
            else:
                try:
                    client.add_flight(draft.payload)
                except RequestRejected as exc:
                    results.append(Result(path, "failed", str(exc), draft.payload, draft.warnings))
                    print(f"{prefix} FAIL {label}  {exc}")
                    continue
                results.append(Result(path, "uploaded", summary, draft.payload, draft.warnings))
                print(f"{prefix} OK   {label}  {summary}")
                if draft.launch_stamp:
                    existing.add(draft.launch_stamp)  # also catch duplicates inside this batch

            for warning in draft.warnings:
                print(f"           note: {warning}")

    except KeyboardInterrupt:
        print("\nInterrupted.", file=sys.stderr)
    except NotAuthenticated as exc:
        print(f"\nerror: {exc}", file=sys.stderr)

    # Summary
    elapsed = time.monotonic() - started
    counts = {status: 0 for status in ("uploaded", "existing", "dry-run", "invalid", "failed")}
    for result in results:
        counts[result.status] = counts.get(result.status, 0) + 1
    not_processed = len(files) - len(results)

    print()
    print(
        "Summary: "
        + ", ".join(f"{count} {status}" for status, count in counts.items() if count)
        + (f", {not_processed} not processed" if not_processed else "")
        + f"  ({elapsed:.1f}s)"
    )

    problems = [r for r in results if r.status in ("failed", "invalid")]
    if problems:
        print()
        print("Problems:")
        for result in problems:
            print(f"  - {result.path}: {result.detail}")

    if args.report:
        report = {
            "server": api_base,
            "user": username,
            "dryRun": args.dry_run,
            "startedAt": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime(started_wall)),
            "durationSeconds": round(elapsed, 1),
            "results": [
                {
                    "file": str(result.path),
                    "status": result.status,
                    "detail": result.detail,
                    "warnings": result.warnings,
                    "flight": None
                    if result.flight is None
                    else {key: value for key, value in result.flight.items() if key != "igcData"},
                }
                for result in results
            ],
        }
        Path(args.report).expanduser().write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"Report written to {args.report}")

    if problems or not_processed:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
