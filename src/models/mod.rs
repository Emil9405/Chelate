// src/models/mod.rs

// src/models/mod.rs

// 1. Объявляем модули
pub mod batch;
pub mod equipment;
pub mod experiment;
pub mod reagent;
pub mod room;
pub mod user;

// 2. Ре-экспортируем содержимое (Re-export), чтобы структуры были доступны как crate::models::StructName
pub use batch::*;
pub use equipment::*;
pub use experiment::*;
pub use reagent::*;
pub use room::*;
pub use user::*;

use serde::{Deserialize, Serialize};

// ==================== COMMON / SHARED ====================

/// Параметры поискового запроса
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<i32>,
}

/// Общая статистика для дашборда
#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_reagents: i64,
    pub total_batches: i64,
    pub total_equipment: i64,
    pub total_experiments: i64,
    pub active_experiments: i64,
    pub low_stock_batches: i64,
    pub expiring_soon_batches: i64,
    pub educational_experiments: i64,
    pub research_experiments: i64,
}