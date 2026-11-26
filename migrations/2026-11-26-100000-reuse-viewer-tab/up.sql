-- User preference for reusing an already-open IGC track viewer tab: when
-- enabled (default), "view track" clicks navigate the existing viewer tab
-- (identified by a fixed window name) instead of opening another tab each
-- time.
--
-- The column is only used by builds with the igcviewer feature enabled;
-- it is harmless otherwise.
ALTER TABLE users
    ADD COLUMN reuse_viewer_tab BOOLEAN NOT NULL DEFAULT TRUE;
