//! Flight views.

use std::{collections::HashMap, io::Cursor, sync::Arc};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{
    naive::{NaiveDate, NaiveDateTime, NaiveTime},
    DateTime, Utc,
};
use diesel::pg::data_types::PgInterval;
use rocket::{
    delete, get,
    http::{ContentType, Header, Status},
    post,
    request::Request,
    response::{self, Responder, Response},
    routes,
    serde::json::Json,
    Route,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth, data,
    models::{Glider, Location, NewFlight, User},
    process_igc::parse_igc_data,
    responders::ApiError,
};

// API types

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiFlightLocation {
    id: i32,
    name: String,
    country_code: String,
    elevation: i32,
}

impl From<Location> for ApiFlightLocation {
    fn from(location: Location) -> Self {
        ApiFlightLocation {
            id: location.id,
            name: location.name,
            country_code: location.country,
            elevation: location.elevation,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiFlightListItem {
    /// Flight ID
    id: i32,
    /// The user-defined flight number
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<i32>,
    /// The name of the glider used
    #[serde(skip_serializing_if = "Option::is_none")]
    glider_name: Option<String>,
    /// Inlined launch location
    #[serde(skip_serializing_if = "Option::is_none")]
    launch_at: Option<i32>,
    /// Inlined landing location
    #[serde(skip_serializing_if = "Option::is_none")]
    landing_at: Option<i32>,
    /// Time of launch
    #[serde(skip_serializing_if = "Option::is_none")]
    launch_time: Option<DateTime<Utc>>,
    /// Time of landing
    #[serde(skip_serializing_if = "Option::is_none")]
    landing_time: Option<DateTime<Utc>>,
    /// Flight duration in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_seconds: Option<u64>,
    /// GPS track length
    #[serde(skip_serializing_if = "Option::is_none")]
    track_distance: Option<f32>,
    /// XContest tracktype (free_flight, flat_triangle or fai_triangle)
    #[serde(skip_serializing_if = "Option::is_none")]
    xcontest_tracktype: Option<String>,
    /// XContest distance
    #[serde(skip_serializing_if = "Option::is_none")]
    xcontest_distance: Option<f32>,
    /// XContest URL
    #[serde(skip_serializing_if = "Option::is_none")]
    xcontest_url: Option<String>,
    /// Comment your flight
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    /// Link to a video of your flight
    #[serde(skip_serializing_if = "Option::is_none")]
    video_url: Option<String>,
    /// Whether you hiked up to launch
    hikeandfly: bool,
    /// Whether an IGC file is present for this flight
    has_igc: bool,
    /// Pilot in command (P1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pilot_in_command: Option<String>,
    /// Second pilot / instructor / student / passenger (P2)
    #[serde(skip_serializing_if = "Option::is_none")]
    second_pilot: Option<String>,
    /// Operational role: PIC, Dual, Instructor or ICUS
    flight_capacity: String,
    /// Launch type: Aerotow, Winch, Car or Self-Launch
    #[serde(skip_serializing_if = "Option::is_none")]
    launch_type: Option<String>,
    /// Release altitude in metres
    #[serde(skip_serializing_if = "Option::is_none")]
    release_altitude_m: Option<i32>,
    /// Height gained during the tow, in metres
    #[serde(skip_serializing_if = "Option::is_none")]
    tow_height_m: Option<i32>,
    /// Tow plane registration
    #[serde(skip_serializing_if = "Option::is_none")]
    tug_registration: Option<String>,
    /// Maximum altitude in metres
    #[serde(skip_serializing_if = "Option::is_none")]
    max_altitude_m: Option<i32>,
    /// Height gain in metres
    #[serde(skip_serializing_if = "Option::is_none")]
    height_gain_m: Option<i32>,
    /// Whether the flight ended in an outlanding
    is_outlanding: bool,
    /// Engine run time in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    engine_run_time_seconds: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiFlights {
    flights: Vec<ApiFlightListItem>,
    locations: HashMap<i32, ApiFlightLocation>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiFlight {
    /// Flight ID
    id: i32,
    /// The user-defined flight number
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<i32>,
    /// The glider used
    #[serde(skip_serializing_if = "Option::is_none")]
    glider_name: Option<String>,
    /// The ID of the glider used
    #[serde(skip_serializing_if = "Option::is_none")]
    glider_id: Option<i32>,
    /// Launch location
    #[serde(skip_serializing_if = "Option::is_none")]
    launch_at: Option<ApiFlightLocation>,
    /// Landing location
    #[serde(skip_serializing_if = "Option::is_none")]
    landing_at: Option<ApiFlightLocation>,
    /// Time of launch
    #[serde(skip_serializing_if = "Option::is_none")]
    launch_time: Option<DateTime<Utc>>,
    /// Time of landing
    #[serde(skip_serializing_if = "Option::is_none")]
    landing_time: Option<DateTime<Utc>>,
    /// Flight duration in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_seconds: Option<u64>,
    /// GPS track length
    #[serde(skip_serializing_if = "Option::is_none")]
    track_distance: Option<f32>,
    /// XContest tracktype (free_flight, flat_triangle or fai_triangle)
    #[serde(skip_serializing_if = "Option::is_none")]
    xcontest_tracktype: Option<String>,
    /// XContest distance
    #[serde(skip_serializing_if = "Option::is_none")]
    xcontest_distance: Option<f32>,
    /// XContest URL
    #[serde(skip_serializing_if = "Option::is_none")]
    xcontest_url: Option<String>,
    /// Comment your flight
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    /// Link to a video of your flight
    #[serde(skip_serializing_if = "Option::is_none")]
    video_url: Option<String>,
    /// Whether you hiked up to launch
    hikeandfly: bool,
    /// Whether an IGC file is present for this flight
    has_igc: bool,
    /// Pilot in command (P1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pilot_in_command: Option<String>,
    /// Second pilot / instructor / student / passenger (P2)
    #[serde(skip_serializing_if = "Option::is_none")]
    second_pilot: Option<String>,
    /// Operational role: PIC, Dual, Instructor or ICUS
    flight_capacity: String,
    /// Launch type: Aerotow, Winch, Car or Self-Launch
    #[serde(skip_serializing_if = "Option::is_none")]
    launch_type: Option<String>,
    /// Release altitude in metres
    #[serde(skip_serializing_if = "Option::is_none")]
    release_altitude_m: Option<i32>,
    /// Height gained during the tow, in metres
    #[serde(skip_serializing_if = "Option::is_none")]
    tow_height_m: Option<i32>,
    /// Tow plane registration
    #[serde(skip_serializing_if = "Option::is_none")]
    tug_registration: Option<String>,
    /// Maximum altitude in metres
    #[serde(skip_serializing_if = "Option::is_none")]
    max_altitude_m: Option<i32>,
    /// Height gain in metres
    #[serde(skip_serializing_if = "Option::is_none")]
    height_gain_m: Option<i32>,
    /// Whether the flight ended in an outlanding
    is_outlanding: bool,
    /// Engine run time in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    engine_run_time_seconds: Option<i64>,
}

// Forms

/// Valid operational roles on a flight.
const FLIGHT_CAPACITIES: [&str; 4] = ["PIC", "Dual", "Instructor", "ICUS"];

/// Valid launch types.
const LAUNCH_TYPES: [&str; 4] = ["Aerotow", "Winch", "Car", "Self-Launch"];

/// Convert a duration in seconds into a PostgreSQL interval.
fn interval_from_seconds(seconds: Option<i64>) -> Result<Option<PgInterval>, String> {
    match seconds {
        Some(seconds) if seconds < 0 => Err("Durations must not be negative".into()),
        Some(seconds) => {
            let microseconds = seconds
                .checked_mul(1_000_000)
                .ok_or_else(|| "Duration is too large".to_string())?;
            Ok(Some(PgInterval {
                months: 0,
                days: 0,
                microseconds,
            }))
        }
        None => Ok(None),
    }
}

/// Convert a PostgreSQL interval into a duration in seconds.
fn interval_to_seconds(interval: Option<PgInterval>) -> Option<i64> {
    interval.map(|interval| {
        let months = i64::from(interval.months) * 30 * 24 * 60 * 60;
        let days = i64::from(interval.days) * 24 * 60 * 60;
        let seconds = interval.microseconds / 1_000_000;
        months + days + seconds
    })
}

/// Convert empty or whitespace-only optional strings to `None`.
fn empty_to_none(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FlightAddUpdateForm {
    /// Flight number
    number: Option<i32>,
    /// Glider ID
    glider: Option<i32>,
    /// Launch site ID
    launch_site: Option<i32>,
    /// Landing site ID
    landing_site: Option<i32>,
    /// Launch date
    launch_date: Option<NaiveDate>,
    /// Launch time
    launch_time: Option<NaiveTime>,
    /// Landing time
    landing_time: Option<NaiveTime>,
    /// Whether this was a hikeandfly tour
    hikeandfly: Option<bool>,
    /// Track distance in km
    track_distance: Option<f32>,
    /// XContest tracktype
    xcontest_tracktype: Option<String>,
    /// XContest distance in km
    xcontest_distance: Option<f32>,
    /// XContest URL
    xcontest_url: Option<String>,
    /// Flight comment
    comment: Option<String>,
    /// Flight video URL
    video_url: Option<String>,
    /// IGC file bytes as URL-safe base64 string
    igc_data: Option<String>,
    /// Pilot in command (P1)
    pilot_in_command: Option<String>,
    /// Second pilot / instructor / student / passenger (P2)
    second_pilot: Option<String>,
    /// Operational role: PIC, Dual, Instructor or ICUS
    flight_capacity: Option<String>,
    /// Launch type: Aerotow, Winch, Car or Self-Launch
    launch_type: Option<String>,
    /// Release altitude in metres
    release_altitude_m: Option<i32>,
    /// Height gained during the tow, in metres above the launch site
    tow_height_m: Option<i32>,
    /// Tow plane registration
    tug_registration: Option<String>,
    /// Maximum altitude in metres
    max_altitude_m: Option<i32>,
    /// Height gain in metres
    height_gain_m: Option<i32>,
    /// Whether the flight ended in an outlanding
    #[serde(default)]
    is_outlanding: bool,
    /// Engine run time in seconds
    engine_run_time_seconds: Option<i64>,
}

// API endpoints

#[get("/flights")]
pub async fn list(database: data::Database, user: auth::AuthUser) -> Json<ApiFlights> {
    let user = Arc::new(user.into_inner());

    // Get all flights for user
    let flights = database
        .run({
            let user = user.clone();
            move |db| data::get_flights_for_user(db, &user)
        })
        .await;

    // Get all gliders for user
    let glider_name_map = database
        .run({
            let user = user.clone();
            move |db| data::get_gliders_for_user(db, &user)
        })
        .await
        .into_iter()
        .map(|glider| (glider.id, glider.to_string()))
        .collect::<HashMap<i32, String>>();

    // Get all locations used
    let mut location_ids = flights
        .iter()
        .flat_map(|flight| vec![flight.launch_at, flight.landing_at])
        .flatten()
        .collect::<Vec<_>>();
    location_ids.sort_unstable();
    location_ids.dedup();
    let location_map = database
        .run(move |db| data::get_locations_with_ids(db, &location_ids))
        .await
        .into_iter()
        .map(|location| (location.id, location.into()))
        .collect::<HashMap<i32, ApiFlightLocation>>();

    // Get all flight IDs with IGC files
    let flight_ids_with_igc = database
        .run({
            let user = user.clone();
            move |db| data::get_flight_ids_with_igc_for_user(db, &user)
        })
        .await;

    // Add details to flights
    let api_flights = flights
        .into_iter()
        .map(|flight| {
            // Look up glider name
            let glider_name = flight.glider_id.and_then(|id| glider_name_map.get(&id).cloned());

            // Calculate duration
            let duration_seconds = match (flight.launch_time, flight.landing_time) {
                (Some(launch), Some(landing)) => {
                    let duration = (landing - launch).num_seconds();
                    if duration < 0 {
                        None
                    } else {
                        Some(duration as u64)
                    }
                }
                _ => None,
            };

            // Look up whether an IGC file exists for this flight
            let has_igc = flight_ids_with_igc.contains(&flight.id);

            ApiFlightListItem {
                id: flight.id,
                number: flight.number,
                glider_name,
                launch_at: flight.launch_at,
                landing_at: flight.landing_at,
                launch_time: flight.launch_time,
                landing_time: flight.landing_time,
                duration_seconds,
                track_distance: flight.track_distance,
                xcontest_tracktype: flight.xcontest_tracktype,
                xcontest_distance: flight.xcontest_distance,
                xcontest_url: flight.xcontest_url,
                comment: flight.comment,
                video_url: flight.video_url,
                hikeandfly: flight.hikeandfly,
                has_igc,
                pilot_in_command: flight.pilot_in_command,
                second_pilot: flight.second_pilot,
                flight_capacity: flight.flight_capacity,
                launch_type: flight.launch_type,
                release_altitude_m: flight.release_altitude_m,
                tow_height_m: flight.tow_height_m,
                tug_registration: flight.tug_registration,
                max_altitude_m: flight.max_altitude_m,
                height_gain_m: flight.height_gain_m,
                is_outlanding: flight.is_outlanding,
                engine_run_time_seconds: interval_to_seconds(flight.engine_run_time),
            }
        })
        .collect::<Vec<_>>();

    // Render template
    Json(ApiFlights {
        locations: location_map,
        flights: api_flights,
    })
}

#[get("/flights", rank = 2)]
pub fn list_nologin() -> ApiError {
    ApiError::MissingAuthentication
}

#[get("/flights/<id>")]
pub async fn get(id: i32, database: data::Database, user: auth::AuthUser) -> Result<Json<ApiFlight>, Status> {
    let user = user.into_inner();

    // Get flight
    let flight = match database.run(move |db| data::get_flight_with_id(db, id)).await {
        Some(flight) => Arc::new(flight),
        None => return Err(Status::NotFound),
    };

    // Ownership check
    if flight.user_id != user.id {
        return Err(Status::Forbidden);
    }

    // Check whether flight has IGC data
    let has_igc = database
        .run({
            let flight = flight.clone();
            move |db| data::flight_has_igc(db, &flight)
        })
        .await;

    // Resolve foreign keys
    let launch_at = database
        .run({
            let flight = flight.clone();
            move |db| flight.launch_at.and_then(|id| data::get_location_by_id(db, id))
        })
        .await
        .map(|location| location.into());
    let landing_at = database
        .run({
            let flight = flight.clone();
            move |db| flight.landing_at.and_then(|id| data::get_location_by_id(db, id))
        })
        .await
        .map(|location| location.into());
    let glider_name = database
        .run({
            let flight = flight.clone();
            move |db| flight.glider_id.and_then(|id| data::get_glider_by_id(db, id))
        })
        .await
        .map(|glider| glider.to_string());

    // Calculate duration
    let duration_seconds = match (flight.launch_time, flight.landing_time) {
        (Some(launch), Some(landing)) => {
            let duration = (landing - launch).num_seconds();
            if duration < 0 {
                None
            } else {
                Some(duration as u64)
            }
        }
        _ => None,
    };

    // Render template
    let flight = Arc::try_unwrap(flight).expect("cannot unwrap flight Arc");
    Ok(Json(ApiFlight {
        id: flight.id,
        number: flight.number,
        glider_name,
        glider_id: flight.glider_id,
        launch_at,
        landing_at,
        launch_time: flight.launch_time,
        landing_time: flight.landing_time,
        duration_seconds,
        track_distance: flight.track_distance,
        xcontest_tracktype: flight.xcontest_tracktype,
        xcontest_distance: flight.xcontest_distance,
        xcontest_url: flight.xcontest_url,
        comment: flight.comment,
        video_url: flight.video_url,
        hikeandfly: flight.hikeandfly,
        has_igc,
        pilot_in_command: flight.pilot_in_command,
        second_pilot: flight.second_pilot,
        flight_capacity: flight.flight_capacity,
        launch_type: flight.launch_type,
        release_altitude_m: flight.release_altitude_m,
        tow_height_m: flight.tow_height_m,
        tug_registration: flight.tug_registration,
        max_altitude_m: flight.max_altitude_m,
        height_gain_m: flight.height_gain_m,
        is_outlanding: flight.is_outlanding,
        engine_run_time_seconds: interval_to_seconds(flight.engine_run_time),
    }))
}

#[get("/flights/<id>", rank = 2)]
#[allow(unused_variables)]
pub fn get_nologin(id: i32) -> ApiError {
    ApiError::MissingAuthentication
}

impl FlightAddUpdateForm {
    /// Validate a `FlightAddForm` submission and convert it to a `NewFlight` model.
    ///
    /// If validation fails, return error message.
    async fn into_new_flight(mut self, user: &User, database: &data::Database) -> Result<NewFlight, String> {
        // Normalize optional free-text fields
        self.pilot_in_command = empty_to_none(self.pilot_in_command);
        self.second_pilot = empty_to_none(self.second_pilot);
        self.launch_type = empty_to_none(self.launch_type);
        self.tug_registration = empty_to_none(self.tug_registration);

        // Look up and validate glider
        if let Some(glider_id) = self.glider {
            match database
                .run(move |db| data::get_glider_by_id(db, glider_id))
                .await
            {
                Some(glider) => {
                    // Glider found, validate ownership
                    if glider.user_id != user.id {
                        return Err("Invalid glider".into());
                    }
                }
                None => return Err("Invalid glider".into()),
            }
        }

        // Look up and validate locations
        let user_location_ids = database
            .run({
                let user = user.clone();
                move |db| data::get_locations_for_user(db, &user)
            })
            .await
            .into_iter()
            .map(|location| location.id)
            .collect::<Vec<_>>();
        if let Some(ref location_id) = self.launch_site {
            if !user_location_ids.contains(location_id) {
                return Err("Invalid launch location".into());
            }
        }
        if let Some(ref location_id) = self.landing_site {
            if !user_location_ids.contains(location_id) {
                return Err("Invalid landing location".into());
            }
        }

        // Validate distances (must be valid and non-negative)
        if let Some(distance) = self.track_distance {
            if distance.is_infinite()
                || distance.is_subnormal()
                || distance.is_nan()
                || distance.is_sign_negative()
            {
                return Err(format!("Invalid GPS track distance: {}", distance));
            }
        }
        if let Some(distance) = self.xcontest_distance {
            if distance.is_infinite()
                || distance.is_subnormal()
                || distance.is_nan()
                || distance.is_sign_negative()
            {
                return Err(format!("Invalid XContest scored distance: {}", distance));
            }
        }

        // Validate and combine date and time
        let date_parts = self.launch_date.map(|_| 1).unwrap_or_default()
            + self.launch_time.map(|_| 1).unwrap_or_default()
            + self.landing_time.map(|_| 1).unwrap_or_default();
        if date_parts > 0 && date_parts < 3 {
            return Err("If you specify launch date, launch time or landing time, \
                        then the other two values must be provided as well"
                .into());
        }
        let launch_time = self.launch_time.map(|time| {
            let ndt = NaiveDateTime::new(self.launch_date.unwrap(), time);
            DateTime::from_naive_utc_and_offset(ndt, Utc) // TODO: Timezone?
        });
        let landing_time = self.landing_time.map(|time| {
            let ndt = NaiveDateTime::new(self.launch_date.unwrap(), time);
            DateTime::from_naive_utc_and_offset(ndt, Utc) // TODO: Timezone?
        });

        // Validate sailplane-specific fields
        let flight_capacity = self.flight_capacity.unwrap_or_else(|| "PIC".into());
        if !FLIGHT_CAPACITIES.contains(&flight_capacity.as_str()) {
            return Err(format!("Invalid flight capacity: {}", flight_capacity));
        }
        if let Some(ref launch_type) = self.launch_type {
            if !LAUNCH_TYPES.contains(&launch_type.as_str()) {
                return Err(format!("Invalid launch type: {}", launch_type));
            }
        }
        for (name, value) in [
            ("release altitude", self.release_altitude_m),
            ("tow height", self.tow_height_m),
            ("maximum altitude", self.max_altitude_m),
            ("height gain", self.height_gain_m),
        ] {
            if value.is_some_and(|value| value < 0) {
                return Err(format!("{} must not be negative", name));
            }
        }
        for (name, value) in [
            ("Pilot in command", self.pilot_in_command.as_deref()),
            ("Second pilot", self.second_pilot.as_deref()),
        ] {
            if value.is_some_and(|value| value.chars().count() > 128) {
                return Err(format!("{} must not exceed 128 characters", name));
            }
        }
        if let Some(ref registration) = self.tug_registration {
            if registration.chars().count() > 16 {
                return Err("Tow plane registration must not exceed 16 characters".into());
            }
        }
        let engine_run_time = interval_from_seconds(self.engine_run_time_seconds)?;

        // Create model
        Ok(NewFlight {
            number: self.number,
            user_id: user.id,
            glider_id: self.glider,
            launch_at: self.launch_site,
            landing_at: self.landing_site,
            launch_time,
            landing_time,
            hikeandfly: self.hikeandfly.unwrap_or_default(),
            track_distance: self.track_distance,
            xcontest_tracktype: self.xcontest_tracktype,
            xcontest_distance: self.xcontest_distance,
            xcontest_url: self.xcontest_url,
            comment: self.comment,
            video_url: self.video_url,
            pilot_in_command: self.pilot_in_command,
            second_pilot: self.second_pilot,
            flight_capacity,
            launch_type: self.launch_type,
            release_altitude_m: self.release_altitude_m,
            tow_height_m: self.tow_height_m,
            tug_registration: self.tug_registration,
            max_altitude_m: self.max_altitude_m,
            height_gain_m: self.height_gain_m,
            is_outlanding: self.is_outlanding,
            engine_run_time,
        })
    }
}

#[post("/flights", data = "<data>")]
pub async fn add(
    user: auth::AuthUser,
    database: data::Database,
    data: Json<FlightAddUpdateForm>,
) -> Result<Status, ApiError> {
    log::debug!("flights::add");
    let user = user.into_inner();

    // TODO: Test error handling for too-large IGC file

    // Decode IGC data
    let igc_bytes = if let Some(ref base64) = data.igc_data {
        Some(
            URL_SAFE_NO_PAD
                .decode(base64)
                .map_err(|e| ApiError::InvalidData {
                    message: format!("Invalid base64 data for IGC file: {}", e),
                })?,
        )
    } else {
        None
    };

    // Convert request data into `NewFlight`
    let new_flight = data
        .into_inner()
        .into_new_flight(&user, &database)
        .await
        .map_err(|e| ApiError::InvalidData {
            message: format!("Invalid flight data: {}", e),
        })?;

    // Insert flight into database
    database
        .run(move |db| {
            data::create_flight(db, &new_flight, igc_bytes);
            log::info!("Created flight for user {}", user.id);
            if let Some(glider_id) = new_flight.glider_id {
                data::update_user_last_glider(db, &user, glider_id);
            }
        })
        .await;

    Ok(Status::Created)
}

#[post("/flights/add", rank = 2)]
pub fn add_nologin() -> ApiError {
    ApiError::MissingAuthentication
}

/// Result summary of an IGC back-fill run.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiBackfillResult {
    flights_scanned: usize,
    flights_updated: usize,
    gliders_set: usize,
    locations_set: usize,
    crew_set: usize,
    altitudes_set: usize,
}

/// Try to match the glider referenced in an IGC file (registration from
/// HFGIDGLIDERID, type/model from HFGTYGLIDERTYPE) against the user's gliders.
/// Only returns a glider ID if the match is unambiguous.
fn match_glider(igc_gliderid: Option<&str>, igc_glidertype: Option<&str>, gliders: &[Glider]) -> Option<i32> {
    let normalize = |s: &str| s.trim().to_lowercase();

    if let Some(registration) = igc_gliderid {
        let registration = normalize(registration);
        if !registration.is_empty() {
            let matches: Vec<&Glider> = gliders
                .iter()
                .filter(|g| {
                    g.registration
                        .as_deref()
                        .map(|r| normalize(r) == registration)
                        .unwrap_or(false)
                })
                .collect();
            if matches.len() == 1 {
                return Some(matches[0].id);
            }
        }
    }

    if let Some(glider_type) = igc_glidertype {
        let glider_type = normalize(glider_type);
        if !glider_type.is_empty() {
            let matches: Vec<&Glider> = gliders
                .iter()
                .filter(|g| {
                    let model = normalize(&g.model);
                    !model.is_empty()
                        && (model == glider_type
                            || model.contains(&glider_type)
                            || glider_type.contains(&model))
                })
                .collect();
            if matches.len() == 1 {
                return Some(matches[0].id);
            }
        }
    }

    None
}

/// Re-parse the IGC files of all flights of the specified user and back-fill
/// empty fields: glider, launch/landing site, crew, max altitude and height
/// gain. Existing values are never overwritten.
fn backfill_igc_for_user(
    db: &mut diesel::PgConnection,
    user: &User,
) -> diesel::QueryResult<ApiBackfillResult> {
    let flights_with_igc = data::get_flights_with_igc_for_user(db, user);
    let gliders = data::get_gliders_for_user(db, user);

    let mut result = ApiBackfillResult {
        flights_scanned: flights_with_igc.len(),
        flights_updated: 0,
        gliders_set: 0,
        locations_set: 0,
        crew_set: 0,
        altitudes_set: 0,
    };

    for (mut flight, igc) in flights_with_igc {
        let mut changed = false;
        let info = match parse_igc_data(&igc.data, user, db) {
            crate::process_igc::FlightInfoResult::Success(info) => info,
            crate::process_igc::FlightInfoResult::Error { msg } => {
                log::warn!("Could not parse IGC of flight {}: {}", flight.id, msg);
                continue;
            }
        };

        // Glider
        if flight.glider_id.is_none() {
            if let Some(glider_id) =
                match_glider(info.gliderid.as_deref(), info.glidertype.as_deref(), &gliders)
            {
                flight.glider_id = Some(glider_id);
                result.gliders_set += 1;
                changed = true;
            }
        }

        // Launch and landing sites
        if flight.launch_at.is_none() {
            if let Some(location_id) = info.launch.and_then(|launch| launch.location_id) {
                flight.launch_at = Some(location_id);
                result.locations_set += 1;
                changed = true;
            }
        }
        if flight.landing_at.is_none() {
            if let Some(location_id) = info.landing.and_then(|landing| landing.location_id) {
                flight.landing_at = Some(location_id);
                result.locations_set += 1;
                changed = true;
            }
        }

        // Crew
        if flight.pilot_in_command.is_none() {
            if let Some(pilot) = info.pilot_in_command.as_ref().or(info.pilot.as_ref()) {
                flight.pilot_in_command = Some(pilot.clone());
                result.crew_set += 1;
                changed = true;
            }
        }
        if flight.second_pilot.is_none() {
            if let Some(second_pilot) = &info.second_pilot {
                flight.second_pilot = Some(second_pilot.clone());
                result.crew_set += 1;
                changed = true;
            }
        }

        // Altitude metrics
        if flight.max_altitude_m.is_none() {
            flight.max_altitude_m = info.max_altitude_m;
            if info.max_altitude_m.is_some() {
                result.altitudes_set += 1;
                changed = true;
            }
        }
        if flight.height_gain_m.is_none() {
            flight.height_gain_m = info.height_gain_m;
            if info.height_gain_m.is_some() {
                result.altitudes_set += 1;
                changed = true;
            }
        }

        if changed {
            data::update_flight(db, &flight);
            result.flights_updated += 1;
        }
    }

    Ok(result)
}

/// Back-fill flight fields from the stored IGC files.
#[post("/flights/backfill_igc")]
pub async fn backfill_igc(
    user: auth::AuthUser,
    database: data::Database,
) -> Result<Json<ApiBackfillResult>, ApiError> {
    log::debug!("flights::backfill_igc");
    let user = user.into_inner();
    let result = database
        .run(move |db| backfill_igc_for_user(db, &user))
        .await
        .map_err(|e| {
            log::error!("Back-filling IGC data failed: {e}");
            ApiError::IoError {
                message: "Could not back-fill IGC data".into(),
            }
        })?;
    log::info!(
        "Back-filled IGC data: {} flights scanned, {} updated",
        result.flights_scanned,
        result.flights_updated
    );
    Ok(Json(result))
}

#[post("/flights/backfill_igc", rank = 2)]
pub fn backfill_igc_nologin() -> ApiError {
    ApiError::MissingAuthentication
}

#[post("/flights/<id>", data = "<data>")]
pub async fn edit(
    user: auth::AuthUser,
    database: data::Database,
    id: i32,
    data: Json<FlightAddUpdateForm>,
) -> Result<Status, ApiError> {
    log::debug!("flights::add");
    let user = user.into_inner();

    // Decode IGC data
    let igc_bytes = if let Some(ref base64) = data.igc_data {
        Some(
            URL_SAFE_NO_PAD
                .decode(base64)
                .map_err(|e| ApiError::InvalidData {
                    message: format!("Invalid base64 data for IGC file: {}", e),
                })?,
        )
    } else {
        None
    };
    log::info!("flight igc {:?}", igc_bytes.is_some());

    // Get flight
    let mut flight = database
        .run(move |db| data::get_flight_with_id(db, id))
        .await
        .ok_or(ApiError::NotFound)?;

    // Ownership check
    if flight.user_id != user.id {
        return Err(ApiError::NotFound);
    }

    // TODO: Test error handling for too-large IGC file

    // Convert request data into `NewFlight`
    let new_flight = data
        .into_inner()
        .into_new_flight(&user, &database)
        .await
        .map_err(|e| ApiError::InvalidData {
            message: format!("Invalid flight data: {}", e),
        })?;

    // Update existing flight
    flight.number = new_flight.number;
    flight.glider_id = new_flight.glider_id;
    flight.launch_at = new_flight.launch_at;
    flight.landing_at = new_flight.landing_at;
    flight.launch_time = new_flight.launch_time;
    flight.landing_time = new_flight.landing_time;
    flight.hikeandfly = new_flight.hikeandfly;
    flight.track_distance = new_flight.track_distance;
    flight.xcontest_tracktype = new_flight.xcontest_tracktype;
    flight.xcontest_distance = new_flight.xcontest_distance;
    flight.xcontest_url = new_flight.xcontest_url;
    flight.comment = new_flight.comment;
    flight.video_url = new_flight.video_url;
    flight.pilot_in_command = new_flight.pilot_in_command;
    flight.second_pilot = new_flight.second_pilot;
    flight.flight_capacity = new_flight.flight_capacity;
    flight.launch_type = new_flight.launch_type;
    flight.release_altitude_m = new_flight.release_altitude_m;
    flight.tow_height_m = new_flight.tow_height_m;
    flight.tug_registration = new_flight.tug_registration;
    flight.max_altitude_m = new_flight.max_altitude_m;
    flight.height_gain_m = new_flight.height_gain_m;
    flight.is_outlanding = new_flight.is_outlanding;
    flight.engine_run_time = new_flight.engine_run_time;

    // Save changes
    // TODO: Error handling
    database
        .run(move |db| {
            data::update_flight(db, &flight);
            // Note: Only add IGC data if flight doesn't have IGC data yet. Never modify IGC.
            let has_igc = data::flight_has_igc(db, &flight);
            if !has_igc {
                if let Some(data) = igc_bytes {
                    data::update_igc(db, &flight, &data);
                }
            }
        })
        .await;

    Ok(Status::NoContent)
}

#[post("/flights/<id>", rank = 3)]
#[allow(unused_variables)]
pub fn edit_nologin(id: i32) -> ApiError {
    ApiError::MissingAuthentication
}

/// Delete a flight.
///
/// - Return "HTTP 204 No Content" if flight was deleted.
/// - Return "HTTP 404 Not Found" if flight was not found.
/// - Return "HTTP 403 Forbidden" if flight does not belong to user.
#[delete("/flights/<id>")]
pub async fn delete(user: auth::AuthUser, database: data::Database, id: i32) -> Result<Status, Status> {
    let user = user.into_inner();

    // Get data
    let flight = match database.run(move |db| data::get_flight_with_id(db, id)).await {
        Some(flight) => flight,
        None => return Err(Status::NotFound),
    };

    // Ownership check
    if flight.user_id != user.id {
        return Err(Status::Forbidden);
    }

    // Delete database entry
    let flight_id = flight.id;
    database
        .run(move |db| data::delete_flight(db, flight))
        .await
        .map(|()| {
            log::info!("Deleted flight with ID {}", flight_id);
            Status::NoContent
        })
        .map_err(|e| {
            log::error!("Could not delete flight with ID {}: {}", flight_id, e);
            Status::InternalServerError
        })
}

#[delete("/flights/<id>", rank = 2)]
#[allow(unused_variables)]
pub fn delete_nologin(id: i32) -> ApiError {
    ApiError::MissingAuthentication
}

// IGC Download

/// Responder that returns the `data` bytes with the `content_type` header as a
/// file download (disposition=attachment).
pub struct FileAttachment {
    data: Vec<u8>,
    content_type: ContentType,
    filename: String,
}

impl FileAttachment {
    /// Note: Ensure that the `filename` does not need any escaping / encoding!
    ///       There will be no encoding done by this function.
    pub fn new(data: Vec<u8>, content_type: ContentType, filename: String) -> Self {
        Self {
            data,
            content_type,
            filename,
        }
    }
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for FileAttachment {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(self.content_type)
            .header(Header::new(
                "content-disposition",
                format!("attachment; filename=\"{}\"", self.filename),
            ))
            .sized_body(self.data.len(), Cursor::new(self.data))
            .ok()
    }
}

#[get("/flights/<id>/igc")]
pub async fn igc_download(
    user: auth::AuthUser,
    database: data::Database,
    id: i32,
) -> Result<FileAttachment, Status> {
    let user = user.into_inner();

    // Get flight
    let flight = match database.run(move |db| data::get_flight_with_id(db, id)).await {
        Some(flight) => flight,
        None => return Err(Status::NotFound),
    };

    // Ownership check
    if flight.user_id != user.id {
        return Err(Status::Forbidden);
    }

    // Get IGC data
    let flight_id = flight.id;
    match database
        .run(move |db| data::get_igc_for_flight(db, &flight))
        .await
    {
        Some(igc) => Ok(FileAttachment::new(
            igc.data,
            ContentType::new("application", "octet-stream"),
            format!("flight{}.igc", flight_id),
        )),
        None => Err(Status::NotFound),
    }
}

/// Return vec of all API routes.
pub fn api_routes() -> Vec<Route> {
    routes![
        list,
        list_nologin,
        get,
        get_nologin,
        add,
        add_nologin,
        edit,
        edit_nologin,
        backfill_igc,
        backfill_igc_nologin,
        delete,
        delete_nologin,
        igc_download,
    ]
}

// TODO: API tests for flight submission and editing

#[cfg(test)]
mod tests {
    use rocket::{self, http::ContentType, local::blocking::Client};

    use crate::{
        models::{NewFlight, NewGlider},
        test_utils::{make_test_config, DbTestContext},
    };

    use diesel_geography::types::GeogPoint;

    use super::*;

    /// Create a new test client. Cookie tracking is disabled.
    fn make_client() -> Client {
        let app = rocket::custom(make_test_config())
            .attach(data::Database::fairing())
            .mount("/", api_routes());
        Client::untracked(app).expect("valid rocket instance")
    }

    #[test]
    fn delete_flight() {
        let ctx = DbTestContext::new();
        let client = make_client();

        macro_rules! delete_flight {
            ($id:expr, $cookie:expr) => {
                client
                    .delete(format!("/flights/{}", $id))
                    .private_cookie($cookie)
                    .cookie(ctx.username_cookie())
                    .dispatch()
            };
        }

        // Add flights
        let flight1 = data::create_flight(
            &mut *ctx.force_get_conn(),
            &NewFlight {
                number: Some(1),
                user_id: ctx.testuser1.user.id,
                ..Default::default()
            },
            None,
        );
        let flight2 = data::create_flight(
            &mut *ctx.force_get_conn(),
            &NewFlight {
                number: Some(2),
                user_id: ctx.testuser2.user.id,
                ..Default::default()
            },
            None,
        );

        // Delete flight 1 from user 1: OK
        let resp = delete_flight!(flight1.id, ctx.auth_cookie_user1());
        assert_eq!(resp.status(), Status::NoContent);

        // Delete flight 2 from user 1: Forbidden
        let resp = delete_flight!(flight2.id, ctx.auth_cookie_user1());
        assert_eq!(resp.status(), Status::Forbidden);

        // Delete flight 2 from user 2: OK
        let resp = delete_flight!(flight2.id, ctx.auth_cookie_user2());
        assert_eq!(resp.status(), Status::NoContent);

        // Delete non-existing flight: Not found
        let resp = delete_flight!(flight2.id, ctx.auth_cookie_user2());
        assert_eq!(resp.status(), Status::NotFound);
    }

    #[test]
    fn add_sailplane_flight() {
        let ctx = DbTestContext::new();
        let client = make_client();

        let glider = data::create_glider(
            &mut *ctx.force_get_conn(),
            NewGlider {
                user_id: ctx.testuser1.user.id,
                manufacturer: "Schleicher".into(),
                model: "ASK 21".into(),
                registration: Some("VH-SXQ".into()),
                is_two_seater: true,
                fai_class: Some("Two-Seater".into()),
                ..Default::default()
            },
        )
        .expect("Could not create glider");

        let body = format!(
            r#"{{
                "number": 7,
                "glider": {},
                "launchDate": "2026-11-19",
                "launchTime": "10:00:00",
                "landingTime": "11:30:00",
                "pilotInCommand": "Alex Doe",
                "secondPilot": "Sam Instructor",
                "flightCapacity": "Dual",
                "launchType": "Aerotow",
                "releaseAltitudeM": 1200,
                "towHeightM": 300,
                "tugRegistration": "VH-TUG",
                "maxAltitudeM": 2400,
                "heightGainM": 1800,
                "isOutlanding": true,
                "engineRunTimeSeconds": 90
            }}"#,
            glider.id
        );
        let resp = client
            .post("/flights")
            .header(ContentType::JSON)
            .body(body)
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp.status(), Status::Created);

        let flights = data::get_flights_for_user(&mut *ctx.force_get_conn(), &ctx.testuser1.user);
        assert_eq!(flights.len(), 1);
        let flight = &flights[0];
        assert_eq!(flight.glider_id, Some(glider.id));
        assert_eq!(flight.pilot_in_command.as_deref(), Some("Alex Doe"));
        assert_eq!(flight.second_pilot.as_deref(), Some("Sam Instructor"));
        assert_eq!(flight.flight_capacity, "Dual");
        assert_eq!(flight.launch_type.as_deref(), Some("Aerotow"));
        assert_eq!(flight.release_altitude_m, Some(1200));
        assert_eq!(flight.tow_height_m, Some(300));
        assert_eq!(flight.tug_registration.as_deref(), Some("VH-TUG"));
        assert_eq!(flight.max_altitude_m, Some(2400));
        assert_eq!(flight.height_gain_m, Some(1800));
        assert!(flight.is_outlanding);
        assert_eq!(flight.engine_run_time.map(|i| i.microseconds), Some(90_000_000));

        // Invalid launch type must be rejected
        let body = format!(
            r#"{{"glider": {}, "launchType": "Teleport", "launchDate": "2026-11-19", "launchTime": "10:00:00", "landingTime": "11:30:00"}}"#,
            glider.id
        );
        let resp = client
            .post("/flights")
            .header(ContentType::JSON)
            .body(body)
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp.status(), Status::BadRequest);
    }

