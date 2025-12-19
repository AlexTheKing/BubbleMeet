'use client';

import RoomJoiner from "@/app/components/RoomJoiner";
import assert from "assert";
import {useEffect, useState} from "react";
import {v4 as uuidv4} from "uuid";
import OneOnOneCompanionView from "./components/OneOnOneCompanionView";
import GridCompanionView from "./components/GridCompanionView";
import MicrophoneController from "./components/controls/MicrophoneController";
import VideoController from "./components/controls/VideoController";
import { StreamWithSettings } from "./types";

function getSignalingServerHost() {
    return (
        process.env.NEXT_PUBLIC_SIGNALING_SERVER_URL ?
            process.env.NEXT_PUBLIC_SIGNALING_SERVER_URL :
            document.location.host
    );
}

function getTurnServerHost() {
    // return (
    //     process.env.NEXT_PUBLIC_TURN_SERVER_URL ?
    //         process.env.NEXT_PUBLIC_TURN_SERVER_URL :
    //         document.location.host
    // );
    // return "192.168.100.146"
    return "localhost"
}

function getRoomUrl(roomId: string) {
    return `wss://${getSignalingServerHost()}/api/v1/rooms/${roomId}`
}

enum MessageType {
    OFFER = "Offer",
    ANSWER = "Answer",
    ICE_CANDIDATE = "ICECandidate",
    STREAM_CONTROL = "StreamControl",
    PING = "Ping",
}

interface AbstractSignalingMessage {
    type: MessageType
}

interface OfferSignalingMessage extends AbstractSignalingMessage {
    type: MessageType.OFFER,
    participant_id: string,
    description: RTCSessionDescription
}

interface AnswerSignalingMessage extends AbstractSignalingMessage {
    type: MessageType.ANSWER,
    participant_id: string,
    description: RTCSessionDescription
}

interface ICECandidateSignalingMessage extends AbstractSignalingMessage {
    type: MessageType.ICE_CANDIDATE,
    participant_id: string,
    candidate: RTCIceCandidateInit
}

interface StreamControlSignalingMessage extends AbstractSignalingMessage {
    type: MessageType.STREAM_CONTROL,
    participant_id: string,
    stream_id: string,
    is_audio_enabled: boolean,
    is_video_enabled: boolean
}

interface PingSignalingMessage extends AbstractSignalingMessage {
    type: MessageType.PING,
}

type SignalingMessage = 
    | OfferSignalingMessage
    | AnswerSignalingMessage
    | ICECandidateSignalingMessage
    | StreamControlSignalingMessage
    | PingSignalingMessage;

