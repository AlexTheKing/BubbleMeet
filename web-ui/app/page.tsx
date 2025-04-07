'use client';

import RoomJoiner from "@/app/components/room-joiner";
import assert from "assert";
import {MutableRefObject, useEffect, useRef, useState} from "react";
import {v4 as uuidv4} from "uuid";
import { FaMicrophone, FaMicrophoneSlash, FaVideo, FaVideoSlash } from "react-icons/fa";

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
    const [remoteVideos, setRemoteVideos] = useState<MediaStream[]>([]);
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
            setRemoteVideos((previousVideos) => [
                ...streams,
                ...previousVideos.filter(previousVideo => 
                    !streams.some(stream => stream.id === previousVideo.id)
                )
            ]);
            assert(streams.length <= 1, 'Multiple streams received');
            let stream = streams[0];
            stream.onremovetrack = ({track}) => {
                console.log(`Track ${track.id} was removed`);
                if (!stream.getTracks().length) {
                  setRemoteVideos((previousVideos) => previousVideos.filter(previousVideo => previousVideo.id !== stream.id));
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

    function getVideoRefCallback(videoStream: MediaStream | null): ((ref: HTMLVideoElement) => void) {
        return (ref: HTMLVideoElement) => {
            if (ref && videoStream !== null) {
                ref.srcObject = videoStream;
                ref.onloadedmetadata = () => ref.play();
            }
        }
    }

    function isStreamAudioEnabled(videoStream: MediaStream | null): boolean {
        if (videoStream === null) {
            return false;
        }
        return videoStream.getAudioTracks().some(track => track.enabled);
    }

    function isStreamVideoEnabled(videoStream: MediaStream | null): boolean {
        if (videoStream === null) {
            return false;
        }
        return videoStream.getVideoTracks().some(track => track.enabled);
    }

    function buildVideoGrid() {
        interface VideoParams {
            streamKey: string,
            videoRefCallback: (ref: HTMLVideoElement) => void,
            isStreamAudioEnabled: boolean,
            isStreamVideoEnabled: boolean
        }

        const videosParams = remoteVideos.map<VideoParams>((videoStream, index) => {
            return {
                streamKey: `${videoStream.id}-${index}`,
                videoRefCallback: getVideoRefCallback(videoStream),
                isStreamAudioEnabled: isStreamAudioEnabled(videoStream),
                isStreamVideoEnabled: isStreamVideoEnabled(videoStream)
            }
        });

        videosParams.push({
            streamKey: "local",
            videoRefCallback: getVideoRefCallback(localMediaStream),
            isStreamAudioEnabled: isStreamAudioEnabled(localMediaStream),
            isStreamVideoEnabled: isStreamVideoEnabled(localMediaStream)
        });

        return videosParams.map(
            ({streamKey, videoRefCallback, isStreamAudioEnabled, isStreamVideoEnabled}, index) => {
                let widthClass = "w-full";
                let heightClass = "h-full";
                const totalVideos = videosParams.length;
                if (totalVideos === 2) {
                    widthClass = "w-[calc(50%-0.5rem)]";
                } else if (totalVideos === 3 || totalVideos === 4) {
                    widthClass = "w-[calc(50%-0.5rem)]";
                    heightClass = "h-[calc(50%-0.5rem)]";
                } else if (totalVideos === 5) {
                    widthClass = index < 3 ? "w-[calc(33.3%-0.66rem)]" : "w-[calc(50%-0.5rem)]";
                    heightClass = "h-[calc(50%-0.5rem)]";
                } else if (totalVideos >= 6 && totalVideos <= 9) {
                    widthClass = "w-[calc(33.3%-0.66rem)]";
                    heightClass = "h-[calc(33.3%-0.66rem)]";
                }

                return (
                    <div key={streamKey} className={`${widthClass} ${heightClass} flex items-center justify-center`}>
                        <div className="relative w-full h-full overflow-hidden">
                            {isStreamVideoEnabled ? (
                                <video
                                    ref={videoRefCallback}
                                    autoPlay={true}
                                    muted={!isStreamAudioEnabled}
                                    className="absolute inset-0 w-full h-full object-cover rounded-lg"
                                />
                            ) : (
                                <div className="w-full h-full bg-gray-700 flex items-center justify-center">
                                    <span className="text-white">Video Off</span>
                                </div>
                            )}

                            {
                                !isStreamAudioEnabled && (
                                    <div className="absolute right-0 top-0 mr-3 mt-3 p-1 rounded-[50px] bg-zinc-800/35">
                                        <FaMicrophoneSlash size={16}/>
                                    </div>
                                )
                            }
                        </div>
                    </div>
                );
            }
        )
    }

    const oneOnOneView = remoteVideos.length === 1;

    function toggleAudio() {
        if (localMediaStream) {
            localMediaStream.getAudioTracks().forEach(track => track.enabled = !track.enabled);
            setIsLocalAudioEnabled(!isLocalAudioEnabled);
        }
    }

    function toggleVideo() {
        if (localMediaStream) {
            localMediaStream.getVideoTracks().forEach(track => track.enabled = !track.enabled);
            setIsLocalVideoEnabled(!isLocalVideoEnabled);
        }
    }

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
                                <>
                                    <div className="h-full w-full flex items-center justify-center">
                                        <div className="relative w-full h-full overflow-hidden">
                                            <video
                                                ref={getVideoRefCallback(remoteVideos[0])}
                                                autoPlay={true}
                                                className="absolute inset-0 w-full h-full object-cover"
                                            />
                                        </div>
                                    </div>
                                    {
                                        isLocalVideoEnabled && (
                                            <div className="fixed bottom-4 right-4 w-64 h-48 rounded-lg overflow-hidden shadow-lg">
                                                <video
                                                    id="local"
                                                    ref={getVideoRefCallback(localMediaStream)}
                                                    autoPlay={true}
                                                    muted={true}
                                                    className="w-full h-full object-cover"
                                                />
                                            </div>
                                        )
                                    }
                                </>
                            ) : (
                                <div className="flex flex-wrap justify-center items-center h-full p-2 gap-4">
                                    {
                                        buildVideoGrid()
                                    }
                                </div>
                            )
                        }
                        </div>
                        <div className="p-4 flex justify-center space-x-2" hidden={isRoomJoinerShown}>
                            <div className={`${isLocalAudioEnabled ? 'rounded-[50px] bg-zinc-800' : 'rounded-xl bg-red-800'} p-4 transition-all duration-300 flex items-center`}>
                            {
                                isLocalAudioEnabled ? 
                                <FaMicrophone size={28} onClick={toggleAudio}/> : 
                                <FaMicrophoneSlash size={28} onClick={toggleAudio}/>
                            }
                            </div>
                            <div className={`${isLocalVideoEnabled ? 'rounded-[50px] bg-zinc-800' : 'rounded-xl bg-red-800'} p-4 transition-all duration-300 flex items-center`}>
                            {
                                isLocalVideoEnabled ? 
                                <FaVideo size={28} onClick={toggleVideo}/> : 
                                <FaVideoSlash size={28} onClick={toggleVideo}/>
                            }
                            </div>
                        </div>
                    </div>
                )
            }
        </main>
    );
}
