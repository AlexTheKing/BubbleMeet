import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:signaling_api/signaling_api.dart';
import 'package:signaling_repository/signaling_repository.dart';
import 'package:ui/features/room/bloc/room_bloc.dart';
import 'package:ui/features/room/view/room_page.dart';

class RoomJoinerPage extends StatefulWidget {
  const RoomJoinerPage({super.key});

  static Route<String> route() {
    return MaterialPageRoute(builder: (_) => const RoomJoinerPage());
  }

  @override
  State<StatefulWidget> createState() => _RoomJoinerState();
}

class _RoomJoinerState extends State<RoomJoinerPage> {
  final TextEditingController _textController = TextEditingController();

  String get _roomId => _textController.text;

  @override
  void dispose() {
    _textController.dispose();
    super.dispose();
  }

  String buildEndpoint(String roomId) {
    const host = String.fromEnvironment(
      'SIGNALING_SERVER_URL',
      defaultValue: 'localhost',
    );
    return 'wss://$host/api/v1/rooms/$roomId';
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Row(
        children: [
          Expanded(
            child: Padding(
              padding: const EdgeInsets.all(8),
              child: TextField(
                controller: _textController,
                decoration: const InputDecoration(
                  labelText: 'Room ID',
                  hintText: 'Room ID',
                ),
              ),
            ),
          ),
          IconButton(
            key: const Key('roomJoinerPage_input_iconButton'),
            icon: const Icon(Icons.search, semanticLabel: 'Join'),
            onPressed: () => Navigator.of(context).push(
              MaterialPageRoute(
                builder: (context) => RepositoryProvider(
                  create: (context) => SignalingRepository(
                    signalingApi: SignalingApi.fromString(
                      buildEndpoint(_roomId),
                    ),
                  ),
                  dispose: (repository) => repository.dispose(),
                  child: BlocProvider(
                    create: (context) => RoomBloc(
                      signalingRepository: context.read<SignalingRepository>(),
                      roomId: _roomId,
                    )..add(RoomJoining()),
                    child: RoomPage(),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
