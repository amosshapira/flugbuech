use std::fmt::{self, Display};

use chrono::{DateTime, NaiveDate, Utc};
use diesel::pg::data_types::PgInterval;
use diesel::sql_types::{BigInt, Bool, Date, Double, Integer, Nullable, Text};
use diesel::{Associations, Identifiable, Queryable};
use diesel_geography::{sql_types::Geography, types::GeogPoint};
use serde::Serialize;

use crate::schema::{flights, gliders, igcs, locations, users};

#[derive(Identifiable, Queryable, Serialize, PartialEq, Debug, Clone)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub username: String,
    /// Password is automatically hashed on insert or update
    /// by a PostgreSQL trigger.
    pub password: String,
    /// Last used glider
    pub last_glider_id: Option<i32>,
    /// E-mail address
    pub email: String,
    /// When the user was created
    pub signed_up: DateTime<Utc>,
    /// Whether the user has opted in to receive news
    pub news_opt_in: bool,
    /// Preferred unit for displaying altitudes ("m" or "ft")
    pub altitude_unit: String,
    /// Preferred unit for displaying distances ("km", "mi" or "nm")
    pub distance_unit: String,
}

#[derive(Identifiable, Queryable, Associations, AsChangeset, Serialize, PartialEq, Debug, Clone)]
#[diesel(treat_none_as_null = true)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(table_name = gliders)]
pub struct Glider {
    pub id: i32,
    pub user_id: i32,
    /// The model name, e.g. "Epsilon 8"
    pub model: String,
    /// The manufacturer name, e.g. "Advance"
    pub manufacturer: String,
    /// When was the glider acquired?
    pub since: Option<NaiveDate>,
    /// When was the glider sold / given away / thrown away?
    pub until: Option<NaiveDate>,
    /// Where did you get the glider from? (e.g. a shop, or a website)
    pub source: Option<String>,
    /// How much did the glider cost, in your currency?
    pub cost: Option<i32>,
    /// Add arbitrary comments about this glider
    pub comment: Option<String>,
    /// Aircraft registration / tail number, e.g. "VH-GOB"
    pub registration: Option<String>,
    /// Competition ID / callsign, e.g. "GOB" or "GU"
    pub competition_id: Option<String>,
    /// Whether the glider has two seats
    pub is_two_seater: bool,
    /// FAI class, e.g. "Club", "Standard", "15m", "Open" or "Two-Seater"
    pub fai_class: Option<String>,
    /// Whether the glider is a motorglider (self-launching or sustainer)
    pub is_motorglider: bool,
}

impl Display for Glider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{} {}", self.manufacturer, self.model)?;
        if let Some(ref registration) = self.registration {
            write!(f, " ({})", registration)?;
        }
        Ok(())
    }
}

#[derive(Debug, QueryableByName, Serialize)]
#[diesel(table_name = gliders)]
pub struct GliderWithStats {
    // For field descriptions, see `Glider` model
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Integer)]
    pub user_id: i32,
    #[diesel(sql_type = Text)]
    pub model: String,
    #[diesel(sql_type = Text)]
    pub manufacturer: String,
    #[diesel(sql_type = Nullable<Date>)]
    pub since: Option<NaiveDate>,
    #[diesel(sql_type = Nullable<Date>)]
    pub until: Option<NaiveDate>,
    #[diesel(sql_type = Nullable<Text>)]
    pub source: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub cost: Option<i32>,
    #[diesel(sql_type = Nullable<Text>)]
    pub comment: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub registration: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub competition_id: Option<String>,
    #[diesel(sql_type = Bool)]
    pub is_two_seater: bool,
    #[diesel(sql_type = Nullable<Text>)]
    pub fai_class: Option<String>,
    #[diesel(sql_type = Bool)]
    pub is_motorglider: bool,
    #[diesel(sql_type = BigInt)]
    pub flights: i64,
    #[diesel(sql_type = BigInt)]
    pub seconds: i64,
    #[diesel(sql_type = Bool)]
    pub seconds_complete: bool,
}

