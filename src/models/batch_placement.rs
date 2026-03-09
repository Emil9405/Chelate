// src/models/batch_placement.rs
//! Placement model (v3 — container-based)
//! Links a BatchContainer to a StoragePosition.
//! One container can be in at most one position (UNIQUE container_id).

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ==================== BATCH PLACEMENT (v3 — container → position) ====================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct BatchPlacement {
    pub id: String,
    pub container_id: String,
    pub position_id: String,
    pub placed_by: Option<String>,
    pub placed_at: DateTime<Utc>,
    pub notes: Option<String>,
}

/// Backward-compat alias: PlacementWithRoom
/// Used by batch_handlers BatchResponse. 
/// Now returns container-based placement with full location path.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct PlacementWithLocation {
    // Container data
    pub container_id: String,
    pub sequence_number: i64,
    pub container_quantity: f64,
    pub is_opened: bool,
    pub container_status: String,
    // Placement data
    pub placement_id: Option<String>,
    pub position_id: Option<String>,
    pub placed_by: Option<String>,
    // Location hierarchy
    pub position_name: Option<String>,
    pub position_label: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub zone_type: Option<String>,
    pub room_id: Option<String>,
    pub room_name: Option<String>,
    pub room_color: Option<String>,
}

impl PlacementWithLocation {
    /// "Lab-1 → Cabinet A → Shelf 2" or "Not placed"
    pub fn full_path(&self) -> String {
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

/// Backward-compat alias
pub type PlacementWithRoom = PlacementWithLocation;

/// Inventory item for a position (room inventory view)
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PositionInventoryItem {
    // Container
    pub container_id: String,
    pub sequence_number: i64,
    pub container_quantity: f64,
    pub is_opened: bool,
    pub container_status: String,
    // Position
    pub position_id: String,
    pub position_name: String,
    // Batch
    pub batch_id: String,
    pub batch_number: String,
    pub lot_number: Option<String>,
    pub unit: String,
    pub total_quantity: f64,
    pub expiry_date: Option<DateTime<Utc>>,
    pub batch_status: String,
    // Reagent
    pub reagent_id: String,
    pub reagent_name: String,
    pub formula: Option<String>,
    pub cas_number: Option<String>,
    pub hazard_pictograms: Option<String>,
}
