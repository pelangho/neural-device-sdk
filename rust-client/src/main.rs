use crc32fast::Hasher;
use std::convert::TryInto;
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use thiserror::Error;

const SERVER_ADDRESS: &str = "127.0.0.1:9000";

const PACKET_SIZE: usize = 40;
const DATA_SIZE: usize = 36;
const CHANNEL_COUNT: usize = 8;

const EXPECTED_MAGIC: [u8; 2] = *b"NR";
const SUPPORTED_PROTOCOL_VERSION: u8 = 1;
const NEURAL_DATA_MESSAGE_TYPE: u8 = 1;

const RECORDING_PACKET_LIMIT: usize = 500;
const OUTPUT_PATH: &str = "../recordings/session.csv";

const HIGH_TEMPERATURE_THRESHOLD_C: f32 = 40.0;
const LOW_BATTERY_THRESHOLD_PERCENT: u8 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceState {
    Disconnected,
    Connecting,
    Recording,
    Degraded,
    Completed,
}

fn transition_state(
    current_state: &mut DeviceState,
    next_state: DeviceState,
) {
    if *current_state != next_state {
        println!(
            "STATE TRANSITION: {:?} -> {:?}",
            current_state,
            next_state
        );

        *current_state = next_state;
    }
}

#[derive(Debug)]
struct NeuralDataPacket {
    protocol_version: u8,
    message_type: u8,
    sequence_number: u32,
    timestamp_microseconds: u64,
    spike_counts: [u16; CHANNEL_COUNT],
    battery_percent: u8,
    temperature_celsius: f32,
}

#[derive(Debug, Error)]
enum PacketError {
    #[error("invalid packet length: expected {expected}, received {actual}")]
    InvalidLength { expected: usize, actual: usize },

    #[error("invalid magic bytes: {0:?}")]
    InvalidMagic([u8; 2]),

    #[error("unsupported protocol version: {0}")]
    UnsupportedProtocolVersion(u8),

    #[error("unexpected message type: {0}")]
    UnexpectedMessageType(u8),

    #[error("invalid channel count: expected {expected}, received {actual}")]
    InvalidChannelCount { expected: usize, actual: usize },

    #[error(
        "CRC mismatch: received {received:#010x}, calculated {calculated:#010x}"
    )]
    CrcMismatch { received: u32, calculated: u32 },

    #[error("failed to convert packet bytes")]
    ByteConversion,
}

fn read_u16_be(bytes: &[u8]) -> Result<u16, PacketError> {
    let array: [u8; 2] = bytes
        .try_into()
        .map_err(|_| PacketError::ByteConversion)?;

    Ok(u16::from_be_bytes(array))
}

fn read_i16_be(bytes: &[u8]) -> Result<i16, PacketError> {
    let array: [u8; 2] = bytes
        .try_into()
        .map_err(|_| PacketError::ByteConversion)?;

    Ok(i16::from_be_bytes(array))
}

fn read_u32_be(bytes: &[u8]) -> Result<u32, PacketError> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| PacketError::ByteConversion)?;

    Ok(u32::from_be_bytes(array))
}

fn read_u64_be(bytes: &[u8]) -> Result<u64, PacketError> {
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| PacketError::ByteConversion)?;

    Ok(u64::from_be_bytes(array))
}

fn calculate_crc32(bytes: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize()
}

