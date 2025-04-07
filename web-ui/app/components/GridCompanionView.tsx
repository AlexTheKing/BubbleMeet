import CameraView from "./CameraView";


interface GridCompanionViewParams {
    stream: MediaStream | null,
    streamKey: string,
    isAudioEnabled: boolean,
    isVideoEnabled: boolean
}

function isStreamAudioEnabled(stream: MediaStream | null): boolean {
    if (stream === null) {
        return false;
    }
    return stream.getAudioTracks().some(track => track.enabled);
}

function isStreamVideoEnabled(stream: MediaStream | null): boolean {
    if (stream === null) {
        return false;
    }
    return stream.getVideoTracks().some(track => track.enabled);
}

export default function GridCompanionView({localStream, remoteStreams: remoteStreams}: {localStream: MediaStream | null, remoteStreams: MediaStream[]}) {
    if (remoteStreams.length > 9) {
        throw new Error("GridCompanionView can only display up to 9 remote streams");
    }

    const companionViewParams = remoteStreams.map<GridCompanionViewParams>((stream, index) => {
        return {
            stream: stream,
            streamKey: `${stream.id}-${index}`,
            isAudioEnabled: isStreamAudioEnabled(stream),
            isVideoEnabled: isStreamVideoEnabled(stream)
        }
    });

    const localStreamKey = "local";
    companionViewParams.push({
        stream: localStream,
        streamKey: localStreamKey,
        isAudioEnabled: isStreamAudioEnabled(localStream),
        isVideoEnabled: isStreamVideoEnabled(localStream)
    });

    const grid = companionViewParams.map(
        ({stream, streamKey, isAudioEnabled: isStreamAudioEnabled, isVideoEnabled: isStreamVideoEnabled}, index) => {
            let widthClass = "w-full";
            let heightClass = "h-full";
            const totalVideos = companionViewParams.length;
            if (totalVideos === 2) {
                widthClass = "w-[calc(50%-0.5rem)]";
            } else if (totalVideos === 3 || totalVideos === 4) {
                widthClass = "w-[calc(50%-0.5rem)]";
                heightClass = "h-[calc(50%-0.5rem)]";
            } else if (totalVideos === 5) {
                widthClass = index < 3 ? "w-[calc(33.3%-0.66rem)]" : "w-[calc(50%-0.5rem)]";
                heightClass = "h-[calc(50%-0.5rem)]";
            } else if (totalVideos == 6) {
                widthClass = "w-[calc(50%-0.5rem)]";
                heightClass = "h-[calc(33.3%-0.66rem)]"
            } else if (totalVideos >= 7) {
                widthClass = "w-[calc(33.3%-0.66rem)]";
                heightClass = "h-[calc(33.3%-0.66rem)]";
            }

            return (
                <div key={streamKey} className={`${widthClass} ${heightClass} flex items-center justify-center`}>
                    <div className="relative w-full h-full overflow-hidden">
                        <CameraView
                            isLocal={streamKey === localStreamKey}
                            stream={stream}
                            isVideoEnabled={isStreamVideoEnabled}
                            isAudioEnabled={isStreamAudioEnabled}/>
                    </div>
                </div>
            );
        }
    );

    return (
        <div className="flex flex-wrap justify-center items-center h-full p-2 gap-4">
            {grid}
        </div>
    )

}