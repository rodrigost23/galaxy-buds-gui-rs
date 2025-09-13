use bluer::{
    Session, Uuid,
    rfcomm::{
        Profile, Role, Stream,
        stream::{OwnedReadHalf, OwnedWriteHalf},
    },
};
use futures::StreamExt;
use galaxy_buds_rs::message;
use relm4::{Sender, Worker, prelude::*};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    runtime::Runtime,
    sync::Mutex,
};
use tracing::{debug, debug_span, error, info, trace, trace_span, warn};

use crate::{
    consts::SAMSUNG_SPP_UUID,
    model::{
        buds_message::{BudsCommand, BudsMessage},
        device_info::DeviceInfo,
    },
};

const READ_BUFFER_SIZE: usize = 2048;

/// Input messages for the `BluetoothWorker`.
#[derive(Debug)]
pub enum BudsWorkerInput {
    /// Starts the discovery and connection process.
    Connect,
    /// Disconnects from the current device.
    Disconnect,
    /// Sends a raw byte payload to the device.
    SendData(Vec<u8>),
    /// Encodes and sends a `BudsCommand` to the device.
    SendCommand(BudsCommand),
}

/// Output messages from the `BluetoothWorker`.
#[derive(Debug)]
pub enum BudsWorkerOutput {
    /// Emitted when a connection is successfully established.
    Connected,
    /// Emitted when the device is disconnected.
    Disconnected,
    /// Emitted when a `BudsMessage` is received from the device.
    DataReceived(BudsMessage),
    /// Emitted when an error occurs.
    Error(String),
}

/// A `relm4::Worker` that manages the Bluetooth connection and communication
/// with a Galaxy Buds device.
#[derive(Debug)]
pub struct BluetoothWorker {
    device: DeviceInfo,
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
    runtime: Arc<Runtime>,
    is_running: Arc<AtomicBool>,
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

        let writer = Arc::new(Mutex::new(None));
        let is_running = Arc::new(AtomicBool::new(false));

        Self {
            device,
            writer,
            runtime,
            is_running,
        }
    }

    /// Handles discrete events from the UI. Each message is processed in a short-lived async task.
    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.runtime
            .block_on(self.handle_input(msg, sender.output_sender()));
    }
}

impl BluetoothWorker {
    /// Asynchronously handles an input message.
    async fn handle_input(&self, msg: BudsWorkerInput, sender: &Sender<<Self as Worker>::Output>) {
        let span = debug_span!("BudsCommand", msg=?msg);
        debug!(parent: &span, "start handle");

        match msg {
            BudsWorkerInput::Connect => self.connect(sender).await,
            BudsWorkerInput::Disconnect => {
                self.is_running.store(false, Ordering::Relaxed);
                // Dropping the writer will close the connection, causing the read task to terminate.
                *self.writer.lock().await = None;
                if sender.send(BudsWorkerOutput::Disconnected).is_err() {
                    warn!("UI receiver dropped, could not send Disconnected message.");
                }
            }
            BudsWorkerInput::SendData(data) => self.send_data(sender, data).await,
            BudsWorkerInput::SendCommand(cmd) => self.send_data(sender, cmd.to_bytes()).await,
        }
        debug!(parent: &span, "end handle");
    }

    /// Establishes a connection and spawns the reading task.
    async fn connect(&self, sender: &Sender<BudsWorkerOutput>) {
        match self.connect_and_get_stream().await {
            Ok(stream) => {
                // Split reader and writer streams
                let (reader, writer) = stream.into_split();
                *self.writer.lock().await = Some(writer);

                // Run reader loop in background

                self.is_running.store(true, Ordering::Relaxed);
                relm4::spawn(read_task(
                    reader,
                    sender.clone(),
                    Arc::clone(&self.is_running),
                ));

                // Request manager info after connecting
                self.send_data(&sender, BudsCommand::ManagerInfo.to_bytes())
                    .await;

                if sender.send(BudsWorkerOutput::Connected).is_err() {
                    warn!("UI receiver dropped, could not send Connected message.");
                }
            }
            Err(e) => {
                let err_msg = format!("Connection failed: {}", e);
                error!("{}", err_msg);
                if sender.send(BudsWorkerOutput::Error(err_msg)).is_err() {
                    warn!("UI receiver dropped, could not send Error message.");
                }
            }
        }
    }

