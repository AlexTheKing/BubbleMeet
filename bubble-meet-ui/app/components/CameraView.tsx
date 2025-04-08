import { memo, useState, useMemo } from "react";
import { FaMicrophoneSlash } from "react-icons/fa";
import { StreamWithSettings } from "../types";

const Video = memo(function Video({isLocal, mediaStream, onMetadataLoaded}: {isLocal: boolean, mediaStream: MediaStream, onMetadataLoaded: () => void}) {
    return (
        <video
            ref={(ref: HTMLVideoElement) => {
                if (ref && mediaStream !== null) {
                    ref.srcObject = mediaStream;
                    ref.onloadedmetadata = () => {
                        ref.play();
                        onMetadataLoaded();
                    };
                }
            }}
            autoPlay={true}
            muted={isLocal}
            className="absolute inset-0 w-full h-full object-cover rounded-lg"
        />
    );
}, (prevProps, nextProps) => {
    return prevProps.isLocal === nextProps.isLocal && prevProps.mediaStream === nextProps.mediaStream;
});

export default function CameraView({isLocal, stream}: {isLocal: boolean, stream: StreamWithSettings}) {    
    const [isMetadataLoaded, setIsMetadataLoaded] = useState(false);

    return (
        <>
            {
                stream.settings.isVideoEnabled ? (
                    <>
                        <Video isLocal={isLocal} mediaStream={stream.mediaStream} onMetadataLoaded={() => setIsMetadataLoaded(true)} />
                        <div hidden={isMetadataLoaded} className="absolute inset-0 w-full h-full bg-gray-700 flex items-center justify-center">
                            <span className="text-white">Loading...</span>
                        </div>
                    </>
                    
                ) : (
                    <div className="w-full h-full bg-gray-700 flex items-center justify-center">
                        <span className="text-white">Video Off</span>
                    </div>
                )
            }
            {
                !stream.settings.isAudioEnabled && (
                    <div className="absolute right-0 top-0 mr-3 mt-3 p-1 rounded-[50px] bg-zinc-800/35">
                        <FaMicrophoneSlash size={16}/>
                    </div>
                )
            }
        </>
    )
}