use bluer::{
    Session,
    rfcomm::{
        Profile, Role, Stream,
        stream::{OwnedReadHalf, OwnedWriteHalf},
    },
};
use futures::StreamExt;

use relm4::{Sender, Worker, prelude::*};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    runtime::Runtime,
    sync::Mutex,
};
use tracing::{debug, debug_span, error, info, trace};

use crate::model::{
    buds_message::{BudsCommand, BudsMessage},
    device_info::DeviceInfo,
};

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

#[derive(Debug)]
pub struct BluetoothWorker {
    device: DeviceInfo,
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
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

        let writer = Arc::new(Mutex::new(None));

        Self {
            device,
            writer,
            runtime,
        }
    }

    /// Handles discrete events from the UI. Each message is processed in a short-lived async task.
    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.runtime
            .block_on(self.handle_input(msg, sender.output_sender()));
    }
}

impl BluetoothWorker {
    async fn handle_input(&self, msg: BudsWorkerInput, sender: &Sender<<Self as Worker>::Output>) {
        let span = debug_span!("BudsCommand", msg=?msg);
        debug!(parent: &span, "start handle");

        match msg {
            BudsWorkerInput::Connect => self.connect(sender).await,
            BudsWorkerInput::Disconnect => {
                *self.writer.lock().await = None; // Dropping the stream closes the connection.
                sender.send(BudsWorkerOutput::Disconnected).unwrap();
            }
            BudsWorkerInput::SendData(data) => self.send_data(&sender, data).await,
            BudsWorkerInput::SendCommand(cmd) => self.send_data(&sender, cmd.to_bytes()).await,
        }
        debug!(parent: &span, "end handle");
    }

    async fn connect(&self, sender: &Sender<BudsWorkerOutput>) {
        match self.connect_and_get_stream().await {
            Ok(stream) => {
                // Split reader and writer streams
                let (reader, writer) = stream.into_split();
                *self.writer.lock().await = Some(writer);

                // Run reader loop in background
                relm4::spawn(read_task(reader, sender.clone()));

                self.send_data(&sender, BudsCommand::ManagerInfo.to_bytes())
                    .await;

                sender.send(BudsWorkerOutput::Connected).unwrap();
            }
            Err(e) => {
                sender.send(BudsWorkerOutput::Error(e.to_string())).unwrap();
            }
        }
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
    async fn send_data(&self, sender: &Sender<<BluetoothWorker as Worker>::Output>, data: Vec<u8>) {
        if let Some(stream) = self.writer.lock().await.as_mut() {
            if let Err(e) = stream.write_all(&data).await {
                sender.send(BudsWorkerOutput::Error(e.to_string())).unwrap();
            }
        } else {
            sender
                .send(BudsWorkerOutput::Error("Not connected".to_string()))
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
                trace!("Read {} bytes", n);
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
