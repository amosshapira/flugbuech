-- User preference for how the IGC track viewer ("view track" button) is
-- opened: in a separate browser tab (default) or in the same tab.
--
-- The column is only used by builds with the igcviewer feature enabled;
-- it is harmless otherwise.
ALTER TABLE users
    ADD COLUMN open_viewer_in_new_tab BOOLEAN NOT NULL DEFAULT TRUE;
