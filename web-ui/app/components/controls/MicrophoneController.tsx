import { FaMicrophone, FaMicrophoneSlash } from "react-icons/fa";


export default function MicrophoneController({isAudioEnabled, onSwitch}: {isAudioEnabled: boolean, onSwitch: (isAudioEnabled: boolean) => void}) {    
    return (
        <div className={`${isAudioEnabled ? 'rounded-[50px] bg-zinc-800' : 'rounded-xl bg-red-800'} p-4 transition-all duration-200 flex items-center`}>
        {
            isAudioEnabled ? 
            <FaMicrophone size={28} onClick={() => onSwitch(!isAudioEnabled)}/> : 
            <FaMicrophoneSlash size={28} onClick={() => onSwitch(!isAudioEnabled)}/>
        }
        </div>
    )
}