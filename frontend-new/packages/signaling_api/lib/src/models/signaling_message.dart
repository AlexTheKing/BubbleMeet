import 'package:signaling_api/signaling_api.dart';

abstract class SignalingMessage {
  final SignalingMessageType type;
  SignalingMessage(this.type);

  JsonMap toJson();
}

class OfferSignalingMessage extends SignalingMessage {
  final String participantId;
  final RTCSessionDescription description;

  OfferSignalingMessage({
    required this.participantId,
    required this.description,
  }) : super(SignalingMessageType.offer);

  @override
  JsonMap toJson() => {
    'type': type.value,
    'participant_id': participantId,
    'description': description.toMap(),
  };
}

class AnswerSignalingMessage extends SignalingMessage {
  final String participantId;
  final RTCSessionDescription description;

  AnswerSignalingMessage({
    required this.participantId,
    required this.description,
  }) : super(SignalingMessageType.answer);

  @override
  JsonMap toJson() => {
    'type': type.value,
    'participant_id': participantId,
    'description': description.toMap(),
  };
}

class ICECandidateSignalingMessage extends SignalingMessage {
  final String participantId;
  final RTCIceCandidate candidate;

  ICECandidateSignalingMessage({
    required this.participantId,
    required this.candidate,
  }) : super(SignalingMessageType.iceCandidate);

  @override
  JsonMap toJson() => {
    'type': type.value,
    'participant_id': participantId,
    'candidate': candidate.toMap(),
  };
}

class StreamControlSignalingMessage extends SignalingMessage {
  final String participantId;
  final String streamId;
  final bool isAudioEnabled;
  final bool isVideoEnabled;

  StreamControlSignalingMessage({
    required this.participantId,
    required this.streamId,
    required this.isAudioEnabled,
    required this.isVideoEnabled,
  }) : super(SignalingMessageType.streamControl);

  @override
  JsonMap toJson() => {
    'type': type.value,
    'participant_id': participantId,
    'stream_id': streamId,
    'is_audio_enabled': isAudioEnabled,
    'is_video_enabled': isVideoEnabled,
  };
}

class PingSignalingMessage extends SignalingMessage {
  PingSignalingMessage() : super(SignalingMessageType.ping);

  @override
  JsonMap toJson() => {'type': type.value};
}
