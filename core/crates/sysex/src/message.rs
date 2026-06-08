//! Roland SysEx message framing: building RQ1/DT1/Identity requests and parsing
//! inbound messages. The wire constants are private to this module.

use crate::checksum::roland_checksum;

// ── Wire constants ──
const EXCLUSIVE: u8 = 0xF0; // SysEx start
const EOX: u8 = 0xF7; // End of Exclusive
const ROLAND_ID: u8 = 0x41; // Manufacturer ID (Roland)
const UNIVERSAL_NON_REALTIME: u8 = 0x7E;
const GENERAL_INFO: u8 = 0x06; // Universal sub-ID#1
const IDENTITY_REQUEST: u8 = 0x01; // Universal sub-ID#2
const IDENTITY_REPLY: u8 = 0x02; // Universal sub-ID#2
const CMD_RQ1: u8 = 0x11; // Data Request 1
const CMD_DT1: u8 = 0x12; // Data Set 1

/// Build a Data Request (RQ1): asks the module to send back the data at
/// `address` of length `size` as a DT1.
pub fn build_rq1(device_id: u8, model_id: &[u8], address: [u8; 4], size: [u8; 4]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8);
    payload.extend_from_slice(&address);
    payload.extend_from_slice(&size);
    frame_roland(device_id, model_id, CMD_RQ1, &payload)
}

/// Build a Data Set (DT1): writes `data` to `address`.
pub fn build_dt1(device_id: u8, model_id: &[u8], address: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(4 + data.len());
    payload.extend_from_slice(&address);
    payload.extend_from_slice(data);
    frame_roland(device_id, model_id, CMD_DT1, &payload)
}

/// Build a Universal Non-realtime Identity Request (`F0 7E dev 06 01 F7`).
pub fn build_identity_request(device_id: u8) -> Vec<u8> {
    vec![
        EXCLUSIVE,
        UNIVERSAL_NON_REALTIME,
        device_id,
        GENERAL_INFO,
        IDENTITY_REQUEST,
        EOX,
    ]
}

/// Wrap `payload` (address + data) in a Roland SysEx frame with checksum.
fn frame_roland(device_id: u8, model_id: &[u8], command: u8, payload: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(model_id.len() + payload.len() + 6);
    msg.push(EXCLUSIVE);
    msg.push(ROLAND_ID);
    msg.push(device_id);
    msg.extend_from_slice(model_id);
    msg.push(command);
    msg.extend_from_slice(payload);
    msg.push(roland_checksum(payload));
    msg.push(EOX);
    msg
}

/// A parsed, complete SysEx message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SysexMessage {
    /// Data Set 1 — a write to the module, or its reply to an RQ1.
    Dt1 {
        device_id: u8,
        address: [u8; 4],
        data: Vec<u8>,
    },
    /// Universal Identity Reply — identifies the connected device (incl. firmware
    /// version in `version`).
    IdentityReply {
        device_id: u8,
        manufacturer_id: u8,
        family: [u8; 2],
        member: [u8; 2],
        version: [u8; 4],
    },
    /// A well-formed SysEx message we don't (need to) interpret.
    Unknown,
}

/// Why a byte slice could not be parsed as a SysEx message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    TooShort,
    NotSysex,
    BadChecksum,
    Malformed,
}

/// Parse a single complete SysEx message (`F0 … F7`).
///
/// `roland_model_id` is the active device's Model ID, needed to frame Roland DT1
/// messages. Universal messages (Identity Reply) are parsed regardless; pass an
/// empty slice before the device is known.
pub fn parse(bytes: &[u8], roland_model_id: &[u8]) -> Result<SysexMessage, ParseError> {
    if bytes.len() < 2 {
        return Err(ParseError::TooShort);
    }
    if bytes[0] != EXCLUSIVE || bytes[bytes.len() - 1] != EOX {
        return Err(ParseError::NotSysex);
    }
    match bytes[1] {
        UNIVERSAL_NON_REALTIME => parse_identity_reply(bytes),
        ROLAND_ID => parse_roland(bytes, roland_model_id),
        _ => Ok(SysexMessage::Unknown),
    }
}

