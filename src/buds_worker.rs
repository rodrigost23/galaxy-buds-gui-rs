use bluer::{
    Session,
    rfcomm::{
        Profile, Role, Stream,
        stream::{OwnedReadHalf, OwnedWriteHalf},
    },
};
use futures::StreamExt;

use relm4::{Sender, Worker, prelude::*};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    runtime::Runtime,
    sync::{Mutex, MutexGuard},
    time::timeout,
};
use tracing::{debug, debug_span, error, info};

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
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
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

    fn init(device: Self::Init, _sender: ComponentSender<Self>) -> Self {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime"),
        );

        let state = WorkerState {
            writer: Arc::new(Mutex::new(None)),
        };

        Self {
            device,
            state,
            runtime,
        }
    }

    /// Handles discrete events from the UI. Each message is processed in a short-lived async task.
    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.runtime.block_on(self.handle_input(msg, sender));
    }
}

impl BluetoothWorker {
    async fn handle_input(&self, msg: BudsWorkerInput, sender: ComponentSender<Self>) {
        let span = debug_span!("BudsCommand", msg=?msg);
        debug!(parent: &span, "start handle");
        let state = self.state.clone();
        let mut writer_guard = state.writer.lock().await;

        match msg {
            BudsWorkerInput::Connect => match self.connect_and_get_stream().await {
                Ok(stream) => {
                    // Split reader and writer streams
                    let (reader, writer) = stream.into_split();
                    *writer_guard = Some(writer);

                    // Run reader loop in background
                    relm4::spawn(read_task(reader, sender.output_sender().clone()));

                    self.send_data(&sender, writer_guard, BudsCommand::ManagerInfo.to_bytes()).await;

                    sender.output(BudsWorkerOutput::Connected).unwrap();
                }
                Err(e) => {
                    sender
                        .output(BudsWorkerOutput::Error(e.to_string()))
                        .unwrap();
                }
            },
            BudsWorkerInput::Disconnect => {
                *writer_guard = None; // Dropping the stream closes the connection.
                sender.output(BudsWorkerOutput::Disconnected).unwrap();
            }
            BudsWorkerInput::SendData(data) => {
                self.send_data(&sender, writer_guard, data).await;
            }
            BudsWorkerInput::SendCommand(cmd) => {
                self.send_data(&sender, writer_guard, cmd.to_bytes()).await;
            }
        }
        debug!(parent: &span, "end handle");
    }
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
    async fn send_data(
        &self,
        sender: &ComponentSender<BluetoothWorker>,
        mut writer_guard: MutexGuard<'_, Option<OwnedWriteHalf>>,
        data: Vec<u8>,
    ) {
        if let Some(stream) = writer_guard.as_mut() {
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

async fn read_task(mut stream: OwnedReadHalf, sender: Sender<BudsWorkerOutput>) {
    let span = debug_span!("Stream read loop");
    debug!(parent: &span, "Start reading");
    loop {
        let mut buffer = [0u8; 2048];

        match stream.read(&mut buffer).await {
            Ok(0) => {
                error!(parent: &span, "Stream closed by peer");
                sender
                    .send(BudsWorkerOutput::Error("Stream closed by peer".to_string()))
                    .unwrap();
                sender.send(BudsWorkerOutput::Disconnected).unwrap();
                break;
            }
            Ok(n) => {
                let buff = &buffer[..n];

                match BudsMessage::from_bytes(buff) {
                    Some(msg) => {
                        sender.send(BudsWorkerOutput::DataReceived(msg)).unwrap();
                    }
                    None => continue,
                };
            }
            Err(e) => {
                error!(parent: &span, "Read error {:?}", e);
                sender
                    .send(BudsWorkerOutput::Error(format!("Read error: {}", e)))
                    .unwrap();
                sender.send(BudsWorkerOutput::Disconnected).unwrap();
                break;
            }
        }
    }
    debug!(parent: &span, "Stop reading");
}
