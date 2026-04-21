use dioxus::prelude::*;

use crate::components::layout::AuthedLayout;
use crate::pages::admin::Admin;
use crate::pages::catalog::Catalog;
use crate::pages::face::Face;
use crate::pages::forum::Forum;
use crate::pages::home::Home;
use crate::pages::internship::Internship;
use crate::pages::login::Login;
use crate::pages::warehouse::Warehouse;
use crate::pages::work_orders::WorkOrders;

#[derive(Clone, PartialEq, Routable)]
pub enum Route {
    #[route("/login")]
    Login {},

    #[layout(AuthedLayout)]
    #[route("/")]
    Home {},

    #[route("/catalog")]
    Catalog {},

    #[route("/forum")]
    Forum {},

    #[route("/internship")]
    Internship {},

    #[route("/warehouse")]
    Warehouse {},

    #[route("/work-orders")]
    WorkOrders {},

    #[route("/face")]
    Face {},

    #[route("/admin")]
    Admin {},
}

#[cfg(test)]
mod tests {
    use super::Route;

    #[test]
    fn route_strings_match_expected_paths() {
        assert_eq!(Route::Login {}.to_string(), "/login");
        assert_eq!(Route::Home {}.to_string(), "/");
        assert_eq!(Route::Catalog {}.to_string(), "/catalog");
        assert_eq!(Route::Forum {}.to_string(), "/forum");
        assert_eq!(Route::Internship {}.to_string(), "/internship");
        assert_eq!(Route::Warehouse {}.to_string(), "/warehouse");
        assert_eq!(Route::WorkOrders {}.to_string(), "/work-orders");
        assert_eq!(Route::Face {}.to_string(), "/face");
        assert_eq!(Route::Admin {}.to_string(), "/admin");
    }

    #[test]
    fn every_route_path_is_unique() {
        let mut paths = vec![
            Route::Login {}.to_string(),
            Route::Home {}.to_string(),
            Route::Catalog {}.to_string(),
            Route::Forum {}.to_string(),
            Route::Internship {}.to_string(),
            Route::Warehouse {}.to_string(),
            Route::WorkOrders {}.to_string(),
            Route::Face {}.to_string(),
            Route::Admin {}.to_string(),
        ];
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), 9);
    }
}
