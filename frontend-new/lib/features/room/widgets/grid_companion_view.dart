import 'package:flutter/material.dart';
import 'package:ui/core/models/stream_with_settings.dart';
import 'camera_view.dart';

class GridCompanionView extends StatelessWidget {
  final StreamWithSettings localStream;
  final List<StreamWithSettings> remoteStreams;

  const GridCompanionView({
    super.key,
    required this.localStream,
    required this.remoteStreams,
  });

  @override
  Widget build(BuildContext context) {
    if (remoteStreams.length > 9) {
      throw Exception(
        'GridCompanionView can only display up to 9 remote streams',
      );
    }

    final allStreams = [
      ...remoteStreams.map(
        (stream) => _StreamViewParams(
          stream: stream,
          streamKey: stream.mediaStream.id,
          isLocal: false,
        ),
      ),
      _StreamViewParams(stream: localStream, streamKey: 'local', isLocal: true),
    ];

    final totalVideos = allStreams.length;

    return LayoutBuilder(
      builder: (context, constraints) {
        return Padding(
          padding: const EdgeInsets.all(8.0),
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            alignment: WrapAlignment.center,
            children: allStreams.asMap().entries.map((entry) {
              final index = entry.key;
              final params = entry.value;
              return _buildGridItem(
                context,
                constraints,
                params,
                totalVideos,
                index,
              );
            }).toList(),
          ),
        );
      },
    );
  }

  Widget _buildGridItem(
    BuildContext context,
    BoxConstraints constraints,
    _StreamViewParams params,
    int totalVideos,
    int index,
  ) {
    double widthFactor;
    double heightFactor;

    if (totalVideos == 2) {
      widthFactor = 0.5;
      heightFactor = 1.0;
    } else if (totalVideos == 3 || totalVideos == 4) {
      widthFactor = 0.5;
      heightFactor = 0.5;
    } else if (totalVideos == 5) {
      widthFactor = index < 3 ? 1.0 / 3.0 : 0.5;
      heightFactor = 0.5;
    } else if (totalVideos == 6) {
      widthFactor = 0.5;
      heightFactor = 1.0 / 3.0;
    } else {
      widthFactor = 1.0 / 3.0;
      heightFactor = 1.0 / 3.0;
    }

    return SizedBox(
      width: (constraints.maxWidth - 24) * widthFactor,
      height: (constraints.maxHeight - 24) * heightFactor,
      child: CameraView(isLocal: params.isLocal, stream: params.stream),
    );
  }
}

class _StreamViewParams {
  final StreamWithSettings stream;
  final String streamKey;
  final bool isLocal;

  _StreamViewParams({
    required this.stream,
    required this.streamKey,
    required this.isLocal,
  });
}
