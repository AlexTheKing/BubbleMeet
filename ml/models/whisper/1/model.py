import ssl

import numpy as np
import torch
import triton_python_backend_utils as pb_utils
import whisper

ssl._create_default_https_context = ssl._create_unverified_context


class TritonPythonModel:
    def initialize(self, args):
        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        self.model = whisper.load_model("base", device=self.device)

    def execute(self, requests):
        responses = []
        for request in requests:
            audio_sample = pb_utils.get_input_tensor_by_name(
                request, "audio"
            ).as_numpy()
            task = (
                pb_utils.get_input_tensor_by_name(request, "task")
                .as_numpy()[0]
                .decode("utf-8")
            )
            language = pb_utils.get_input_tensor_by_name(
                request, "language"
            ).as_numpy()[0]
            language = language.decode("utf-8") if language != b"" else None
            audio_tensor = torch.from_numpy(audio_sample).to(self.device).squeeze()
            if audio_tensor.abs().max() > 1.0:
                audio_tensor = audio_tensor / audio_tensor.abs().max()
            with torch.no_grad():
                result = self.model.transcribe(
                    audio_tensor,
                    task=task,
                    language=language,
                    fp16=self.device.type == "cuda",
                    no_speech_threshold=0.3,
                    condition_on_previous_text=False,
                    temperature=0.0,
                )
            responses.append(
                pb_utils.InferenceResponse(
                    output_tensors=[
                        pb_utils.Tensor(
                            "text",
                            np.array([result.get("text", "").strip()], dtype=object),
                        ),
                        pb_utils.Tensor(
                            "language", np.array([result.get("language")], dtype=object)
                        ),
                    ]
                )
            )
        return responses
