ALTER TABLE flights
    DROP CONSTRAINT flights_launch_type_check,
    DROP CONSTRAINT flights_flight_capacity_check,
    DROP COLUMN engine_run_time,
    DROP COLUMN is_outlanding,
    DROP COLUMN height_gain_m,
    DROP COLUMN max_altitude_m,
    DROP COLUMN tug_registration,
    DROP COLUMN tow_height_m,
    DROP COLUMN release_altitude_m,
    DROP COLUMN launch_type,
    DROP COLUMN flight_capacity,
    DROP COLUMN second_pilot,
    DROP COLUMN pilot_in_command;

ALTER TABLE users
    DROP CONSTRAINT users_distance_unit_check,
    DROP CONSTRAINT users_altitude_unit_check,
    DROP COLUMN distance_unit,
    DROP COLUMN altitude_unit;

ALTER TABLE gliders
    DROP CONSTRAINT gliders_fai_class_check;

-- Drop the registration-aware uniqueness indexes before removing the columns
-- they depend on.
DROP INDEX gliders_unique_registered;
DROP INDEX gliders_unique_unregistered;

ALTER TABLE gliders
    DROP COLUMN is_motorglider,
    DROP COLUMN fai_class,
    DROP COLUMN is_two_seater,
    DROP COLUMN competition_id,
    DROP COLUMN registration;

-- Restore the previous uniqueness rule. This fails if the database contains
-- multiple gliders with the same manufacturer/model but different
-- registrations.
ALTER TABLE gliders
    ADD CONSTRAINT unique_per_user UNIQUE (user_id, manufacturer, model);
