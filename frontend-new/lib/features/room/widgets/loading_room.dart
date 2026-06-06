import 'package:flutter/material.dart';

class LoadingRoom extends StatelessWidget {
  final String roomId;

  const LoadingRoom({required this.roomId, super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const Text('🏃', style: TextStyle(fontSize: 64)),
        Text(
          'Connecting to the room $roomId',
          style: theme.textTheme.headlineSmall,
        ),
        const Padding(
          padding: EdgeInsets.all(16),
          child: CircularProgressIndicator(),
        ),
      ],
    );
  }
}
