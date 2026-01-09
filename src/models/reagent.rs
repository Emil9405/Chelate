// src/models/reagent.rs
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

// ==================== REAGENT (РЕАКТИВ) ====================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Reagent {
    pub id: String,
    pub name: String,
    pub formula: Option<String>,
    pub cas_number: Option<String>,
    pub manufacturer: Option<String>,
    pub molecular_weight: Option<f64>,
    pub physical_state: Option<String>, 
    pub description: Option<String>,
    pub storage_conditions: Option<String>,
    pub appearance: Option<String>,
    pub hazard_pictograms: Option<String>,
    pub status: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateReagentRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: String,

    #[validate(length(max = 500, message = "Formula cannot exceed 500 characters"))]
    pub formula: Option<String>,

    #[validate(length(max = 50, message = "CAS number cannot exceed 50 characters"))]
    pub cas_number: Option<String>,

    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,

    #[validate(range(min = 0.0001, message = "Molecular weight must be positive (>0)"))]
    pub molecular_weight: Option<f64>,

    #[validate(length(max = 50, message = "Physical state cannot exceed 50 characters"))]
    pub physical_state: Option<String>,

    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,

    #[validate(length(max = 255, message = "Storage conditions cannot exceed 255 characters"))]
    pub storage_conditions: Option<String>,

    #[validate(length(max = 255, message = "Appearance cannot exceed 255 characters"))]
    pub appearance: Option<String>,

    #[validate(length(max = 100, message = "Hazard pictograms cannot exceed 100 characters"))]
    pub hazard_pictograms: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateReagentRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: Option<String>,

    #[validate(length(max = 500, message = "Formula cannot exceed 500 characters"))]
    pub formula: Option<String>,

    #[validate(length(max = 50, message = "CAS number cannot exceed 50 characters"))]
    pub cas_number: Option<String>,

    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,

    #[validate(range(min = 0.0001, message = "Molecular weight must be positive (>0)"))]
    pub molecular_weight: Option<f64>,

    #[validate(length(max = 50, message = "Physical state cannot exceed 50 characters"))]
    pub physical_state: Option<String>,

    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,

    #[validate(length(max = 255, message = "Storage conditions cannot exceed 255 characters"))]
    pub storage_conditions: Option<String>,

    #[validate(length(max = 255, message = "Appearance cannot exceed 255 characters"))]
    pub appearance: Option<String>,

    #[validate(length(max = 100, message = "Hazard pictograms cannot exceed 100 characters"))]
    pub hazard_pictograms: Option<String>,

    pub status: Option<String>,
}

// ==================== REAGENT WITH STOCK ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReagentWithStock {
    #[serde(flatten)]
    pub reagent: Reagent,
    pub total_quantity: Option<f64>,
    pub reserved_quantity: Option<f64>,
    pub available_quantity: Option<f64>,
    pub batches_count: i64,
    #[sqlx(default)]
    pub total_display: String,
}