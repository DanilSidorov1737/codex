-- New goal review/context and host-observed loop protection cannot be ignored by
-- an older execution host. Existing grants, receipts and usage remain intact.
ALTER TABLE organization_coordination_calls ADD COLUMN progress_generation INTEGER NOT NULL DEFAULT 0;
