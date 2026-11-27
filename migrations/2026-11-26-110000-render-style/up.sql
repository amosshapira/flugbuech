-- User preference for the track viewer rendering style:
-- 'ground' (default; maplibre terrain, track clamped onto the terrain
-- surface) or 'deck' (experimental; the terrain mesh + imagery rendered
-- by deck.gl itself, the track flying at true altitude).
--
-- The column is only used by builds with the igcviewer feature enabled;
-- it is harmless otherwise. The value is passed to the viewer through the
-- deep link's 'style' query parameter.
ALTER TABLE users
    ADD COLUMN render_style VARCHAR(8) NOT NULL DEFAULT 'ground';

ALTER TABLE users
    ADD CONSTRAINT users_render_style_check
    CHECK (render_style IN ('ground', 'deck'));