export default function App() {
    const [userId, setUserId] = useState(uuidv4());

    useEffect(() => console.log('User ID:', userId), [userId]);

    const [isRoomJoinerShown, setRoomJoinerShown] = useState(true);
    const [socket, setSocket] = useState<WebSocket | null>(null);
    const [localStream, setLocalStream] = useState<StreamWithSettings | null>(null);
    const [remoteStreams, setRemoteStreams] = useState<StreamWithSettings[]>([]);

    let peerConnection: RTCPeerConnection | null = null;

    function createPeerConnection(localStream: MediaStream, socket: WebSocket) {
        const peerConnection = new RTCPeerConnection({
            iceServers: [
                {
                    urls: [
                        // 'stun:stun.l.google.com:19302',
                        `stun:${getTurnServerHost()}:3478`
                    ]
                }
            ]
        });
        localStream.getTracks().forEach(
            track => peerConnection.addTrack(track, localStream)
        );
        peerConnection.ontrack = ({track, streams: mediaStreams}) => {
            console.log(`Received new track ${track.id}`)
            setRemoteStreams((previousStreams) => [
                ...mediaStreams.map(mediaStream => ({
                    mediaStream: mediaStream,
                    settings: {
                        isAudioEnabled: mediaStream.getAudioTracks().some(track => track.enabled),
                        isVideoEnabled: mediaStream.getVideoTracks().some(track => track.enabled)
                    }
                })),
                ...previousStreams.filter(previousStream => 
                    !mediaStreams.some(mediaStream => mediaStream.id === previousStream.mediaStream.id)
                )
            ]);
            assert(mediaStreams.length <= 1, 'Multiple streams received');
            let mediaStream = mediaStreams[0];
            mediaStream.onremovetrack = ({track}) => {
                console.log(`Track ${track.id} was removed`);
                if (!mediaStream.getTracks().length) {
                  setRemoteStreams((previousStreams) => previousStreams.filter(previousStream => previousStream.mediaStream.id !== mediaStream.id));
                  console.log(`Stream ${mediaStream.id} was removed`);
                }
            };
        };
        peerConnection.onnegotiationneeded = async () => {
            await peerConnection.setLocalDescription();
            const message: OfferSignalingMessage = {
                type: MessageType.OFFER,
                participant_id: userId,
                description: peerConnection?.localDescription!
            };
            socket.send(JSON.stringify(message));
            console.log('Sent offer');
        };
        peerConnection.onicecandidate = ({candidate}) => {
            if (candidate !== null) {
                const message: ICECandidateSignalingMessage = {
                    type: MessageType.ICE_CANDIDATE,
                    candidate: candidate.toJSON(),
                    participant_id: userId
                };
                socket.send(JSON.stringify(message));
            }
        }
        peerConnection.oniceconnectionstatechange = () => {
            console.log('ICE connection state:', peerConnection.iceConnectionState);
            if (peerConnection.iceConnectionState === 'failed' || peerConnection.iceConnectionState === 'disconnected') {
                peerConnection.restartIce();
            }
        };
        
        // Monitor connection state
        peerConnection.onconnectionstatechange = () => {
            console.log('Peer connection state:', peerConnection.connectionState);
            if (peerConnection.connectionState === 'failed') {
                console.error('Peer connection failed, attempting to recover');
                peerConnection.restartIce();
            }
        };
        
        return peerConnection;
    }

    async function handleSignalingMessage(message: SignalingMessage, socket: WebSocket) {
        if (peerConnection === null) {
            console.warn('Peer connection not created yet, skipping message');
            return;
        }
        switch (message.type) {
            case MessageType.ANSWER:
                console.log('Received answer')
                await peerConnection.setRemoteDescription(
                    new RTCSessionDescription(message.description)
                );
                break;
            case MessageType.OFFER:
                console.log('Received offer')
                await peerConnection.setRemoteDescription(
                    new RTCSessionDescription(message.description)
                );
                let answer = await peerConnection.createAnswer();
                await peerConnection.setLocalDescription(answer);
                const signaling_message: AnswerSignalingMessage = {
                    type: MessageType.ANSWER,
                    participant_id: userId,
                    description: peerConnection.localDescription!
                };
                socket.send(JSON.stringify(signaling_message));
                console.log('Sent answer');
                break;
            case MessageType.ICE_CANDIDATE:
                console.log('Received ICECandidate')
                await peerConnection.addIceCandidate(
                    new RTCIceCandidate(message.candidate)
                );
                break;
            case MessageType.STREAM_CONTROL:
                console.log('Received StreamControl')
                setRemoteStreams((previousStreams) => previousStreams.map(stream => {
                    console.log('Updating stream settings stream_id: ', message.stream_id, 'stream.mediaStream.id: ', stream.mediaStream.id)
                    if (stream.mediaStream.id === message.stream_id) {
                        return {
                            mediaStream: stream.mediaStream,
                            settings: {
                                isAudioEnabled: message.is_audio_enabled,
                                isVideoEnabled: message.is_video_enabled
                            }
                        }
                    }
                    return stream;
                }));
                break;
            case MessageType.PING:
                console.log('Received ping')
                break;
            default:
                console.warn('Skipping unknown message:', message);
                break;
        }
    }

    function onJoinCallback(roomId: string) {
        navigator.mediaDevices
            .getUserMedia({video: true, audio: true})
            .then((localStream) => {
                // Prevent Chrome from pausing tracks when tab goes to background
                localStream.getAudioTracks().forEach(track => {
                    track.contentHint = 'speech'; // Helps keep audio active
                    // Prevent track from being stopped by browser
                    track.addEventListener('ended', () => {
                        console.warn('Audio track ended, attempting to restart');
                    });
                });
                
                localStream.getVideoTracks().forEach(track => {
                    track.contentHint = 'detail'; // Helps keep video active
                    // Prevent track from being stopped by browser
                    track.addEventListener('ended', () => {
                        console.warn('Video track ended, attempting to restart');
                    });
                });

                setLocalStream({
                    mediaStream: localStream,
                    settings: {
                        isAudioEnabled: true,
                        isVideoEnabled: true
                    }
                });

                // Store socket reference for visibility change handler
                let socketRef: WebSocket | null = null;
                let keepAliveInterval: NodeJS.Timeout | null = null;
                
                // Keep connection alive when tab goes to background
                const handleVisibilityChange = () => {
                    if (document.hidden) {
                        console.log('Tab went to background, keeping tracks active');
                        // Tab went to background - ensure tracks stay active
                        localStream.getTracks().forEach(track => {
                            if (track.readyState === 'live') {
                                // Force track to stay active
                                const wasEnabled = track.enabled;
                                track.enabled = false;
                                track.enabled = wasEnabled;
                            }
                        });
                        // Keep WebSocket alive by sending a ping
                        if (socketRef && socketRef.readyState === WebSocket.OPEN) {
                            const pingMessage: PingSignalingMessage = {
                                type: MessageType.PING
                            };
                            socketRef.send(JSON.stringify(pingMessage));
                        }
                    } else {
                        console.log('Tab came to foreground');
                    }
                };
                document.addEventListener('visibilitychange', handleVisibilityChange);
                
                const socket = new WebSocket(getRoomUrl(roomId));
                socketRef = socket; // Store reference for visibility change handler
                
                socket.onmessage = async (event) => {
                    await handleSignalingMessage(JSON.parse(event.data), socket);
                }
                socket.onopen = (_) => {
                    if (socket.readyState === WebSocket.OPEN && peerConnection === null) {
                        peerConnection = createPeerConnection(localStream, socket);
                        console.log('Peer connection created, socket opened');
                        
                        // Start keep-alive mechanism: send periodic pings to prevent WebSocket timeout
                        keepAliveInterval = setInterval(() => {
                            if (socket && socket.readyState === WebSocket.OPEN) {
                                const pingMessage: PingSignalingMessage = {
                                    type: MessageType.PING
                                };
                                socket.send(JSON.stringify(pingMessage));
                            }
                        }, 30000); // Send ping every 30 seconds
                    }
                };
                socket.onclose = (_) => {
                    // Clean up keep-alive interval
                    if (keepAliveInterval) {
                        clearInterval(keepAliveInterval);
                        keepAliveInterval = null;
                    }
                    document.removeEventListener('visibilitychange', handleVisibilityChange);
                    
                    if (socket.readyState === WebSocket.CLOSED && peerConnection !== null) {
                        peerConnection.close();
                        peerConnection = null;
                        console.log('Peer connection closed, socket closed');
                    }
                };
                setSocket(socket);
                setRoomJoinerShown(false);
            });
    }

    function toggleLocalStreamSettings(isAudioEnabled: boolean, isVideoEnabled: boolean) {
        if (localStream && socket) {
            localStream.mediaStream.getAudioTracks().forEach(track => track.enabled = isAudioEnabled);
            localStream.mediaStream.getVideoTracks().forEach(track => track.enabled = isVideoEnabled);
            const message: StreamControlSignalingMessage = {
                type: MessageType.STREAM_CONTROL,
                participant_id: userId,
                stream_id: localStream.mediaStream.id,
                is_audio_enabled: isAudioEnabled,
                is_video_enabled: isVideoEnabled
            }
            socket.send(JSON.stringify(message));
            setLocalStream({
                mediaStream: localStream.mediaStream,
                settings: {
                    isAudioEnabled: isAudioEnabled,
                    isVideoEnabled: isVideoEnabled
                }
            });
        }
    }

    const oneOnOneView = remoteStreams.length === 1;

    return (
        <main className="bg-zinc-900 text-white">
            {
                isRoomJoinerShown ? (
                    <div className="h-screen flex flex-col items-center justify-center">
                        <RoomJoiner onJoinCallback={onJoinCallback}/>
                    </div>
                ) : (
                    <div className="h-screen flex flex-col">
                        <div id="videos" className="relative h-full">
                        {
                            oneOnOneView ? (
                                <OneOnOneCompanionView
                                    localStream={localStream!} 
                                    companionStream={remoteStreams[0]}/>
                            ) : (
                                <GridCompanionView
                                    localStream={localStream!}
                                    remoteStreams={remoteStreams}/>
                            )
                        }
                        </div>
                        <div className="p-4 flex justify-center space-x-2" hidden={isRoomJoinerShown}>
                            <MicrophoneController isAudioEnabled={localStream!.settings.isAudioEnabled} onSwitch={(isAudioEnabled) => toggleLocalStreamSettings(isAudioEnabled, localStream!.settings.isVideoEnabled)}/>
                            <VideoController isVideoEnabled={localStream!.settings.isVideoEnabled} onSwitch={(isVideoEnabled) => toggleLocalStreamSettings(localStream!.settings.isAudioEnabled, isVideoEnabled)}/>
                        </div>
                    </div>
                )
            }
        </main>
    );
}
