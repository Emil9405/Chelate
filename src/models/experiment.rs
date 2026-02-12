// src/models/experiment.rs
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

// === ENUMS ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ExperimentType {
    Educational,
    Research,
}

impl Default for ExperimentType {
    fn default() -> Self {
        ExperimentType::Research
    }
}

impl ExperimentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExperimentType::Educational => "educational",
            ExperimentType::Research => "research",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "educational" | "учебный" => Some(ExperimentType::Educational),
            "research" | "исследовательский" => Some(ExperimentType::Research),
            _ => None,
        }
    }

    pub fn requires_time_bounds(&self) -> bool {
        matches!(self, ExperimentType::Educational)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ExperimentType::Educational => "Educational",
            ExperimentType::Research => "Research",
        }
    }

    pub fn display_name_ru(&self) -> &'static str {
        match self {
            ExperimentType::Educational => "Учебный",
            ExperimentType::Research => "Исследовательский",
        }
    }
}

impl std::fmt::Display for ExperimentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// === EXPERIMENT ===

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Experiment {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub experiment_date: DateTime<Utc>,
    #[sqlx(default)]
    pub experiment_type: Option<String>,
    pub instructor: Option<String>,
    pub student_group: Option<String>,
    pub location: Option<String>,
    pub status: String, 
    pub protocol: Option<String>,  
    pub start_date: DateTime<Utc>, 
    pub end_date: Option<DateTime<Utc>>, 
    pub results: Option<String>, 
    pub notes: Option<String>, 
    pub created_by: String,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Experiment {
    pub fn get_experiment_type(&self) -> ExperimentType {
        self.experiment_type
            .as_ref()
            .and_then(|t| ExperimentType::from_str(t))
            .unwrap_or_default()
    }

    pub fn is_educational(&self) -> bool {
        self.get_experiment_type() == ExperimentType::Educational
    }

    pub fn validate_time_bounds(&self) -> Result<(), String> {
        if self.is_educational() {
            if self.end_date.is_none() {
                return Err("Educational experiments require end_date".to_string());
            }
            let end = self.end_date.unwrap();
            if end <= self.start_date {
                return Err("End time must be after start time".to_string());
            }
            let duration = end - self.start_date;
            if duration.num_minutes() < 15 {
                return Err("Educational experiment must be at least 15 minutes".to_string());
            }
            if duration.num_hours() > 8 {
                return Err("Educational experiment cannot exceed 8 hours".to_string());
            }
        }
        Ok(())
    }
}

// === RELATED STRUCTURES ===

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExperimentDocument {
    pub id: String,
    pub experiment_id: String,
    pub filename: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub uploaded_by: String,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExperimentReagent {
    pub id: String,
    pub experiment_id: String,
    pub batch_id: String,
    pub quantity_used: f64,
    pub is_consumed: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExperimentEquipment {
    pub id: String,
    pub experiment_id: String,
    pub equipment_id: String,
    pub quantity_used: i32,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExperimentReagentDetail {
    pub id: String,
    pub batch_id: String,
    pub reagent_id: String,
    pub reagent_name: String,
    pub batch_number: String,
    pub quantity_used: f64,
    pub is_consumed: bool,
    pub unit: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExperimentEquipmentDetail {
    pub id: String,
    pub equipment_id: String,
    pub equipment_name: String,
    pub quantity_used: i32,
    pub unit: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExperimentWithDetails {
    #[serde(flatten)]
    pub experiment: Experiment,
    pub documents: Vec<ExperimentDocument>,
    pub reagents: Vec<ExperimentReagentDetail>,
    pub equipment: Vec<ExperimentEquipmentDetail>,
}

#[derive(Debug, Serialize)]
pub struct ExperimentStats {
    pub total: i64,
    pub planned: i64,
    pub in_progress: i64,
    pub completed: i64,
    pub cancelled: i64,
    pub educational: i64,
    pub research: i64,
}

// === REQUESTS ===

#[derive(Debug, Deserialize, Validate)]
pub struct CreateExperimentRequest {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: String,
    #[validate(length(max = 2000, message = "Description cannot exceed 2000 characters"))]
    pub description: Option<String>,
    pub experiment_date: Option<DateTime<Utc>>,
    #[validate(custom(function = "validate_experiment_type"))]
    pub experiment_type: Option<String>,
    #[validate(length(max = 255, message = "Instructor name cannot exceed 255 characters"))]
    pub instructor: Option<String>,
    #[validate(length(max = 100, message = "Student group cannot exceed 100 characters"))]
    pub student_group: Option<String>,
    #[validate(length(max = 255, message = "Location cannot exceed 255 characters"))]
    pub location: Option<String>,
    #[validate(length(max = 2000, message = "Protocol cannot exceed 2000 characters"))]
    pub protocol: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
}

impl CreateExperimentRequest {
    pub fn validate_educational(&self) -> Result<(), String> {
        let exp_type = self.experiment_type
            .as_ref()
            .and_then(|t| ExperimentType::from_str(t))
            .unwrap_or_default();

        if exp_type == ExperimentType::Educational {
            let start = self.start_date
                .ok_or("Educational experiments require start_date")?;
            let end = self.end_date
                .ok_or("Educational experiments require end_date")?;
            if end <= start {
                return Err("End time must be after start time".to_string());
            }
            let duration = end - start;
            if duration.num_minutes() < 15 {
                return Err("Educational experiment must be at least 15 minutes".to_string());
            }
            if duration.num_hours() > 8 {
                return Err("Educational experiment cannot exceed 8 hours".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateExperimentRequest {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: Option<String>,
    #[validate(length(max = 2000, message = "Description cannot exceed 2000 characters"))]
    pub description: Option<String>,
    pub experiment_date: Option<DateTime<Utc>>,
    #[validate(custom(function = "validate_experiment_type_option"))]
    pub experiment_type: Option<String>,
    #[validate(length(max = 255, message = "Instructor name cannot exceed 255 characters"))]
    pub instructor: Option<String>,
    #[validate(length(max = 100, message = "Student group cannot exceed 100 characters"))]
    pub student_group: Option<String>,
    #[validate(length(max = 255, message = "Location cannot exceed 255 characters"))]
    pub location: Option<String>,
    pub status: Option<String>,
    #[validate(length(max = 2000, message = "Protocol cannot exceed 2000 characters"))]
    pub protocol: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[validate(length(max = 5000, message = "Results cannot exceed 5000 characters"))]
    pub results: Option<String>,
    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AddExperimentReagentRequest {
    pub batch_id: String,
    #[validate(range(min = 0.0, message = "Quantity must be non-negative"))]
    pub quantity_used: f64,
    pub is_consumed: Option<bool>,
    #[validate(length(max = 500, message = "Notes cannot exceed 500 characters"))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AddExperimentEquipmentRequest {
    pub equipment_id: String,
    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    pub quantity_used: i32,
    #[validate(length(max = 500, message = "Notes cannot exceed 500 characters"))]
    pub notes: Option<String>,
}

// === VALIDATORS ===

fn validate_experiment_type(value: &str) -> Result<(), validator::ValidationError> {
    if ExperimentType::from_str(value).is_some() {
        Ok(())
    } else {
        let mut error = validator::ValidationError::new("invalid_experiment_type");
        error.message = Some("Experiment type must be 'educational' or 'research'".into());
        Err(error)
    }
}

fn validate_experiment_type_option(value: &str) -> Result<(), validator::ValidationError> {
    if value.is_empty() || ExperimentType::from_str(value).is_some() {
        Ok(())
    } else {
        let mut error = validator::ValidationError::new("invalid_experiment_type");
        error.message = Some("Experiment type must be 'educational' or 'research'".into());
        Err(error)
    }
}

// === TESTS ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experiment_type_from_str() {
        assert_eq!(ExperimentType::from_str("educational"), Some(ExperimentType::Educational));
        assert_eq!(ExperimentType::from_str("EDUCATIONAL"), Some(ExperimentType::Educational));
        assert_eq!(ExperimentType::from_str("research"), Some(ExperimentType::Research));
        assert_eq!(ExperimentType::from_str("учебный"), Some(ExperimentType::Educational));
        assert_eq!(ExperimentType::from_str("invalid"), None);
    }
    #[test]
    fn test_experiment_type_requires_time_bounds() {
        assert!(ExperimentType::Educational.requires_time_bounds());
        assert!(!ExperimentType::Research.requires_time_bounds());
    }

    #[test]
    fn test_experiment_type_display() {
        assert_eq!(ExperimentType::Educational.as_str(), "educational");
        assert_eq!(ExperimentType::Research.as_str(), "research");
        assert_eq!(ExperimentType::Educational.display_name(), "Educational");
        assert_eq!(ExperimentType::Educational.display_name_ru(), "Учебный");
    }

    #[test]
    fn test_create_experiment_request_validation() {
        let request = CreateExperimentRequest {
            title: "Test Experiment".to_string(),
            description: None,
            experiment_date: Some(Utc::now()),
            experiment_type: Some("educational".to_string()),
            instructor: Some("Dr. Smith".to_string()),
            student_group: Some("Group 101".to_string()),
            location: Some("Lab 101".to_string()),
            protocol: None,
            start_date: Some(Utc::now()),
            end_date: Some(Utc::now() + chrono::Duration::hours(2)),
            notes: None,
        };

        assert!(request.validate_educational().is_ok());
    }

    #[test]
    fn test_educational_experiment_without_end_time() {
        let request = CreateExperimentRequest {
            title: "Test".to_string(),
            description: None,
            experiment_date: Some(Utc::now()),
            experiment_type: Some("educational".to_string()),
            instructor: None,
            student_group: None,
            location: None,
            protocol: None,
            start_date: Some(Utc::now()),
            end_date: None, // Missing!
            notes: None,
        };

        assert!(request.validate_educational().is_err());
    }

}