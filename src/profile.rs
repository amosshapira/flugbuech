//! Profile views.

use chrono::{DateTime, Utc};
use log::error;
use rocket::{get, http::Status, post, routes, serde::json::Json, Route};
use serde::{Deserialize, Serialize};

use crate::{auth, data, responders::ApiError};

/// Valid values for the user's preferred display units.
pub const ALTITUDE_UNITS: [&str; 2] = ["m", "ft"];
pub const DISTANCE_UNITS: [&str; 3] = ["km", "mi", "nm"];

/// Valid values for the page navigated to after login. Mirrors the main
/// navigation destinations for logged-in users.
pub const DEFAULT_PAGES: [&str; 5] = ["/", "/flights/", "/gliders/", "/locations/", "/stats/"];

/// Valid values for the replay marker in the IGC track viewer.
pub const TRACK_MARKERS: [&str; 2] = ["glider", "paraglider"];

/// Valid values for the IGC track viewer rendering style.
pub const RENDER_STYLES: [&str; 2] = ["ground", "deck"];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProfile {
    username: String,
    email: String,
    signed_up: DateTime<Utc>,
    news_opt_in: bool,
    altitude_unit: String,
    distance_unit: String,
    open_viewer_in_new_tab: bool,
    reuse_viewer_tab: bool,
    default_page: String,
    track_marker: String,
    render_style: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApiProfileUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    news_opt_in: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    altitude_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distance_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    open_viewer_in_new_tab: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reuse_viewer_tab: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_page: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    track_marker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    render_style: Option<String>,
}

#[get("/profile")]
pub fn get(user: auth::AuthUser) -> Json<ApiProfile> {
    let user = user.into_inner();
    Json(ApiProfile {
        username: user.username,
        email: user.email,
        signed_up: user.signed_up,
        news_opt_in: user.news_opt_in,
        altitude_unit: user.altitude_unit,
        distance_unit: user.distance_unit,
        open_viewer_in_new_tab: user.open_viewer_in_new_tab,
        reuse_viewer_tab: user.reuse_viewer_tab,
        default_page: user.default_page,
        track_marker: user.track_marker,
        render_style: user.render_style,
    })
}

#[get("/profile", rank = 2)]
pub fn get_nologin() -> ApiError {
    ApiError::MissingAuthentication
}

