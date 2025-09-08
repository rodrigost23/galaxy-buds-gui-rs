use galaxy_buds_rs::{
    message::{
        Message, Payload, extended_status_updated::ExtendedStatusUpdate, find_my_bud, ids, manager,
        status_updated::StatusUpdate,
    },
    model::Model,
};

#[derive(Debug)]
pub enum BudsMessage {
    StatusUpdate(StatusUpdate),
    ExtendedStatusUpdate(ExtendedStatusUpdate),

    Unknown { id: u8, buffer: Vec<u8> },
}

impl BudsMessage {
    /// Parses a raw byte buffer into a BudsMessage.
    ///
    /// Returns `None` for messages that should be ignored, like keep-alives.
    pub fn from_bytes(buff: &[u8]) -> Option<Self> {
        // Basic validation
        if buff.len() < 4 {
            return None;
        }
        let id = buff[3];

        if id == 242 {
            return None;
        }

        // TODO: Support other models
        let message = Message::new(buff, Model::BudsLive);
        let parsed_message = match id {
            ids::STATUS_UPDATED => Self::StatusUpdate(message.into()),
            ids::EXTENDED_STATUS_UPDATED => Self::ExtendedStatusUpdate(message.into()),
            _ => Self::Unknown {
                id,
                buffer: buff.to_vec(),
            },
        };

        Some(parsed_message)
    }
}

#[derive(Debug)]
pub enum BudsCommand {
    ManagerInfo,
    FindStart,
    FindStop,
}

impl BudsCommand {
    /// Serializes the command into a byte buffer to be sent to the device.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            BudsCommand::ManagerInfo => manager::new(true, 34).to_byte_array(),
            BudsCommand::FindStart => find_my_bud::new(true).to_byte_array(),
            BudsCommand::FindStop => find_my_bud::new(false).to_byte_array(),
        }
    }
}
