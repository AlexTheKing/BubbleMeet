part of 'room_bloc.dart';

sealed class RoomEvent {
  const RoomEvent();
}

final class RoomJoining extends RoomEvent {}

final class RoomOfferReceived extends RoomEvent {
  final OfferSignalingMessage message;

  const RoomOfferReceived({required this.message});
}

final class RoomAnswerReceived extends RoomEvent {
  final AnswerSignalingMessage message;

  const RoomAnswerReceived({required this.message});
}

final class RoomICECandidateReceived extends RoomEvent {
  final ICECandidateSignalingMessage message;

  const RoomICECandidateReceived({required this.message});
}

final class RoomStreamControlReceived extends RoomEvent {
  final StreamControlSignalingMessage message;

  const RoomStreamControlReceived({required this.message});
}

final class RoomJoined extends RoomEvent {}

final class RoomLeft extends RoomEvent {}

final class RoomJoiningFailed extends RoomEvent {}

final class RoomNewTrackReceived extends RoomEvent {
  final RTCTrackEvent event;

  const RoomNewTrackReceived({required this.event});
}
