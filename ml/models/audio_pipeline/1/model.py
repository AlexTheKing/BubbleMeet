import asyncio

import numpy as np
import triton_python_backend_utils as pb_utils


class TritonPythonModel:
    def initialize(self, args):
        self.model_config = args["model_config"]

    async def execute(self, requests):
        futures = []
        for request in requests:
            audio_samples = pb_utils.get_input_tensor_by_name(
                request, "audio"
            ).as_numpy()
            tasks = [
                row.item().decode("utf-8")
                for row in pb_utils.get_input_tensor_by_name(request, "task").as_numpy()
            ]
            languages = [
                row.item().decode("utf-8")
                for row in pb_utils.get_input_tensor_by_name(
                    request, "language"
                ).as_numpy()
            ]
            futures.append(self.process_request(audio_samples, tasks, languages))

        responses = []
        for texts, detected_languages in await asyncio.gather(*futures):
            responses.append(
                pb_utils.InferenceResponse(
                    output_tensors=[
                        pb_utils.Tensor(
                            "text",
                            np.array([[text] for text in texts], dtype=object),
                        ),
                        pb_utils.Tensor(
                            "language",
                            np.array(
                                [[language] for language in detected_languages],
                                dtype=object,
                            ),
                        ),
                    ]
                )
            )
        return responses

    async def process_request(self, audio_samples, tasks, languages):
        futures = [
            self.process_audio_sample(audio_sample, task, language)
            for audio_sample, task, language in zip(audio_samples, tasks, languages)
        ]
        texts = []
        detected_languages = []
        for text, detected_language in await asyncio.gather(*futures):
            texts.append(text)
            detected_languages.append(detected_language)
        return texts, detected_languages

    async def process_audio_sample(self, audio_sample, task, language):
        timestamps = await self.get_speech_timestamps(audio_sample)
        speech_audio = self.extract_speech_segments(audio_sample, timestamps)
        return (
            await self.run_whisper(speech_audio, task, language)
            if speech_audio is not None
            else ("", language)
        )

    async def get_speech_timestamps(self, audio_sample):
        vad_response = await pb_utils.InferenceRequest(
            model_name="vad",
            requested_output_names=["speech_timestamps"],
            inputs=[pb_utils.Tensor("audio", audio_sample)],
        ).async_exec()
        if vad_response.has_error():
            raise pb_utils.TritonModelException(vad_response.error().message())
        return pb_utils.get_output_tensor_by_name(
            vad_response, "speech_timestamps"
        ).as_numpy()

    def extract_speech_segments(self, audio_sample, timestamps):
        if len(timestamps) == 0:
            return None
        speech_segments = []
        for start_sample, end_sample in timestamps:
            start_idx = max(0, min(start_sample, len(audio_sample) - 1))
            end_idx = max(start_idx + 1, min(end_sample, len(audio_sample)))
            if end_idx > start_idx:
                segment = audio_sample[start_idx:end_idx]
                speech_segments.append(segment)
        return np.concatenate(speech_segments) if len(speech_segments) > 0 else None

    async def run_whisper(self, speech_audio, task, language):
        whisper_response = await pb_utils.InferenceRequest(
            model_name="whisper",
            requested_output_names=["text", "language"],
            inputs=[
                pb_utils.Tensor("audio", speech_audio),
                pb_utils.Tensor("task", np.array([task.encode("utf-8")], dtype=object)),
                pb_utils.Tensor(
                    "language", np.array([language.encode("utf-8")], dtype=object)
                ),
            ],
        ).async_exec()
        if whisper_response.has_error():
            raise pb_utils.TritonModelException(whisper_response.error().message())
        return (
            pb_utils.get_output_tensor_by_name(whisper_response, "text")
            .as_numpy()[0]
            .decode("utf-8"),
            pb_utils.get_output_tensor_by_name(whisper_response, "language")
            .as_numpy()[0]
            .decode("utf-8"),
        )
