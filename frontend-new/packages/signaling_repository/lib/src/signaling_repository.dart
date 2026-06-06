import 'dart:async';

import 'package:signaling_api/signaling_api.dart';

class SignalingRepository {
  final SignalingApi _signalingApi;

  SignalingRepository({required this._signalingApi});

  Stream<SignalingMessage> get messages => _signalingApi.stream;

  void connect() {
    _signalingApi.connect();
  }

  void sendOffer(String participantId, RTCSessionDescription offer) {
    _signalingApi.send(
      OfferSignalingMessage(participantId: participantId, description: offer),
    );
  }

  void sendAnswer(String participantId, RTCSessionDescription answer) {
    _signalingApi.send(
      AnswerSignalingMessage(participantId: participantId, description: answer),
    );
  }

  void sendICECandidate(String participantId, RTCIceCandidate candidate) {
    _signalingApi.send(
      ICECandidateSignalingMessage(
        participantId: participantId,
        candidate: candidate,
      ),
    );
  }

  void sendStreamControl(
    String participantId,
    String streamId,
    bool isAudioEnabled,
    bool isVideoEnabled,
  ) {
    _signalingApi.send(
      StreamControlSignalingMessage(
        participantId: participantId,
        streamId: streamId,
        isAudioEnabled: isAudioEnabled,
        isVideoEnabled: isVideoEnabled,
      ),
    );
  }

  void dispose() {
    _signalingApi.close();
  }
}
