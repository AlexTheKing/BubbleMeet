# Bubble Meet ML - Triton Inference Server

This directory contains the NVIDIA Triton Inference Server setup for audio processing with Voice Activity Detection (VAD) and Whisper transcription/translation models.

## Models

1. **VAD (Voice Activity Detection)**: Uses Silero VAD to detect speech segments in audio
2. **Whisper**: OpenAI Whisper model for transcription and translation
3. **audio_pipeline**: Performance-optimized pipeline that runs VAD first, then processes only detected speech segments with Whisper. This should reduce processing time for streaming audio (e.g., video chat) where there's often silence between speech segments.

## Setup

### Prerequisites

- Docker

### Building Python Environment

The Triton Python backend requires a packaged Python environment. Build it using:

```bash
docker build -t triton-env-builder .
docker run -v ./triton-env:/output triton-env-builder
```

This will create `triton-env/python_env.tar.gz` which contains all Python dependencies.

### Verifying Models

Check if models are loaded:

```bash
curl http://localhost:8000/v2/models
```

### Model Parameters

- **VAD**: Configured for 16kHz audio, uses Silero VAD model - very fast inference
- **Whisper**: Uses "tiny" model by default (can be changed in `models/whisper/1/model.py`)
  - **Optimized for pre-filtered audio**: Since VAD already filters silence, Whisper parameters are tuned:
    - `no_speech_threshold=0.3` (lower than default 0.6) - trusts that VAD already filtered silence
    - `condition_on_previous_text=False` - faster for streaming chunks, no context needed
    - `temperature=0.0` - deterministic, faster inference