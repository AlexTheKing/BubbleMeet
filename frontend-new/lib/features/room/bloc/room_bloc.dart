import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';
import 'package:signaling_api/signaling_api.dart';
import 'package:signaling_repository/signaling_repository.dart';
import 'package:ui/core/models/stream_with_settings.dart';
import 'package:uuid/uuid.dart';

part 'room_event.dart';
part 'room_state.dart';

class RoomBloc extends Bloc<RoomEvent, RoomState> {
  final SignalingRepository _signalingRepository;

  StreamSubscription? _streamSubscription;

  RoomBloc({required this._signalingRepository, required String roomId})
    : super(RoomState.joining(roomId: roomId)) {
    on<RoomJoining>(_onRoomJoining);
    on<RoomOfferReceived>(_onRoomOfferReceived);
    on<RoomAnswerReceived>(_onRoomAnswerReceived);
    on<RoomICECandidateReceived>(_onRoomICECandidateReceived);
    on<RoomNewTrackReceived>(_onRoomTrackReceived);
    on<RoomStreamControlReceived>(_onRoomStreamControlReceived);
    on<RoomJoined>(_onRoomJoined);
    on<RoomLeft>(_onRoomLeft);
    on<RoomJoiningFailed>(_onRoomJoiningFailed);
  }

  Future<void> _onRoomJoining(
    RoomJoining event,
    Emitter<RoomState> emit,
  ) async {
    try {
      print('Joining room ${state.roomId}');
      final String participantId = Uuid().v4();
      _signalingRepository.connect();
      _streamSubscription = _signalingRepository.messages.listen((message) {
        switch (message.type) {
          case SignalingMessageType.offer:
            add(RoomOfferReceived(message: message as OfferSignalingMessage));
            break;
          case SignalingMessageType.answer:
            add(RoomAnswerReceived(message: message as AnswerSignalingMessage));
            break;
          case SignalingMessageType.iceCandidate:
            add(
              RoomICECandidateReceived(
                message: message as ICECandidateSignalingMessage,
              ),
            );
            break;
          case SignalingMessageType.streamControl:
            add(
              RoomStreamControlReceived(
                message: message as StreamControlSignalingMessage,
              ),
            );
            break;
          case SignalingMessageType.ping:
            print(message);
            break;
        }
      });

      final peerConnection = await setupPeerConnection();
      final localStream = await setupLocalStream(peerConnection);

      peerConnection.onTrack = (RTCTrackEvent event) =>
          add(RoomNewTrackReceived(event: event));

      peerConnection.onIceCandidate = (RTCIceCandidate candidate) {
        if (candidate.candidate != null) {
          _signalingRepository.sendICECandidate(participantId, candidate);
        }
      };

      peerConnection.onIceConnectionState = (RTCIceConnectionState state) {
        if (state == RTCIceConnectionState.RTCIceConnectionStateFailed) {
          peerConnection.restartIce();
        }
      };

      peerConnection.onConnectionState = (RTCPeerConnectionState state) => {
        switch (state) {
          RTCPeerConnectionState.RTCPeerConnectionStateConnected => add(
            RoomJoined(),
          ),
          RTCPeerConnectionState.RTCPeerConnectionStateFailed => add(
            RoomJoiningFailed(),
          ),
          _ => {},
        },
      };

      final offer = await peerConnection.createOffer();
      await peerConnection.setLocalDescription(offer);
      _signalingRepository.sendOffer(participantId, offer);
      print('Sent offer');

      emit(
        state.copyWith(
          participantId: participantId,
          peerConnection: peerConnection,
          localStream: localStream,
        ),
      );
    } catch (_) {
      add(RoomJoiningFailed());
    }
  }

  Future<RTCPeerConnection> setupPeerConnection() async {
    const turn = String.fromEnvironment(
      'TURN_SERVER_URL',
      defaultValue: 'localhost',
    );
    final configuration = {
      'iceServers': [
        {
          'urls': ['stun:$turn:3478'],
        },
      ],
    };

    return await createPeerConnection(configuration, {});
  }

  Future<StreamWithSettings> setupLocalStream(
    RTCPeerConnection peerConnection,
  ) async {
    final isAudioEnabled = true;
    final isVideoEnabled = true;
    final localMediaStream = await navigator.mediaDevices.getUserMedia({
      'audio': isAudioEnabled,
      'video': isVideoEnabled,
    });
    localMediaStream.getTracks().forEach((track) {
      peerConnection.addTrack(track, localMediaStream);
    });
    return StreamWithSettings(
      mediaStream: localMediaStream,
      settings: StreamSettings(
        isAudioEnabled: isAudioEnabled,
        isVideoEnabled: isVideoEnabled,
      ),
    );
  }