    /// Performs the full Bluetooth connection and profile registration dance.
    async fn connect_and_get_stream(
        &self,
    ) -> Result<Stream, Box<dyn std::error::Error + Send + Sync>> {
        let session = Session::new().await?;
        let device = self.device.device.clone();

        debug!("Connecting to device {}...", device.address());
        device.connect().await?;
        info!("Device connected.");

        // let spp_uuid = bluer::id::ServiceClass::SerialPort.into();
        let spp_uuid: Uuid = SAMSUNG_SPP_UUID.parse()?;
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
            Err("No connection request received".into())
        }
    }

    /// Sends a byte payload to the device via the RFCOMM stream.
    async fn send_data(&self, sender: &Sender<<BluetoothWorker as Worker>::Output>, data: Vec<u8>) {
        if let Some(stream) = self.writer.lock().await.as_mut() {
            if let Err(e) = stream.write_all(&data).await {
                let err_msg = format!("Send data failed: {}", e);
                error!("{}", err_msg);
                if sender.send(BudsWorkerOutput::Error(err_msg)).is_err() {
                    warn!("UI receiver dropped, could not send Error message.");
                }
            }
        } else {
            let err_msg = "Cannot send data: Not connected".to_string();
            error!("{}", err_msg);
            if sender.send(BudsWorkerOutput::Error(err_msg)).is_err() {
                warn!("UI receiver dropped, could not send Error message.");
            }
        }
    }
}

/// Asynchronous task that continuously reads from the RFCOMM stream.
///
/// It runs in a loop, waiting for incoming data, parsing it into `BudsMessage`s,
/// and sending them to the UI. The loop terminates when the `is_running` flag
/// is set to false or a fatal error occurs.
async fn read_task(
    mut stream: OwnedReadHalf,
    sender: Sender<BudsWorkerOutput>,
    is_running: Arc<AtomicBool>,
) {
    let span = trace_span!("Stream read loop");
    let _enter = span.enter();
    debug!("Start reading");
    let mut read_buffer: Vec<u8> = Vec::new();

    while is_running.load(Ordering::Relaxed) {
        let mut temp_buffer = [0u8; READ_BUFFER_SIZE];

        match stream.read(&mut temp_buffer).await {
            Ok(0) => {
                info!("Stream closed by peer");
                break;
            }
            Ok(n) => {
                read_buffer.extend_from_slice(&temp_buffer[..n]);
                trace!(
                    "Read {} bytes. Current buffer size: {}",
                    n,
                    read_buffer.len()
                );
                for message_frame in process_buffer(&mut read_buffer) {
                    if let Some(msg) = BudsMessage::from_bytes(&message_frame) {
                        if sender.send(BudsWorkerOutput::DataReceived(msg)).is_err() {
                            warn!("UI receiver dropped, could not send DataReceived message.");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                // Only log error if we were expecting to be running.
                if is_running.load(Ordering::Relaxed) {
                    error!(parent: &span, "Read error: {}", e);
                    let err_msg = format!("Read error: {}", e);
                    if sender.send(BudsWorkerOutput::Error(err_msg)).is_err() {
                        warn!("UI receiver dropped, could not send Error message.");
                    }
                }
                break;
            }
        }
    }

    // Ensure we always send a disconnected message on exit.
    if sender.send(BudsWorkerOutput::Disconnected).is_err() {
        warn!("UI receiver dropped, could not send final Disconnected message.");
    }
    is_running.store(false, Ordering::Relaxed);
    debug!(parent: &span, "Stop reading");
}

fn process_buffer(buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let span = trace_span!("Process buffer");
    let _enter = span.enter();

    let mut messages_frames = Vec::new();

    loop {
        // Find the start and end of the next message.
        let bom_pos = buffer.iter().position(|&b| b == message::BOM);
        let eom_pos = buffer.iter().position(|&b| b == message::EOM);

        match (bom_pos, eom_pos) {
            // Complete message:
            (Some(start), Some(end)) if start < end => {
                // If there was garbage data before the BOM, log and discard it.
                if start > 0 {
                    trace!("Discarding {} bytes of garbage data.", start);
                }

                let message_frame = &buffer[start..=end];
                trace!("Found message with {} bytes.", message_frame.len());
                messages_frames.push(message_frame.to_vec());

                // Remove the processed message and any preceding garbage,
                // and continue loop
                buffer.drain(..=end);
            }
            // Found only beginning of message; message is incomplete.
            (Some(start), _) => {
                // Discard any garbage before the first valid BOM we found.
                if start > 0 {
                    buffer.drain(..start);
                }
                trace!("Found incomplete message with {} bytes.", buffer.len());
                // Break the loop and keep buffer with incomplete message.
                break;
            }
            // No BOM found; either buffer is empty or there is only garbage.
            _ => {
                if !buffer.is_empty() {
                    trace!("No BOM found, clearing buffer of {} bytes.", buffer.len());
                    buffer.clear();
                }
                break;
            }
        }
    }
    return messages_frames;
}
