// src/models/room.rs
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub capacity: Option<i32>,
    pub color: Option<String>,
    pub status: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomStatus {
    Available,
    Reserved,
    Occupied,
    Maintenance,
    Unavailable,
}

impl RoomStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoomStatus::Available => "available",
            RoomStatus::Reserved => "reserved",
            RoomStatus::Occupied => "occupied",
            RoomStatus::Maintenance => "maintenance",
            RoomStatus::Unavailable => "unavailable",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "available" => Some(RoomStatus::Available),
            "reserved" => Some(RoomStatus::Reserved),
            "occupied" => Some(RoomStatus::Occupied),
            "maintenance" => Some(RoomStatus::Maintenance),
            "unavailable" => Some(RoomStatus::Unavailable),
            _ => None,
        }
    }

    pub fn is_valid(s: &str) -> bool {
        Self::from_str(s).is_some()
    }

    pub const fn all_values() -> &'static [&'static str] {
        &["available", "reserved", "occupied", "maintenance", "unavailable"]
    }
}

impl Default for RoomStatus {
    fn default() -> Self {
        RoomStatus::Available
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateRoomRequest {
    #[validate(length(min = 1, max = 100, message = "Room name must be between 1 and 100 characters"))]
    pub name: String,
    #[validate(length(max = 500, message = "Description cannot exceed 500 characters"))]
    pub description: Option<String>,
    #[validate(range(min = 1, max = 1000, message = "Capacity must be between 1 and 1000"))]
    pub capacity: Option<i32>,
    #[validate(length(max = 20, message = "Color code cannot exceed 20 characters"))]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRoomRequest {
    #[validate(length(min = 1, max = 100, message = "Room name must be between 1 and 100 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 500, message = "Description cannot exceed 500 characters"))]
    pub description: Option<String>,
    #[validate(range(min = 1, max = 1000, message = "Capacity must be between 1 and 1000"))]
    pub capacity: Option<i32>,
    #[validate(length(max = 20, message = "Color code cannot exceed 20 characters"))]
    pub color: Option<String>,
    pub status: Option<String>,
}

pub fn validate_room_status(value: &str) -> Result<(), validator::ValidationError> {
    if RoomStatus::is_valid(value) {
        Ok(())
    } else {
        let mut error = validator::ValidationError::new("invalid_room_status");
        error.message = Some("Room status must be 'available', 'reserved', 'occupied', 'maintenance', or 'unavailable'".into());
        Err(error)
    }
}