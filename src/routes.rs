use actix_web::web;
use crate::health::get_health;
use crate::watchlist::{create_watchlist, delete_watchlist, retrieve_all_watchlist};
use crate::watchlistgroup::{create_watchlist_group, delete_watchlist_group, retrieve_all_watchlist_groups, update_watchlist_group};

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(
            web::resource("health")
                .route(web::get().to(get_health))
        )
        .service(
            web::scope("/api/v1")
                .service(
                    web::scope("/watchlistgroup")
                        .route("", web::get().to(retrieve_all_watchlist_groups))
                        .route("", web::post().to(create_watchlist_group))
                        .route("/{group_id}", web::put().to(update_watchlist_group))
                        .route("/{group_id}", web::delete().to(delete_watchlist_group))
                )
                .service(
                    web::scope("/watchlist")
                        .route("", web::post().to(create_watchlist))
                        .route("/{group_id}", web::get().to(retrieve_all_watchlist))
                        .route("", web::delete().to(delete_watchlist))
                )
    );
}