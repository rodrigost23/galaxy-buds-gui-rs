use galaxy_buds_rs::message::{
    extended_status_updated::ExtendedStatusUpdate, status_updated::StatusUpdate,
};

#[derive(Debug)]
pub enum BudsStatus {
    StatusUpdate(StatusUpdate),
    ExtendedStatusUpdate(ExtendedStatusUpdate),
    None,
}

impl BudsStatus {
    pub fn battery_text(&self) -> String {
        let (battery_left, battery_right) = match self {
            BudsStatus::StatusUpdate(s) => (
                Some(s.battery_left.to_string()),
                Some(s.battery_right.to_string()),
            ),
            BudsStatus::ExtendedStatusUpdate(s) => (
                Some(s.battery_left.to_string()),
                Some(s.battery_right.to_string()),
            ),
            _ => (None, None),
        };

        match (battery_left, battery_right) {
            (Some(left), Some(right)) => {
                if left == right {
                    format!("L / R {}%", left)
                } else {
                    format!("L {}% / R {}%", left, right)
                }
            }
            _ => "N/A".to_string(),
        }
    }

    pub fn case_battery_text(&self) -> String {
        match self {
            BudsStatus::StatusUpdate(s) => format!("{}%", s.battery_case),
            BudsStatus::ExtendedStatusUpdate(s) => format!("{}%", s.battery_case),
            BudsStatus::None => "N/A".to_string(),
        }
    }
}