#[post("/profile", data = "<data>")]
pub async fn edit(
    database: data::Database,
    user: auth::AuthUser,
    data: Json<ApiProfileUpdate>,
) -> Result<Status, ApiError> {
    let user = user.into_inner();
    if let Some(altitude_unit) = data.altitude_unit.as_deref().map(str::to_string) {
        // Units can only be updated when both values are provided and valid.
        let distance_unit = data.distance_unit.as_deref().map(str::to_string);
        let Some(distance_unit) = distance_unit else {
            return Err(ApiError::InvalidData {
                message: "altitudeUnit and distanceUnit must always be provided together".into(),
            });
        };
        if !ALTITUDE_UNITS.contains(&altitude_unit.as_str())
            || !DISTANCE_UNITS.contains(&distance_unit.as_str())
        {
            return Err(ApiError::InvalidData {
                message: format!(
                    "Invalid units, altitude must be one of {ALTITUDE_UNITS:?} and distance one of {DISTANCE_UNITS:?}"
                ),
            });
        }
        let user_for_units = user.clone();
        if let Err(e) = database
            .run(move |db| data::update_units(db, &user_for_units, &altitude_unit, &distance_unit))
            .await
        {
            error!("Updating user's units failed: {e}");
            return Err(ApiError::IoError {
                message: "Could not update units".into(),
            });
        }
    }
    let user_for_viewer = user.clone();
    let user_for_reuse = user.clone();
    let user_for_page = user.clone();
    let user_for_marker = user.clone();
    let user_for_style = user.clone();
    if let Some(news_opt_in) = data.news_opt_in {
        if let Err(e) = database
            .run(move |db| data::update_news_opt_in(db, &user, news_opt_in))
            .await
        {
            error!("Updating user's news opt-in failed: {e}");
            return Err(ApiError::IoError {
                message: "Could not update news opt-in".into(),
            });
        }
    }
    if let Some(open_viewer_in_new_tab) = data.open_viewer_in_new_tab {
        if let Err(e) = database
            .run(move |db| data::update_open_viewer_in_new_tab(db, &user_for_viewer, open_viewer_in_new_tab))
            .await
        {
            error!("Updating user's viewer preference failed: {e}");
            return Err(ApiError::IoError {
                message: "Could not update viewer preference".into(),
            });
        }
    }
    if let Some(reuse_viewer_tab) = data.reuse_viewer_tab {
        if let Err(e) = database
            .run(move |db| data::update_reuse_viewer_tab(db, &user_for_reuse, reuse_viewer_tab))
            .await
        {
            error!("Updating user's viewer tab reuse preference failed: {e}");
            return Err(ApiError::IoError {
                message: "Could not update viewer tab reuse preference".into(),
            });
        }
    }
    if let Some(default_page) = data.default_page.clone() {
        if !DEFAULT_PAGES.contains(&default_page.as_str()) {
            return Err(ApiError::InvalidData {
                message: format!("Invalid page, must be one of {DEFAULT_PAGES:?}"),
            });
        }
        if let Err(e) = database
            .run(move |db| data::update_default_page(db, &user_for_page, &default_page))
            .await
        {
            error!("Updating user's default page failed: {e}");
            return Err(ApiError::IoError {
                message: "Could not update default page".into(),
            });
        }
    }
    if let Some(track_marker) = data.track_marker.clone() {
        if !TRACK_MARKERS.contains(&track_marker.as_str()) {
            return Err(ApiError::InvalidData {
                message: format!("Invalid marker, must be one of {TRACK_MARKERS:?}"),
            });
        }
        if let Err(e) = database
            .run(move |db| data::update_track_marker(db, &user_for_marker, &track_marker))
            .await
        {
            error!("Updating user's track marker failed: {e}");
            return Err(ApiError::IoError {
                message: "Could not update track marker".into(),
            });
        }
    }
    if let Some(render_style) = data.render_style.clone() {
        if !RENDER_STYLES.contains(&render_style.as_str()) {
            return Err(ApiError::InvalidData {
                message: format!("Invalid style, must be one of {RENDER_STYLES:?}"),
            });
        }
        if let Err(e) = database
            .run(move |db| data::update_render_style(db, &user_for_style, &render_style))
            .await
        {
            error!("Updating user's render style failed: {e}");
            return Err(ApiError::IoError {
                message: "Could not update render style".into(),
            });
        }
    }
    Ok(Status::NoContent)
}

#[post("/profile", rank = 2)]
#[allow(unused_variables)]
pub fn edit_nologin() -> ApiError {
    ApiError::MissingAuthentication
}

