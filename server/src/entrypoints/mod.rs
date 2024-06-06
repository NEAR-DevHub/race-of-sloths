use rocket::fairing::AdHoc;

pub mod leaderboards;
pub mod types;
pub mod user;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Installing entrypoints", |rocket| async {
        rocket.attach(user::stage()).attach(leaderboards::stage())
    })
}
