select
    r.id as repo_id,
    r.name as repo,
    o.login as organization,
    o.full_name as organization_full_name
from
    repos as r
    JOIN organizations o ON r.organization_id = o.id for
update
