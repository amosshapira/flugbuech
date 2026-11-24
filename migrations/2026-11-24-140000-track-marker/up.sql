-- User preference for the marker used by the IGC track viewer replay:
-- 'glider' (the default) or 'paraglider'.
--
-- The column is only used by builds with the igcviewer feature enabled;
-- it is harmless otherwise. The value is passed to the viewer through the
-- deep link's 'marker' query parameter.
ALTER TABLE users
    ADD COLUMN track_marker VARCHAR(16) NOT NULL DEFAULT 'glider';

ALTER TABLE users
    ADD CONSTRAINT users_track_marker_check
    CHECK (track_marker IN ('glider', 'paraglider'));
