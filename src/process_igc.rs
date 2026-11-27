//! Process IGC files and extract relevant information.

use std::io::{self, BufRead, BufReader, Cursor};

use flat_projection::{FlatPoint, FlatProjection};
use igc::{
    records::{HRecord, Record},
    util::RawPosition,
};
use num_traits::Float;
use rocket::{
    data::{Data, ToByteUnit},
    post, routes,
    serde::json::Json,
    Route,
};
use serde::Serialize;

use crate::{auth, data, models};

#[derive(Debug, PartialEq, Clone, Serialize)]
struct LatLng {
    lat: f64,
    lng: f64,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchLandingInfo {
    pub(crate) pos: LatLng,
    pub(crate) alt: i16,
    pub(crate) time_hms: (u8, u8, u8),
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) location_id: Option<i32>,
}

#[derive(Default, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightInfo {
    /// Name of the pilot, as configured in the flight instrument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pilot: Option<String>,
    /// Name of the pilot in command (P1), from the HFPLTPILOTINCHARGE header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pilot_in_command: Option<String>,
    /// Name of the second crew member (P2), from the HFCM2CREW2 header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) second_pilot: Option<String>,
    /// Name of the glider, as configured in the flight instrument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) glidertype: Option<String>,
    /// Glider registration / ID, from the HFGIDGLIDERID header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gliderid: Option<String>,
    /// Name of the launch site, as configured in the flight instrument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) site: Option<String>,
    /// Date of flight (YYYY, MM, DD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) date_ymd: Option<(u16, u8, u8)>,
    /// Lauch infos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) launch: Option<LaunchLandingInfo>,
    /// Landing infos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) landing: Option<LaunchLandingInfo>,
    /// Track length in kilometers.
    pub(crate) track_distance: f64,
    /// Maximum barometric altitude above mean sea level, in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_altitude_m: Option<i32>,
    /// Barometric height gain from launch to the highest point, in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) height_gain_m: Option<i32>,
    /// Whether the flight landed away from a known location.
    pub(crate) is_outlanding: bool,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum FlightInfoResult {
    Success(FlightInfo),
    Error { msg: String },
}

#[derive(Debug, PartialEq, Clone)]
struct FlatPointString<T>(Vec<FlatPoint<T>>);

impl<T: Float> FlatPointString<T> {
    fn new() -> Self {
        Self(vec![])
    }

    fn add_point(&mut self, point: FlatPoint<T>) {
        self.0.push(point);
    }

    /// Return the flight path length in km.
    fn length(&self) -> T {
        self.0
            .windows(2)
            .map(|pair| pair[0].distance(&pair[1]))
            .fold(T::zero(), |total, distance| total + distance)
    }
}

