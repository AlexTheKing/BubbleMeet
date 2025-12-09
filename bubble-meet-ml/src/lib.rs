pub mod whisper;

use std::path::PathBuf;

use candle_core::{Device, Tensor};

use candle_nn::VarBuilder;
use candle_transformers::models::whisper::model::Whisper;
use candle_transformers::models::whisper::{Config, DTYPE};
use hf_hub::{Repo, RepoType, api::tokio::Api};
use tokenizers::Tokenizer;

use crate::whisper::model::{Decoder, Model, ModelType, Task, token_id};
use crate::whisper::multilingual;

struct TranscriberBuilder {}
impl TranscriberBuilder {
    async fn get_config_tokenizer_model_weights(
        model_type: ModelType,
    ) -> Result<(PathBuf, PathBuf, PathBuf), anyhow::Error> {
        let api = Api::new()?;
        let (model_id, revision) = model_type.model_and_revision();
        let repo = api.repo(Repo::with_revision(
            model_id.to_string(),
            RepoType::Model,
            revision.to_string(),
        ));
        let config = repo.get("config.json").await?;
        let tokenizer = repo.get("tokenizer.json").await?;
        let model = repo.get("model.safetensors").await?;
        Ok((config, tokenizer, model))
    }

    async fn build(self) -> Result<Transcriber, anyhow::Error> {
        let device = Device::Cpu;
        let model_type = ModelType::TinyEn;

        let (config_filename, tokenizer_filename, model_filename) =
            Self::get_config_tokenizer_model_weights(model_type).await?;
        let config: Config = serde_json::from_str(&std::fs::read_to_string(config_filename)?)?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(anyhow::Error::msg)?;

        let mel_bytes = match config.num_mel_bins {
            80 => include_bytes!("melfilters.bytes").as_slice(),
            128 => include_bytes!("melfilters128.bytes").as_slice(),
            nmel => anyhow::bail!("unexpected num_mel_bins {nmel}"),
        };
        let mut mel_filters = vec![0f32; mel_bytes.len() / 4];
        <byteorder::LittleEndian as byteorder::ByteOrder>::read_f32_into(
            mel_bytes,
            &mut mel_filters,
        );

        let mut model = {
            let vb =
                unsafe { VarBuilder::from_mmaped_safetensors(&[model_filename], DTYPE, &device)? };
            Model::Normal(Whisper::load(&vb, config)?)
        };

        // let language_token = if model_type.is_multilingual() {
        //     Some(multilingual::detect_language(&mut model, &tokenizer, &mel)?)
        // } else {
        //     None
        // };
        let language_token = None;
        let timestamps = false;
        let max_initial_timestamp_index = None;

        let mut dc = Decoder::new(
            model,
            tokenizer,
            47,
            &device,
            language_token,
            Some(Task::Transcribe),
            timestamps,
            max_initial_timestamp_index,
            true,
        )?;
        Ok(Transcriber { decoder: dc })
    }
}

struct Transcriber {
    decoder: Decoder,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // let (pcm_data, sample_rate) = candle_examples::audio::pcm_decode(input)?;
    // if sample_rate != m::SAMPLE_RATE as u32 {
    // anyhow::bail!("input file must have a {} sampling rate", m::SAMPLE_RATE)
    // }
    // println!("pcm data loaded {}", pcm_data.len());
    // let mel = audio::pcm_to_mel(&config, &pcm_data, &mel_filters);
    // let mel_len = mel.len();

    // dc.run(&mel)?;
    Ok(())
}
