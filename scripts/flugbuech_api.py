"""Shared HTTP client for the Flugbuech REST API.

Used by `upload_igc.py` and `delete_all_flights.py`. Standard library only.

Flugbuech (https://github.com/dbrgn/flugbuech) authenticates with plain cookie
sessions, so every script here logs in once and then reuses the cookie jar.
"""

from __future__ import annotations

import argparse
import getpass
import http.cookiejar
import json
import os
import re
import ssl
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any, Sequence

USER_AGENT = "flugbuech-tools/1.0"
DEFAULT_URL = "https://flights.home"

# Rocket.toml: json = "10 MiB". IGC data is inlined as base64 in the JSON body.
JSON_LIMIT_BYTES = 10 * 1024 * 1024
# src/main.rs: MAX_IGC_UPLOAD_BYTES
MAX_IGC_UPLOAD_BYTES = 50 * 1024 * 1024

_TS_RE = re.compile(r"(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}:\d{2})")


# --------------------------------------------------------------------------- #
# Errors
# --------------------------------------------------------------------------- #


class FlugbuechError(Exception):
    """A fatal problem (bad URL, unreachable server, wrong credentials, ...)."""


class NotAuthenticated(FlugbuechError):
    """The session cookie is missing or no longer valid (HTTP 401)."""


class InvalidIgc(FlugbuechError):
    """The server could not parse the IGC file."""


class RequestRejected(FlugbuechError):
    """The server refused a write request (HTTP 400/403/413/...)."""

    def __init__(self, status: int, message: str) -> None:
        super().__init__(f"HTTP {status}: {message}")
        self.status = status
        self.message = message


# Kept for backwards compatibility with the first version of upload_igc.py.
FlightRejected = RequestRejected


# --------------------------------------------------------------------------- #
# URL + cookie helpers
# --------------------------------------------------------------------------- #


class _PermissiveCookiePolicy(http.cookiejar.DefaultCookiePolicy):
    """Ignore cookie path scoping.

    Rocket creates the session cookies without an explicit `Path` attribute
    (src/auth.rs: `make_cookie`), so the RFC 6265 default path would be
    `/api/v1/auth` and the cookies would never be sent to `/api/v1/flights`.
    Since we only ever talk to one host, accept every cookie everywhere.
    """

    def return_ok(self, cookie, request):  # type: ignore[no-untyped-def]
        return True

    def domain_return_ok(self, domain, request):  # type: ignore[no-untyped-def]
        return True

    def path_return_ok(self, path, request):  # type: ignore[no-untyped-def]
        return True


def api_base_from_url(url: str) -> str:
    """Normalise a site/paste URL into the API root, e.g. https://flights.home/api/v1."""
    url = url.strip().rstrip("/")
    if url.endswith("/api/v1"):
        return url
    parts = urllib.parse.urlsplit(url)
    if not parts.scheme or not parts.netloc:
        raise FlugbuechError(f"Invalid URL: {url!r} (expected something like https://flights.home)")
    path = parts.path
    for suffix in ("/flights", "/index.html", "/api"):
        if path.endswith(suffix):
            path = path[: -len(suffix)]
    return urllib.parse.urlunsplit((parts.scheme, parts.netloc, path.rstrip("/") + "/api/v1", "", ""))


def api_error_message(payload: bytes) -> str:
    """Extract `code reason: description` from a Flugbuech error body."""
    text = payload.decode("utf-8", "replace").strip()
    try:
        data = json.loads(text)
    except ValueError:
        return " ".join(text.split())[:300] or "(empty response)"
    error = data.get("error") if isinstance(data, dict) else None
    if isinstance(error, dict):
        reason = error.get("reason") or "Error"
        description = error.get("description") or ""
        code = error.get("code")
        prefix = f"{code} " if code else ""
        return f"{prefix}{reason}: {description}".strip()
    return " ".join(text.split())[:300]


def normalize_timestamp(value: Any) -> str | None:
    """Turn `2024-05-01T13:42:26+00:00` / `...Z` / `... .123Z` into `2024-05-01T13:42:26`."""
    if not isinstance(value, str):
        return None
    match = _TS_RE.match(value)
    return f"{match.group(1)}T{match.group(2)}" if match else None


# --------------------------------------------------------------------------- #
# Client
# --------------------------------------------------------------------------- #


