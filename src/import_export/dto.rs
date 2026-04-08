// src/import_export/dto.rs
//! Data Transfer Objects for import/export operations

use serde::{Deserialize, Serialize};
use super::deserialize_flexible_date;

// ==========================================
// REAGENT IMPORT DTO
// ==========================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ReagentImportDto {
    #[serde(alias = "Name", alias = "reagent_name", alias = "Название")]
    pub name: String,
    
    #[serde(alias = "Formula", alias = "chemical_formula", alias = "Формула")]
    pub formula: Option<String>,
    
    #[serde(alias = "CAS", alias = "cas", alias = "cas_number", alias = "CAS Number")]
    pub cas_number: Option<String>,
    
    #[serde(alias = "Molecular weight", alias = "MW", alias = "Molecular Weight", alias = "Mol. Weight")]
    pub molecular_weight: Option<f64>,
    
    #[serde(alias = "Manufacturer", alias = "manufacturer", alias = "Производитель")]
    pub manufacturer: Option<String>,
    
    #[serde(alias = "Description", alias = "description", alias = "Описание")]
    pub description: Option<String>,
    
    #[serde(alias = "Catalog Number", alias = "cat_number", alias = "Catalogue No", alias = "Catalog #")]
    pub catalog_number: Option<String>,

    #[serde(alias = "Storage_cond", alias = "Storage", alias = "Storage conditions", alias = "Safety")]
    pub storage: Option<String>, 
    
    #[serde(alias = "Appearance", alias = "Color")]
    pub appearance: Option<String>,

    #[serde(alias = "Added by", alias = "User", alias = "Owner", alias = "Владелец")]
    pub owner: Option<String>,

    #[serde(alias = "Added at", alias = "Date added", alias = "created_at")]
    pub added_at: Option<String>,

    // Batch fields
    #[serde(alias = "Lot number", alias = "Lot Number", alias = "batch_number", alias = "Партия")]
    pub batch_number: Option<String>,
    
    #[serde(alias = "Pack_size", alias = "Pack size", alias = "Pack Size", alias = "PackSize", alias = "pack_size", alias = "Unit Size", alias = "UnitSize")]
    pub pack_size: Option<f64>,
    
    #[serde(alias = "Quantity", alias = "quantity", alias = "Количество")]
    pub quantity: Option<f64>,
    
    #[serde(alias = "Units", alias = "units", alias = "Unit", alias = "unit", alias = "Единицы",)]
    pub units: Option<String>,
    
    #[serde(alias = "Expiry Date", alias = "expiry_date", alias = "expiration_date", alias = "Срок годности")]
    #[serde(default, deserialize_with = "deserialize_flexible_date")] 
    pub expiry_date: Option<String>,
    
    #[serde(alias = "Place", alias = "Location", alias = "location", alias = "Место хранения")]
    pub location: Option<String>,

    #[serde(alias = "Hazard", alias = "hazard_pictograms", alias = "GHS", alias = "Pictograms", alias = "Hazard Pictograms")]
    pub hazard_pictograms: Option<String>,

    // Container fields
    #[serde(alias = "Containers", alias = "containers", alias = "Container Count", alias = "container_count", alias = "Кол-во ёмкостей")]
    pub container_count: Option<i32>,
}

// ==========================================
// BATCH IMPORT DTO
// ==========================================

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchImportDto {
    #[serde(alias = "Reagent Name", alias = "reagent_name")]
    pub reagent_name: String,
    #[serde(alias = "Batch Number", alias = "batch_number", alias = "Lot Number", alias = "Lot number")]
    pub batch_number: String,
    #[serde(alias = "Catalog Number", alias = "cat_number", alias = "Catalogue No", alias = "Catalog #")]
    pub cat_number: Option<String>,
    #[serde(alias = "Manufacturer", alias = "manufacturer", alias = "Производитель")]
    pub manufacturer: Option<String>,
    pub supplier: Option<String>,
    #[serde(alias = "quantity", alias = "Quantity", alias = "Amount")]
    pub quantity: f64, 
    #[serde(alias = "unit", alias = "Unit", alias = "units", alias = "Units", alias = "Umits")]
    pub units: String,
    #[serde(alias = "Pack_size", alias = "Pack size", alias = "Pack Size", alias = "PackSize", alias = "pack_size", alias = "Unit Size")]
    pub pack_size: Option<f64>,
    
    #[serde(default, deserialize_with = "deserialize_flexible_date")]
    pub expiration_date: Option<String>,
    
    pub location: Option<String>,
    pub notes: Option<String>,
}

// ==========================================
// EQUIPMENT IMPORT DTO
// ==========================================

#[derive(Debug, Serialize, Deserialize)]
pub struct EquipmentImportDto {
    pub name: String,
    #[serde(alias = "type")]
    pub equipment_type: String,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
}

// ==========================================
// PREPARED STRUCTS FOR BULK INSERT
// ==========================================

pub(crate) struct PreparedReagent {
    pub id: String,
    pub name: String,
    pub formula: Option<String>,
    pub cas_number: Option<String>,
    pub manufacturer: Option<String>,
    pub description: Option<String>,
    pub storage: Option<String>,
    pub appearance: Option<String>,
    pub hazard_pictograms: Option<String>,
    pub molecular_weight: Option<f64>,
    pub owner_id: String,
    pub created_at: String,
}

pub(crate) struct PreparedBatch {
    pub id: String,
    pub reagent_id: String,
    pub batch_number: String,
    pub cat_number: Option<String>,
    pub manufacturer: Option<String>,
    pub quantity: f64,
    pub unit: String,
    pub pack_size: Option<f64>,
    pub expiry_date: Option<String>,
    pub location: Option<String>,       // original string (backward compat)
    pub owner_id: String,
    pub is_new: bool,                    // false = existing batch (skip containers)
    pub container_count: i32,            // how many containers to create
    pub position_id: Option<String>,     // resolved from location path
}

pub(crate) struct PreparedContainer {
    pub id: String,
    pub batch_id: String,
    pub sequence_number: i64,
    pub quantity: f64,
    pub original_quantity: f64,
}

pub(crate) struct PreparedPlacement {
    pub id: String,
    pub container_id: String,
    pub position_id: String,
    pub placed_by: String,
}