#[derive(Insertable, Default)]
#[diesel(table_name = gliders)]
pub struct NewGlider {
    pub user_id: i32,
    pub model: String,
    pub manufacturer: String,
    pub since: Option<NaiveDate>,
    pub until: Option<NaiveDate>,
    pub source: Option<String>,
    pub cost: Option<i32>,
    pub comment: Option<String>,
    pub registration: Option<String>,
    pub competition_id: Option<String>,
    pub is_two_seater: bool,
    pub fai_class: Option<String>,
    pub is_motorglider: bool,
}

#[derive(Identifiable, Queryable, Associations, AsChangeset, Serialize, PartialEq, Debug, Clone)]
#[diesel(treat_none_as_null = true)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(table_name = locations)]
pub struct Location {
    pub id: i32,
    pub name: String,
    pub country: String,
    pub elevation: i32,
    pub user_id: i32,
    pub geog: Option<GeogPoint>,
}

#[derive(Insertable, Default)]
#[diesel(table_name = locations)]
pub struct NewLocation {
    pub name: String,
    pub country: String,
    pub elevation: i32,
    pub user_id: i32,
    pub geog: Option<GeogPoint>,
}

#[derive(QueryableByName, PartialEq, Debug, Clone)]
pub struct LocationWithDistance {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Text)]
    pub country: String,
    #[diesel(sql_type = Integer)]
    pub elevation: i32,
    #[diesel(sql_type = Integer)]
    pub user_id: i32,
    #[diesel(sql_type = Geography)]
    pub geog: GeogPoint,
    #[diesel(sql_type = Double)]
    pub distance: f64,
}

/// Locations with a count (e.g. landing count).
#[derive(QueryableByName, Serialize, PartialEq, Debug, Clone)]
pub struct LocationWithCount {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Text)]
    pub country: String,
    #[diesel(sql_type = Integer)]
    pub elevation: i32,
    #[diesel(sql_type = Integer)]
    pub user_id: i32,
    #[diesel(sql_type = Nullable<Geography>)]
    pub geog: Option<GeogPoint>,
    #[diesel(sql_type = BigInt)]
    pub count: i64,
}

#[derive(Identifiable, Queryable, Associations, AsChangeset, PartialEq, Debug, Clone)]
#[diesel(treat_none_as_null = true)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(belongs_to(Glider, foreign_key = glider_id))]
#[diesel(table_name = flights)]
pub struct Flight {
    /// Primary key
    pub id: i32,
    /// The user-defined flight number
    pub number: Option<i32>,
    /// The pilot
    pub user_id: i32,
    /// The glider
    pub glider_id: Option<i32>,
    /// Launch location
    pub launch_at: Option<i32>,
    /// Landing location
    pub landing_at: Option<i32>,
    /// Time of launch
    pub launch_time: Option<DateTime<Utc>>,
    /// Time of landing
    pub landing_time: Option<DateTime<Utc>>,
    /// GPS track length
    pub track_distance: Option<f32>,
    /// XContest tracktype (free_flight, flat_triangle or fai_triangle)
    /// TODO: Proper newtype
    pub xcontest_tracktype: Option<String>,
    /// XContest distance
    pub xcontest_distance: Option<f32>,
    /// XContest URL
    pub xcontest_url: Option<String>,
    /// Comment your flight
    pub comment: Option<String>,
    /// Link to a video of your flight
    pub video_url: Option<String>,
    /// When the flight was entered into Flugbuech
    pub created_at: DateTime<Utc>,
    /// Whether you hiked up to launch
    pub hikeandfly: bool,
    /// Pilot in command (P1)
    pub pilot_in_command: Option<String>,
    /// Second pilot / instructor / student / passenger (P2)
    pub second_pilot: Option<String>,
    /// Operational role: PIC, Dual, Instructor or ICUS
    pub flight_capacity: String,
    /// Launch type: Aerotow, Winch, Car or Self-Launch
    pub launch_type: Option<String>,
    /// Release altitude above mean sea level, in metres
    pub release_altitude_m: Option<i32>,
    /// Height gained during the tow, in metres above the launch site
    pub tow_height_m: Option<i32>,
    /// Registration of the tow plane
    pub tug_registration: Option<String>,
    /// Maximum altitude above mean sea level, in metres
    pub max_altitude_m: Option<i32>,
    /// Total height gain, in metres
    pub height_gain_m: Option<i32>,
    /// Whether the flight ended in an outlanding
    pub is_outlanding: bool,
    /// Time the engine was running (motorgliders)
    pub engine_run_time: Option<PgInterval>,
}

