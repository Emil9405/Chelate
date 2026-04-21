// src/query_builders/filters/enums.rs
//! Все статусы и типы через единый макрос define_status_enum!

use serde::{Serialize, Deserialize};

/// Макрос для генерации status/type enum с as_str, from_str, is_valid, Display
macro_rules! define_status_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $variant:ident => $str_val:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        $vis enum $name {
            $( $variant ),+
        }

        impl $name {
            #[inline]
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( $name::$variant => $str_val ),+
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                match s.to_lowercase().as_str() {
                    $( $str_val => Some($name::$variant), )+
                    _ => None,
                }
            }

            #[inline]
            pub fn is_valid(s: &str) -> bool {
                Self::from_str(s).is_some()
            }

            /// Все допустимые значения
            pub const fn all_values() -> &'static [&'static str] {
                &[ $( $str_val ),+ ]
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::from_str(s).ok_or_else(|| format!("Invalid {}: '{}'", stringify!($name), s))
            }
        }
    };
}

// ==================== СТАТУСЫ КОМНАТ ====================

define_status_enum! {
    /// Статус комнаты/помещения
    /// 
    /// Жизненный цикл:
    /// - Available: свободна, можно бронировать
    /// - Reserved: забронирована на будущее
    /// - Occupied: идёт эксперимент
    /// - Maintenance: плановое обслуживание
    /// - Unavailable: недоступна (закрыта, санитарный день и т.д.)
    pub enum RoomStatus {
        Available => "available",
        Reserved => "reserved",
        Occupied => "occupied",
        Maintenance => "maintenance",
        Unavailable => "unavailable",
    }
}

impl Default for RoomStatus {
    fn default() -> Self {
        RoomStatus::Available
    }
}

// ==================== СТАТУСЫ ПАРТИЙ ====================

define_status_enum! {
    /// Статус партии реагента
    pub enum BatchStatus {
        Available => "available",
        LowStock => "low_stock",
        Expired => "expired",
        Reserved => "reserved",
        Depleted => "depleted",
    }
}

// ==================== ВИДИМОСТЬ РЕАГЕНТА ====================

define_status_enum! {
    /// Видимость реагента в списках.
    ///
    /// - Public (default): виден в главном списке реагентов и в поиске
    /// - Hidden: скрыт из дефолтного списка, виден только при toggle "Show hidden".
    ///   Батчи скрытого реагента остаются видимыми в комнатах и инвентаре.
    pub enum ReagentVisibility {
        Public => "public",
        Hidden => "hidden",
    }
}

impl Default for ReagentVisibility {
    fn default() -> Self {
        ReagentVisibility::Public
    }
}

// ==================== СТАТУСЫ ЭКСПЕРИМЕНТОВ ====================

define_status_enum! {
    /// Статус эксперимента
    pub enum ExperimentStatus {
        Draft => "draft",
        Scheduled => "scheduled",
        InProgress => "in_progress",
        Completed => "completed",
        Cancelled => "cancelled",
    }
}

// ==================== ТИПЫ ОБОРУДОВАНИЯ ====================

define_status_enum! {
    /// Тип оборудования
    /// 
    /// - Equipment: общее оборудование
    /// - Labware: лабораторная посуда (legacy)
    /// - Instrument: измерительные приборы
    /// - Glassware: стеклянная посуда
    /// - Safety: защитное оборудование
    /// - Storage: хранилища, холодильники
    /// - Consumable: расходные материалы
    /// - Other: прочее
    pub enum EquipmentType {
        Equipment => "equipment",
        Labware => "labware",
        Instrument => "instrument",
        Glassware => "glassware",
        Safety => "safety",
        Storage => "storage",
        Consumable => "consumable",
        Other => "other",
    }
}

impl Default for EquipmentType {
    fn default() -> Self {
        EquipmentType::Instrument
    }
}

// ==================== СТАТУСЫ ОБОРУДОВАНИЯ ====================

define_status_enum! {
    /// Статус оборудования
    /// 
    /// - Available: доступно для использования
    /// - InUse: используется в эксперименте
    /// - Maintenance: на обслуживании
    /// - Damaged: повреждено
    /// - Calibration: на калибровке
    /// - Retired: выведено из эксплуатации
    pub enum EquipmentStatus {
        Available => "available",
        InUse => "in_use",
        Maintenance => "maintenance",
        Damaged => "damaged",
        Calibration => "calibration",
        Retired => "retired",
    }
}

impl Default for EquipmentStatus {
    fn default() -> Self {
        EquipmentStatus::Available
    }
}

// ==================== ТИПЫ ФАЙЛОВ ОБОРУДОВАНИЯ ====================

define_status_enum! {
    /// Тип файла оборудования
    pub enum EquipmentFileType {
        Manual => "manual",
        Image => "image",
        Certificate => "certificate",
        Specification => "specification",
        MaintenanceLog => "maintenance_log",
        Other => "other",
    }
}

impl EquipmentFileType {
    /// Разрешённые MIME-типы для данного типа файла
    pub fn allowed_mime_types(&self) -> &'static [&'static str] {
        match self {
            EquipmentFileType::Manual => &[
                "application/pdf",
                "application/msword",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "text/plain",
            ],
            EquipmentFileType::Image => &[
                "image/jpeg",
                "image/png",
                "image/gif",
                "image/webp",
                "image/svg+xml",
            ],
            EquipmentFileType::Certificate => &[
                "application/pdf",
                "image/jpeg",
                "image/png",
            ],
            EquipmentFileType::Specification => &[
                "application/pdf",
                "application/msword",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "application/vnd.ms-excel",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ],
            EquipmentFileType::MaintenanceLog => &[
                "application/pdf",
                "text/plain",
                "text/csv",
            ],
            EquipmentFileType::Other => &[
                "application/pdf",
                "application/octet-stream",
            ],
        }
    }

    #[inline]
    pub fn is_mime_type_allowed(&self, mime_type: &str) -> bool {
        self.allowed_mime_types().contains(&mime_type)
    }
}

