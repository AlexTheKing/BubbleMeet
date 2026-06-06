import 'package:flutter/material.dart';

class VideoController extends StatelessWidget {
  final bool isVideoEnabled;
  final Function(bool) onSwitch;

  const VideoController({
    super.key,
    required this.isVideoEnabled,
    required this.onSwitch,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: () => onSwitch(!isVideoEnabled),
      child: Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: isVideoEnabled ? Colors.grey[800] : Colors.red[800],
          borderRadius: BorderRadius.circular(25),
        ),
        child: Icon(
          isVideoEnabled ? Icons.videocam : Icons.videocam_off,
          size: 28,
          color: Colors.white,
        ),
      ),
    );
  }
}