#[derive(Insertable)]
#[diesel(table_name = flights)]
pub struct NewFlight {
    /// The user-defined flight number
    pub number: Option<i32>,
    /// The pilot
    pub user_id: i32,
    /// The glider
    pub glider_id: Option<i32>,
    /// Launch location
    pub launch_at: Option<i32>,
    /// Landing location
    pub landing_at: Option<i32>,
    /// Time of launch
    pub launch_time: Option<DateTime<Utc>>,
    /// Time of landing
    pub landing_time: Option<DateTime<Utc>>,
    /// GPS track length
    pub track_distance: Option<f32>,
    /// XContest tracktype (free_flight, flat_triangle or fai_triangle)
    pub xcontest_tracktype: Option<String>,
    /// XContest distance
    pub xcontest_distance: Option<f32>,
    /// XContest URL
    pub xcontest_url: Option<String>,
    /// Comment your flight
    pub comment: Option<String>,
    /// Link to a video of your flight
    pub video_url: Option<String>,
    /// Whether you hiked up to launch
    pub hikeandfly: bool,
    /// Pilot in command (P1)
    pub pilot_in_command: Option<String>,
    /// Second pilot / instructor / student / passenger (P2)
    pub second_pilot: Option<String>,
    /// Operational role: PIC, Dual, Instructor or ICUS
    pub flight_capacity: String,
    /// Launch type: Aerotow, Winch, Car or Self-Launch
    pub launch_type: Option<String>,
    /// Release altitude above mean sea level, in metres
    pub release_altitude_m: Option<i32>,
    /// Height gained during the tow, in metres above the launch site
    pub tow_height_m: Option<i32>,
    /// Registration of the tow plane
    pub tug_registration: Option<String>,
    /// Maximum altitude above mean sea level, in metres
    pub max_altitude_m: Option<i32>,
    /// Total height gain, in metres
    pub height_gain_m: Option<i32>,
    /// Whether the flight ended in an outlanding
    pub is_outlanding: bool,
    /// Time the engine was running (motorgliders)
    pub engine_run_time: Option<PgInterval>,
}

impl Default for NewFlight {
    fn default() -> Self {
        Self {
            number: None,
            user_id: 0,
            glider_id: None,
            launch_at: None,
            landing_at: None,
            launch_time: None,
            landing_time: None,
            track_distance: None,
            xcontest_tracktype: None,
            xcontest_distance: None,
            xcontest_url: None,
            comment: None,
            video_url: None,
            hikeandfly: false,
            pilot_in_command: None,
            second_pilot: None,
            flight_capacity: "PIC".into(),
            launch_type: None,
            release_altitude_m: None,
            tow_height_m: None,
            tug_registration: None,
            max_altitude_m: None,
            height_gain_m: None,
            is_outlanding: false,
            engine_run_time: None,
        }
    }
}

#[derive(Identifiable, Queryable, Insertable, PartialEq, Debug, Clone)]
#[diesel(primary_key(flight_id))]
#[diesel(table_name = igcs)]
pub struct Igc {
    /// The flight
    pub flight_id: i32,
    /// IGC file contents
    pub data: Vec<u8>,
}
