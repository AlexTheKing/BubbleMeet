// import 'package:flutter/material.dart';
// import 'package:flutter_webrtc/flutter_webrtc.dart';
// import 'package:uuid/uuid.dart';
// import 'package:web_socket_channel/web_socket_channel.dart';
// import 'dart:convert';

// import 'core/models/stream_with_settings.dart';
// import 'models/signaling_message.dart';
// import 'widgets/room_joiner.dart';
// import 'widgets/one_on_one_companion_view.dart';
// import 'widgets/grid_companion_view.dart';
// import 'widgets/microphone_controller.dart';
// import 'widgets/video_controller.dart';
// import 'dart:io';

// const uuid = Uuid();

// String getSignalingServerHost() {
//   return Platform.environment['SIGNALING_SERVER_URL'] ?? 'localhost';
// }

// String getTurnServerHost() {
//   return Platform.environment['TURN_SERVER_URL'] ?? 'localhost';
// }

// String getRoomUrl(String roomId) {
//   final host = getSignalingServerHost();
//   return 'wss://$host/api/v1/rooms/$roomId';
// }

// void main() {
//   WidgetsFlutterBinding.ensureInitialized();
//   runApp(const App());
// }

// class App extends StatelessWidget {
//   const App({super.key});

//   @override
//   Widget build(BuildContext context) {
//     return MaterialApp(
//       title: 'Bubble Meet',
//       theme: ThemeData(
//         colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
//         useMaterial3: true,
//       ),
//       home: const VideoChatPage(),
//     );
//   }
// }

// class VideoChatPage extends StatefulWidget {
//   const VideoChatPage({super.key});

//   @override
//   State<VideoChatPage> createState() => _VideoChatPageState();
// }

// class _VideoChatPageState extends State<VideoChatPage> {
//   final String _userId = uuid.v4();
//   bool _isRoomJoinerShown = true;
//   WebSocketChannel? _socket;
//   StreamWithSettings? _localStream;
//   List<StreamWithSettings> _remoteStreams = [];
//   RTCPeerConnection? _peerConnection;

//   @override
//   void initState() {
//     super.initState();
//     print('User ID: $_userId');
//   }

//   @override
//   void dispose() {
//     _cleanup();
//     super.dispose();
//   }

//   void _cleanup() {
//     _socket?.sink.close();
//     _peerConnection?.close();
//     _localStream?.mediaStream.dispose();
//     for (var stream in _remoteStreams) {
//       stream.mediaStream.dispose();
//     }
//   }

//   Future<RTCPeerConnection> _createPeerConnection(
//     MediaStream localStream,
//     WebSocketChannel socket,
//   ) async {
//     final configuration = {
//       'iceServers': [
//         {
//           'urls': ['stun:${getTurnServerHost()}:3478'],
//         },
//       ],
//     };

//     final peerConnection = await createPeerConnection(configuration, {});

//     // Add local tracks
//     localStream.getTracks().forEach((track) {
//       peerConnection.addTrack(track, localStream);
//     });

//     // Handle incoming tracks
//     peerConnection.onTrack = (RTCTrackEvent event) {
//       print('Received new track ${event.track.id}');
//       if (event.streams.isNotEmpty) {
//         final mediaStream = event.streams[0];

//         // Check if audio/video are enabled
//         final audioTracks = mediaStream.getAudioTracks();
//         final videoTracks = mediaStream.getVideoTracks();
//         final isAudioEnabled =
//             audioTracks.isNotEmpty && audioTracks.any((track) => track.enabled);
//         final isVideoEnabled =
//             videoTracks.isNotEmpty && videoTracks.any((track) => track.enabled);

//         setState(() {
//           // Remove existing stream with same ID if any
//           _remoteStreams.removeWhere((s) => s.mediaStream.id == mediaStream.id);

//           // Add new stream
//           _remoteStreams.add(
//             StreamWithSettings(
//               mediaStream: mediaStream,
//               settings: StreamSettings(
//                 isAudioEnabled: isAudioEnabled,
//                 isVideoEnabled: isVideoEnabled,
//               ),
//             ),
//           );
//         });

//         // Handle track removal - monitor stream tracks
//         _monitorStreamForRemoval(mediaStream);
//       }
//     };

