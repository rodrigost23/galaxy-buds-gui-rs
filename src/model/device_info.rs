use bluer::DeviceProperty;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
}

impl DeviceInfo {
    pub fn from_properties(props: Vec<DeviceProperty>) -> Self {
        let name = props
            .iter()
            .find_map(|prop| match prop {
                DeviceProperty::Name(n) => Some(n.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "Unknown Device".into());

        DeviceInfo { name }
    }
}