fn decode_packet(packet: &[u8]) -> Result<NeuralDataPacket, PacketError> {
    if packet.len() != PACKET_SIZE {
        return Err(PacketError::InvalidLength {
            expected: PACKET_SIZE,
            actual: packet.len(),
        });
    }

    let magic = [packet[0], packet[1]];

    if magic != EXPECTED_MAGIC {
        return Err(PacketError::InvalidMagic(magic));
    }

    let protocol_version = packet[2];

    if protocol_version != SUPPORTED_PROTOCOL_VERSION {
        return Err(PacketError::UnsupportedProtocolVersion(
            protocol_version,
        ));
    }

    let message_type = packet[3];

    if message_type != NEURAL_DATA_MESSAGE_TYPE {
        return Err(PacketError::UnexpectedMessageType(message_type));
    }

    let sequence_number = read_u32_be(&packet[4..8])?;
    let timestamp_microseconds = read_u64_be(&packet[8..16])?;
    let channel_count = packet[16] as usize;

    if channel_count != CHANNEL_COUNT {
        return Err(PacketError::InvalidChannelCount {
            expected: CHANNEL_COUNT,
            actual: channel_count,
        });
    }

    let mut spike_counts = [0_u16; CHANNEL_COUNT];
    let mut offset = 17;

    for spike_count in &mut spike_counts {
        *spike_count = read_u16_be(&packet[offset..offset + 2])?;
        offset += 2;
    }

    let battery_percent = packet[33];
    let temperature_encoded = read_i16_be(&packet[34..36])?;
    let temperature_celsius = temperature_encoded as f32 / 100.0;

    let received_crc = read_u32_be(&packet[36..40])?;
    let calculated_crc = calculate_crc32(&packet[..DATA_SIZE]);

    if received_crc != calculated_crc {
        return Err(PacketError::CrcMismatch {
            received: received_crc,
            calculated: calculated_crc,
        });
    }

    Ok(NeuralDataPacket {
        protocol_version,
        message_type,
        sequence_number,
        timestamp_microseconds,
        spike_counts,
        battery_percent,
        temperature_celsius,
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut device_state = DeviceState::Disconnected;

    transition_state(
        &mut device_state,
        DeviceState::Connecting,
    );

    println!("Connecting to simulated neural device at {SERVER_ADDRESS}...");

    let mut stream = TcpStream::connect(SERVER_ADDRESS)?;

    println!("Connected successfully.");

    transition_state(
        &mut device_state,
        DeviceState::Recording,
    );

    let output_path = Path::new(OUTPUT_PATH);

    if let Some(parent_directory) = output_path.parent() {
        create_dir_all(parent_directory)?;
    }

    let output_file = File::create(output_path)?;
    let mut writer = BufWriter::new(output_file);

    writeln!(
        writer,
        "sequence_number,timestamp_microseconds,\
        channel_1,channel_2,channel_3,channel_4,\
        channel_5,channel_6,channel_7,channel_8,\
        battery_percent,temperature_celsius"
    )?;

    let mut previous_sequence: Option<u32> = None;
    let mut valid_packet_count = 0_usize;
    let mut rejected_packet_count = 0_usize;
    let mut sequence_warning_count = 0_usize;

    let mut high_temperature_warning_count = 0_usize;
    let mut low_battery_warning_count = 0_usize;
    let mut maximum_temperature_celsius = f32::MIN;
    let mut minimum_battery_percent = u8::MAX; 

    while valid_packet_count < RECORDING_PACKET_LIMIT {
        let mut packet_bytes = [0_u8; PACKET_SIZE];

        if let Err(error) = stream.read_exact(&mut packet_bytes) {
            eprintln!("Connection ended before recording completed: {error}");
            break;
        }

        match decode_packet(&packet_bytes) {
            Ok(packet) => {
                if let Some(previous) = previous_sequence {
                    let expected = previous.wrapping_add(1);

                    if packet.sequence_number != expected {
                        sequence_warning_count += 1;

                        eprintln!(
                            "Sequence warning: expected {}, received {}",
                            expected,
                            packet.sequence_number
                        );

                        transition_state(
                            &mut device_state,
                            DeviceState::Degraded,
                        );   
                    }
                }

                previous_sequence = Some(packet.sequence_number);
                maximum_temperature_celsius =
                    maximum_temperature_celsius.max(packet.temperature_celsius);

                minimum_battery_percent =
                    minimum_battery_percent.min(packet.battery_percent);

                if packet.temperature_celsius > HIGH_TEMPERATURE_THRESHOLD_C {
                    high_temperature_warning_count += 1;

                    eprintln!(
                         "HEALTH WARNING: High device temperature: {:.2}°C",
                         packet.temperature_celsius
                    );
                    transition_state(
                        &mut device_state,
                        DeviceState::Degraded,
                    );
                }

                if packet.battery_percent < LOW_BATTERY_THRESHOLD_PERCENT {
                   low_battery_warning_count += 1;

                   eprintln!(
                        "HEALTH WARNING: Low battery: {}%",
                        packet.battery_percent
                   );
                   transition_state(
                    &mut device_state,
                    DeviceState::Degraded,
                   );
                }

                writeln!(
                    writer,
                    "{},{},{},{},{},{},{},{},{},{},{},{:.2}",
                    packet.sequence_number,
                    packet.timestamp_microseconds,
                    packet.spike_counts[0],
                    packet.spike_counts[1],
                    packet.spike_counts[2],
                    packet.spike_counts[3],
                    packet.spike_counts[4],
                    packet.spike_counts[5],
                    packet.spike_counts[6],
                    packet.spike_counts[7],
                    packet.battery_percent,
                    packet.temperature_celsius,
                )?;

                valid_packet_count += 1;

                if valid_packet_count % 50 == 0 {
                    println!(
                        "Recorded {}/{} packets",
                        valid_packet_count,
                        RECORDING_PACKET_LIMIT
                    );
                }
            }

            Err(error) => {
                rejected_packet_count += 1;
                eprintln!("Rejected packet: {error}");
                
                transition_state(
                    &mut device_state,
                    DeviceState::Degraded,
                );
            }
        }
    }

    writer.flush()?;

    transition_state(
        &mut device_state,
        DeviceState::Completed,
    );

    println!("Recording complete.");
    println!("Valid packets recorded: {valid_packet_count}");
    println!("Rejected packets: {rejected_packet_count}");
    println!("Sequence warnings: {sequence_warning_count}");
    println!(
        "High-temperature warnings: {high_temperature_warning_count}"
    );
    println!(
        "Low-battery warnings: {low_battery_warning_count}"
    );
    println!(
        "Maximum temperature: {:.2}°C",
        maximum_temperature_celsius
    );
    println!(
        "Minimum battery: {}%",
        minimum_battery_percent
    );
    println!("Saved recording to: {OUTPUT_PATH}");
    println!("Final device state: {:?}", device_state);

    Ok(())
}
