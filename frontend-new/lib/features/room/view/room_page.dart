import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:ui/features/room/bloc/room_bloc.dart';
import 'package:ui/features/room/widgets/widgets.dart';

class RoomPage extends StatelessWidget {
  const RoomPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: BlocBuilder<RoomBloc, RoomState>(
          builder: (context, state) {
            return switch (state.status) {
              RoomConnectionStatus.joining => LoadingRoom(roomId: state.roomId),
              RoomConnectionStatus.joined => LoadedRoom(
                localStream: state.localStream!,
                remoteStreams: state.remoteStreams,
              ),
              RoomConnectionStatus.left => LeftRoom(),
              RoomConnectionStatus.error => ErrorRoom(),
            };
          },
        ),
      ),
    );
  }
}
