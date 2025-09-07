use bluer::{Device, DeviceProperty};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub device: Device,
}

impl DeviceInfo {
    pub async fn from_device(device: Device) -> Self {
        let props = device.all_properties().await.unwrap();
        let name = props
            .iter()
            .find_map(|prop| match prop {
                DeviceProperty::Name(n) => Some(n.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "Unknown Device".into());

        DeviceInfo { name, device }
    }
}
