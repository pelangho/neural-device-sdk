import random
import time
import struct
import zlib
import socket

PACKET_RATE_HZ = 100
SAMPLE_INTERVAL_SECONDS = 1 / PACKET_RATE_HZ
HOST = "127.0.0.1"
PORT = 9000

FAULT_INJECTION_ENABLED = True
DROP_EVERY_N_PACKETS = 40
CORRUPT_EVERY_N_PACKETS = 25
DUPLICATE_EVERY_N_PACKETS = 60

BATTERY_DRAIN_EVERY_N_PACKETS = 50
BATTERY_DRAIN_AMOUNT = 1

NORMAL_TEMPERATURE_C = 37.0
THERMAL_EVENT_START_PACKET = 300
THERMAL_EVENT_END_PACKET = 350
THERMAL_EVENT_TEMPERATURE_C = 42.0

class SimulatedNeuralDevice:
    """A simulated neural recording device."""

    def __init__(self):
       self.device_id = "SIM-001"
       self.sequence_number = 0
       self.battery_percent = 100
       self.temperature_celsius = 37.0
       self.is_recording = False
       self.channel_count = 8
       self.baseline_firing_rates = [
            2,
            5,
            8,
            1,
            10,
            4,
            7,
            3,
       ]

    def generate_spike_counts(self):
           
           
           spike_counts = []
           for i in range(self.channel_count):
               baseline = self.baseline_firing_rates[i]
               count = max(
                    0,
                    baseline + random.randint(-2, 2)
               )
               spike_counts.append(count)

           return spike_counts
    def update_telemetry(self):
        """Update simulated battery and temperature telemetry."""

        if (
            self.sequence_number > 0
            and self.sequence_number % BATTERY_DRAIN_EVERY_N_PACKETS == 0
        ):
            self.battery_percent = max(
                0,
                self.battery_percent - BATTERY_DRAIN_AMOUNT,
            )

        if (
            THERMAL_EVENT_START_PACKET
            <= self.sequence_number
            < THERMAL_EVENT_END_PACKET
        ):
            target_temperature = THERMAL_EVENT_TEMPERATURE_C
        else:
            target_temperature = NORMAL_TEMPERATURE_C

        temperature_noise = random.uniform(-0.15, 0.15)

        self.temperature_celsius = (
            target_temperature + temperature_noise
        )
    def generate_sample(self):
         self.update_telemetry()
         sample = {
              "device_id": self.device_id,
              "sequence_number": self.sequence_number,
              "timestamp_microseconds": time.monotonic_ns() // 1000,
              "spike_counts": self.generate_spike_counts(),
              "battery_percent": self.battery_percent,
              "temperature_celsius": self.temperature_celsius,
         }

         self.sequence_number += 1

         return sample
    def encode_sample(self, sample):
         magic = b"NR"
         protocol_version = 1
         message_type = 1
         channel_count = self.channel_count

         temperature_encoded = int(
              sample["temperature_celsius"] * 100
         )

         packet_without_crc = struct.pack(
              "!2sBBIQB8HBh",
              magic,
              protocol_version,
              message_type,
              sample["sequence_number"],
            sample["timestamp_microseconds"],
            channel_count,
            *sample["spike_counts"],
            sample["battery_percent"],
            temperature_encoded,
         )
         checksum = zlib.crc32(packet_without_crc)

         packet = packet_without_crc + struct.pack(
            "!I",
            checksum,
        )

         return packet
    def decode_packet(self, packet):
        if len(packet) != 40:
            raise ValueError(
                f"Expected 40 bytes, received {len(packet)}"
            )

        packet_without_crc = packet[:36]
        received_crc = struct.unpack("!I", packet[36:40])[0]
        calculated_crc = zlib.crc32(packet_without_crc)

        if received_crc != calculated_crc:
            raise ValueError("CRC validation failed")

        unpacked = struct.unpack(
            "!2sBBIQB8HBh",
            packet_without_crc
        )

        magic = unpacked[0]
        protocol_version = unpacked[1]
        message_type = unpacked[2]
        sequence_number = unpacked[3]
        timestamp_microseconds = unpacked[4]
        channel_count = unpacked[5]

        spike_counts = list(unpacked[6:14])
        battery_percent = unpacked[14]
        temperature_encoded = unpacked[15]
        temperature_celsius = temperature_encoded / 100.0

        return {
            "magic": magic,
            "protocol_version": protocol_version,
            "message_type": message_type,
            "sequence_number": sequence_number,
            "timestamp_microseconds": timestamp_microseconds,
            "channel_count": channel_count,
            "spike_counts": spike_counts,
            "battery_percent": battery_percent,
            "temperature_celsius": temperature_celsius,
            "crc_valid": True,
        }
def run_server():
    device = SimulatedNeuralDevice()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server_socket:
        server_socket.setsockopt(
            socket.SOL_SOCKET,
            socket.SO_REUSEADDR,
            1,
        )

        server_socket.bind((HOST, PORT))
        server_socket.listen(1)

        print(f"Simulator listening on {HOST}:{PORT}")
        print("Waiting for a client...")

        connection, client_address = server_socket.accept()

        with connection:
            print(f"Client connected from {client_address}")

            try:
                while True:
                    sample = device.generate_sample()
                    packet = device.encode_sample(sample)

                    sequence_number = sample["sequence_number"]
                    packet_number = sequence_number + 1

                     # Simulate a missing application-level packet.
                    if (
                            FAULT_INJECTION_ENABLED
                            and packet_number % DROP_EVERY_N_PACKETS == 0
                     ):
                        print(f"DROPPED packet {sequence_number}")
                        time.sleep(SAMPLE_INTERVAL_SECONDS)
                        continue

                     # Corrupt one byte after the CRC has already been calculated.
                    if (
                         FAULT_INJECTION_ENABLED
                         and packet_number % CORRUPT_EVERY_N_PACKETS == 0
                      ):
                         corrupted_packet = bytearray(packet)
                         corrupted_packet[20] ^= 0b0000_0001
                         packet = bytes(corrupted_packet)

                         print(f"CORRUPTED packet {sequence_number}")

                    connection.sendall(packet)

                    print(
                         f"Sent packet {sequence_number} "
                         f"({len(packet)} bytes)"
                    )

                    # Send the same packet a second time.
                    if (
                        FAULT_INJECTION_ENABLED
                        and packet_number % DUPLICATE_EVERY_N_PACKETS == 0
                    ):
                        connection.sendall(packet)
                        print(f"DUPLICATED packet {sequence_number}")

                    time.sleep(SAMPLE_INTERVAL_SECONDS)

            except BrokenPipeError:
                print("Client disconnected.")

            except ConnectionResetError:
                print("Connection reset by client.")

            except KeyboardInterrupt:
                print("\nSimulator stopped.")


if __name__ == "__main__":
    run_server()