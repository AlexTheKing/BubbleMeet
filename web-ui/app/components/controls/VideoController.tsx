import { FaVideo, FaVideoSlash } from "react-icons/fa";


export default function VideoController({isVideoEnabled, onSwitch}: {isVideoEnabled: boolean, onSwitch: (isVideoEnabled: boolean) => void}) {    
    return (
        <div className={`${isVideoEnabled ? 'rounded-[50px] bg-zinc-800' : 'rounded-xl bg-red-800'} p-4 transition-all duration-200 flex items-center`}>
        {
            isVideoEnabled ? 
            <FaVideo size={28} onClick={() => onSwitch(!isVideoEnabled)}/> : 
            <FaVideoSlash size={28} onClick={() => onSwitch(!isVideoEnabled)}/>
        }
        </div>
    )
}