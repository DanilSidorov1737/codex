-- New owner definition proposals and narrow known-startup recovery controls
-- must never be silently ignored by an older execution host. SQLx rejects stores
-- with this unknown migration. Existing receipts and budgets remain intact.
SELECT 1;
