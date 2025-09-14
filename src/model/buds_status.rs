use galaxy_buds_rs::message::{
    bud_property::NoiseControlMode, extended_status_updated::ExtendedStatusUpdate, noise_controls_updated::NoiseControlsUpdated, status_updated::StatusUpdate
};

pub trait UpdateFrom<T> {
    fn update(&mut self, source: T);
}

#[derive(Debug)]
pub struct BudsStatus {
    battery_left: i8,
    battery_right: i8,
    battery_case: i8,
    noise_control_mode: NoiseControlMode,
}

impl BudsStatus {
    pub fn battery_text(&self) -> String {
        if self.battery_left == self.battery_right {
            format!("L / R {}%", self.battery_left)
        } else {
            format!("L {}% / R {}%", self.battery_left, self.battery_right)
        }
    }

    pub fn case_battery_text(&self) -> String {
        format!("{}%", self.battery_case)
    }

    pub fn noise_control_mode(&self) -> NoiseControlMode {
        self.noise_control_mode
    }

    pub fn noise_control_mode_text(&self) -> String {
        match self.noise_control_mode() {
            NoiseControlMode::NoiseReduction => "Noise Reduction".to_string(),
            NoiseControlMode::AmbientSound => "Ambient Sound".to_string(),
            NoiseControlMode::Off => "Off".to_string(),
        }
    }
}
impl UpdateFrom<&StatusUpdate> for BudsStatus {
    fn update(&mut self, status: &StatusUpdate) {
        self.battery_left = status.battery_left;
        self.battery_right = status.battery_right;
        self.battery_case = status.battery_case;
    }
}

impl UpdateFrom<&ExtendedStatusUpdate> for BudsStatus {
    fn update(&mut self, status: &ExtendedStatusUpdate) {
        self.battery_left = status.battery_left;
        self.battery_right = status.battery_right;
        self.battery_case = status.battery_case;
        self.noise_control_mode = noise_control_from_status_update(status);
    }
}

impl UpdateFrom<&NoiseControlsUpdated> for BudsStatus {
    fn update(&mut self, update: &NoiseControlsUpdated) {
        self.noise_control_mode = update.noise_control_mode;
    }
}

impl From<&ExtendedStatusUpdate> for BudsStatus {
    fn from(status: &ExtendedStatusUpdate) -> Self {
        Self {
            battery_left: status.battery_left,
            battery_right: status.battery_right,
            battery_case: status.battery_case,
            noise_control_mode: noise_control_from_status_update(status),
        }
    }
}

fn noise_control_from_status_update(status: &ExtendedStatusUpdate) -> NoiseControlMode {
    if status.noise_reduction {
        NoiseControlMode::NoiseReduction
    } else if status.ambient_sound_enabled {
        NoiseControlMode::AmbientSound
    } else {
        NoiseControlMode::Off
    }
}