fn parse_igc(reader: impl BufRead, user: &models::User, db: &mut diesel::PgConnection) -> FlightInfoResult {
    log::info!("Parsing IGC file for user {}", user.id);

    // Split lines in IGC file
    //
    // NOTE: This will yield a vector of Vec<u8>. We cannot use `.lines()`
    //       directly since that will fail if the data is invalid UTF8, and we
    //       want to parse leniently in case of invalid UTF8 data.
    //       Unfortunately when splitting by '\n' there can still be remaining
    //       '\r' characters that must be trimmed later.
    let lines = match reader.split(b'\n').collect::<Result<Vec<Vec<u8>>, io::Error>>() {
        Ok(res) => res,
        Err(e) => {
            return FlightInfoResult::Error {
                msg: format!("I/O Error: {}", e),
            }
        }
    };

    // Prepare FlightInfo instance
    let mut info = FlightInfo::default();

    // Vector to collect track coordinates projected from WGS84 into a
    // cartesian coordinate system
    let mut projection: Option<FlatProjection<f64>> = None;
    let mut flight_path = FlatPointString::new();

    // Barometric altitude tracking for soaring metrics
    let mut launch_altitude_m: Option<i16> = None;
    let mut max_altitude_m: Option<i16> = None;

    for line_bytes in &lines {
        let line = String::from_utf8_lossy(line_bytes);
        match Record::parse_line(line.trim()) {
            Ok(Record::H(h @ HRecord { mnemonic: "PLT", .. })) => {
                let value = h.data.trim();
                if !is_present(value) {
                    // "NIL" means "not available"
                } else if h.friendly_name == Some("PILOTINCHARGE") {
                    info.pilot_in_command = Some(value.into());
                } else {
                    info.pilot = Some(value.into());
                }
            }
            Ok(Record::H(h @ HRecord { mnemonic: "CM2", .. })) => {
                let value = h.data.trim();
                if h.friendly_name == Some("CREW2") && is_present(value) {
                    info.second_pilot = Some(value.into());
                }
            }
            Ok(Record::H(h @ HRecord { mnemonic: "GTY", .. })) => {
                info.glidertype = Some(h.data.trim().into());
            }
            Ok(Record::H(h @ HRecord { mnemonic: "GID", .. })) => {
                info.gliderid = Some(h.data.trim().into());
            }
            Ok(Record::H(h @ HRecord { mnemonic: "SIT", .. })) => {
                info.site = Some(h.data.trim().into());
            }
            Ok(Record::H(h @ HRecord { mnemonic: "DTE", .. })) => {
                let string_val = h.data.trim();
                // Date formats:
                // - Skytraxx: DDMMYY
                // - XCTrack:  DDMMYY,[??]
                if string_val.len() == 6 || (string_val.len() == 9 && &string_val[6..7] == ",") {
                    if let (Ok(day), Ok(month), Ok(year)) = (
                        string_val.get(0..2).unwrap().parse::<u8>(),
                        string_val.get(2..4).unwrap().parse::<u8>(),
                        string_val.get(4..6).unwrap().parse::<u16>(),
                    ) {
                        let year = if year > 85 { 1900 + year } else { 2000 + year };
                        info.date_ymd = Some((year, month, day));
                    }
                } else {
                    log::warn!("Unexpected H record DTE format: {:?}", h);
                }
            }
            Ok(Record::B(b)) => {
                // Track barometric altitude for soaring metrics
                if launch_altitude_m.is_none() {
                    launch_altitude_m = Some(b.pressure_alt);
                }
                max_altitude_m = Some(
                    max_altitude_m
                        .map(|altitude| altitude.max(b.pressure_alt))
                        .unwrap_or(b.pressure_alt),
                );

                // Extract raw float coordinates
                let RawPosition {
                    lat: raw_lat,
                    lon: raw_lng,
                } = b.pos;
                let lat = raw_lat.into();
                let lng = raw_lng.into();
                let pos = LatLng { lat, lng };

                // Initialize projection with the first coordinate in the track
                if projection.is_none() {
                    projection = Some(FlatProjection::new(lng, lat));
                }

                // Project the coordinate onto a flat coordinate system
                let flat_point = projection.unwrap().project(lng, lat);
                flight_path.add_point(flat_point);

                // TODO: More elaborate launch detection using altitude
                if info.launch.is_none() {
                    // Create launch info
                    info.launch = Some(LaunchLandingInfo {
                        pos,
                        alt: b.gps_alt,
                        time_hms: (b.timestamp.hours, b.timestamp.minutes, b.timestamp.seconds),
                        location_id: None,
                    });
                } else {
                    info.landing = Some(LaunchLandingInfo {
                        pos,
                        alt: b.gps_alt,
                        time_hms: (b.timestamp.hours, b.timestamp.minutes, b.timestamp.seconds),
                        location_id: None,
                    });
                }
            }
            Ok(_rec) => {}
            Err(e) => {
                return FlightInfoResult::Error {
                    msg: format!("Error parsing lines: {:?}", e),
                }
            }
        }
    }
    info.track_distance = flight_path.length();
    info.max_altitude_m = max_altitude_m.map(i32::from);
    info.height_gain_m = match (max_altitude_m, launch_altitude_m) {
        (Some(max), Some(launch)) => Some(i32::from(max.saturating_sub(launch))),
        _ => None,
    };

    // Find locations within 1000 meters of launch and landing
    let max_distance = 1000.0;
    if let Some(ref mut launch) = info.launch {
        launch.location_id =
            data::get_locations_around_point(db, user, launch.pos.lat, launch.pos.lng, max_distance)
                .get(0)
                .map(|location| location.id);
    }
    if let Some(ref mut landing) = info.landing {
        landing.location_id =
            data::get_locations_around_point(db, user, landing.pos.lat, landing.pos.lng, max_distance)
                .get(0)
                .map(|location| location.id);
    }

    // A landing away from any known location is considered an outlanding.
    info.is_outlanding = info
        .landing
        .as_ref()
        .is_some_and(|landing| landing.location_id.is_none());

    FlightInfoResult::Success(info)
}

