// Services module - Business logic components
//
// Contains pure business logic for plugin cleaning operations.

pub mod cleaning;

pub use cleaning::{CleaningService, CleaningError, CleaningStats, CleanResult, CleanStatus};
