mod inference;

use crate::inference::{
    InferTensorContents, ModelInferRequest, ModelInferResponse,
    grpc_inference_service_client::GrpcInferenceServiceClient,
    model_infer_request::{InferInputTensor, InferRequestedOutputTensor},
};
pub enum AudioTask {
    Transcribe,
    Translate,
}

impl AudioTask {
    pub fn to_string(&self) -> String {
        match self {
            AudioTask::Transcribe => "transcribe".to_string(),
            AudioTask::Translate => "translate".to_string(),
        }
    }
}

pub struct TranscribeResponse {
    pub text: String,
    pub language: String,
}

impl TranscribeResponse {
    pub fn new(text: String, language: String) -> Self {
        Self { text, language }
    }
}

pub struct AudioPipelineMLClient {
    client: GrpcInferenceServiceClient<tonic::transport::Channel>,
}

impl AudioPipelineMLClient {
    pub async fn new(host: String, port: u16) -> Result<AudioPipelineMLClient, anyhow::Error> {
        let client =
            GrpcInferenceServiceClient::connect(format!("http://{}:{}", host, port)).await?;
        Ok(Self { client })
    }

    fn create_transcribe_request(
        &self,
        audio: Vec<f32>,
        task: AudioTask,
        language: Option<String>,
    ) -> ModelInferRequest {
        let audio_shape = vec![1i64, audio.len() as i64];
        ModelInferRequest {
            model_name: "audio_pipeline".to_string(),
            model_version: String::new(),
            id: String::new(),
            parameters: std::collections::HashMap::new(),
            inputs: vec![
                InferInputTensor {
                    name: "audio".to_string(),
                    datatype: "FP32".to_string(),
                    shape: audio_shape,
                    parameters: std::collections::HashMap::new(),
                    contents: Some(InferTensorContents {
                        bool_contents: vec![],
                        int_contents: vec![],
                        int64_contents: vec![],
                        uint_contents: vec![],
                        uint64_contents: vec![],
                        fp32_contents: audio,
                        fp64_contents: vec![],
                        bytes_contents: vec![],
                    }),
                },
                InferInputTensor {
                    name: "task".to_string(),
                    datatype: "BYTES".to_string(),
                    shape: vec![1, 1],
                    parameters: std::collections::HashMap::new(),
                    contents: Some(InferTensorContents {
                        bool_contents: vec![],
                        int_contents: vec![],
                        int64_contents: vec![],
                        uint_contents: vec![],
                        uint64_contents: vec![],
                        fp32_contents: vec![],
                        fp64_contents: vec![],
                        bytes_contents: vec![task.to_string().as_bytes().to_vec()],
                    }),
                },
                InferInputTensor {
                    name: "language".to_string(),
                    datatype: "BYTES".to_string(),
                    shape: vec![1, 1],
                    parameters: std::collections::HashMap::new(),
                    contents: Some(InferTensorContents {
                        bool_contents: vec![],
                        int_contents: vec![],
                        int64_contents: vec![],
                        uint_contents: vec![],
                        uint64_contents: vec![],
                        fp32_contents: vec![],
                        fp64_contents: vec![],
                        bytes_contents: vec![language.unwrap_or_default().as_bytes().to_vec()],
                    }),
                },
            ],
            outputs: vec![
                InferRequestedOutputTensor {
                    name: "text".to_string(),
                    parameters: std::collections::HashMap::new(),
                },
                InferRequestedOutputTensor {
                    name: "language".to_string(),
                    parameters: std::collections::HashMap::new(),
                },
            ],
            raw_input_contents: vec![],
        }
    }

    fn extract_string_output(&self, response: &ModelInferResponse, name: &str) -> String {
        let idx = response.outputs.iter().position(|o| o.name == name);
        idx.and_then(|idx| {
            response.outputs.get(idx).and_then(|o| {
                o.contents
                    .as_ref()
                    .and_then(|c| c.bytes_contents.get(0))
                    .or_else(|| response.raw_output_contents.get(idx))
            })
        })
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .unwrap()
    }

    fn create_vad_request(&self, audio: Vec<f32>) -> ModelInferRequest {
        ModelInferRequest {
            model_name: "vad".to_string(),
            model_version: String::new(),
            id: String::new(),
            parameters: std::collections::HashMap::new(),
            inputs: vec![InferInputTensor {
                name: "audio".to_string(),
                datatype: "FP32".to_string(),
                shape: vec![audio.len() as i64],
                parameters: std::collections::HashMap::new(),
                contents: Some(InferTensorContents {
                    bool_contents: vec![],
                    int_contents: vec![],
                    int64_contents: vec![],
                    uint_contents: vec![],
                    uint64_contents: vec![],
                    fp32_contents: audio,
                    fp64_contents: vec![],
                    bytes_contents: vec![],
                }),
            }],
            outputs: vec![InferRequestedOutputTensor {
                name: "speech_timestamps".to_string(),
                parameters: std::collections::HashMap::new(),
            }],
            raw_input_contents: vec![],
        }
    }

    pub async fn detect_voice_activity(&mut self, audio: Vec<f32>) -> Result<bool, anyhow::Error> {
        let request = self.create_vad_request(audio);
        let response = self.client.model_infer(request).await?;
        let response_ref = response.get_ref();

        let timestamps_idx = response_ref
            .outputs
            .iter()
            .position(|o| o.name == "speech_timestamps");

        if let Some(idx) = timestamps_idx {
            // Check int_contents first (for INT32 data)
            if let Some(output) = response_ref.outputs.get(idx) {
                if let Some(contents) = &output.contents {
                    // speech_timestamps is INT32, shape [-1, 2]
                    // int_contents contains flattened [start1, end1, start2, end2, ...]
                    if !contents.int_contents.is_empty() {
                        return Ok(true);
                    }
                }
            }

            // Fallback: check raw_output_contents if int_contents is empty
            if let Some(raw_data) = response_ref.raw_output_contents.get(idx) {
                if !raw_data.is_empty() {
                    // INT32 is 4 bytes per value, shape is [-1, 2] so at least 8 bytes for one timestamp pair
                    if raw_data.len() >= 8 {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    pub async fn transcribe(
        &mut self,
        audio: Vec<f32>,
        task: AudioTask,
        language: Option<String>,
    ) -> Result<TranscribeResponse, anyhow::Error> {
        let request = self.create_transcribe_request(audio, task, language);
        let response = self.client.model_infer(request).await?;
        let response_ref = response.get_ref();
        let text_output = self.extract_string_output(response_ref, "text");
        let lang_output = self.extract_string_output(response_ref, "language");
        Ok(TranscribeResponse::new(text_output, lang_output))
    }
}
