// Shared constants for the work-order + review module.
// Values are spec-defined and must not vary at runtime
// (reputation score must be reproducible).

pub const MAX_IMAGES_PER_REVIEW: i64 = 5;
pub const MAX_IMAGE_SIZE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_REVIEWS_PER_DAY: i64 = 3;
pub const REVIEW_WINDOW_DAYS: i64 = 14;
pub const DECAY_DAYS: f64 = 180.0;
