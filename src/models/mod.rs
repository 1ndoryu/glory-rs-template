pub mod article;
pub mod media;
mod note;
pub mod notification;
pub mod preferences;
pub mod product;
pub mod project;
pub mod resource;
pub mod settings;
pub mod user;
pub mod workspace;
pub mod workspace_overlay;

pub use note::{CreateNoteRequest, Note, PaginatedNotes, PaginationParams, UpdateNoteRequest};
pub use notification::{
    CreateNotificationRequest, Notification, NotificationList, UpdateNotificationStatusRequest,
};
pub use resource::{EditorialState, LifecycleState, Resource, ResourceKind, VisibilityState};
pub use user::{AuthResponse, LoginRequest, RegisterRequest, User, UserResponse};
