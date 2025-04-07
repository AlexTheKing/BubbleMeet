import CameraView from "./CameraView";


export default function OneOnOneCompanionView({localStream, companionStream, isLocalVideoEnabled, isLocalAudioEnabled}: {localStream: MediaStream, companionStream: MediaStream, isLocalVideoEnabled: boolean, isLocalAudioEnabled: boolean}) {    
    return (
        <>
            <div className="h-full w-full flex items-center justify-center">
                <div className="relative w-full h-full overflow-hidden">
                    <CameraView
                        isLocal={false}
                        stream={companionStream}
                        isVideoEnabled={true}
                        isAudioEnabled={true}/>
                </div>
            </div>
            {
                isLocalVideoEnabled && (
                    <div className="fixed bottom-4 right-4 w-64 h-48 rounded-lg overflow-hidden shadow-lg">
                        <CameraView
                            isLocal={true}
                            stream={localStream}
                            isVideoEnabled={true} 
                            isAudioEnabled={isLocalAudioEnabled}/>
                    </div>
                )
            }
        </>
    )
}