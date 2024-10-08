use rocket::fairing::AdHoc;
use utoipa::OpenApi;

pub mod aliases;
pub mod leaderboards;
pub mod statistics;
pub mod user;

use race_of_sloths_server::types;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Race-of-sloths server API",
        version = "0.0.1",
    ),
    paths(
        leaderboards::get_leaderboard,
        leaderboards::get_repos,
        user::get_user,
        user::get_user_contributions,
        user::get_badge,
        statistics::get_statistics
    ),
    components(schemas(
        types::PaginatedResponse<types::LeaderboardResponse>,
        types::PaginatedLeaderboardResponse,
        types::PaginatedResponse<types::RepoResponse>,
        types::PaginatedRepoResponse,
        types::PaginatedResponse<types::UserContributionResponse>,
        types::PaginatedUserContributionResponse,
        types::UserContributionResponse,
        types::LeaderboardResponse,
        types::RepoResponse,
        types::UserProfile,
        types::GithubMeta,
        types::Streak,
        types::Statistics
    )),
    tags(
        (name = "Race of Sloths", description = "Race of Sloths endpoints.")
    ),
)]
pub struct ApiDoc;

pub fn stage(font: String) -> AdHoc {
    AdHoc::on_ignite("Installing entrypoints", |rocket| async {
        rocket
            .attach(user::stage(font))
            .attach(leaderboards::stage())
            .attach(aliases::stage())
            .attach(statistics::stage())
    })
}