  // Handle incoming tracks
  // peerConnection.onTrack = (RTCTrackEvent event) {
  //   print('Received new track ${event.track.id}');
  //   if (event.streams.isNotEmpty) {
  //     final mediaStream = event.streams[0];

  //     final audioTracks = mediaStream.getAudioTracks();
  //     final videoTracks = mediaStream.getVideoTracks();
  //     final isAudioEnabled =
  //         audioTracks.isNotEmpty && audioTracks.any((track) => track.enabled);
  //     final isVideoEnabled =
  //         videoTracks.isNotEmpty && videoTracks.any((track) => track.enabled);

  // setState(() {
  //   // Remove existing stream with same ID if any
  //   _remoteStreams.removeWhere((s) => s.mediaStream.id == mediaStream.id);

  //   // Add new stream
  //   _remoteStreams.add(
  //     StreamWithSettings(
  //       mediaStream: mediaStream,
  //       settings: StreamSettings(
  //         isAudioEnabled: isAudioEnabled,
  //         isVideoEnabled: isVideoEnabled,
  //       ),
  //     ),
  //   );
  // });

  // Handle track removal - monitor stream tracks
  // _monitorStreamForRemoval(mediaStream);
  //   }
  // };

  Future<void> _onRoomOfferReceived(
    RoomOfferReceived event,
    Emitter<RoomState> emit,
  ) async {
    try {
      print('Received offer');
      await state.peerConnection!.setRemoteDescription(
        RTCSessionDescription(
          event.message.description.sdp,
          event.message.description.type,
        ),
      );

      final answer = await state.peerConnection!.createAnswer();
      await state.peerConnection!.setLocalDescription(answer);

      _signalingRepository.sendAnswer(state.participantId!, answer);
      print('Sent answer');
    } catch (_) {
      add(RoomJoiningFailed());
    }
  }

  Future<void> _onRoomAnswerReceived(
    RoomAnswerReceived event,
    Emitter<RoomState> emit,
  ) async {
    try {
      print('Received answer');
      await state.peerConnection!.setRemoteDescription(
        RTCSessionDescription(
          event.message.description.sdp,
          event.message.description.type,
        ),
      );
    } catch (_) {
      add(RoomJoiningFailed());
    }
  }

  Future<void> _onRoomICECandidateReceived(
    RoomICECandidateReceived event,
    Emitter<RoomState> emit,
  ) async {
    print('Received ICECandidate');
    await state.peerConnection!.addCandidate(event.message.candidate);
  }

  Future<void> _onRoomTrackReceived(
    RoomNewTrackReceived event,
    Emitter<RoomState> emit,
  ) async {
    print('Received new track ${event.event.track.id}');
    if (event.event.streams.isNotEmpty) {
      final mediaStream = event.event.streams[0];

      final audioTracks = mediaStream.getAudioTracks();
      final videoTracks = mediaStream.getVideoTracks();
      final isAudioEnabled =
          audioTracks.isNotEmpty && audioTracks.any((track) => track.enabled);
      final isVideoEnabled =
          videoTracks.isNotEmpty && videoTracks.any((track) => track.enabled);

      final updatedRemoteStreams = List.of(state.remoteStreams);
      updatedRemoteStreams.removeWhere(
        (s) => s.mediaStream.id == mediaStream.id,
      );
      updatedRemoteStreams.add(
        StreamWithSettings(
          mediaStream: mediaStream,
          settings: StreamSettings(
            isAudioEnabled: isAudioEnabled,
            isVideoEnabled: isVideoEnabled,
          ),
        ),
      );

      emit(state.copyWith(remoteStreams: updatedRemoteStreams));

      // _monitorStreamForRemoval(mediaStream);
    }
  }

  Future<void> _onRoomStreamControlReceived(
    RoomStreamControlReceived event,
    Emitter<RoomState> emit,
  ) async {}

  Future<void> _onRoomJoined(RoomJoined event, Emitter<RoomState> emit) async {
    print('Successfully joined room ${state.roomId}');
    emit(state.copyWith(status: RoomConnectionStatus.joined));
  }

  Future<void> _onRoomLeft(RoomLeft event, Emitter<RoomState> emit) async {
    emit(state.copyWith(status: RoomConnectionStatus.left));
    await cleanup();
  }

  Future<void> _onRoomJoiningFailed(
    RoomJoiningFailed event,
    Emitter<RoomState> emit,
  ) async {
    emit(state.copyWith(status: RoomConnectionStatus.error));
    await cleanup();
  }

  Future<void> cleanup() async {
    try {
      await state.peerConnection!.close();
    } catch (error) {
      print('During cleanup error occurred: $error');
    }
  }

  @override
  Future<void> close() {
    _streamSubscription?.cancel();
    return super.close();
  }
}
