-- Typed supervision/replan/knowledge controls change protected admission and
-- retrieval semantics. Older runtimes must refuse this store rather than ignore
-- unfamiliar policy data. SQLx's missing-migration check supplies that floor.
-- Existing grants, tables, ledgers and accepted results remain unchanged.
SELECT 1;
