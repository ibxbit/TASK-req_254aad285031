// Some wrappers are intentionally shipped even if no page wires them up
// yet — they round out the admin/mentor/warehouse toolkits so adding a
// new control in a page is a single-line change.
#![allow(dead_code)]

pub mod catalog;
pub mod client;
pub mod face;
pub mod forum;
pub mod internships;
pub mod warehouse;
pub mod workorders;
