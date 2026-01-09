// src/models/batch.rs
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Batch {
    pub id: String,
    pub reagent_id: String,
    pub lot_number: Option<String>,
    pub batch_number: String,
    pub cat_number: Option<String>,
    pub quantity: f64,
    pub original_quantity: f64,
    pub reserved_quantity: f64,
    pub unit: String,
    pub expiry_date: Option<DateTime<Utc>>,
    pub supplier: Option<String>,
    pub manufacturer: Option<String>,
    pub received_date: DateTime<Utc>,
    pub status: String,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct BatchWithReagentInfo {
    pub id: String,
    pub reagent_id: String,
    pub reagent_name: String,
    pub lot_number: Option<String>,
    pub batch_number: String,
    pub cat_number: Option<String>,
    pub quantity: f64,
    pub original_quantity: f64,
    pub reserved_quantity: f64,
    pub unit: String,
    pub expiry_date: Option<DateTime<Utc>>,
    pub supplier: Option<String>,
    pub manufacturer: Option<String>,
    pub received_date: DateTime<Utc>,
    pub status: String,
    pub location: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateBatchRequest {
    #[validate(length(max = 100, message = "Lot number cannot exceed 100 characters"))]
    pub lot_number: Option<String>,
    #[validate(length(min = 1, max = 100, message = "Batch number must be between 1 and 100 characters"))]
    pub batch_number: String,
    #[validate(length(max = 100, message = "Cat number cannot exceed 100 characters"))]
    pub cat_number: Option<String>,
    #[validate(range(min = 0.0, message = "Quantity must be non-negative"))]
    pub quantity: f64,
    #[validate(length(min = 1, max = 20, message = "Unit must be between 1 and 20 characters"))]
    pub unit: String,
    pub expiry_date: Option<DateTime<Utc>>,
    #[validate(length(max = 255, message = "Supplier cannot exceed 255 characters"))]
    pub supplier: Option<String>,
    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,
    #[validate(length(max = 255, message = "Location cannot exceed 255 characters"))]
    pub location: Option<String>,
    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
    pub received_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateBatchRequest {
    #[validate(length(max = 100, message = "Lot number cannot exceed 100 characters"))]
    pub lot_number: Option<String>,
    #[validate(length(min = 1, max = 100, message = "Batch number must be between 1 and 100 characters"))]
    pub batch_number: Option<String>,
    #[validate(length(max = 100, message = "Cat number cannot exceed 100 characters"))]
    pub cat_number: Option<String>,
    #[validate(range(min = 0.0, message = "Quantity must be non-negative"))]
    pub quantity: Option<f64>,
    #[validate(length(min = 1, max = 20, message = "Unit must be between 1 and 20 characters"))]
    pub unit: Option<String>,
    pub expiry_date: Option<DateTime<Utc>>,
    #[validate(length(max = 255, message = "Supplier name cannot exceed 255 characters"))]
    pub supplier: Option<String>,
    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,
    #[validate(length(max = 255, message = "Location cannot exceed 255 characters"))]
    pub location: Option<String>,
    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
    pub received_date: Option<DateTime<Utc>>,
    pub status: Option<String>,
}