/// Parse in-memory IGC data for the specified user, return parsed data.
/// Whether an IGC header value carries actual data. "NIL" is the IGC
/// convention for "not available".
fn is_present(value: &str) -> bool {
    !value.eq_ignore_ascii_case("NIL")
}

pub fn parse_igc_data(data: &[u8], user: &models::User, db: &mut diesel::PgConnection) -> FlightInfoResult {
    let buf_reader = BufReader::new(Cursor::new(data));
    parse_igc(buf_reader, user, db)
}

/// Extract all B record fixes of an IGC file as geographic coordinates.
///
/// Returns a vector of `[longitude, latitude]` pairs (GeoJSON coordinate
/// order). Hemisphere signs (N/S/E/W) are handled by the `igc` crate.
///
/// Returns `None` if the data cannot be read or contains no B record fixes
/// (e.g. because it is not a valid IGC file).
pub fn extract_igc_track(data: &[u8]) -> Option<Vec<[f64; 2]>> {
    // Split lines in IGC file, leniently regarding invalid UTF8 data and
    // remaining '\r' characters (see `parse_igc`).
    let lines = BufReader::new(Cursor::new(data))
        .split(b'\n')
        .collect::<Result<Vec<Vec<u8>>, io::Error>>()
        .ok()?;

    let mut coordinates = Vec::new();
    for line_bytes in &lines {
        let line = String::from_utf8_lossy(line_bytes);
        if let Ok(Record::B(fix)) = Record::parse_line(line.trim()) {
            let lat = f64::from(fix.pos.lat);
            let lng = f64::from(fix.pos.lon);
            coordinates.push([lng, lat]);
        }
    }

    if coordinates.is_empty() {
        None
    } else {
        Some(coordinates)
    }
}

/// Process IGC file, return parsed data.
#[post(
    "/flights/add/process_igc",
    format = "application/octet-stream",
    data = "<data>"
)]
pub async fn process_igc(
    user: auth::AuthUser,
    database: data::Database,
    data: Data<'_>,
) -> Json<FlightInfoResult> {
    let user = user.into_inner();

    // Open IGC file
    let igc_bytes = match data.open(crate::MAX_IGC_UPLOAD_BYTES.bytes()).into_bytes().await {
        Ok(capped_vec) if capped_vec.is_complete() => capped_vec.into_inner(),
        Ok(_) => {
            return Json(FlightInfoResult::Error {
                msg: "Too many bytes received while reading IGC data".into(),
            })
        }
        Err(e) => {
            log::error!("Error while reading IGC data: {}", e);
            return Json(FlightInfoResult::Error {
                msg: "Error while reading IGC data".into(),
            });
        }
    };
    let buf_reader = BufReader::new(Cursor::new(igc_bytes));

    // Process data
    Json(database.run(move |db| parse_igc(buf_reader, &user, db)).await)
}

/// Return vec of all API routes.
pub fn api_routes() -> Vec<Route> {
    routes![process_igc]
}

#[cfg(test)]
mod tests {
    use crate::test_utils::DbTestContext;

    use super::*;

    fn process(data: &str) -> Result<FlightInfo, String> {
        let ctx = DbTestContext::new();
        let reader = BufReader::new(Cursor::new(data));
        let result = parse_igc(reader, &ctx.testuser1.user, &mut ctx.force_get_conn());
        match result {
            FlightInfoResult::Success(info) => Ok(info),
            FlightInfoResult::Error { msg } => Err(msg),
        }
    }

    #[test]
    fn parse_simple_igc() {
        let data = include_str!("../testdata/skytraxx.igc");
        let info = process(data).unwrap();
        assert_eq!(info.pilot, Some("Danilo".to_string()));
        assert_eq!(info.glidertype, Some("Epsilon 8".to_string()));
        assert_eq!(info.site, Some("Hitzeggen".to_string()));
        assert_eq!(info.date_ymd, Some((2019, 7, 22)));
        assert_eq!(
            info.launch,
            Some(LaunchLandingInfo {
                pos: LatLng {
                    lat: 46.71985,
                    lng: 9.149533333333334
                },
                alt: 1568,
                time_hms: (13, 42, 26),
                location_id: None
            })
        );
        assert_eq!(
            info.landing,
            Some(LaunchLandingInfo {
                pos: LatLng {
                    lat: 46.70665,
                    lng: 9.153933333333333,
                },
                alt: 1300,
                time_hms: (13, 46, 7),
                location_id: None,
            })
        );
        assert!(
            info.track_distance > 1.98988,
            "Track distance is {:?}, not >1.98988",
            info.track_distance
        );
        assert!(
            info.track_distance < 1.98989,
            "Track distance is {:?}, not <1.98989",
            info.track_distance
        );
        assert_eq!(info.max_altitude_m, Some(1455));
        assert_eq!(info.height_gain_m, Some(0));
    }

