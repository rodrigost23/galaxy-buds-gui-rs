use bluer::{
    Session,
    rfcomm::{Profile, Role, Stream},
};
use futures::StreamExt;

use relm4::{Worker, prelude::*};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    runtime::Runtime,
    sync::Mutex,
    time::{sleep, timeout},
};
use tracing::{Level, debug, debug_span, error, event, info, span};

use crate::model::{
    buds_message::{BudsCommand, BudsMessage},
    device_info::DeviceInfo,
};

// --- Worker I/O ---

#[derive(Debug)]
pub enum BudsWorkerInput {
    /// Starts the discovery and connection process.
    Connect,
    /// Disconnects from the current device.
    Disconnect,
    /// Sends a raw byte payload to the device.
    SendData(Vec<u8>),
    SendCommand(BudsCommand),
}

#[derive(Debug)]
pub enum BudsWorkerOutput {
    Connected,
    Disconnected,
    DataReceived(BudsMessage),
    Error(String),
}

// --- Worker Implementation ---

#[derive(Clone, Debug)]
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

#[derive(Debug)]
pub struct BluetoothWorker {
    device: DeviceInfo,
    state: WorkerState,
    runtime: Arc<Runtime>,
}

impl Worker for BluetoothWorker {
    type Init = DeviceInfo;
    type Input = BudsWorkerInput;
    type Output = BudsWorkerOutput;

    fn init(device: Self::Init, sender: ComponentSender<Self>) -> Self {
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
                let span = debug_span!("Stream read loop");
                let mut buffer = [0u8; 2048];
                let mut stream_guard = reader_state.stream.lock().await;

                if let Some(stream) = stream_guard.as_mut() {
                    match timeout(Duration::from_millis(100), stream.read(&mut buffer)).await {
                        Ok(r) => match r {
                            Ok(0) => {
                                error!(parent: &span, "Stream closed by peer");
                                sender
                                    .output(BudsWorkerOutput::Error(
                                        "Stream closed by peer".to_string(),
                                    ))
                                    .unwrap();
                                *stream_guard = None; // Mark as disconnected.
                                sender.output(BudsWorkerOutput::Disconnected).unwrap();
                            }
                            Ok(n) => {
                                let buff = &buffer[..n];

                                match BudsMessage::from_bytes(buff) {
                                    Some(msg) => {
                                        sender.output(BudsWorkerOutput::DataReceived(msg)).unwrap();
                                    }
                                    None => continue,
                                };
                            }
                            Err(e) => {
                                error!(parent: &span, "Read error {:?}", e);
                                sender
                                    .output(BudsWorkerOutput::Error(format!("Read error: {}", e)))
                                    .unwrap();
                                *stream_guard = None; // Mark as disconnected.
                                sender.output(BudsWorkerOutput::Disconnected).unwrap();
                            }
                        },
                        Err(_) => continue,
                    }
                } else {
                    // Drop the lock before sleeping to allow other tasks to acquire it.
                    drop(stream_guard);
                    sleep(Duration::from_millis(50)).await;
                }
            }
        });

        Self {
            device,
            state,
            runtime,
        }
    }

    /// Handles discrete events from the UI. Each message is processed in a short-lived async task.
    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        let span = debug_span!("BudsCommand", msg=?msg);
        let state = self.state.clone();

        debug!(parent: &span, "Try to block");
        self.runtime.block_on(async {
            debug!(parent: &span, "Blocked");
            match msg {
                BudsWorkerInput::Connect => match self.connect_and_get_stream().await {
                    Ok(stream) => {
                        let mut stream_guard = state.stream.lock().await;
                        *stream_guard = Some(stream);
                        sender.output(BudsWorkerOutput::Connected).unwrap();

                        sender.input(BudsWorkerInput::SendData(
                            BudsCommand::ManagerInfo.to_bytes(),
                        ));
                    }
                    Err(e) => {
                        sender
                            .output(BudsWorkerOutput::Error(e.to_string()))
                            .unwrap();
                    }
                },
                BudsWorkerInput::Disconnect => {
                    let mut stream_guard = state.stream.lock().await;
                    *stream_guard = None; // Dropping the stream closes the connection.
                    sender.output(BudsWorkerOutput::Disconnected).unwrap();
                }
                BudsWorkerInput::SendData(data) => {
                    self.send_data(&sender, data).await;
                }
                BudsWorkerInput::SendCommand(cmd) => {
                    self.send_data(&sender, cmd.to_bytes()).await;
                }
            }

            debug!(parent: &span, "Unblock");
        });
    }
}

impl BluetoothWorker {
    /// Performs the full bluetooth connection and profile registration dance.
    async fn connect_and_get_stream(
        &self,
    ) -> Result<Stream, Box<dyn std::error::Error + Send + Sync>> {
        let session = Session::new().await?;
        let device = self.device.device.clone();

        debug!("Connecting to device...");
        device.connect().await?;
        info!("Connected.");

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
        debug!("SPP Profile registered. Waiting for connection...");

        if let Some(req) = handle.next().await {
            debug!("Connection request from {:?} accepted.", req.device());
            let stream = req.accept()?;
            info!("RFCOMM stream established.");
            Ok(stream)
        } else {
            error!("No connection request received");
            Err("No connection request received".into())
        }
    }

    // Send data to stream
    async fn send_data(&self, sender: &ComponentSender<BluetoothWorker>, data: Vec<u8>) {
        let state = self.state.clone();
        let mut stream_guard = state.stream.lock().await;
        if let Some(stream) = stream_guard.as_mut() {
            if let Err(e) = stream.write_all(&data).await {
                sender
                    .output(BudsWorkerOutput::Error(e.to_string()))
                    .unwrap();
            }
        } else {
            sender
                .output(BudsWorkerOutput::Error("Not connected".to_string()))
                .unwrap();
        }
    }
}
