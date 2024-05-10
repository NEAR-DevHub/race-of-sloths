use octocrab::models::AuthorAssociation;

#[derive(Debug, Clone)]
pub struct User {
    pub login: String,
    pub contributor_type: AuthorAssociation,
}

impl User {
    pub fn new(login: String, contributor_type: AuthorAssociation) -> Self {
        Self {
            login,
            contributor_type,
        }
    }

    pub fn is_participant(&self) -> bool {
        // https://docs.github.com/en/graphql/reference/enums#commentauthorassociation
        // We probably shouldn't allow collaborators / members / owners to get points
        // as they are already part of the project
        self.contributor_type == AuthorAssociation::Contributor
            || self.contributor_type == AuthorAssociation::FirstTimeContributor
            || self.contributor_type == AuthorAssociation::FirstTimer
            || self.contributor_type == AuthorAssociation::None
    }

    pub fn is_maintainer(&self) -> bool {
        self.contributor_type == AuthorAssociation::Owner
            || self.contributor_type == AuthorAssociation::Member
            || self.contributor_type == AuthorAssociation::Collaborator
    }
}

#[derive(Debug, Clone)]
pub struct PrMetadata {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub author: User,
    pub title: String,
    pub started: chrono::DateTime<chrono::Utc>,
    pub merged: Option<chrono::DateTime<chrono::Utc>>,
    pub full_id: String,
}

impl TryFrom<octocrab::models::pulls::PullRequest> for PrMetadata {
    type Error = anyhow::Error;

    fn try_from(pr: octocrab::models::pulls::PullRequest) -> anyhow::Result<Self> {
        let repo = pr.base.repo.map(|repo| (repo.owner, repo.name));

        if let (
            Some((Some(owner), repo)),
            Some(user),
            Some(title),
            Some(author_association),
            Some(created_at),
        ) = (
            repo,
            pr.user,
            pr.title,
            pr.author_association,
            pr.created_at,
        ) {
            let full_id = format!("{}/{}/{}", owner.login, repo, pr.number);
            Ok(Self {
                owner: owner.login,
                repo,
                number: pr.number,
                author: User::new(user.login, author_association),
                title,
                started: created_at,
                merged: pr.merged_at,
                full_id,
            })
        } else {
            Err(anyhow::anyhow!("Missing required fields"))
        }
    }
}
