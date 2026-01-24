# Livestream Pipeline

A WebRTC-based livestream system built with Rust and GStreamer that enables real-time audio/video streaming between producer and consumer clients.

## Overview

This project implements a peer-to-peer livestream pipeline with three main components:

- **Producer**: Captures audio/video from local devices (camera + microphone) and streams via WebRTC
- **Consumer**: Receives and plays back the WebRTC stream
- **Relay**: TCP server that relays WebRTC signaling messages between peers

## Architecture

The system uses a split pipeline design:

**Producer Pipeline**: `v4l2src/pulsesrc → audioconvert/videoconvert → encode (opus/x264) → RTP payloading → webrtcbin`

**Consumer Pipeline**: `webrtcbin → decodebin → audioconvert/videoconvert → scale/resample → autosink`

WebRTC signaling (SDP offers/answers and ICE candidates) is handled via JSON messages exchanged through the relay server.

## Requirements

- Rust (2024 edition)
- GStreamer 1.0 with WebRTC support
- v4l2 (Video4Linux2) for camera input
- PulseAudio for microphone input

## Usage

### Start the Relay Server

```bash
cargo run --bin relay
```

The relay server listens on `0.0.0.0:8080` and broadcasts signaling messages between connected peers.

### Start the Consumer

```bash
cargo run --bin consumer
```

Receives the WebRTC stream and plays it back using your system's audio/video outputs.

### Start the Producer

```bash
cargo run --bin producer
```

Captures from your default camera and microphone, encodes with low-latency settings, and establishes a WebRTC connection.

## Configuration

STUN/TURN server settings can be modified in [mediaproducer.rs](src/mediaproducer.rs) and [mediaconsumer.rs](src/mediaconsumer.rs). The default HOST is set to `0.0.0.0` in [lib.rs](src/lib.rs).

Video encoding is optimized for low latency with ultrafast presets and zero-latency tuning.
