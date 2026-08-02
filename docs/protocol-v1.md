# Neural Device Communication Protocol v1

## Overview

This document defines the Version 1 binary communication protocol used between the Python neural-device simulator and the Rust host SDK.

The protocol transports neural spike-count data together with device telemetry over a TCP connection. The host validates every received packet before decoding and recording it.

---

## Packet Format

Each packet is exactly **40 bytes** long.

| Offset | Size | Field | Type |
|------:|-----:|----------------------|-------------------------|
| 0 | 2 | Magic bytes | `0x4E 0x52` (`"NR"`) |
| 2 | 1 | Protocol version | Unsigned 8-bit integer |
| 3 | 1 | Message type | Unsigned 8-bit integer |
| 4 | 4 | Sequence number | Unsigned 32-bit integer |
| 8 | 8 | Device timestamp (µs) | Unsigned 64-bit integer |
| 16 | 1 | Channel count | Unsigned 8-bit integer |
| 17 | 16 | Spike counts | Eight unsigned 16-bit integers |
| 33 | 1 | Battery percentage | Unsigned 8-bit integer |
| 34 | 2 | Temperature ×100 | Signed 16-bit integer |
| 36 | 4 | CRC32 checksum | Unsigned 32-bit integer |

**Total packet size:** **40 bytes**

---

## Protocol Constants

| Field | Value |
|--------|------|
| Magic bytes | `0x4E 0x52` (`"NR"`) |
| Protocol version | `1` |
| Message type | `1` |
| Channel count | `8` |

---

## Byte Order

All multibyte integer fields use **big-endian network byte order**.

---

## Sequence Numbers

Sequence numbers begin at **0** and increase by **1** for every generated sample.

The host SDK validates sequence continuity to detect:

- Missing packets
- Duplicate packets
- Unexpected packet ordering

---

## Temperature Encoding

Temperature is transmitted as:

```
Temperature (°C) × 100
```

Example:

| Temperature | Encoded value |
|------------:|-------------:|
| 37.00°C | 3700 |
| 42.14°C | 4214 |

The host converts the encoded integer back to floating-point Celsius values after decoding.

---

## CRC32 Validation

CRC32 is computed over the **first 36 bytes** of the packet.

The final four bytes contain the transmitted checksum.

The Rust SDK performs the following steps:

1. Read the packet.
2. Recompute the CRC32 checksum.
3. Compare the calculated checksum with the transmitted checksum.
4. Reject the packet if the values differ.

This mechanism detects intentionally corrupted packets generated during simulator fault injection.

---

## Device Telemetry

Every packet contains device-health information in addition to neural activity.

| Field | Description |
|--------|-------------|
| Battery percentage | Remaining simulated battery level |
| Temperature | Simulated device temperature |

The Rust SDK monitors these values and reports:

- High-temperature warnings
- Low-battery warnings
- Maximum observed temperature
- Minimum observed battery percentage

---

## Fault Injection

The simulator can intentionally introduce communication faults for testing SDK robustness.

Supported fault types include:

- Packet corruption (CRC mismatch)
- Packet drops (missing sequence numbers)
- Duplicate packet transmission

These faults allow the Rust SDK to verify protocol integrity, sequence validation, and degraded-session handling.

---

## Version History

### Version 1

Initial protocol implementation featuring:

- Fixed-size 40-byte packets
- CRC32 integrity verification
- Neural spike-count data
- Device telemetry
- Sequence-number validation