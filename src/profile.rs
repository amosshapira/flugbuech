//! Profile views.

use chrono::{DateTime, Utc};
use log::error;
use rocket::{get, http::Status, post, routes, serde::json::Json, Route};
use serde::{Deserialize, Serialize};

use crate::{auth, data, responders::ApiError};

/// Valid values for the user's preferred display units.
pub const ALTITUDE_UNITS: [&str; 2] = ["m", "ft"];
pub const DISTANCE_UNITS: [&str; 3] = ["km", "mi", "nm"];

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
}
