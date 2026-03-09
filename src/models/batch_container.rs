// src/models/batch_container.rs
//! Model: physical container (jar/bottle) within a batch
//! One Batch has N containers. Each container may be placed at 0..1 position.

use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

// ==================== BATCH CONTAINER ====================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct BatchContainer {
    pub id: String,
    pub batch_id: String,
    pub sequence_number: i64,
    #[serde(rename = "container_quantity")]
    pub quantity: f64,
    pub original_quantity: f64,
    pub is_opened: bool,            // SQLite INTEGER 0/1 → bool via sqlx
    pub opened_at: Option<DateTime<Utc>>,
    pub opened_by: Option<String>,
    #[serde(rename = "container_status")]
    pub status: String,             // full | partial | empty | disposed
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Container with its placement location (if placed)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct ContainerWithLocation {
    // Container fields
    pub id: String,
    pub batch_id: String,
    pub sequence_number: i64,
    pub quantity: f64,
    pub original_quantity: f64,
    pub is_opened: bool,
    pub opened_at: Option<DateTime<Utc>>,
    pub opened_by: Option<String>,
    pub container_status: String,
    pub container_notes: Option<String>,
    pub container_created_at: DateTime<Utc>,
    pub container_updated_at: DateTime<Utc>,
    // Placement + Location (NULL if not placed)
    pub placement_id: Option<String>,
    pub position_id: Option<String>,
    pub position_name: Option<String>,
    pub position_label: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub zone_type: Option<String>,
    pub room_id: Option<String>,
    pub room_name: Option<String>,
    pub room_color: Option<String>,
}

impl ContainerWithLocation {
    /// "Lab-1 → Cabinet A → Shelf 2" or "Not placed"
    pub fn location_path(&self) -> String {
        match (&self.room_name, &self.zone_name, &self.position_name) {
            (Some(room), Some(zone), Some(pos)) => {
                format!("{} → {} → {}", room, zone, pos)
            }
            _ => "Not placed".to_string(),
        }
    }

    pub fn is_placed(&self) -> bool {
        self.placement_id.is_some()
    }
}

/// Aggregate stats for batch containers
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerStats {
    pub container_count: i64,
    pub opened_count: i64,
    pub sealed_count: i64,
    pub placed_count: i64,
    pub unplaced_count: i64,
    pub empty_count: i64,
}

// ==================== REQUESTS ====================

#[derive(Debug, Deserialize, Validate)]
pub struct CreateContainerRequest {
    #[validate(range(min = 0.001, message = "Quantity must be positive"))]
    pub quantity: f64,

    #[validate(length(max = 500, message = "Notes cannot exceed 500 characters"))]
    pub notes: Option<String>,
}

/// Request to split a batch into N containers by pack_size
#[derive(Debug, Deserialize, Validate)]
pub struct SplitBatchRequest {
    #[validate(range(min = 0.001, message = "Pack size must be positive"))]
    pub pack_size: f64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PlaceContainerRequest {
    pub position_id: String,

    #[validate(length(max = 500, message = "Notes cannot exceed 500 characters"))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveContainerRequest {
    pub new_position_id: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UseFromContainerRequest {
    #[validate(range(min = 0.001, message = "Quantity must be positive"))]
    pub quantity: f64,

    #[validate(length(max = 500, message = "Purpose cannot exceed 500 characters"))]
    pub purpose: Option<String>,

    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
}