//     // Create initial offer after adding tracks
//     // This handles the negotiation needed scenario
//     Future.microtask(() async {
//       try {
//         final offer = await peerConnection.createOffer();
//         await peerConnection.setLocalDescription(offer);

//         final message = OfferSignalingMessage(
//           userId: _userId,
//           description: offer,
//         );
//         socket.sink.add(jsonEncode(message.toJson()));
//         print('Sent initial offer');
//       } catch (e) {
//         print('Error creating initial offer: $e');
//       }
//     });

//     // Handle ICE candidates
//     peerConnection.onIceCandidate = (RTCIceCandidate candidate) {
//       if (candidate.candidate != null) {
//         final message = ICECandidateSignalingMessage(
//           userId: _userId,
//           candidate: candidate,
//         );
//         socket.sink.add(jsonEncode(message.toJson()));
//       }
//     };

//     // Handle ICE connection state changes
//     peerConnection.onIceConnectionState = (RTCIceConnectionState state) {
//       print('ICE connection state: $state');
//       if (state == RTCIceConnectionState.RTCIceConnectionStateFailed) {
//         peerConnection.restartIce();
//       }
//     };

//     return peerConnection;
//   }

//   void _monitorStreamForRemoval(MediaStream mediaStream) {
//     // Periodically check if stream has no tracks
//     Future.delayed(const Duration(seconds: 1), () {
//       if (mediaStream.getTracks().isEmpty) {
//         setState(() {
//           _remoteStreams.removeWhere((s) => s.mediaStream.id == mediaStream.id);
//         });
//         print('Stream ${mediaStream.id} was removed');
//       } else {
//         _monitorStreamForRemoval(mediaStream);
//       }
//     });
//   }

//   Future<void> _handleSignalingMessage(
//     SignalingMessage message,
//     WebSocketChannel socket,
//   ) async {
//     if (_peerConnection == null) {
//       print('Peer connection not created yet, skipping message');
//       return;
//     }

//     try {
//       switch (message.type) {
//         case SignalingMessageType.answer:
//           print('Received answer');
//           final answerMessage = message as AnswerSignalingMessage;
//           await _peerConnection!.setRemoteDescription(
//             RTCSessionDescription(
//               answerMessage.description.sdp ?? '',
//               answerMessage.description.type ?? 'answer',
//             ),
//           );
//           break;

//         case SignalingMessageType.offer:
//           print('Received offer');
//           final offerMessage = message as OfferSignalingMessage;
//           await _peerConnection!.setRemoteDescription(
//             RTCSessionDescription(
//               offerMessage.description.sdp ?? '',
//               offerMessage.description.type ?? 'offer',
//             ),
//           );

//           final answer = await _peerConnection!.createAnswer();
//           await _peerConnection!.setLocalDescription(answer);

//           final signalingMessage = AnswerSignalingMessage(
//             userId: _userId,
//             description: answer,
//           );
//           socket.sink.add(jsonEncode(signalingMessage.toJson()));
//           print('Sent answer');
//           break;

//         case SignalingMessageType.iceCandidate:
//           print('Received ICECandidate');
//           final iceMessage = message as ICECandidateSignalingMessage;
//           await _peerConnection!.addCandidate(iceMessage.candidate);
//           break;

//         case SignalingMessageType.streamControl:
//           print('Received StreamControl');
//           final streamControlMessage = message as StreamControlSignalingMessage;
//           setState(() {
//             _remoteStreams = _remoteStreams.map((stream) {
//               if (stream.mediaStream.id == streamControlMessage.streamId) {
//                 return StreamWithSettings(
//                   mediaStream: stream.mediaStream,
//                   settings: StreamSettings(
//                     isAudioEnabled: streamControlMessage.isAudioEnabled,
//                     isVideoEnabled: streamControlMessage.isVideoEnabled,
//                   ),
//                 );
//               }
//               return stream;
//             }).toList();
//           });
//           break;

//         case SignalingMessageType.ping:
//           print('Received ping');
//           break;
//       }
//     } catch (e) {
//       print('Error handling signaling message: $e');
//     }
//   }

//   Future<void> _onJoinCallback(String roomId) async {
//     try {
//       // Get user media
//       final localMediaStream = await navigator.mediaDevices.getUserMedia({
//         'audio': true,
//         'video': true,
//       });

