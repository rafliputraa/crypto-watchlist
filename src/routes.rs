use actix_web::web;
use crate::health::get_health;
use crate::watchlistgroup::{create_watchlist_group, delete_watchlist_group, retrieve_all_watchlist_groups, update_watchlist_group};

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/health", web::get().to(get_health))
        .service(
            web::scope("/api/v1")
                .service(
                    web::scope("/watchlistgroup")
                        .route("/{user_id}", web::get().to(retrieve_all_watchlist_groups))
                        .route("/{user_id}", web::post().to(create_watchlist_group))
                        .route("/{user_id}", web::put().to(update_watchlist_group))
                        .route("/{user_id}", web::delete().to(delete_watchlist_group))
                )
    );
}