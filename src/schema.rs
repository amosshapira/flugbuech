#![allow(unused_imports)]

table! {
    use diesel::sql_types::*;
    use diesel_geography::sql_types::*;

    flights (id) {
        id -> Int4,
        number -> Nullable<Int4>,
        user_id -> Int4,
        glider_id -> Nullable<Int4>,
        launch_at -> Nullable<Int4>,
        landing_at -> Nullable<Int4>,
        launch_time -> Nullable<Timestamptz>,
        landing_time -> Nullable<Timestamptz>,
        track_distance -> Nullable<Float4>,
        xcontest_tracktype -> Nullable<Text>,
        xcontest_distance -> Nullable<Float4>,
        xcontest_url -> Nullable<Text>,
        comment -> Nullable<Text>,
        video_url -> Nullable<Text>,
        created_at -> Timestamptz,
        hikeandfly -> Bool,
        pilot_in_command -> Nullable<Varchar>,
        second_pilot -> Nullable<Varchar>,
        flight_capacity -> Varchar,
        launch_type -> Nullable<Varchar>,
        release_altitude_m -> Nullable<Int4>,
        tow_height_m -> Nullable<Int4>,
        tug_registration -> Nullable<Varchar>,
        max_altitude_m -> Nullable<Int4>,
        height_gain_m -> Nullable<Int4>,
        is_outlanding -> Bool,
        engine_run_time -> Nullable<Interval>,
    }
}

table! {
    use diesel::sql_types::*;
    use diesel_geography::sql_types::*;

    gliders (id) {
        id -> Int4,
        user_id -> Int4,
        model -> Varchar,
        manufacturer -> Varchar,
        since -> Nullable<Date>,
        until -> Nullable<Date>,
        source -> Nullable<Text>,
        cost -> Nullable<Int4>,
        comment -> Nullable<Text>,
        registration -> Nullable<Varchar>,
        competition_id -> Nullable<Varchar>,
        is_two_seater -> Bool,
        fai_class -> Nullable<Varchar>,
        is_motorglider -> Bool,
    }
}

table! {
    use diesel::sql_types::*;
    use diesel_geography::sql_types::*;

    igcs (flight_id) {
        flight_id -> Int4,
        data -> Bytea,
    }
}

table! {
    use diesel::sql_types::*;
    use diesel_geography::sql_types::*;

    locations (id) {
        id -> Int4,
        name -> Text,
        country -> Bpchar,
        elevation -> Int4,
        user_id -> Int4,
        geog -> Nullable<Geography>,
    }
}

table! {
    use diesel::sql_types::*;
    use diesel_geography::sql_types::*;

    spatial_ref_sys (srid) {
        srid -> Int4,
        auth_name -> Nullable<Varchar>,
        auth_srid -> Nullable<Int4>,
        srtext -> Nullable<Varchar>,
        proj4text -> Nullable<Varchar>,
    }
}

table! {
    use diesel::sql_types::*;
    use diesel_geography::sql_types::*;

    users (id) {
        id -> Int4,
        username -> Varchar,
        password -> Text,
        last_glider_id -> Nullable<Int4>,
        email -> Text,
        signed_up -> Timestamptz,
        news_opt_in -> Bool,
        altitude_unit -> Varchar,
        distance_unit -> Varchar,
        open_viewer_in_new_tab -> Bool,
        reuse_viewer_tab -> Bool,
        default_page -> Varchar,
        track_marker -> Varchar,
        render_style -> Varchar,
    }
}

joinable!(flights -> gliders (glider_id));
joinable!(flights -> users (user_id));
joinable!(igcs -> flights (flight_id));
joinable!(locations -> users (user_id));

allow_tables_to_appear_in_same_query!(flights, gliders, igcs, locations, spatial_ref_sys, users,);
