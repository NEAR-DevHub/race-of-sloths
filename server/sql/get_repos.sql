select
    r.id as repo_id,
    r.name as repo,
    o.name as organization
from
    repos as r
    JOIN organizations o ON r.organization_id = o.id
