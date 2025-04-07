'use client';

import RoomJoiner from "@/app/components/RoomJoiner";
import assert from "assert";
import {useState} from "react";
import {v4 as uuidv4} from "uuid";
import { FaMicrophone, FaMicrophoneSlash, FaVideo, FaVideoSlash } from "react-icons/fa";
import OneOnOneCompanionView from "./components/OneOnOneCompanionView";
import GridCompanionView from "./components/GridCompanionView";
import MicrophoneController from "./components/controls/MicrophoneController";
import VideoController from "./components/controls/VideoController";

// const SIGNALING_SERVER_URL = "192.168.0.107:8000";
const SIGNALING_SERVER_URL = "localhost:8000";

function getRoomUrl(roomId: string) {
    // return `${SIGNALING_SERVER_URL}/rooms/${roomId}`
    // let host = document.location.host.split(':')[0];
    return `ws://${SIGNALING_SERVER_URL}/rooms/${roomId}`
}

enum MessageType {
    OFFER = "Offer",
    ANSWER = "Answer",
    ICE_CANDIDATE = "ICECandidate"
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
    candidate: RTCIceCandidate
}

type SignalingMessage = OfferSignalingMessage | AnswerSignalingMessage | ICECandidateSignalingMessage;

export default function App() {
    const [userId, setUserId] = useState(uuidv4());
    const [isRoomJoinerShown, setRoomJoinerShown] = useState(true);
    const [localMediaStream, setLocalMediaStream] = useState<MediaStream | null>(null);
    const [remoteStreams, setRemoteStreams] = useState<MediaStream[]>([]);
    const [isLocalAudioEnabled, setIsLocalAudioEnabled] = useState(true);
    const [isLocalVideoEnabled, setIsLocalVideoEnabled] = useState(true);

    let peerConnection: RTCPeerConnection | null = null;

    function createPeerConnection(localMediaStream: MediaStream, socket: WebSocket) {
        const peerConnection = new RTCPeerConnection();
        localMediaStream.getTracks().forEach(
            track => peerConnection.addTrack(track, localMediaStream)
        );
        peerConnection.ontrack = ({track, streams}) => {
            console.log(`Received new track ${track.id}`)
            setRemoteStreams((previousStreams) => [
                ...streams,
                ...previousStreams.filter(previousStream => 
                    !streams.some(stream => stream.id === previousStream.id)
                )
            ]);
            assert(streams.length <= 1, 'Multiple streams received');
            let stream = streams[0];
            stream.onremovetrack = ({track}) => {
                console.log(`Track ${track.id} was removed`);
                if (!stream.getTracks().length) {
                  setRemoteStreams((previousStreams) => previousStreams.filter(previousStream => previousStream.id !== stream.id));
                  console.log(`Stream ${stream.id} was removed`);
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
                    candidate: candidate,
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
                peerConnection.addIceCandidate(
                    new RTCIceCandidate(message.candidate)
                );
                break;
            default:
                console.warn('Unknown message, skipping message');
                break;
        }
    }

    function onJoinCallback(roomId: string) {
        navigator.mediaDevices
            .getUserMedia({video: true, audio: true})
            .then((localMediaStream) => {
                setLocalMediaStream(localMediaStream);

                const socket = new WebSocket(getRoomUrl(roomId));
                socket.onmessage = async (event) => {
                    await handleSignalingMessage(JSON.parse(event.data), socket);
                }
                socket.onopen = (_) => {
                    if (socket.readyState === WebSocket.OPEN && peerConnection === null) {
                        peerConnection = createPeerConnection(localMediaStream, socket);
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
            });
        setRoomJoinerShown(false);
    }


    function toggleAudio(isAudioEnabled: boolean) {
        if (localMediaStream) {
            localMediaStream.getAudioTracks().forEach(track => track.enabled = isAudioEnabled);
            setIsLocalAudioEnabled(isAudioEnabled);
        }
    }

    function toggleVideo(isVideoEnabled: boolean) {
        if (localMediaStream) {
            localMediaStream.getVideoTracks().forEach(track => track.enabled = isVideoEnabled);
            setIsLocalVideoEnabled(isVideoEnabled);
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
                                    localStream={localMediaStream!} 
                                    companionStream={remoteStreams[0]} 
                                    isLocalVideoEnabled={isLocalVideoEnabled} 
                                    isLocalAudioEnabled={isLocalAudioEnabled}/>
                            ) : (
                                <GridCompanionView
                                    localStream={localMediaStream!} 
                                    remoteStreams={remoteStreams}/>
                            )
                        }
                        </div>
                        <div className="p-4 flex justify-center space-x-2" hidden={isRoomJoinerShown}>
                            <MicrophoneController isAudioEnabled={isLocalAudioEnabled} onSwitch={toggleAudio}/>
                            <VideoController isVideoEnabled={isLocalVideoEnabled} onSwitch={toggleVideo}/>
                        </div>
                    </div>
                )
            }
        </main>
    );
}
