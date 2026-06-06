import 'package:flutter/material.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';
import 'package:ui/core/models/stream_with_settings.dart';

class CameraView extends StatefulWidget {
  final bool isLocal;
  final StreamWithSettings stream;

  const CameraView({super.key, required this.isLocal, required this.stream});

  @override
  State<CameraView> createState() => _CameraViewState();
}

class _CameraViewState extends State<CameraView> {
  final RTCVideoRenderer _renderer = RTCVideoRenderer();

  @override
  void initState() {
    super.initState();
    _initializeRenderer();
  }

  Future<void> _initializeRenderer() async {
    await _renderer.initialize();
    _renderer.srcObject = widget.stream.mediaStream;
  }

  @override
  void didUpdateWidget(CameraView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.stream.mediaStream.id != widget.stream.mediaStream.id) {
      _renderer.srcObject = widget.stream.mediaStream;
    }
    // Force rebuild when settings change
    if (oldWidget.stream.settings.isVideoEnabled !=
            widget.stream.settings.isVideoEnabled ||
        oldWidget.stream.settings.isAudioEnabled !=
            widget.stream.settings.isAudioEnabled) {
      setState(() {});
    }
  }

  @override
  void dispose() {
    _renderer.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        if (widget.stream.settings.isVideoEnabled)
          Positioned.fill(
            child: ClipRRect(
              borderRadius: BorderRadius.circular(8),
              child: RTCVideoView(
                _renderer,
                mirror: widget.isLocal,
                objectFit: RTCVideoViewObjectFit.RTCVideoViewObjectFitCover,
              ),
            ),
          )
        else
          Container(
            color: Colors.grey[700],
            child: const Center(
              child: Text('Video Off', style: TextStyle(color: Colors.white)),
            ),
          ),
        if (!widget.stream.settings.isAudioEnabled)
          Positioned(
            right: 12,
            top: 12,
            child: Container(
              padding: const EdgeInsets.all(4),
              decoration: BoxDecoration(
                color: Colors.black54,
                borderRadius: BorderRadius.circular(20),
              ),
              child: const Icon(Icons.mic_off, size: 16, color: Colors.white),
            ),
          ),
      ],
    );
  }
}