// ==================== ТИПЫ ОБСЛУЖИВАНИЯ ====================

define_status_enum! {
    /// Тип обслуживания оборудования
    pub enum MaintenanceType {
        Scheduled => "scheduled",
        Unscheduled => "unscheduled",
        Calibration => "calibration",
        Repair => "repair",
        Inspection => "inspection",
        Cleaning => "cleaning",
        PartReplacement => "part_replacement",
    }
}

// ==================== СТАТУСЫ ОБСЛУЖИВАНИЯ ====================

define_status_enum! {
    /// Статус обслуживания
    pub enum MaintenanceStatus {
        Scheduled => "scheduled",
        InProgress => "in_progress",
        Completed => "completed",
        Cancelled => "cancelled",
        Overdue => "overdue",
    }
}

// ==================== СТАТУСЫ ЗАПЧАСТЕЙ ====================

define_status_enum! {
    /// Статус запчасти
    pub enum PartStatus {
        Good => "good",
        NeedsAttention => "needs_attention",
        NeedsReplacement => "needs_replacement",
        Replaced => "replaced",
        Missing => "missing",
    }
}

impl Default for PartStatus {
    fn default() -> Self {
        PartStatus::Good
    }
}

// ==================== ТЕСТЫ ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_status() {
        assert_eq!(RoomStatus::Available.as_str(), "available");
        assert_eq!(RoomStatus::from_str("available"), Some(RoomStatus::Available));
        assert_eq!(RoomStatus::from_str("AVAILABLE"), Some(RoomStatus::Available));
        assert!(RoomStatus::is_valid("maintenance"));
        assert!(!RoomStatus::is_valid("invalid"));
    }

    #[test]
    fn test_batch_status() {
        assert_eq!(BatchStatus::LowStock.as_str(), "low_stock");
        assert_eq!(BatchStatus::from_str("low_stock"), Some(BatchStatus::LowStock));
    }

    #[test]
    fn test_equipment_type() {
        assert_eq!(EquipmentType::Instrument.as_str(), "instrument");
        assert_eq!(EquipmentType::from_str("instrument"), Some(EquipmentType::Instrument));
        assert_eq!(EquipmentType::from_str("glassware"), Some(EquipmentType::Glassware));
        assert_eq!(EquipmentType::from_str("safety"), Some(EquipmentType::Safety));
        assert_eq!(EquipmentType::from_str("storage"), Some(EquipmentType::Storage));
        assert_eq!(EquipmentType::from_str("other"), Some(EquipmentType::Other));
        assert!(EquipmentType::is_valid("consumable"));
        assert!(!EquipmentType::is_valid("unknown_type"));
    }

    #[test]
    fn test_equipment_status() {
        assert_eq!(EquipmentStatus::InUse.as_str(), "in_use");
        assert_eq!(EquipmentStatus::from_str("in_use"), Some(EquipmentStatus::InUse));
        assert_eq!(EquipmentStatus::from_str("calibration"), Some(EquipmentStatus::Calibration));
        assert!(EquipmentStatus::is_valid("calibration"));
        assert!(EquipmentStatus::is_valid("retired"));
    }

    #[test]
    fn test_equipment_file_type_mime() {
        let image = EquipmentFileType::Image;
        assert!(image.is_mime_type_allowed("image/jpeg"));
        assert!(!image.is_mime_type_allowed("application/pdf"));

        let manual = EquipmentFileType::Manual;
        assert!(manual.is_mime_type_allowed("application/pdf"));
        assert!(!manual.is_mime_type_allowed("image/jpeg"));
    }

    #[test]
    fn test_all_values() {
        let room_values = RoomStatus::all_values();
        assert!(room_values.contains(&"available"));
        assert!(room_values.contains(&"maintenance"));

        let equipment_types = EquipmentType::all_values();
        assert!(equipment_types.contains(&"instrument"));
        assert!(equipment_types.contains(&"glassware"));
        assert!(equipment_types.contains(&"safety"));
        assert!(equipment_types.contains(&"storage"));
        assert!(equipment_types.contains(&"other"));
        assert_eq!(equipment_types.len(), 8);
    }

    #[test]
    fn test_from_str_trait() {
        let status: Result<RoomStatus, _> = "available".parse();
        assert!(status.is_ok());
        assert_eq!(status.unwrap(), RoomStatus::Available);

        let invalid: Result<RoomStatus, _> = "invalid".parse();
        assert!(invalid.is_err());

        let eq_type: Result<EquipmentType, _> = "glassware".parse();
        assert!(eq_type.is_ok());
        assert_eq!(eq_type.unwrap(), EquipmentType::Glassware);
    }

    #[test]
    fn test_defaults() {
        assert_eq!(EquipmentType::default(), EquipmentType::Instrument);
        assert_eq!(EquipmentStatus::default(), EquipmentStatus::Available);
        assert_eq!(PartStatus::default(), PartStatus::Good);
        assert_eq!(RoomStatus::default(), RoomStatus::Available);
    }
}