use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortMode {
    BestRated,
    SoonestAvailable,
    LowestPrice,
}

impl SortMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "best_rated" => Some(Self::BestRated),
            "soonest_available" => Some(Self::SoonestAvailable),
            "lowest_price" => Some(Self::LowestPrice),
            _ => None,
        }
    }
}

// --- Service ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub rating: f64,
    pub coverage_radius_miles: i32,
    pub zip_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateServiceRequest {
    pub name: String,
    pub description: String,
    pub price: f64,
    pub rating: Option<f64>,
    pub coverage_radius_miles: i32,
    pub zip_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateServiceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<f64>,
    pub rating: Option<f64>,
    pub coverage_radius_miles: Option<i32>,
    pub zip_code: Option<String>,
}

// --- Category ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategoryRequest {
    pub parent_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignCategoryRequest {
    pub category_id: String,
}

// --- Tag ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignTagRequest {
    pub tag_id: String,
}

// --- Availability ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityWindow {
    pub id: String,
    pub service_id: String,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAvailabilityRequest {
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
}

// --- Comparison ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceComparison {
    pub service: Service,
    pub categories: Vec<Category>,
    pub tags: Vec<Tag>,
    pub availability: Vec<AvailabilityWindow>,
}