//       setState(() {
//         _localStream = StreamWithSettings(
//           mediaStream: localMediaStream,
//           settings: StreamSettings(isAudioEnabled: true, isVideoEnabled: true),
//         );
//       });

//       // Create WebSocket connection
//       final socket = WebSocketChannel.connect(Uri.parse(getRoomUrl(roomId)));

//       socket.stream.listen(
//         (message) {
//           try {
//             final json = jsonDecode(message as String) as Map<String, dynamic>;
//             final signalingMessage = parseSignalingMessage(json);
//             _handleSignalingMessage(signalingMessage, socket);
//           } catch (e) {
//             print('Error parsing message: $e');
//           }
//         },
//         onError: (error) {
//           print('WebSocket error: $error');
//         },
//         onDone: () {
//           print('WebSocket closed');
//           if (_peerConnection != null) {
//             _peerConnection!.close();
//             setState(() {
//               _peerConnection = null;
//             });
//           }
//         },
//       );

//       // Wait for socket to be ready
//       await Future.delayed(const Duration(milliseconds: 500));

//       // Create peer connection
//       final peerConnection = await _createPeerConnection(
//         localMediaStream,
//         socket,
//       );

//       setState(() {
//         _socket = socket;
//         _peerConnection = peerConnection;
//         _isRoomJoinerShown = false;
//       });

//       print('Peer connection created, socket opened');
//     } catch (e) {
//       print('Error joining room: $e');
//       if (mounted) {
//         ScaffoldMessenger.of(
//           context,
//         ).showSnackBar(SnackBar(content: Text('Error joining room: $e')));
//       }
//     }
//   }

//   void _toggleLocalStreamSettings(bool isAudioEnabled, bool isVideoEnabled) {
//     if (_localStream != null && _socket != null) {
//       // Update track states
//       _localStream!.mediaStream.getAudioTracks().forEach((track) {
//         track.enabled = isAudioEnabled;
//       });
//       _localStream!.mediaStream.getVideoTracks().forEach((track) {
//         track.enabled = isVideoEnabled;
//       });

//       // Send stream control message
//       final message = StreamControlSignalingMessage(
//         userId: _userId,
//         streamId: _localStream!.mediaStream.id,
//         isAudioEnabled: isAudioEnabled,
//         isVideoEnabled: isVideoEnabled,
//       );
//       _socket!.sink.add(jsonEncode(message.toJson()));

//       // Update local state
//       setState(() {
//         _localStream = StreamWithSettings(
//           mediaStream: _localStream!.mediaStream,
//           settings: StreamSettings(
//             isAudioEnabled: isAudioEnabled,
//             isVideoEnabled: isVideoEnabled,
//           ),
//         );
//       });
//     }
//   }

//   @override
//   Widget build(BuildContext context) {
//     return Scaffold(
//       backgroundColor: Colors.grey[900],
//       body: _isRoomJoinerShown
//           ? Center(child: RoomJoiner(onJoinCallback: _onJoinCallback))
//           : Column(
//               children: [
//                 Expanded(
//                   child: _remoteStreams.length == 1
//                       ? OneOnOneCompanionView(
//                           localStream: _localStream!,
//                           companionStream: _remoteStreams[0],
//                         )
//                       : GridCompanionView(
//                           localStream: _localStream!,
//                           remoteStreams: _remoteStreams,
//                         ),
//                 ),
//                 Padding(
//                   padding: const EdgeInsets.all(16.0),
//                   child: Row(
//                     mainAxisAlignment: MainAxisAlignment.center,
//                     children: [
//                       MicrophoneController(
//                         isAudioEnabled: _localStream!.settings.isAudioEnabled,
//                         onSwitch: (isAudioEnabled) =>
//                             _toggleLocalStreamSettings(
//                               isAudioEnabled,
//                               _localStream!.settings.isVideoEnabled,
//                             ),
//                       ),
//                       const SizedBox(width: 8),
//                       VideoController(
//                         isVideoEnabled: _localStream!.settings.isVideoEnabled,
//                         onSwitch: (isVideoEnabled) =>
//                             _toggleLocalStreamSettings(
//                               _localStream!.settings.isAudioEnabled,
//                               isVideoEnabled,
//                             ),
//                       ),
//                     ],
//                   ),
//                 ),
//               ],
//             ),
//     );
//   }
// }
