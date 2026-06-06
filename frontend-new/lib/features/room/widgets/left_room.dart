import 'package:flutter/material.dart';

class LeftRoom extends StatelessWidget {
  const LeftRoom({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const Text('🙈', style: TextStyle(fontSize: 64)),
        Text('Goodbye!', style: theme.textTheme.headlineSmall),
      ],
    );
  }
}
