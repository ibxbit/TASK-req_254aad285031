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