/// Return vec of all API routes.
pub fn api_routes() -> Vec<Route> {
    routes![get, get_nologin, edit, edit_nologin]
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use diesel::PgConnection;
    use rocket::{
        self,
        http::{ContentType, Status},
        local::blocking::Client,
        serde::json,
    };

    use crate::{
        data,
        test_utils::{make_test_config, DbTestContext},
    };

    use super::*;

    /// Create a new test client with cookie tracking.
    fn make_api_client() -> Client {
        let app = rocket::custom(make_test_config())
            .attach(data::Database::fairing())
            .mount("/", api_routes());
        Client::tracked(app).expect("valid rocket instance")
    }

    #[test]
    fn update_news_opt_in() {
        let ctx = DbTestContext::new();
        let client = make_api_client();

        /// Helper function: Assert news opt-in state
        fn assert_news_opt_in(conn_mutex: &Mutex<PgConnection>, user_id: i32, opted_in: bool) {
            let mut conn = conn_mutex.lock().unwrap();
            let user = data::get_user(&mut conn, user_id).expect("User not found");
            assert_eq!(user.news_opt_in, opted_in);
        }

        /// Helper macro: Send request
        macro_rules! update_profile_news_opt_in {
            ($opt_in:expr) => {
                client
                    .post("/profile")
                    .header(ContentType::JSON)
                    .body(
                        json::to_string(&ApiProfileUpdate {
                            news_opt_in: Some($opt_in),
                            altitude_unit: None,
                            distance_unit: None,
                            open_viewer_in_new_tab: None,
                            reuse_viewer_tab: None,
                            default_page: None,
                            track_marker: None,
                            render_style: None,
                        })
                        .unwrap(),
                    )
                    .private_cookie(ctx.auth_cookie_user1())
                    .cookie(ctx.username_cookie())
                    .dispatch()
            };
        }

        // Initially, not opted in to news
        assert_news_opt_in(&ctx.conn, ctx.testuser1.user.id, false);

        // Opt in
        let resp1 = update_profile_news_opt_in!(true);
        assert_eq!(resp1.status(), Status::NoContent);
        assert_news_opt_in(&ctx.conn, ctx.testuser1.user.id, true);

        // Opt out again
        let resp2 = update_profile_news_opt_in!(false);
        assert_eq!(resp2.status(), Status::NoContent);
        assert_news_opt_in(&ctx.conn, ctx.testuser1.user.id, false);

        // Change is idempotent
        let resp3 = update_profile_news_opt_in!(false);
        assert_eq!(resp3.status(), Status::NoContent);
        assert_news_opt_in(&ctx.conn, ctx.testuser1.user.id, false);
    }

    #[test]
    fn update_units() {
        let ctx = DbTestContext::new();
        let client = make_api_client();

        /// Helper function: Assert the user's unit preferences
        fn assert_units(conn_mutex: &Mutex<PgConnection>, user_id: i32, altitude: &str, distance: &str) {
            let mut conn = conn_mutex.lock().unwrap();
            let user = data::get_user(&mut conn, user_id).expect("User not found");
            assert_eq!(user.altitude_unit, altitude);
            assert_eq!(user.distance_unit, distance);
        }

        // Initially, the database defaults apply
        assert_units(&ctx.conn, ctx.testuser1.user.id, "m", "km");

        // Switch to imperial and nautical miles
        let resp1 = client
            .post("/profile")
            .header(ContentType::JSON)
            .body(
                json::to_string(&ApiProfileUpdate {
                    news_opt_in: None,
                    altitude_unit: Some("ft".into()),
                    distance_unit: Some("nm".into()),
                    open_viewer_in_new_tab: None,
                    reuse_viewer_tab: None,
                    default_page: None,
                    track_marker: None,
                    render_style: None,
                })
                .unwrap(),
            )
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp1.status(), Status::NoContent);
        assert_units(&ctx.conn, ctx.testuser1.user.id, "ft", "nm");

        // Switching back to metric works, too
        let resp2 = client
            .post("/profile")
            .header(ContentType::JSON)
            .body(
                json::to_string(&ApiProfileUpdate {
                    news_opt_in: None,
                    altitude_unit: Some("m".into()),
                    distance_unit: Some("mi".into()),
                    open_viewer_in_new_tab: None,
                    reuse_viewer_tab: None,
                    default_page: None,
                    track_marker: None,
                    render_style: None,
                })
                .unwrap(),
            )
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp2.status(), Status::NoContent);
        assert_units(&ctx.conn, ctx.testuser1.user.id, "m", "mi");

        // Invalid values are rejected and don't change anything
        let resp3 = client
            .post("/profile")
            .header(ContentType::JSON)
            .body(
                json::to_string(&ApiProfileUpdate {
                    news_opt_in: None,
                    altitude_unit: Some("meters".into()),
                    distance_unit: Some("nm".into()),
                    open_viewer_in_new_tab: None,
                    reuse_viewer_tab: None,
                    default_page: None,
                    track_marker: None,
                    render_style: None,
                })
                .unwrap(),
            )
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp3.status(), Status::BadRequest);
        assert_units(&ctx.conn, ctx.testuser1.user.id, "m", "mi");

        // Units are also reported when loading the profile
        let resp4 = client
            .get("/profile")
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp4.status(), Status::Ok);
        let body = resp4.into_string().expect("body");
        assert!(body.contains(r#""altitudeUnit":"m""#));
        assert!(body.contains(r#""distanceUnit":"mi""#));
    }

    #[test]
    fn update_default_page() {
        let ctx = DbTestContext::new();
        let client = make_api_client();

        /// Helper function: Assert default page state
        fn assert_default_page(conn_mutex: &Mutex<PgConnection>, user_id: i32, page: &str) {
            let mut conn = conn_mutex.lock().unwrap();
            let user = data::get_user(&mut conn, user_id).expect("User not found");
            assert_eq!(user.default_page, page);
        }

        /// Helper macro: Send request
        macro_rules! update_default_page {
            ($page:expr) => {
                client
                    .post("/profile")
                    .header(ContentType::JSON)
                    .body(
                        json::to_string(&ApiProfileUpdate {
                            news_opt_in: None,
                            altitude_unit: None,
                            distance_unit: None,
                            open_viewer_in_new_tab: None,
                            reuse_viewer_tab: None,
                            default_page: Some($page.into()),
                            track_marker: None,
                            render_style: None,
                        })
                        .unwrap(),
                    )
                    .private_cookie(ctx.auth_cookie_user1())
                    .cookie(ctx.username_cookie())
                    .dispatch()
            };
        }

        // Initially, the database default (home page) applies
        assert_default_page(&ctx.conn, ctx.testuser1.user.id, "/");

        // Switch to the flights list
        let resp1 = update_default_page!("/flights/");
        assert_eq!(resp1.status(), Status::NoContent);
        assert_default_page(&ctx.conn, ctx.testuser1.user.id, "/flights/");

        // Switch to the stats page
        let resp2 = update_default_page!("/stats/");
        assert_eq!(resp2.status(), Status::NoContent);
        assert_default_page(&ctx.conn, ctx.testuser1.user.id, "/stats/");

        // Invalid values are rejected and don't change anything
        for invalid in ["/etc/passwd", "flights", "https://example.com/"] {
            let resp3 = update_default_page!(invalid);
            assert_eq!(resp3.status(), Status::BadRequest);
            assert_default_page(&ctx.conn, ctx.testuser1.user.id, "/stats/");
        }

        // The preference is also reported when loading the profile
        let resp4 = client
            .get("/profile")
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp4.status(), Status::Ok);
        let body = resp4.into_string().expect("body");
        assert!(body.contains(r#""defaultPage":"/stats/""#));
    }

    #[test]
    fn update_track_marker() {
        let ctx = DbTestContext::new();
        let client = make_api_client();

        /// Helper function: Assert track marker state
        fn assert_track_marker(conn_mutex: &Mutex<PgConnection>, user_id: i32, marker: &str) {
            let mut conn = conn_mutex.lock().unwrap();
            let user = data::get_user(&mut conn, user_id).expect("User not found");
            assert_eq!(user.track_marker, marker);
        }

        /// Helper macro: Send request
        macro_rules! update_track_marker {
            ($marker:expr) => {
                client
                    .post("/profile")
                    .header(ContentType::JSON)
                    .body(
                        json::to_string(&ApiProfileUpdate {
                            news_opt_in: None,
                            altitude_unit: None,
                            distance_unit: None,
                            open_viewer_in_new_tab: None,
                            reuse_viewer_tab: None,
                            default_page: None,
                            track_marker: Some($marker.into()),
                            render_style: None,
                        })
                        .unwrap(),
                    )
                    .private_cookie(ctx.auth_cookie_user1())
                    .cookie(ctx.username_cookie())
                    .dispatch()
            };
        }

        // Initially, the database default (glider) applies
        assert_track_marker(&ctx.conn, ctx.testuser1.user.id, "glider");

        // Switch to the paraglider marker
        let resp1 = update_track_marker!("paraglider");
        assert_eq!(resp1.status(), Status::NoContent);
        assert_track_marker(&ctx.conn, ctx.testuser1.user.id, "paraglider");

        // Switch back to the glider marker
        let resp2 = update_track_marker!("glider");
        assert_eq!(resp2.status(), Status::NoContent);
        assert_track_marker(&ctx.conn, ctx.testuser1.user.id, "glider");

        // Invalid values are rejected and don't change anything
        let resp3 = update_track_marker!("balloon");
        assert_eq!(resp3.status(), Status::BadRequest);
        assert_track_marker(&ctx.conn, ctx.testuser1.user.id, "glider");
    }

    #[test]
    fn update_render_style() {
        let ctx = DbTestContext::new();
        let client = make_api_client();

        /// Helper function: Assert render style state
        fn assert_render_style(conn_mutex: &Mutex<PgConnection>, user_id: i32, style: &str) {
            let mut conn = conn_mutex.lock().unwrap();
            let user = data::get_user(&mut conn, user_id).expect("User not found");
            assert_eq!(user.render_style, style);
        }

        /// Helper macro: Send request
        macro_rules! update_render_style {
            ($style:expr) => {
                client
                    .post("/profile")
                    .header(ContentType::JSON)
                    .body(
                        json::to_string(&ApiProfileUpdate {
                            news_opt_in: None,
                            altitude_unit: None,
                            distance_unit: None,
                            open_viewer_in_new_tab: None,
                            reuse_viewer_tab: None,
                            default_page: None,
                            track_marker: None,
                            render_style: Some($style.into()),
                        })
                        .unwrap(),
                    )
                    .private_cookie(ctx.auth_cookie_user1())
                    .cookie(ctx.username_cookie())
                    .dispatch()
            };
        }

        // Initially, the database default (ground) applies
        assert_render_style(&ctx.conn, ctx.testuser1.user.id, "ground");

        // Switch to the experimental deck style
        let resp1 = update_render_style!("deck");
        assert_eq!(resp1.status(), Status::NoContent);
        assert_render_style(&ctx.conn, ctx.testuser1.user.id, "deck");

        // Switch back to the ground style
        let resp2 = update_render_style!("ground");
        assert_eq!(resp2.status(), Status::NoContent);
        assert_render_style(&ctx.conn, ctx.testuser1.user.id, "ground");

        // Invalid values are rejected and don't change anything
        let resp3 = update_render_style!("flat");
        assert_eq!(resp3.status(), Status::BadRequest);
        assert_render_style(&ctx.conn, ctx.testuser1.user.id, "ground");
    }

    #[test]
    fn update_open_viewer_in_new_tab() {
        let ctx = DbTestContext::new();
        let client = make_api_client();

        /// Helper function: Assert viewer tab preference state
        fn assert_open_viewer_in_new_tab(
            conn_mutex: &Mutex<PgConnection>,
            user_id: i32,
            open_in_new_tab: bool,
        ) {
            let mut conn = conn_mutex.lock().unwrap();
            let user = data::get_user(&mut conn, user_id).expect("User not found");
            assert_eq!(user.open_viewer_in_new_tab, open_in_new_tab);
        }

        /// Helper macro: Send request
        macro_rules! update_open_viewer_in_new_tab {
            ($open_in_new_tab:expr) => {
                client
                    .post("/profile")
                    .header(ContentType::JSON)
                    .body(
                        json::to_string(&ApiProfileUpdate {
                            news_opt_in: None,
                            altitude_unit: None,
                            distance_unit: None,
                            open_viewer_in_new_tab: Some($open_in_new_tab),
                            reuse_viewer_tab: None,
                            default_page: None,
                            track_marker: None,
                            render_style: None,
                        })
                        .unwrap(),
                    )
                    .private_cookie(ctx.auth_cookie_user1())
                    .cookie(ctx.username_cookie())
                    .dispatch()
            };
        }

        // Initially, the database default (open in new tab) applies
        assert_open_viewer_in_new_tab(&ctx.conn, ctx.testuser1.user.id, true);

        // Switch to opening the viewer in the same tab
        let resp1 = update_open_viewer_in_new_tab!(false);
        assert_eq!(resp1.status(), Status::NoContent);
        assert_open_viewer_in_new_tab(&ctx.conn, ctx.testuser1.user.id, false);

        // Switching back to a separate tab works, too
        let resp2 = update_open_viewer_in_new_tab!(true);
        assert_eq!(resp2.status(), Status::NoContent);
        assert_open_viewer_in_new_tab(&ctx.conn, ctx.testuser1.user.id, true);

        // Change is idempotent
        let resp3 = update_open_viewer_in_new_tab!(true);
        assert_eq!(resp3.status(), Status::NoContent);
        assert_open_viewer_in_new_tab(&ctx.conn, ctx.testuser1.user.id, true);

        // The preference is also reported when loading the profile
        let resp4 = client
            .get("/profile")
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp4.status(), Status::Ok);
        let body = resp4.into_string().expect("body");
        assert!(body.contains(r#""openViewerInNewTab":true"#));
    }

    #[test]
    fn update_reuse_viewer_tab() {
        let ctx = DbTestContext::new();
        let client = make_api_client();

        /// Helper function: Assert viewer tab reuse preference state
        fn assert_reuse_viewer_tab(conn_mutex: &Mutex<PgConnection>, user_id: i32, reuse_tab: bool) {
            let mut conn = conn_mutex.lock().unwrap();
            let user = data::get_user(&mut conn, user_id).expect("User not found");
            assert_eq!(user.reuse_viewer_tab, reuse_tab);
        }

        /// Helper macro: Send request
        macro_rules! update_reuse_viewer_tab {
            ($reuse_tab:expr) => {
                client
                    .post("/profile")
                    .header(ContentType::JSON)
                    .body(
                        json::to_string(&ApiProfileUpdate {
                            news_opt_in: None,
                            altitude_unit: None,
                            distance_unit: None,
                            open_viewer_in_new_tab: None,
                            reuse_viewer_tab: Some($reuse_tab),
                            default_page: None,
                            track_marker: None,
                            render_style: None,
                        })
                        .unwrap(),
                    )
                    .private_cookie(ctx.auth_cookie_user1())
                    .cookie(ctx.username_cookie())
                    .dispatch()
            };
        }

        // Initially, the database default (reuse the viewer tab) applies
        assert_reuse_viewer_tab(&ctx.conn, ctx.testuser1.user.id, true);

        // Switch to always opening a new viewer tab
        let resp1 = update_reuse_viewer_tab!(false);
        assert_eq!(resp1.status(), Status::NoContent);
        assert_reuse_viewer_tab(&ctx.conn, ctx.testuser1.user.id, false);

        // Switching back to reusing the viewer tab works, too
        let resp2 = update_reuse_viewer_tab!(true);
        assert_eq!(resp2.status(), Status::NoContent);
        assert_reuse_viewer_tab(&ctx.conn, ctx.testuser1.user.id, true);

        // Change is idempotent
        let resp3 = update_reuse_viewer_tab!(true);
        assert_eq!(resp3.status(), Status::NoContent);
        assert_reuse_viewer_tab(&ctx.conn, ctx.testuser1.user.id, true);

        // The preference is also reported when loading the profile
        let resp4 = client
            .get("/profile")
            .private_cookie(ctx.auth_cookie_user1())
            .cookie(ctx.username_cookie())
            .dispatch();
        assert_eq!(resp4.status(), Status::Ok);
        let body = resp4.into_string().expect("body");
        assert!(body.contains(r#""reuseViewerTab":true"#));
    }
}