    /// Back-fill flight fields from stored IGC data via the API.
    #[test]
    fn backfill_igc() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

        let ctx = DbTestContext::new();
        let client = make_client();

        // Create a flight without glider, sites, crew or altitude metrics,
        // with an IGC file referencing glider "VH-BFT" and flying from
        // 48.0N / 11.0E.
        let igc = "\
HFGTYGLIDERTYPE: DG-505
HFGIDGLIDERID: VH-BFT
HFPLTPILOTINCHARGE: Bea Pilot
B1000004800000N01100000EA0100001000
B1001004800000N01100000EA0200002000
B1002004800000N01100000EA0150001500";
        let body = format!(
            r#"{{
                "launchDate": "2026-11-19",
                "launchTime": "10:00:00",
                "landingTime": "11:00:00",
                "igcData": "{}"
            }}"#,
            URL_SAFE_NO_PAD.encode(igc)
        );
        let resp = client
            .post("/flights")
            .header(ContentType::JSON)
            .body(body)
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp.status(), Status::Created);

        // Without gliders or locations, nothing can be back-filled
        let resp = client
            .post("/flights/backfill_igc")
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let result: ApiBackfillResult = resp.into_json().expect("body");
        assert_eq!(
            result,
            ApiBackfillResult {
                flights_scanned: 1,
                // Crew and altitude metrics can be filled from the IGC data
                // alone, so the flight is already updated in this run.
                flights_updated: 1,
                gliders_set: 0,
                locations_set: 0,
                crew_set: 1, // P1 name from the IGC header
                altitudes_set: 2,
            }
        );

        // Now add the glider referenced in the IGC file and a location at the
        // launch/landing coordinates, after the fact.
        let glider = data::create_glider(
            &mut *ctx.force_get_conn(),
            NewGlider {
                user_id: ctx.testuser1.user.id,
                manufacturer: "DG".into(),
                model: "DG-505".into(),
                registration: Some("VH-BFT".into()),
                ..Default::default()
            },
        )
        .expect("Could not create glider");
        let location = data::create_location(
            &mut *ctx.force_get_conn(),
            crate::models::NewLocation {
                name: "Nearest site".into(),
                country: "DE".into(),
                elevation: 500,
                user_id: ctx.testuser1.user.id,
                geog: Some(GeogPoint {
                    x: 11.0,
                    y: 48.0,
                    srid: None,
                }),
            },
        );

        // Back-fill again: glider and sites can be filled now
        // (crew fields and altitude metrics are already set and remain untouched).
        let resp = client
            .post("/flights/backfill_igc")
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let result: ApiBackfillResult = resp.into_json().expect("body");
        assert_eq!(
            result,
            ApiBackfillResult {
                flights_scanned: 1,
                flights_updated: 1,
                gliders_set: 1,
                locations_set: 2,
                crew_set: 0,
                altitudes_set: 0,
            }
        );

        // Assert the flight was updated
        let flights = data::get_flights_for_user(&mut *ctx.force_get_conn(), &ctx.testuser1.user);
        assert_eq!(flights.len(), 1);
        let flight = &flights[0];
        assert_eq!(flight.glider_id, Some(glider.id));
        assert_eq!(flight.launch_at, Some(location.id));
        assert_eq!(flight.landing_at, Some(location.id));
        assert_eq!(flight.pilot_in_command.as_deref(), Some("Bea Pilot"));
        assert_eq!(flight.max_altitude_m, Some(2000));
        assert_eq!(flight.height_gain_m, Some(1000));

        // A second run changes nothing (idempotent)
        let resp = client
            .post("/flights/backfill_igc")
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let result: ApiBackfillResult = resp.into_json().expect("body");
        assert_eq!(
            result,
            ApiBackfillResult {
                flights_scanned: 1,
                flights_updated: 0,
                gliders_set: 0,
                locations_set: 0,
                crew_set: 0,
                altitudes_set: 0,
            }
        );

        // Endpoint requires authentication
        let resp = client.post("/flights/backfill_igc").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }
}
