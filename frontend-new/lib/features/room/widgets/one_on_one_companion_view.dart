import 'package:flutter/material.dart';
import 'package:ui/core/models/stream_with_settings.dart';
import 'camera_view.dart';

class OneOnOneCompanionView extends StatelessWidget {
  final StreamWithSettings localStream;
  final StreamWithSettings companionStream;

  const OneOnOneCompanionView({
    super.key,
    required this.localStream,
    required this.companionStream,
  });

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        Positioned.fill(
          child: CameraView(isLocal: false, stream: companionStream),
        ),
        if (localStream.settings.isVideoEnabled)
          Positioned(
            bottom: 16,
            right: 16,
            child: SizedBox(
              width: 256,
              height: 192,
              child: CameraView(isLocal: true, stream: localStream),
            ),
          ),
      ],
    );
  }
}
