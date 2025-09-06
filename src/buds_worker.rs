use bluer::{
    Device, Session, Uuid,
    rfcomm::{Profile, Role, Stream},
};
use futures::{StreamExt, pin_mut};

use relm4::{Worker, prelude::*};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    runtime::Runtime,
    sync::Mutex,
    time::sleep,
};

use crate::buds_message::BudsMessage;

// --- Worker I/O ---

#[derive(Debug)]
pub enum BluetoothWorkerInput {
    /// Starts the discovery and connection process.
    Connect,
    /// Disconnects from the current device.
    Disconnect,
    /// Sends a raw byte payload to the device.
    SendData(Vec<u8>),
}

#[derive(Debug)]
pub enum BluetoothWorkerOutput {
    Connected,
    Disconnected,
    DataReceived(BudsMessage),
    Error(String),
}

// --- Worker Implementation ---

#[derive(Clone)]
struct WorkerState {
    // The RFCOMM stream is wrapped in several layers for safe concurrent access:
    // - `Option`: The stream only exists when we are connected.
    // - `Mutex`: An async-aware lock that ensures only one task can access the
    //   `Option<Stream>` at a time. This prevents data races.
    // - `Arc`: An "Atomically Reference Counted" smart pointer. It allows multiple
    //   owners of the same data (the Mutex), making it possible to share the
    //   stream between the reader task and the `update` function.
    stream: Arc<Mutex<Option<Stream>>>,
}

pub struct BluetoothWorker {
    state: WorkerState,
    runtime: Arc<Runtime>,
}

impl Worker for BluetoothWorker {
    type Init = ();
    type Input = BluetoothWorkerInput;
    type Output = BluetoothWorkerOutput;

    fn init(_init: Self::Init, sender: ComponentSender<Self>) -> Self {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime"),
        );

        let state = WorkerState {
            stream: Arc::new(Mutex::new(None)),
        };

        let reader_state = state.clone();
        let rt_handle = runtime.handle().clone();

        // Spawn a persistent task to continuously read from the bluetooth stream.
        rt_handle.spawn(async move {
            loop {
                let mut buffer = [0u8; 2048];
                let mut stream_guard = reader_state.stream.lock().await;

                if let Some(stream) = stream_guard.as_mut() {
                    match stream.read(&mut buffer).await {
                        Ok(0) => {
                            sender
                                .output(BluetoothWorkerOutput::Error(
                                    "Stream closed by peer".to_string(),
                                ))
                                .unwrap();
                            *stream_guard = None; // Mark as disconnected.
                            sender.output(BluetoothWorkerOutput::Disconnected).unwrap();
                        }
                        Ok(n) => {
                            let buff = &buffer[..n];

                            match BudsMessage::from_bytes(buff) {
                                Some(msg) => {
                                    sender
                                        .output(BluetoothWorkerOutput::DataReceived(msg))
                                        .unwrap();
                                }
                                None => continue,
                            };
                        }
                        Err(e) => {
                            sender
                                .output(BluetoothWorkerOutput::Error(format!("Read error: {}", e)))
                                .unwrap();
                            *stream_guard = None; // Mark as disconnected.
                            sender.output(BluetoothWorkerOutput::Disconnected).unwrap();
                        }
                    }
                } else {
                    // Drop the lock before sleeping to allow other tasks to acquire it.
                    drop(stream_guard);
                    sleep(Duration::from_millis(50)).await;
                }
            }
        });

        Self { state, runtime }
    }

    /// Handles discrete events from the UI. Each message is processed in a short-lived async task.
    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        let state = self.state.clone();
        self.runtime.block_on(async {
            match msg {
                BluetoothWorkerInput::Connect => match connect_and_get_stream().await {
                    Ok(stream) => {
                        let mut stream_guard = state.stream.lock().await;
                        *stream_guard = Some(stream);
                        sender.output(BluetoothWorkerOutput::Connected).unwrap();
                    }
                    Err(e) => {
                        sender
                            .output(BluetoothWorkerOutput::Error(e.to_string()))
                            .unwrap();
                    }
                },
                BluetoothWorkerInput::Disconnect => {
                    let mut stream_guard = state.stream.lock().await;
                    *stream_guard = None; // Dropping the stream closes the connection.
                    sender.output(BluetoothWorkerOutput::Disconnected).unwrap();
                }
                BluetoothWorkerInput::SendData(data) => {
                    let mut stream_guard = state.stream.lock().await;
                    if let Some(stream) = stream_guard.as_mut() {
                        if let Err(e) = stream.write_all(&data).await {
                            sender
                                .output(BluetoothWorkerOutput::Error(e.to_string()))
                                .unwrap();
                        }
                    } else {
                        sender
                            .output(BluetoothWorkerOutput::Error("Not connected".to_string()))
                            .unwrap();
                    }
                }
            }
        });
    }
}

// --- Async Helper Functions ---

/// Performs the full bluetooth connection and profile registration dance.
async fn connect_and_get_stream() -> Result<Stream, Box<dyn std::error::Error + Send + Sync>> {
    let session = Session::new().await?;
    let device = discover_galaxy_buds(&session).await?;

    println!("Connecting to device...");
    device.connect().await?;
    println!("Connected.");

    let spp_uuid = bluer::id::ServiceClass::SerialPort.into();
    let profile = Profile {
        uuid: spp_uuid,
        role: Some(Role::Client),
        require_authentication: Some(false),
        require_authorization: Some(false),
        auto_connect: Some(true),
        ..Default::default()
    };
    let mut handle = session.register_profile(profile).await?;
    println!("SPP Profile registered. Waiting for connection...");

    if let Some(req) = handle.next().await {
        println!("Connection request from {:?} accepted.", req.device());
        let stream = req.accept()?;
        println!("RFCOMM stream established.");
        Ok(stream)
    } else {
        Err("No connection request received".into())
    }
}

/// Scans for and returns the first bluetooth device matching the Galaxy Buds SPP UUID.
async fn discover_galaxy_buds(
    session: &Session,
) -> Result<Device, Box<dyn std::error::Error + Send + Sync>> {
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let addrs = adapter.device_addresses().await?;
    let devices = addrs.iter().filter_map(|addr| adapter.device(*addr).ok());
    pin_mut!(devices);

    let custom_spp_uuid: Uuid = "2e73a4ad-332d-41fc-90e2-16bef06523f2".parse()?;

    while let Some(device) = devices.next() {
        if let Ok(Some(uuids)) = device.uuids().await {
            if uuids.contains(&custom_spp_uuid) {
                println!("Found Galaxy Buds: {:?}", device.name().await?);
                return Ok(device);
            }
        }
    }
    Err("No Galaxy Buds device found".into())
}
