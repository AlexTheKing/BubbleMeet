import 'package:flutter_webrtc/flutter_webrtc.dart';

class StreamSettings {
  final bool isAudioEnabled;
  final bool isVideoEnabled;

  StreamSettings({
    required this.isAudioEnabled,
    required this.isVideoEnabled,
  });
}

class StreamWithSettings {
  final MediaStream mediaStream;
  final StreamSettings settings;

  StreamWithSettings({
    required this.mediaStream,
    required this.settings,
  });
}

