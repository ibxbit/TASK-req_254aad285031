#[macro_use]
extern crate rocket;

pub mod audit;
pub mod auth;
pub mod config;
pub mod crypto;
pub mod db;
pub mod face;
pub mod forum;
pub mod internships;
pub mod logging;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod warehouse_audit;
pub mod workorders;

use rocket::fairing::AdHoc;

#[launch]
fn rocket() -> _ {
    let cfg = config::AppConfig::load("./config/config.toml")
        .expect("failed to load ./config/config.toml (copy from config/config.example.toml)");

    let db_url = cfg.database.url.clone();
    let storage_cfg = cfg.storage.clone();
    let policy_cfg = cfg.policy.clone();

    let encryptor = crypto::Encryptor::from_file(&cfg.encryption.key_file).expect(
        "failed to load encryption key file — generate with: \
         head -c 32 /dev/urandom > ./config/master.key",
    );

    let figment = rocket::Config::figment()
        .merge(("address", cfg.server.bind_address.clone()))
        .merge(("port", cfg.server.port))
        .merge(("limits.file", "10 MiB"))
        .merge(("limits.data-form", "11 MiB"));

    rocket::custom(figment)
        .manage(storage_cfg)
        .manage(policy_cfg)
        .manage(encryptor)
        .attach(AdHoc::on_ignite("db_pool", move |rocket| async move {
            let pool = db::init_pool(&db_url)
                .await
                .expect("failed to initialize MySQL pool");
            rocket.manage(pool)
        }))
        .mount(
            "/api",
            routes![
                // Health
                routes::health::healthcheck,
                // Auth
                routes::auth::register,
                routes::auth::login,
                routes::auth::logout,
                routes::auth::me,
                // Admin: users + teams
                routes::admin::users::list,
                routes::admin::users::create,
                routes::admin::users::update_role,
                routes::admin::users::update_password,
                routes::admin::users::update_status,
                routes::admin::users::update_sensitive,
                routes::admin::teams::list,
                routes::admin::teams::create,
                routes::admin::teams::delete,
                routes::admin::teams::list_members,
                routes::admin::teams::add_member,
                routes::admin::teams::remove_member,
                // Forum: zones
                routes::forum::zones::list,
                routes::forum::zones::create,
                routes::forum::zones::update,
                routes::forum::zones::delete,
                // Forum: boards
                routes::forum::boards::list,
                routes::forum::boards::get,
                routes::forum::boards::create,
                routes::forum::boards::update,
                routes::forum::boards::delete,
                routes::forum::boards::list_moderators,
                routes::forum::boards::add_moderator,
                routes::forum::boards::remove_moderator,
                routes::forum::boards::list_rules,
                routes::forum::boards::create_rule,
                routes::forum::boards::delete_rule,
                routes::forum::boards::allow_team,
                routes::forum::boards::disallow_team,
                // Forum: posts
                routes::forum::posts::list_by_board,
                routes::forum::posts::get,
                routes::forum::posts::create,
                routes::forum::posts::pin,
                // Forum: comments
                routes::forum::comments::list_by_post,
                routes::forum::comments::create,
                routes::forum::comments::delete,
                // Catalog: services
                routes::catalog::services::create,
                routes::catalog::services::update,
                routes::catalog::services::get,
                routes::catalog::services::search,
                routes::catalog::services::compare,
                routes::catalog::services::favorite,
                // Catalog: categories
                routes::catalog::categories::list,
                routes::catalog::categories::create,
                routes::catalog::categories::assign,
                // Catalog: tags
                routes::catalog::tags::list,
                routes::catalog::tags::create,
                routes::catalog::tags::assign,
                // Catalog: availability
                routes::catalog::availability::create,
                // Work orders + reviews
                routes::workorders::orders::create,
                routes::workorders::orders::get,
                routes::workorders::orders::complete,
                routes::workorders::reviews::create,
                routes::workorders::reviews::create_follow_up,
                routes::workorders::reviews::list_for_service,
                routes::workorders::reviews::pin,
                routes::workorders::reviews::collapse,
                routes::workorders::reviews::create_tag,
                routes::workorders::reviews::list_tags,
                routes::workorders::reviews::assign_tag,
                routes::workorders::images::upload,
                routes::workorders::reputation::get,
                // Internships
                routes::internships::plans::create,
                routes::internships::reports::create,
                routes::internships::reports::add_comment,
                routes::internships::reports::approve,
                routes::internships::attachments::upload,
                routes::internships::dashboard::get,
                // Warehouse
                routes::warehouse::warehouses::create,
                routes::warehouse::warehouses::rename,
                routes::warehouse::warehouses::delete,
                routes::warehouse::warehouses::tree,
                routes::warehouse::warehouses::history,
                routes::warehouse::zones::create,
                routes::warehouse::zones::rename,
                routes::warehouse::zones::delete,
                routes::warehouse::zones::history,
                routes::warehouse::bins::create,
                routes::warehouse::bins::update,
                routes::warehouse::bins::history,
                // Face data
                routes::face::records::create,
                routes::face::records::validate,
                routes::face::records::deactivate,
                routes::face::records::list_for_user,
                routes::face::records::record_liveness,
                // Audit
                routes::audit::events::verify,
                routes::audit::events::list_for_entity,
            ],
        )
}