    /// Parse sailplane crew headers and compute barometric altitude metrics.
    #[test]
    fn parse_sailplane_crew_and_altitudes() {
        let data = "\
HFGTYGLIDERTYPE: ASK 21
HFGIDGLIDERID: VH-ASK
HFPLTPILOTINCHARGE: Alex Doe
HFCM2CREW2: Sam Instructor
B1200004800000N01100000EA0100001500
B1201004800000N01100000EA0160001600
B1202004800000N01100000EA0140001400";
        let info = process(data).unwrap();
        assert_eq!(info.pilot, None);
        assert_eq!(info.glidertype, Some("ASK 21".to_string()));
        assert_eq!(info.gliderid, Some("VH-ASK".to_string()));
        assert_eq!(info.pilot_in_command, Some("Alex Doe".to_string()));
        assert_eq!(info.second_pilot, Some("Sam Instructor".to_string()));
        assert_eq!(info.max_altitude_m, Some(1600));
        assert_eq!(info.height_gain_m, Some(600));
        assert!(
            info.is_outlanding,
            "Unknown landing location must be an outlanding"
        );
    }

    /// "NIL" header values (IGC convention for "not available", e.g. written
    /// by Naviter Oudie) must not be stored as crew names.
    #[test]
    fn parse_nil_crew_headers() {
        let data = "\
HFPLTPILOTINCHARGE: Alex Doe
HFCM2CREW2:NIL
B1200004800000N01100000EA0100001500";
        let info = process(data).unwrap();
        assert_eq!(info.pilot_in_command, Some("Alex Doe".to_string()));
        assert_eq!(info.second_pilot, None);
    }

    /// Extract B record fixes as GeoJSON coordinates, including southern /
    /// western hemispheres, and ignoring lines without fixes.
    #[test]
    fn extract_track_coordinates() {
        let data = "\
HFPLTPILOTINCHARGE: Alex Doe
B1000004800000N01100000EA0100001000
B1001004801000S01100000WA0200002000
A000XXX";
        let track = extract_igc_track(data.as_bytes()).expect("track coordinates");
        assert_eq!(track.len(), 2);
        assert_eq!(track[0], [11.0, 48.0]);
        assert_eq!(track[1][0], -11.0);
        assert!((track[1][1] - -(48.0 + 1000.0 / 60_000.0)).abs() < 1e-9);
    }

    /// Data without any B record fixes yields no track.
    #[test]
    fn extract_track_coordinates_none() {
        assert_eq!(extract_igc_track(b"HFPLTPILOTINCHARGE: Alex Doe"), None);
        assert_eq!(extract_igc_track(b""), None);
    }

    /// Parse IGC data with only two lines: pilot name and site.
    #[test]
    fn parse_minimal() {
        let data = "HFPLTPILOT: Chrigel Maurer\nHPSITSITE: Interlaken";
        let info = process(data).unwrap();
        assert_eq!(
            info,
            FlightInfo {
                pilot: Some("Chrigel Maurer".to_string()),
                site: Some("Interlaken".to_string()),
                ..Default::default()
            }
        );
    }

    /// Parse IGC data with windows line endings.
    #[test]
    fn regression_27_cr_endings() {
        let data = "HFPLTPILOT: Chrigel Maurer\r\nI023636LAD3737LOD\r\n";
        let info = process(data).unwrap();
        assert_eq!(
            info,
            FlightInfo {
                pilot: Some("Chrigel Maurer".to_string()),
                ..Default::default()
            }
        );
    }

    /// Handle XCTrack date format.
    #[test]
    fn regression_30_xctrack_date_format() {
        let data = "HFDTEDATE:280719,02";
        let info = process(data).unwrap();
        assert_eq!(
            info,
            FlightInfo {
                date_ymd: Some((2019, 7, 28)),
                ..Default::default()
            }
        );
    }
}
