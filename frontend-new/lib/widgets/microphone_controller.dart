import 'package:flutter/material.dart';

class MicrophoneController extends StatelessWidget {
  final bool isAudioEnabled;
  final Function(bool) onSwitch;

  const MicrophoneController({
    super.key,
    required this.isAudioEnabled,
    required this.onSwitch,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: () => onSwitch(!isAudioEnabled),
      child: Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: isAudioEnabled ? Colors.grey[800] : Colors.red[800],
          borderRadius: BorderRadius.circular(25),
        ),
        child: Icon(
          isAudioEnabled ? Icons.mic : Icons.mic_off,
          size: 28,
          color: Colors.white,
        ),
      ),
    );
  }
}

