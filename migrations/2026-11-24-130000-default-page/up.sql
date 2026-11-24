-- User preference for the page to land on after login.
--
-- The allowed values mirror the main navigation destinations for logged-in
-- users. The frontend redirects to this page after login unless an explicit
-- redirect target was requested.
ALTER TABLE users
    ADD COLUMN default_page VARCHAR(32) NOT NULL DEFAULT '/';

ALTER TABLE users
    ADD CONSTRAINT users_default_page_check
    CHECK (default_page IN ('/', '/flights/', '/gliders/', '/locations/', '/stats/'));
