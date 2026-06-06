import 'package:flutter/material.dart';

class ErrorRoom extends StatelessWidget {
  const ErrorRoom({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const Text('🙈', style: TextStyle(fontSize: 64)),
        Text('Something went wrong!', style: theme.textTheme.headlineSmall),
      ],
    );
  }
}
