import ssl

import numpy as np
import torch
import triton_python_backend_utils as pb_utils
from silero_vad import get_speech_timestamps

ssl._create_default_https_context = ssl._create_unverified_context


class TritonPythonModel:
    def initialize(self, args):
        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        self.model, self.utils = torch.hub.load(
            repo_or_dir="snakers4/silero-vad", model="silero_vad"
        )
        self.model.to(self.device)
        self.model.eval()

    def execute(self, requests):
        responses = []
        for request in requests:
            audio_tensor = (
                torch.from_numpy(
                    pb_utils.get_input_tensor_by_name(request, "audio").as_numpy()
                )
                .unsqueeze(0)
                .unsqueeze(0)
                .to(self.device)
            )
            with torch.no_grad():
                speech_timestamps = get_speech_timestamps(
                    audio_tensor,
                    self.model,
                    sampling_rate=16000,
                    threshold=0.5,
                    min_speech_duration_ms=250,
                    max_speech_duration_s=float("inf"),
                    min_silence_duration_ms=100,
                    window_size_samples=512,
                    speech_pad_ms=30,
                )
            timestamps = (
                np.array(
                    [[ts["start"], ts["end"]] for ts in speech_timestamps],
                    dtype=np.int32,
                )
                if speech_timestamps
                else np.array([], dtype=np.int32).reshape(0, 2)
            )
            responses.append(
                pb_utils.InferenceResponse(
                    output_tensors=[pb_utils.Tensor("speech_timestamps", timestamps)]
                )
            )
        return responses