/// `F0 7E dd 06 02 mm ff ff nn nn vv vv vv vv F7` (single-byte manufacturer).
fn parse_identity_reply(b: &[u8]) -> Result<SysexMessage, ParseError> {
    if b.len() < 5 || b[3] != GENERAL_INFO || b[4] != IDENTITY_REPLY {
        return Ok(SysexMessage::Unknown);
    }
    // Only the common single-byte-manufacturer layout is needed for now.
    if b.len() != 15 {
        return Ok(SysexMessage::Unknown);
    }
    Ok(SysexMessage::IdentityReply {
        device_id: b[2],
        manufacturer_id: b[5],
        family: [b[6], b[7]],
        member: [b[8], b[9]],
        version: [b[10], b[11], b[12], b[13]],
    })
}

/// `F0 41 dev <model_id…> cmd <address+data…> sum F7`.
fn parse_roland(b: &[u8], model_id: &[u8]) -> Result<SysexMessage, ParseError> {
    if model_id.is_empty() {
        // Can't locate the command/payload without knowing the Model ID length.
        return Ok(SysexMessage::Unknown);
    }
    let cmd_idx = 3 + model_id.len();
    // Need at least: F0 41 dev model cmd <≥1 payload byte> sum F7.
    if b.len() < cmd_idx + 3 {
        return Err(ParseError::TooShort);
    }
    if &b[3..cmd_idx] != model_id {
        return Ok(SysexMessage::Unknown);
    }
    let command = b[cmd_idx];
    let body = &b[cmd_idx + 1..b.len() - 1]; // between command and EOX
    let Some((&checksum, payload)) = body.split_last() else {
        return Err(ParseError::Malformed);
    };
    if roland_checksum(payload) != checksum {
        return Err(ParseError::BadChecksum);
    }
    match command {
        CMD_DT1 => {
            if payload.len() < 4 {
                return Err(ParseError::Malformed);
            }
            Ok(SysexMessage::Dt1 {
                device_id: b[2],
                address: [payload[0], payload[1], payload[2], payload[3]],
                data: payload[4..].to_vec(),
            })
        }
        // RQ1 is something we send, not receive; accept other commands as Unknown.
        _ => Ok(SysexMessage::Unknown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V31_MODEL: [u8; 3] = [0x01, 0x06, 0x01];

    #[test]
    fn parse_rejects_bad_checksum() {
        let mut bytes = build_dt1(0x10, &V31_MODEL, [0x04, 0x00, 0x52, 0x21], &[0x01]);
        let n = bytes.len();
        bytes[n - 2] ^= 0x7F; // corrupt the checksum byte
        assert_eq!(parse(&bytes, &V31_MODEL), Err(ParseError::BadChecksum));
    }

    #[test]
    fn parse_identity_reply_fields() {
        let bytes = [
            0xF0, 0x7E, 0x10, 0x06, 0x02, 0x41, 0x06, 0x01, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00,
            0xF7,
        ];
        assert_eq!(
            parse(&bytes, &[]),
            Ok(SysexMessage::IdentityReply {
                device_id: 0x10,
                manufacturer_id: 0x41,
                family: [0x06, 0x01],
                member: [0x00, 0x00],
                version: [0x00, 0x00, 0x02, 0x00],
            })
        );
    }

    #[test]
    fn parse_dt1_unknown_without_model_id() {
        // Before the device is identified we don't know the Model ID length.
        let bytes = build_dt1(0x10, &V31_MODEL, [0x04, 0x00, 0x52, 0x21], &[0x01]);
        assert_eq!(parse(&bytes, &[]), Ok(SysexMessage::Unknown));
    }

    #[test]
    fn parse_non_sysex_errs() {
        assert_eq!(
            parse(&[0x90, 0x40, 0x7F], &V31_MODEL),
            Err(ParseError::NotSysex)
        );
    }

    #[test]
    fn parse_too_short_errs() {
        assert_eq!(parse(&[0xF0], &V31_MODEL), Err(ParseError::TooShort));
    }
}