class Client:
    def __init__(
        self,
        api_base: str,
        *,
        insecure: bool = False,
        ca_bundle: str | None = None,
        timeout: float = 180.0,
        retries: int = 2,
        verbose: bool = False,
    ) -> None:
        self.api_base = api_base.rstrip("/")
        self.timeout = timeout
        self.retries = max(0, retries)
        self.verbose = verbose

        context = ssl.create_default_context(cafile=ca_bundle)
        if insecure:
            context.check_hostname = False
            context.verify_mode = ssl.CERT_NONE

        self.jar = http.cookiejar.CookieJar(_PermissiveCookiePolicy())
        self.opener = urllib.request.build_opener(
            urllib.request.HTTPSHandler(context=context),
            urllib.request.HTTPCookieProcessor(self.jar),
        )

    # -- low level ---------------------------------------------------------- #

    def _request(self, method: str, path: str, *, body: bytes | None = None, content_type: str | None = None) -> tuple[int, bytes]:
        url = self.api_base + path
        headers = {"User-Agent": USER_AGENT, "Accept": "application/json, text/plain, */*;q=0.8"}
        if content_type is not None:
            headers["Content-Type"] = content_type

        attempt = 0
        while True:
            if self.verbose:
                size = f" ({len(body)} bytes)" if body is not None else ""
                print(f"    -> {method} {url}{size}", file=sys.stderr, flush=True)

            request = urllib.request.Request(url, data=body, headers=headers, method=method)
            try:
                with self.opener.open(request, timeout=self.timeout) as response:
                    return response.status, response.read()
            except urllib.error.HTTPError as exc:
                payload = exc.read()
                if exc.code >= 500 and attempt < self.retries:
                    attempt += 1
                    time.sleep(2 ** (attempt - 1))
                    continue
                return exc.code, payload
            except (urllib.error.URLError, TimeoutError, OSError) as exc:
                reason = getattr(exc, "reason", exc)
                if isinstance(reason, ssl.SSLCertVerificationError):
                    raise FlugbuechError(
                        f"TLS certificate for {url} could not be verified ({reason}).\n"
                        "If this instance uses a self-signed certificate, re-run with --insecure "
                        "or point --ca-bundle at the CA certificate."
                    ) from exc
                if attempt < self.retries:
                    attempt += 1
                    time.sleep(2 ** (attempt - 1))
                    continue
                raise FlugbuechError(f"Could not reach {url}: {reason}") from exc

    # -- authentication ----------------------------------------------------- #

    def login(self, username: str, password: str) -> None:
        body = json.dumps({"username": username, "password": password}).encode("utf-8")
        status, payload = self._request("POST", "/auth/login", body=body, content_type="application/json")
        if status == 204:
            return
        if status in (401, 403):
            raise FlugbuechError("Login failed: wrong username or password.")
        raise FlugbuechError(f"Login failed: HTTP {status}: {api_error_message(payload)}")

    # -- flights ------------------------------------------------------------ #

    def list_flights(self) -> dict[str, Any]:
        status, payload = self._request("GET", "/flights")
        if status == 200:
            return json.loads(payload.decode("utf-8"))
        if status == 401:
            raise NotAuthenticated("Not authenticated (HTTP 401). Is the login cookie accepted?")
        raise FlugbuechError(f"Could not list flights: HTTP {status}: {api_error_message(payload)}")

    def list_gliders(self) -> list[dict[str, Any]]:
        status, payload = self._request("GET", "/gliders")
        if status != 200:
            raise FlugbuechError(f"Could not list gliders: HTTP {status}: {api_error_message(payload)}")
        try:
            return list(json.loads(payload.decode("utf-8")))
        except ValueError:
            return []

    def process_igc(self, raw: bytes) -> dict[str, Any]:
        status, payload = self._request(
            "POST", "/flights/add/process_igc", body=raw, content_type="application/octet-stream"
        )
        if status == 401:
            raise NotAuthenticated("Not authenticated (HTTP 401) while parsing IGC data.")
        if status == 413:
            raise InvalidIgc("HTTP 413: file too large for the server's upload limit.")
        if status != 200:
            raise InvalidIgc(f"HTTP {status}: {api_error_message(payload)}")

        try:
            parsed = json.loads(payload.decode("utf-8"))
        except ValueError:
            raise InvalidIgc(f"Unexpected response from process_igc: {payload[:200]!r}") from None

        if parsed.get("type") != "success":
            raise InvalidIgc(str(parsed.get("msg") or "the server could not parse this IGC file"))
        return parsed

    def add_flight(self, flight: dict[str, Any]) -> None:
        body = json.dumps(flight).encode("utf-8")
        status, payload = self._request("POST", "/flights", body=body, content_type="application/json")
        if status == 201:
            return
        if status == 401:
            raise NotAuthenticated("Not authenticated (HTTP 401) while creating the flight.")
        if status == 413:
            raise RequestRejected(
                413,
                "payload too large - the server's JSON limit is probably the default 10 MiB "
                '(Rocket.toml: json = "10 MiB"); the IGC file is inlined as base64 in the '
                "request and grows by ~33%",
            )
        raise RequestRejected(status, api_error_message(payload))

    def delete_flight(self, flight_id: int) -> None:
        """Delete a flight. Returns normally on 204 and on 404 (already gone)."""
        status, payload = self._request("DELETE", f"/flights/{flight_id}")
        if status in (204, 404):
            return
        if status == 401:
            raise NotAuthenticated("Not authenticated (HTTP 401) while deleting a flight.")
        raise RequestRejected(status, api_error_message(payload))

    def download_igc(self, flight_id: int) -> bytes | None:
        """Return the IGC bytes of a flight, or None if the flight has no IGC."""
        status, payload = self._request("GET", f"/flights/{flight_id}/igc")
        if status == 200:
            return payload
        if status == 404:
            return None
        if status == 401:
            raise NotAuthenticated("Not authenticated (HTTP 401) while downloading IGC data.")
        if status == 403:
            raise RequestRejected(403, "this flight does not belong to the logged-in user")
        raise RequestRejected(status, api_error_message(payload))


