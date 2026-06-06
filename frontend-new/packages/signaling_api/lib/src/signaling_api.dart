import 'dart:async';
import 'dart:convert';

import 'package:signaling_api/signaling_api.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

class SignalingApi {
  Uri endpoint;
  WebSocketChannel? _channel;

  final StreamController<SignalingMessage> _controller =
      StreamController.broadcast();

  Stream<SignalingMessage> get stream => _controller.stream;

  SignalingApi(this.endpoint);

  factory SignalingApi.fromString(String endpoint) {
    return SignalingApi(Uri.parse(endpoint));
  }

  void connect() {
    _channel = WebSocketChannel.connect(endpoint);
    _channel!.stream.listen(
      (message) =>
          _controller.add(parseSignalingMessage(jsonDecode(message as String))),
      onError: (e) => _controller.addError(e),
      onDone: () => _controller.close(),
    );
  }

  void send(SignalingMessage message) =>
      _channel!.sink.add(jsonEncode(message.toJson()));
  Future<void> close() => _channel!.sink.close();

  SignalingMessage parseSignalingMessage(JsonMap json) {
    final type = SignalingMessageType.fromValue(json['type'] as String);
    final participantId = json['participant_id'] as String;
    switch (type) {
      case SignalingMessageType.offer:
        return OfferSignalingMessage(
          participantId: participantId,
          description: RTCSessionDescription(
            json['description']['sdp'] as String,
            json['description']['type'] as String,
          ),
        );
      case SignalingMessageType.answer:
        return AnswerSignalingMessage(
          participantId: participantId,
          description: RTCSessionDescription(
            json['description']['sdp'] as String,
            json['description']['type'] as String,
          ),
        );
      case SignalingMessageType.iceCandidate:
        return ICECandidateSignalingMessage(
          participantId: participantId,
          candidate: RTCIceCandidate(
            json['candidate']['candidate'] as String,
            json['candidate']['sdpMid'] as String?,
            json['candidate']['sdpMLineIndex'] as int?,
          ),
        );
      case SignalingMessageType.streamControl:
        return StreamControlSignalingMessage(
          participantId: participantId,
          streamId: json['stream_id'] as String,
          isAudioEnabled: json['is_audio_enabled'] as bool,
          isVideoEnabled: json['is_video_enabled'] as bool,
        );
      case SignalingMessageType.ping:
        return PingSignalingMessage();
      default:
        throw Exception('Unhandled message type: $type');
    }
  }
}
