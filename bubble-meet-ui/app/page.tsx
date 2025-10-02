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

function getRoomUrl(roomId: string) {
    const host = (
        process.env.NEXT_PUBLIC_SIGNALING_SERVER_URL ?
            process.env.NEXT_PUBLIC_SIGNALING_SERVER_URL :
            document.location.host
    );
    return `wss://${host}/api/v1/rooms/${roomId}`
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
    user_id: string,
    description: RTCSessionDescription
}

interface AnswerSignalingMessage extends AbstractSignalingMessage {
    type: MessageType.ANSWER,
    user_id: string,
    description: RTCSessionDescription
}

interface ICECandidateSignalingMessage extends AbstractSignalingMessage {
    type: MessageType.ICE_CANDIDATE,
    user_id: string,
    candidate: RTCIceCandidateInit
}

interface StreamControlSignalingMessage extends AbstractSignalingMessage {
    type: MessageType.STREAM_CONTROL,
    user_id: string,
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
                        'stun:stun.l.google.com:19302',
                        'stun:stun1.l.google.com:19302',
                        'stun:stun2.l.google.com:19302',
                        'stun:stun3.l.google.com:19302',
                        'stun:stun4.l.google.com:19302'
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
                user_id: userId,
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
                    user_id: userId
                };
                socket.send(JSON.stringify(message));
            }
        }
        peerConnection.oniceconnectionstatechange = () => {
            if (peerConnection.iceConnectionState === 'failed') {
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
                    user_id: userId,
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
                setLocalStream({
                    mediaStream: localStream,
                    settings: {
                        isAudioEnabled: true,
                        isVideoEnabled: true
                    }
                });

                const socket = new WebSocket(getRoomUrl(roomId));
                socket.onmessage = async (event) => {
                    await handleSignalingMessage(JSON.parse(event.data), socket);
                }
                socket.onopen = (_) => {
                    if (socket.readyState === WebSocket.OPEN && peerConnection === null) {
                        peerConnection = createPeerConnection(localStream, socket);
                        console.log('Peer connection created, socket opened');
                    }
                };
                socket.onclose = (_) => {
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
                user_id: userId,
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
