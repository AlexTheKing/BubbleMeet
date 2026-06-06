import 'package:flutter/material.dart';
import 'package:ui/core/models/stream_with_settings.dart';
import 'package:ui/features/room/widgets/grid_companion_view.dart';
import 'package:ui/features/room/widgets/one_on_one_companion_view.dart';

class LoadedRoom extends StatelessWidget {
  final StreamWithSettings _localStream;
  final List<StreamWithSettings> _remoteStreams;

  const LoadedRoom({
    required this._localStream,
    required this._remoteStreams,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    print('we are here');
    return Column(
      children: [
        Expanded(
          child: _remoteStreams.length == 1
              ? OneOnOneCompanionView(
                  localStream: _localStream,
                  companionStream: _remoteStreams[0],
                )
              : GridCompanionView(
                  localStream: _localStream,
                  remoteStreams: _remoteStreams,
                ),
        ),
        // Padding(
        //   padding: const EdgeInsets.all(16.0),
        //   child: Row(
        //     mainAxisAlignment: MainAxisAlignment.center,
        //     children: [
        //       MicrophoneController(
        //         isAudioEnabled: _localStream!.settings.isAudioEnabled,
        //         onSwitch: (isAudioEnabled) => _toggleLocalStreamSettings(
        //           isAudioEnabled,
        //           _localStream!.settings.isVideoEnabled,
        //         ),
        //       ),
        //       const SizedBox(width: 8),
        //       VideoController(
        //         isVideoEnabled: _localStream!.settings.isVideoEnabled,
        //         onSwitch: (isVideoEnabled) => _toggleLocalStreamSettings(
        //           _localStream!.settings.isAudioEnabled,
        //           isVideoEnabled,
        //         ),
        //       ),
        //     ],
        //   ),
        // ),
      ],
    );
  }
}
