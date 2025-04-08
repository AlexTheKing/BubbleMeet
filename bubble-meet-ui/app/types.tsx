export interface StreamSettings {
    isAudioEnabled: boolean,
    isVideoEnabled: boolean
}

export interface StreamWithSettings {
    mediaStream: MediaStream,
    settings: StreamSettings
}