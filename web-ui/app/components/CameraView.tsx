import { useMemo, useRef, useState } from "react";
import { FaMicrophoneSlash } from "react-icons/fa";


export default function CameraView({isLocal, stream, isVideoEnabled, isAudioEnabled}: {isLocal: boolean, stream: MediaStream | null, isVideoEnabled: boolean, isAudioEnabled: boolean}) {    
    const [isMetadataLoaded, setIsMetadataLoaded] = useState(false);

    const memoizedVideo = useMemo(() => {
        return (
            <video
                ref={(ref: HTMLVideoElement) => {
                    if (ref && stream !== null) {
                        ref.srcObject = stream;
                        ref.onloadedmetadata = () => {
                            ref.play();
                            setIsMetadataLoaded(true);
                        };
                    }
                }}
                autoPlay={true}
                muted={isLocal}
                className="absolute inset-0 w-full h-full object-cover rounded-lg"
            />
        )
    }, [isLocal, stream])
    
    return (
        <>
            {
                isVideoEnabled ? (
                    <>
                        {memoizedVideo}
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
                !isAudioEnabled && (
                    <div className="absolute right-0 top-0 mr-3 mt-3 p-1 rounded-[50px] bg-zinc-800/35">
                        <FaMicrophoneSlash size={16}/>
                    </div>
                )
            }
        </>
    )
}