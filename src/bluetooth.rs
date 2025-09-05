use bluer::{
    Device, Session, Uuid,
    rfcomm::{Profile, Role},
};
use futures::{StreamExt, pin_mut};
use galaxy_buds_rs::{
    message::{self, Message, extended_status_updated::ExtendedStatusUpdate, ids},
    model::Model,
};
use std::sync::mpsc;
use tokio::io::AsyncReadExt;

/// Discovers, verifies, and returns the first available Galaxy Buds device.
pub async fn discover_galaxy_buds(session: &Session) -> Result<Device, Box<dyn std::error::Error>> {
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    println!("Discovering devices...");

    // Get a stream of all existing devices
    let addrs = adapter.device_addresses().await?;
    let devices = addrs.iter().filter_map(|addr| adapter.device(*addr).ok());

    pin_mut!(devices);

    let custom_spp_uuid: Uuid = "2e73a4ad-332d-41fc-90e2-16bef06523f2".parse()?;

    while let Some(device) = devices.next() {
        if let Ok(Some(uuids)) = device.uuids().await {
            if uuids.contains(&custom_spp_uuid) {
                println!("Found Galaxy Buds device: {:?}", device.name().await);
                return Ok(device);
            }
        } else {
            return Err("No UUIDs found for device".into());
        }
    }

    Err("No Galaxy Buds device found".into())
}

pub async fn bluetooth_loop(tx: mpsc::Sender<String>) -> Result<(), Box<dyn std::error::Error>> {
    let session = Session::new().await?;
    let device = discover_galaxy_buds(&session).await?;

    println!("Connecting...");
    device.connect().await?;
    println!("Connected to device: {:?}", device.all_properties().await?);

    let uuids = device.uuids().await?.unwrap_or_default();
    let spp_uuid = bluer::id::ServiceClass::SerialPort.into();
    if !uuids.contains(&spp_uuid) {
        return Err("Device does not support Serial Port Profile (SPP)".into());
    }
    println!("Device supports Serial Port Profile (SPP).");

    println!("Registering SPP profile with UUID: {}", spp_uuid);
    let profile = Profile {
        uuid: spp_uuid,
        role: Some(Role::Client),
        require_authentication: Some(false),
        require_authorization: Some(false),
        auto_connect: Some(true),
        ..Default::default()
    };
    let mut handle = session.register_profile(profile).await?;

    println!("Profile registered. Ready to connect.");

    if let Some(req) = handle.next().await {
        println!("Connection request from {:?} accepted.", req.device());
        let mut stream = req.accept()?;
        println!("RFCOMM stream established. Type messages to send.");

        let mut buffer = [0u8; 2048];

        loop {
            let num_bytes_read = stream.read(&mut buffer).await?;
            let buff = &buffer[..num_bytes_read];

            let id = buff[3].to_be();
            let message = Message::new(buff, Model::BudsLive);

            if id == 242 {
                continue;
            }

            if id == ids::STATUS_UPDATED {
                let msg: message::status_updated::StatusUpdate = message.into();
                tx.send(format!("{:?}", msg))?;
                continue;
            }

            if id == ids::EXTENDED_STATUS_UPDATED {
                let msg: ExtendedStatusUpdate = message.into();
                tx.send(format!("{:?}", msg))?;
                continue;
            }
        }
    } else {
        Err("No connection request received".into())
    }
}