# --------------------------------------------------------------------------- #
# CLI helpers shared by both scripts
# --------------------------------------------------------------------------- #


def add_connection_arguments(parser: argparse.ArgumentParser, *, default_url: str = DEFAULT_URL) -> None:
    parser.add_argument(
        "--url",
        default=os.environ.get("FLUGBUECH_URL", default_url),
        help=f"Flight book URL (default: {default_url}); '/flights/' and '/api/v1' suffixes are stripped",
    )
    parser.add_argument(
        "-u",
        "--username",
        default=os.environ.get("FLUGBUECH_USERNAME"),
        help="username (prompted if omitted)",
    )
    parser.add_argument(
        "-p",
        "--password",
        default=None,
        help="password (prompted if omitted; prefer the prompt or FLUGBUECH_PASSWORD so it stays out of your shell history)",
    )
    parser.add_argument("--insecure", action="store_true", help="do not verify the TLS certificate (self-signed instances)")
    parser.add_argument("--ca-bundle", metavar="FILE", help="custom CA bundle to verify the server certificate")
    parser.add_argument("--timeout", type=float, default=180.0, metavar="SECONDS", help="per-request timeout (default: 180)")
    parser.add_argument("--retries", type=int, default=2, metavar="N", help="retries for network errors and 5xx (default: 2)")
    parser.add_argument("-v", "--verbose", action="store_true", help="show every HTTP request")


def client_from_args(args: argparse.Namespace) -> Client:
    return Client(
        api_base_from_url(args.url),
        insecure=args.insecure,
        ca_bundle=args.ca_bundle,
        timeout=args.timeout,
        retries=args.retries,
        verbose=args.verbose,
    )


def prompt_credentials(username: str | None, password: str | None) -> tuple[str, str]:
    if not username:
        if not sys.stdin.isatty():
            raise FlugbuechError("No username given. Pass --username or set FLUGBUECH_USERNAME.")
        username = input("Username: ").strip()
        if not username:
            raise FlugbuechError("Empty username.")

    if password is None:
        password = os.environ.get("FLUGBUECH_PASSWORD")
    if password is None:
        if not sys.stdin.isatty():
            raise FlugbuechError("No password given. Pass --password or set FLUGBUECH_PASSWORD.")
        password = getpass.getpass(f"Password for {username}: ")
    if not password:
        raise FlugbuechError("Empty password.")
    return username, password


def tail(values: Sequence[str], width: int = 60) -> str:
    """Shorten a string for table output."""
    text = " ".join(str(values).split())
    return text if len(text) <= width else text[: width - 1] + "…"
