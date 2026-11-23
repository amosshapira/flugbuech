-- Sailplane metadata for gliders.
--
-- All fields except the boolean flags are optional, so existing paraglider
-- entries remain valid without changes.
ALTER TABLE gliders
    ADD COLUMN registration VARCHAR(16),
    ADD COLUMN competition_id VARCHAR(8),
    ADD COLUMN is_two_seater BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN fai_class VARCHAR(32),
    ADD COLUMN is_motorglider BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE gliders
    ADD CONSTRAINT gliders_fai_class_check
    CHECK (
        fai_class IS NULL
        OR fai_class IN ('Club', 'Standard', '15m', '18m', '20m', 'Open', 'Two-Seater')
    );

-- Allow multiple gliders of the same model (e.g. two club ASK-21s) when they
-- have different registrations, while keeping the old uniqueness rule for
-- unregistered (typically paraglider) entries.
ALTER TABLE gliders DROP CONSTRAINT unique_per_user;
CREATE UNIQUE INDEX gliders_unique_unregistered
    ON gliders (user_id, manufacturer, model)
    WHERE registration IS NULL;
CREATE UNIQUE INDEX gliders_unique_registered
    ON gliders (user_id, manufacturer, model, registration)
    WHERE registration IS NOT NULL;

-- Sailplane / gliding flight data and CASA-compliant crew recording.
--
-- Existing flights default to PIC (the normal case for paraglider flights), and
-- all other values are optional.
ALTER TABLE flights
    ADD COLUMN pilot_in_command VARCHAR(128),
    ADD COLUMN second_pilot VARCHAR(128),
    ADD COLUMN flight_capacity VARCHAR(16) NOT NULL DEFAULT 'PIC',
    ADD COLUMN launch_type VARCHAR(32),
    ADD COLUMN release_altitude_m INTEGER,
    ADD COLUMN tow_height_m INTEGER,
    ADD COLUMN tug_registration VARCHAR(16),
    ADD COLUMN max_altitude_m INTEGER,
    ADD COLUMN height_gain_m INTEGER,
    ADD COLUMN is_outlanding BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN engine_run_time INTERVAL;

ALTER TABLE flights
    ADD CONSTRAINT flights_flight_capacity_check
    CHECK (flight_capacity IN ('PIC', 'Dual', 'Instructor', 'ICUS'));

ALTER TABLE flights
    ADD CONSTRAINT flights_launch_type_check
    CHECK (
        launch_type IS NULL
        OR launch_type IN ('Aerotow', 'Winch', 'Car', 'Self-Launch')
    );

-- User preferences for displaying altitudes and distances in the UI.
--
-- All measurement data is stored in metric units (metres and kilometres);
-- these columns only control how it is presented to the user.
ALTER TABLE users
    ADD COLUMN altitude_unit VARCHAR(8) NOT NULL DEFAULT 'm',
    ADD COLUMN distance_unit VARCHAR(8) NOT NULL DEFAULT 'km';

ALTER TABLE users
    ADD CONSTRAINT users_altitude_unit_check
    CHECK (altitude_unit IN ('m', 'ft'));

ALTER TABLE users
    ADD CONSTRAINT users_distance_unit_check
    CHECK (distance_unit IN ('km', 'mi', 'nm'));
