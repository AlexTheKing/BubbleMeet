import CameraView from "./CameraView";
import { StreamWithSettings } from "../types";

export default function OneOnOneCompanionView({
    localStream,
    companionStream
}: {
    localStream: StreamWithSettings,
    companionStream: StreamWithSettings
}) {    
    return (
        <>
            <div className="h-full w-full flex items-center justify-center">
                <div className="relative w-full h-full overflow-hidden">
                    <CameraView
                        isLocal={false}
                        stream={companionStream}/>
                </div>
            </div>
            {
                localStream.settings.isVideoEnabled && (
                    <div className="fixed bottom-4 right-4 w-64 h-48 rounded-lg overflow-hidden shadow-lg">
                        <CameraView
                            isLocal={true}
                            stream={localStream}/>
                    </div>
                )
            }
        </>
    )
}