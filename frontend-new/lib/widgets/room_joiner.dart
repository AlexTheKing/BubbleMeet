import 'package:flutter/material.dart';

class RoomJoiner extends StatefulWidget {
  final Function(String) onJoinCallback;

  const RoomJoiner({
    super.key,
    required this.onJoinCallback,
  });

  @override
  State<RoomJoiner> createState() => _RoomJoinerState();
}

class _RoomJoinerState extends State<RoomJoiner> {
  final TextEditingController _roomIdController = TextEditingController();

  @override
  void dispose() {
    _roomIdController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const Text(
          'Enter room number to join:',
          style: TextStyle(
            color: Colors.white70,
            fontSize: 14,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 8),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            SizedBox(
              width: 300,
              child: TextField(
                controller: _roomIdController,
                decoration: const InputDecoration(
                  hintText: 'Room ID',
                  filled: true,
                  fillColor: Colors.white,
                  border: OutlineInputBorder(),
                ),
              ),
            ),
            const SizedBox(width: 8),
            ElevatedButton(
              onPressed: () {
                if (_roomIdController.text.isNotEmpty) {
                  widget.onJoinCallback(_roomIdController.text);
                }
              },
              child: const Text('Join'),
            ),
          ],
        ),
      ],
    );
  }
}

