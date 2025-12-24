//! OBD-II Protocol Definitions

use serde::{Deserialize, Serialize};

/// Supported OBD-II protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObdProtocol {
    /// Automatic protocol detection
    Auto,
    /// SAE J1850 PWM (41.6 kbaud)
    J1850Pwm,
    /// SAE J1850 VPW (10.4 kbaud)
    J1850Vpw,
    /// ISO 9141-2 (10.4 kbaud, 5 baud init)
    Iso9141_2,
    /// ISO 14230-4 KWP (slow init, 10.4 kbaud)
    Iso14230_4Kwp,
    /// ISO 14230-4 KWP (fast init, 10.4 kbaud)
    Iso14230_4KwpFast,
    /// ISO 15765-4 CAN (11 bit ID, 500 kbaud)
    Iso15765_4Can11bit500,
    /// ISO 15765-4 CAN (29 bit ID, 500 kbaud)
    Iso15765_4Can29bit500,
    /// ISO 15765-4 CAN (11 bit ID, 250 kbaud)
    Iso15765_4Can11bit250,
    /// ISO 15765-4 CAN (29 bit ID, 250 kbaud)
    Iso15765_4Can29bit250,
}

impl ObdProtocol {
    /// Get the ELM327 AT command for this protocol
    pub fn to_elm_command(&self) -> &'static str {
        match self {
            ObdProtocol::Auto => "ATSP0",
            ObdProtocol::J1850Pwm => "ATSP1",
            ObdProtocol::J1850Vpw => "ATSP2",
            ObdProtocol::Iso9141_2 => "ATSP3",
            ObdProtocol::Iso14230_4Kwp => "ATSP4",
            ObdProtocol::Iso14230_4KwpFast => "ATSP5",
            ObdProtocol::Iso15765_4Can11bit500 => "ATSP6",
            ObdProtocol::Iso15765_4Can29bit500 => "ATSP7",
            ObdProtocol::Iso15765_4Can11bit250 => "ATSP8",
            ObdProtocol::Iso15765_4Can29bit250 => "ATSP9",
        }
    }

    /// Check if this is a CAN protocol
    pub fn is_can(&self) -> bool {
        matches!(
            self,
            ObdProtocol::Iso15765_4Can11bit500
                | ObdProtocol::Iso15765_4Can29bit500
                | ObdProtocol::Iso15765_4Can11bit250
                | ObdProtocol::Iso15765_4Can29bit250
        )
    }

    /// Get the default baud rate for serial communication
    pub fn default_baud_rate(&self) -> u32 {
        // ELM327 adapters typically use 38400 or higher
        if self.is_can() {
            115200
        } else {
            38400
        }
    }
}

impl Default for ObdProtocol {
    fn default() -> Self {
        ObdProtocol::Auto
    }
}
