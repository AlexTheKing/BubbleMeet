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

type SignalingMessages = OfferSignalingMessage | AnswerSignalingMessage | ICECandidateSignalingMessage;


export default function App() {
    const [userId, setUserId] = useState(uuidv4());
    const [isRoomJoinerShown, setRoomJoinerShown] = useState(true);
    const localVideoRef: MutableRefObject<HTMLVideoElement | null> = useRef(null);
    const [remoteVideos, setRemoteVideos] = useState<MediaStream[]>([]);

    useEffect(() => {
        console.log('My user id:', userId);
    }, [userId]);
    useEffect(() => {
        console.log('RV now:', remoteVideos);
    }, [remoteVideos]);

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
                if (localVideoRef.current === null) {
                    throw new Error("Cannot access local video element!");
                }
                localVideoRef.current.srcObject = localMediaStream;

                
                // let streams = [];
                // for (let i = 0; i < 10; i++) {
                    // streams.push(localMediaStream)
                // }
                // setRemoteVideos(streams);


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
                socket.onopen = (event) => {
                    if (socket.readyState === WebSocket.OPEN && peerConnection === null) {
                        createPeerConnection(localMediaStream, socket);
                    }
                };
            });
        setRoomJoinerShown(false);
    }

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
                    remoteVideos.length === 1 ? (
                        <div className="h-screen w-full flex items-center justify-center">
                            <div className="relative w-full aspect-video overflow-hidden">
                                <video
                                    ref={(ref) => {
                                        if (ref) {
                                            ref.srcObject = remoteVideos[0];
                                        }
                                    }}
                                    autoPlay={true}
                                    className="absolute inset-0 w-full h-full object-cover"
                                />
                            </div>
                        </div>
                    ) : (
                        <div className="flex flex-wrap justify-center items-center h-screen p-4 gap-4">
                            {
                                remoteVideos.map(
                                    (videoStream, index) => {
                                        let widthClass = "w-full"; // Default full width
                                        let heightClass = "h-full"; // Default full height
                                        if (remoteVideos.length === 2) {
                                            widthClass = "w-[calc(50%-0.5rem)]"; // 2 users in a single row, accounting for gap
                                        } else if (remoteVideos.length === 3) {
                                            widthClass = index < 2 ? "w-[calc(50%-0.5rem)]" : "w-full"; // 2-1 pattern
                                            heightClass = index < 2 ? "h-[calc(50%-0.5rem)]" : "h-[calc(50%-0.5rem)]"; // Adjust height for 2-1 pattern
                                        } else if (remoteVideos.length === 4) {
                                            widthClass = "w-[calc(50%-0.5rem)]"; // 2-2 pattern
                                            heightClass = "h-[calc(50%-0.5rem)]"; // Adjust height for 2-2 pattern
                                        } else if (remoteVideos.length === 5) {
                                            widthClass = index < 3 ? "w-[calc(33.3%-0.66rem)]" : "w-[calc(50%-0.5rem)]"; // 3-1, 2-2 pattern
                                            heightClass = index < 3 ? "h-[calc(33.3%-0.66rem)]" : "h-[calc(50%-0.5rem)]"; // Adjust height for 3-1, 2-2 pattern
                                        } else if (remoteVideos.length === 6) {
                                            widthClass = "w-[calc(33.3%-0.66rem)]"; // 3-1, 3-2 pattern
                                            heightClass = "h-[calc(33.3%-0.66rem)]"; // Adjust height for 3-1, 3-2 pattern
                                        } else if (remoteVideos.length === 7) {
                                            widthClass = index < 3 ? "w-[calc(33.3%-0.66rem)]" : (index < 6 ? "w-[calc(33.3%-0.66rem)]" : "w-full"); // 3-1, 3-2, 1-3 pattern
                                            heightClass = index < 3 ? "h-[calc(33.3%-0.66rem)]" : (index < 6 ? "h-[calc(33.3%-0.66rem)]" : "h-[calc(33.3%-0.66rem)]"); // Adjust height for 3-1, 3-2, 1-3 pattern
                                        } else if (remoteVideos.length === 8) {
                                            widthClass = index < 3 ? "w-[calc(33.3%-0.66rem)]" : (index < 6 ? "w-[calc(33.3%-0.66rem)]" : "w-[calc(50%-0.5rem)]"); // 3-1, 3-2, 2-3 pattern
                                            heightClass = index < 3 ? "h-[calc(33.3%-0.66rem)]" : (index < 6 ? "h-[calc(33.3%-0.66rem)]" : "h-[calc(50%-0.5rem)]"); // Adjust height for 3-1, 3-2, 2-3 pattern
                                        } else if (remoteVideos.length >= 9) {
                                            widthClass = "w-[calc(33.3%-0.66rem)]"; // 3-1, 3-2, 3-3 pattern
                                            heightClass = "h-[calc(33.3%-0.66rem)]"; // Adjust height for 3-1, 3-2, 3-3 pattern
                                        }

                                        return (
                                            <div key={`${videoStream.id}-${index}`} className={`relative aspect-video ${widthClass} ${heightClass} flex items-center justify-center`}>
                                                <video
                                                    ref={(ref) => {
                                                        if (ref) {
                                                            ref.srcObject = videoStream;
                                                            ref.onloadedmetadata = () => {
                                                                ref.play();
                                                            }
                                                        }
                                                    }}
                                                    autoPlay={true}
                                                    className="h-full object-cover rounded-lg"
                                                />
                                            </div>
                                        );
                                    }
                                )
                            }
                        </div>
                    )
                }
                <div className="fixed bottom-4 right-4 w-64 h-48 rounded-lg overflow-hidden shadow-lg">
                    <video
                        id="local"
                        ref={localVideoRef}
                        autoPlay={true}
                        muted={true}
                        className="w-full h-full object-cover"
                    />
                </div>
            </div>
        </main>
    );
}
