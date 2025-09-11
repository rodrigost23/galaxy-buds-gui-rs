use bluer::Device;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub address: String,
    pub device: Device,
}

impl DeviceInfo {
    pub async fn from_device(device: Device) -> Self {
        let name = match device.name().await {
            Ok(Some(n)) => n,
            _ => "Unknown".to_string(),
        };

        let address = device.address().to_string();

        DeviceInfo {
            name,
            address,
            device,
        }
    }
}
