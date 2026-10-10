-- Paused/finished state does not prove that a native runtime has been joined.
ALTER TABLE organization_attempts ADD COLUMN occupied INTEGER NOT NULL DEFAULT 1 CHECK(occupied IN (0,1));
CREATE INDEX organization_attempts_occupied ON organization_attempts(occupied);
