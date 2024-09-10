use crate::PRv2;
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
        self.contributor_type == AuthorAssociation::FirstTimeContributor
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
pub struct RepoInfo {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub full_id: String,
}

impl RepoInfo {
    pub fn from_issue(
        issue: octocrab::models::issues::Issue,
        repo: octocrab::models::Repository,
    ) -> Option<Self> {
        let owner = repo.owner?.login;
        let repo = repo.name;
        let number = issue.number;
        let full_id = format!("{}/{}/{}", owner, repo, number);
        Some(Self {
            owner,
            repo,
            number,
            full_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PrMetadata {
    pub repo_info: RepoInfo,
    pub author: User,
    pub created: chrono::DateTime<chrono::Utc>,
    pub merged: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub body: String,
    pub closed: bool,
}

impl From<PRv2> for PrMetadata {
    fn from(pr: PRv2) -> Self {
        let full_id = format!("{}/{}/{}", pr.organization, pr.repo, pr.number);
        Self {
            repo_info: RepoInfo {
                owner: pr.organization,
                repo: pr.repo,
                number: pr.number,
                full_id,
            },
            author: User::new(pr.author, AuthorAssociation::None),
            body: Default::default(),
            created: chrono::DateTime::from_timestamp_nanos(
                pr.created_at.unwrap_or_default() as i64
            ),
            merged: pr
                .merged_at
                .map(|e| chrono::DateTime::from_timestamp_nanos(e as i64)),
            updated_at: chrono::DateTime::from_timestamp_nanos(
                pr.merged_at.or(pr.created_at).unwrap_or(pr.included_at) as i64,
            ),
            closed: false,
        }
    }
}

impl TryFrom<octocrab::models::pulls::PullRequest> for PrMetadata {
    type Error = anyhow::Error;

    fn try_from(pr: octocrab::models::pulls::PullRequest) -> anyhow::Result<Self> {
        let repo = pr.base.repo.map(|repo| (repo.owner, repo.name));
        let body: String = pr
            .body
            .or(pr.body_text)
            .or(pr.body_html)
            .unwrap_or_default();

        if let (
            Some((Some(owner), repo)),
            Some(user),
            Some(author_association),
            Some(created_at),
            Some(updated_at),
        ) = (
            repo,
            pr.user,
            pr.author_association,
            pr.created_at,
            pr.updated_at,
        ) {
            let full_id = format!("{}/{}/{}", owner.login, repo, pr.number);
            Ok(Self {
                repo_info: RepoInfo {
                    owner: owner.login,
                    repo,
                    number: pr.number,
                    full_id,
                },
                body,
                author: User::new(user.login, author_association),
                created: created_at,
                merged: pr.merged_at,
                updated_at,
                closed: pr.closed_at.is_some(),
            })
        } else {
            Err(anyhow::anyhow!("Missing required fields"))
        }
    }
}
