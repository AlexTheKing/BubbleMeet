'use client';

import RoomJoiner from "@/app/components/room-joiner";
import {MutableRefObject, useEffect, useRef, useState} from "react";
import {v4 as uuidv4} from "uuid";

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

interface SignalingMessage {
    type: MessageType
}

interface OfferSignalingMessage extends SignalingMessage {
    type: MessageType.OFFER,
    user_id: string,
    description: RTCSessionDescription
}

interface AnswerSignalingMessage extends SignalingMessage {
    type: MessageType.ANSWER,
    user_id: string,
    description: RTCSessionDescription
}

interface ICECandidateSignalingMessage extends SignalingMessage {
    type: MessageType.ICE_CANDIDATE,
    user_id: string,
    candidate: RTCIceCandidate
}

export default function App() {
    const [userId, setUserId] = useState(uuidv4());
    const [isRoomJoinerShown, setRoomJoinerShown] = useState(true);
    const [localMediaStream, setLocalMediaStream] = useState<MediaStream | null>(null);
    const localVideoRef: MutableRefObject<HTMLVideoElement | null> = useRef(null);
    const [remoteVideos, setRemoteVideos] = useState<MediaStream[]>([]);

    let peerConnection: RTCPeerConnection | null = null;

    function createPeerConnection(localMediaStream: MediaStream, socket: WebSocket) {
        peerConnection = new RTCPeerConnection();
        localMediaStream.getTracks().forEach(
            track => peerConnection?.addTrack(track, localMediaStream)
        );
        peerConnection.ontrack = ({track, streams}) => {
            console.log('Received new track and streams of length:', streams.length)
            console.log('Track', track)
            console.log('Streams', streams)
            setRemoteVideos((previousVideos) => [
                ...streams,
                ...previousVideos.filter(previousVideo => 
                    !streams.some(stream => stream.id === previousVideo.id)
                )
            ]);
        };
        peerConnection.onnegotiationneeded = async () => {
            try {
                await peerConnection?.setLocalDescription();
                const message: OfferSignalingMessage = {
                    type: MessageType.OFFER,
                    user_id: userId,
                    description: peerConnection?.localDescription!
                };
                socket.send(JSON.stringify(message));
                console.log("Sent offer");
            } catch (err) {
                console.error(err);
            }
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
            if (peerConnection?.iceConnectionState === "failed") {
                peerConnection?.restartIce();
            }
        };
        return peerConnection;
    }

    function onJoinCallback(roomId: string) {
        navigator.mediaDevices
            .getUserMedia({video: true, audio: true})
            .then((localMediaStream) => {
                setLocalMediaStream(localMediaStream);

                const socket = new WebSocket(getRoomUrl(roomId));
                socket.onmessage = async (event) => {
                    if (peerConnection !== null) {
                        const message = JSON.parse(event.data);
                        if (message.type == MessageType.ANSWER) {
                            console.log("Received answer!")
                            await peerConnection.setRemoteDescription(
                                new RTCSessionDescription(message.description)
                            );
                        }
                        if (message.type == MessageType.OFFER) {
                            console.log("Received offer as negotiation is needed!")
                            await peerConnection.setRemoteDescription(
                                new RTCSessionDescription(message.description)
                            );
                            let answer = await peerConnection?.createAnswer();
                            await peerConnection?.setLocalDescription(answer);
                            const signaling_message: AnswerSignalingMessage = {
                                type: MessageType.ANSWER,
                                user_id: userId,
                                description: peerConnection?.localDescription!
                            };
                            socket.send(JSON.stringify(signaling_message));
                            console.log("Sent answer!");
                        }
                        if (message.type == MessageType.ICE_CANDIDATE) {
                            peerConnection.addIceCandidate(
                                new RTCIceCandidate(message.candidate)
                            );
                            console.log("Received ICECandidate, added it!")
                        }
                    }
                }
                socket.onopen = (_) => {
                    if (socket.readyState === WebSocket.OPEN && peerConnection === null) {
                        createPeerConnection(localMediaStream, socket);
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

    function buildVideoGrid() {
        const videosParams = remoteVideos.map<[boolean, string, ((ref: HTMLVideoElement) => void)]>((videoStream, index) => [
            false,
            `${videoStream.id}-${index}`,
            getVideoRefCallback(videoStream)
        ]);

        videosParams.push([true, "local", getVideoRefCallback(localMediaStream)]);

        return videosParams.map(
            ([isMuted, key, ref], index) => {
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
                    <div key={key} className={`${widthClass} ${heightClass} flex items-center justify-center`}>
                        <div className="relative w-full aspect-video overflow-hidden">
                            <video
                                ref={ref}
                                autoPlay={true}
                                muted={isMuted}
                                className="absolute inset-0 w-full h-full object-cover rounded-lg"
                            />
                        </div>
                    </div>
                );
            }
        )
    }

    const oneOnOneView = remoteVideos.length === 1;

    return (
        <main className="bg-sky-900 text-white">
            {
                isRoomJoinerShown &&
                <div className="h-screen flex flex-col items-center justify-center">
                    <RoomJoiner onJoinCallback={onJoinCallback}/>
                </div>
            }
            <div id="videos" hidden={isRoomJoinerShown} className="h-screen relative">
                {
                    oneOnOneView ? (
                        <>
                            <div className="h-screen w-full flex items-center justify-center">
                                <div className="relative w-full aspect-video overflow-hidden">
                                    <video
                                        ref={getVideoRefCallback(remoteVideos[0])}
                                        autoPlay={true}
                                        className="absolute inset-0 w-full h-full object-cover"
                                    />
                                </div>
                            </div>
                            <div className="fixed bottom-4 right-4 w-64 h-48 rounded-lg overflow-hidden shadow-lg">
                                <video
                                    id="local"
                                    ref={getVideoRefCallback(localMediaStream)}
                                    autoPlay={true}
                                    muted={true}
                                    className="w-full h-full object-cover"
                                />
                            </div>
                        </>
                    ) : (
                        <div className="flex flex-wrap justify-center items-center h-screen p-4 gap-4">
                            {
                                buildVideoGrid()
                            }
                        </div>
                    )
                }
            </div>
        </main>
    );
}
