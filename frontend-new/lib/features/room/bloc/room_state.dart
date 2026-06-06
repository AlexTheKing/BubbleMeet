part of 'room_bloc.dart';

enum RoomConnectionStatus { joining, joined, left, error }

class RoomState {
  final String roomId;
  final RoomConnectionStatus status;

  String? participantId;
  RTCPeerConnection? peerConnection;
  StreamWithSettings? localStream;
  List<StreamWithSettings> remoteStreams;

  RoomState({
    required this.roomId,
    required this.status,
    this.participantId,
    this.peerConnection,
    this.localStream,
    this.remoteStreams = const [],
  });

  factory RoomState.joining({required String roomId}) =>
      RoomState(roomId: roomId, status: RoomConnectionStatus.joining);

  RoomState copyWith({
    RoomConnectionStatus? status,
    String? participantId,
    RTCPeerConnection? peerConnection,
    StreamWithSettings? localStream,
    List<StreamWithSettings>? remoteStreams,
  }) {
    return RoomState(
      roomId: roomId,
      participantId: participantId ?? this.participantId,
      status: status ?? this.status,
      peerConnection: peerConnection ?? this.peerConnection,
      localStream: localStream ?? this.localStream,
      remoteStreams: remoteStreams ?? this.remoteStreams,
    );
  }
}
