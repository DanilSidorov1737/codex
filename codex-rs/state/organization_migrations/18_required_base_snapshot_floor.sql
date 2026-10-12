-- Older hosts must not discard the required-base marker or omit captured bytes
-- during full recovery. The migration inventory is the compatibility floor;
-- no grants, accounting or existing legacy records are rewritten.
SELECT 1;
