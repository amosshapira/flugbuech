#!/usr/bin/env python3
"""Delete flights from a Flugbuech instance - all of them, or a selected subset.

This uses the same API the web UI uses (https://github.com/dbrgn/flugbuech):

  GET    /api/v1/flights                  -> all flights of the logged-in user
  GET    /api/v1/flights/<id>/igc         -> the stored IGC file (for backups)
  DELETE /api/v1/flights/<id>             -> 204 No Content, or 404 if already gone
  POST   /api/v1/auth/login               -> 204 + session cookie

**There is no undo and no server-side bulk delete.** This script therefore

  * is a dry run unless you pass --yes,
  * prints every flight it is about to remove,
  * requires you to type `delete <count>` interactively (or --yes for scripts),
  * can back the IGC files up locally first with --backup DIR,
  * never touches flights that are not selected by your filters.

Requirements: Python 3.8+ and nothing outside the standard library.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import Any, Sequence

from flugbuech_api import (
    Client,
    FlugbuechError,
    NotAuthenticated,
    RequestRejected,
    add_connection_arguments,
    api_base_from_url,
    client_from_args,
    prompt_credentials,
    tail,
)

_DATE_RE = re.compile(r"^(\d{4})-(\d{2})-(\d{2})")


# --------------------------------------------------------------------------- #
# Selection
# --------------------------------------------------------------------------- #


def iso_date(value: str) -> date:
    try:
        return date.fromisoformat(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"Expected a date as YYYY-MM-DD, got {value!r}") from None


def id_list(value: str) -> list[int]:
    ids: list[int] = []
    for chunk in value.replace(" ", "").split(","):
        if not chunk:
            continue
        try:
            ids.append(int(chunk))
        except ValueError:
            raise argparse.ArgumentTypeError(f"Expected comma-separated flight IDs, got {value!r}") from None
    return ids


def launch_date(flight: dict[str, Any]) -> date | None:
    value = flight.get("launchTime")
    if not isinstance(value, str):
        return None
    match = _DATE_RE.match(value)
    if not match:
        return None
    return date(int(match.group(1)), int(match.group(2)), int(match.group(3)))


def sort_key(flight: dict[str, Any]) -> tuple[str, int]:
    """Oldest first; flights without a launch time sort before everything else."""
    return (str(flight.get("launchTime") or ""), int(flight.get("id") or 0))


def select_flights(flights: list[dict[str, Any]], args: argparse.Namespace) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []
    for flight in flights:
        if args.ids and flight.get("id") not in args.ids:
            continue

        when = launch_date(flight)
        if args.before is not None and (when is None or when >= args.before):
            continue
        if args.after is not None and (when is None or when <= args.after):
            continue

        selected.append(flight)

    if args.keep_newest is not None:
        selected.sort(key=sort_key)
        if args.keep_newest > 0:
            selected = selected[: -args.keep_newest]

    return selected


# --------------------------------------------------------------------------- #
# Output helpers
# --------------------------------------------------------------------------- #


def format_when(flight: dict[str, Any]) -> str:
    value = flight.get("launchTime")
    if not isinstance(value, str) or len(value) < 16:
        return "unknown date"
    # 2024-05-01T13:42:26+00:00 -> 2024-05-01 13:42
    return f"{value[0:10]} {value[11:16]}"


def format_flight(flight: dict[str, Any]) -> str:
    parts = [f'#{flight.get("id")}', format_when(flight)]
    if flight.get("number") is not None:
        parts.append(f'no. {flight["number"]}')
    if flight.get("trackDistance") is not None:
        parts.append(f'{float(flight["trackDistance"]):.2f} km')
    if flight.get("gliderName"):
        parts.append(str(flight["gliderName"]))
    if flight.get("comment"):
        parts.append(tail(flight["comment"], 40))
    if not flight.get("hasIgc"):
        parts.append("no IGC")
    return "  ".join(parts)


# --------------------------------------------------------------------------- #
# Backup
# --------------------------------------------------------------------------- #


@dataclass
class BackupResult:
    directory: Path
    saved: list[int] = field(default_factory=list)
    skipped: list[int] = field(default_factory=list)
    failed: dict[int, str] = field(default_factory=dict)


def backup_flights(client: Client, flights: list[dict[str, Any]], directory: Path) -> BackupResult:
    """Store a manifest plus every IGC file of the selected flights."""
    directory = directory.expanduser()
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "flights.json").write_text(
        json.dumps(
            {
                "createdAt": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime()),
                "server": client.api_base,
                "flights": flights,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )

    result = BackupResult(directory=directory)
    width = len(str(len(flights)))
    for index, flight in enumerate(flights, start=1):
        flight_id = int(flight["id"])
        if not flight.get("hasIgc"):
            result.skipped.append(flight_id)
            continue
        try:
            data = client.download_igc(flight_id)
        except (FlugbuechError, NotAuthenticated) as exc:
            result.failed[flight_id] = str(exc)
            print(f"[{index:>{width}}/{len(flights)}] BACKUP FAIL #{flight_id}  {exc}")
            continue
        if data is None:
            result.skipped.append(flight_id)
            continue
        (directory / f"flight{flight_id}.igc").write_bytes(data)
        result.saved.append(flight_id)
        print(f"[{index:>{width}}/{len(flights)}] BACKUP OK   #{flight_id} -> flight{flight_id}.igc ({len(data)} bytes)")

    return result


# --------------------------------------------------------------------------- #
# Confirmation
# --------------------------------------------------------------------------- #


def confirm(count: int, username: str) -> bool:
    print()
    print(f"You are logged in as {username!r} and are about to PERMANENTLY delete {count} flight(s).")
    print("There is no undo in Flugbuech. IGC files are gone unless you backed them up.")
    answer = input(f'Type "delete {count}" to continue, anything else to abort: ').strip()
    return answer == f"delete {count}"


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #


def parse_args(argv: Sequence[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="delete_all_flights.py",
        description="Delete flights from a Flugbuech flight book (dry run unless --yes is given).",
        epilog=(
            "examples:\n"
            "  ./delete_all_flights.py                                  # dry run: show what would be deleted\n"
            "  ./delete_all_flights.py --backup ~/flugbuech-backup      # dry run + backup plan preview\n"
            "  ./delete_all_flights.py --backup ~/flugbuech-backup --yes\n"
            "  ./delete_all_flights.py --keep-newest 10 --yes           # delete everything except the 10 newest\n"
            "  ./delete_all_flights.py --before 2020-01-01 --yes        # only old flights\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_connection_arguments(parser)
    parser.add_argument("--yes", "-y", action="store_true", help="actually delete (without this it is a dry run)")
    parser.add_argument(
        "--keep-newest",
        type=int,
        metavar="N",
        help="keep the N newest flights of the selection (applied after the other filters)",
    )
    parser.add_argument("--before", type=iso_date, metavar="YYYY-MM-DD", help="only delete flights launched before this date")
    parser.add_argument("--after", type=iso_date, metavar="YYYY-MM-DD", help="only delete flights launched after this date")
    parser.add_argument("--ids", type=id_list, metavar="ID[,ID...]", help="only delete these flight IDs")
    parser.add_argument("--backup", metavar="DIR", help="download manifest + IGC files to DIR before deleting")
    parser.add_argument(
        "--confirm-count",
        type=int,
        metavar="N",
        help="abort unless exactly N flights are selected (guards against changed data)",
    )
    parser.add_argument("--pause", type=float, default=0.0, metavar="SECONDS", help="sleep between deletions (default: 0)")
    parser.add_argument("--report", metavar="FILE", help="write a JSON report of the run to FILE")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)

    if args.keep_newest is not None and args.keep_newest < 0:
        print("error: --keep-newest must not be negative", file=sys.stderr)
        return 2
    if args.pause < 0:
        print("error: --pause must not be negative", file=sys.stderr)
        return 2

    try:
        api_base = api_base_from_url(args.url)
        client: Client = client_from_args(args)
        username, password = prompt_credentials(args.username, args.password)
    except FlugbuechError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    print(f"Server: {api_base}")
    print(f"User:   {username}")
    print()

    try:
        client.login(username, password)
        every_flight = client.list_flights().get("flights", [])
    except FlugbuechError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    selected = select_flights(list(every_flight), args)
    print(f"Flights on the account: {len(every_flight)}")
    print(f"Selected for deletion:  {len(selected)}")
    print()

    if not selected:
        print("Nothing to do.")
        if args.report:
            Path(args.report).expanduser().write_text(
                json.dumps({"server": api_base, "user": username, "deleted": [], "selected": 0}, indent=2) + "\n",
                encoding="utf-8",
            )
        return 0

    if args.confirm_count is not None and args.confirm_count != len(selected):
        print(
            f"error: --confirm-count {args.confirm_count} does not match the {len(selected)} selected flights; aborting",
            file=sys.stderr,
        )
        return 2

    for flight in selected:
        print(f"  {format_flight(flight)}")

    if not args.yes:
        print()
        print("Dry run: nothing was deleted.")
        print("Re-run with --yes (and --backup DIR if you want the IGC files) to delete these flights.")
        if args.backup:
            print(f"With --yes, IGC files would be written to {Path(args.backup).expanduser()}")
        return 0

    try:
        if sys.stdin.isatty():
            if not confirm(len(selected), username):
                print("Aborted, nothing deleted.")
                return 0
        else:
            print()
            print("Non-interactive session: proceeding because --yes was given.")
    except (EOFError, KeyboardInterrupt):
        print("\nAborted, nothing deleted.")
        return 0

    # Backup before any deletion. Flights whose IGC cannot be saved are skipped.
    backup: BackupResult | None = None
    blocked: set[int] = set()
    if args.backup:
        print()
        print(f"Backing up IGC files to {Path(args.backup).expanduser()} ...")
        try:
            backup = backup_flights(client, selected, Path(args.backup))
        except OSError as exc:
            print(f"error: backup failed, nothing deleted: {exc}", file=sys.stderr)
            return 2
        except NotAuthenticated as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 2

        if backup.failed:
            blocked = set(backup.failed)
            print()
            print(
                f"warning: {len(blocked)} flight(s) could not be backed up and will NOT be deleted "
                f"(IDs: {', '.join(str(i) for i in sorted(blocked))})"
            )
        print(
            f"Backup complete: {len(backup.saved)} IGC file(s), "
            f"{len(backup.skipped)} flight(s) without IGC, {len(backup.failed)} failed"
        )

    # Delete
    print()
    width = len(str(len(selected)))
    deleted: list[int] = []
    failed: dict[int, str] = {}
    blocked_by_backup = 0
    aborted_after = 0

    try:
        for index, flight in enumerate(selected, start=1):
            flight_id = int(flight["id"])
            prefix = f"[{index:>{width}}/{len(selected)}]"

            if flight_id in blocked:
                blocked_by_backup += 1
                print(f"{prefix} SKIP #{flight_id}  not deleted because the backup failed")
                continue

            try:
                client.delete_flight(flight_id)
            except NotAuthenticated as exc:
                aborted_after = len(selected) - index + 1
                print(f"\nerror: {exc}", file=sys.stderr)
                break
            except RequestRejected as exc:
                failed[flight_id] = str(exc)
                print(f"{prefix} FAIL #{flight_id}  {exc}")
                continue

            deleted.append(flight_id)
            print(f"{prefix} DEL  #{flight_id}  {format_when(flight)}")
            if args.pause:
                time.sleep(args.pause)

    except KeyboardInterrupt:
        aborted_after = len(selected) - len(deleted) - len(failed) - blocked_by_backup
        print("\nInterrupted. Some flights were already deleted (see above).", file=sys.stderr)

    # Summary
    print()
    summary = f"Deleted {len(deleted)} of {len(selected)} selected flight(s)"
    extras = []
    if failed:
        extras.append(f"{len(failed)} failed")
    if blocked_by_backup:
        extras.append(f"{blocked_by_backup} skipped (backup failed)")
    if aborted_after:
        extras.append(f"{aborted_after} not attempted")
    if extras:
        summary += " (" + ", ".join(extras) + ")"
    print(summary + ".")

    if failed:
        print()
        print("Failed:")
        for flight_id, message in failed.items():
            print(f"  - #{flight_id}: {message}")

    if backup:
        print(
            f"Backup: {len(backup.saved)} IGC file(s) and a manifest in {backup.directory}"
        )

    if args.report:
        report = {
            "server": api_base,
            "user": username,
            "createdAt": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime()),
            "selected": len(selected),
            "deleted": deleted,
            "failed": {str(key): value for key, value in failed.items()},
            "skippedBackupFailed": sorted(blocked),
            "notAttempted": aborted_after,
            "backupDirectory": str(backup.directory) if backup else None,
            "filters": {
                "before": args.before.isoformat() if args.before else None,
                "after": args.after.isoformat() if args.after else None,
                "keepNewest": args.keep_newest,
                "ids": args.ids,
            },
        }
        Path(args.report).expanduser().write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"Report written to {args.report}")

    if failed or blocked_by_backup or aborted_after:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
