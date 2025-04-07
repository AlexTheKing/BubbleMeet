import {useState} from "react";


export default function RoomJoiner({onJoinCallback}: { onJoinCallback: (roomId: string) => void }) {
    let [roomId, setRoomId] = useState("");
    return (
        <div>
            <label className="block text-gray-300 text-sm font-bold mb-2" htmlFor="roomId">
                Enter room number to join:
            </label>
            <div className="flex">
                <input
                    id="roomId"
                    placeholder="Room ID"
                    value={roomId}
                    onChange={e => setRoomId(e.target.value)}
                    className="bg-white shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                />
                <button onClick={() => onJoinCallback(roomId)}
                        className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">Join
                </button>
            </div>
        </div>
    )
}