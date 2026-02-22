use crate::error::{HollyError, Result};
use once_cell::sync::OnceCell;
use std::path::PathBuf;
use std::sync::Mutex;

const MODEL_ID: &str = "sentence-transformers/all-MiniLM-L6-v2";
pub const EMBEDDING_DIM: usize = 384;

static MODEL: OnceCell<Mutex<EmbeddingModel>> = OnceCell::new();

struct EmbeddingModel {
    tokenizer: tokenizers::Tokenizer,
    model: BertModel,
}

// Candle BERT implementation
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};

impl EmbeddingModel {
    fn load(model_dir: &std::path::Path) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let weights_path = model_dir.join("model.safetensors");
        let config_path = model_dir.join("config.json");

        if !tokenizer_path.exists() || !weights_path.exists() {
            return Err(HollyError::Embedding(format!(
                "Model files not found in {}. Run `holly init` to download.",
                model_dir.display()
            )));
        }

        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| HollyError::Embedding(e.to_string()))?;

        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| HollyError::Embedding(e.to_string()))?;
        let config: BertConfig =
            serde_json::from_str(&config_str).map_err(|e| HollyError::Embedding(e.to_string()))?;

        let device = Device::Cpu;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[&weights_path], candle_core::DType::F32, &device)
                .map_err(|e| HollyError::Embedding(e.to_string()))?
        };

        let model =
            BertModel::load(vb, &config).map_err(|e| HollyError::Embedding(e.to_string()))?;

        Ok(EmbeddingModel { tokenizer, model })
    }

    fn encode(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| HollyError::Embedding(e.to_string()))?;

        let device = Device::Cpu;
        let ids: Vec<u32> = encoding.get_ids().to_vec();
        let token_ids = Tensor::new(ids.as_slice(), &device)
            .map_err(|e| HollyError::Embedding(e.to_string()))?
            .unsqueeze(0)
            .map_err(|e| HollyError::Embedding(e.to_string()))?;

        let token_type_ids = token_ids
            .zeros_like()
            .map_err(|e| HollyError::Embedding(e.to_string()))?;

        let embeddings = self
            .model
            .forward(&token_ids, &token_type_ids, None)
            .map_err(|e| HollyError::Embedding(e.to_string()))?;

        // Mean pooling
        let mean = embeddings
            .mean(1)
            .map_err(|e| HollyError::Embedding(e.to_string()))?;

        let vec = mean
            .squeeze(0)
            .map_err(|e| HollyError::Embedding(e.to_string()))?
            .to_vec1::<f32>()
            .map_err(|e| HollyError::Embedding(e.to_string()))?;

        // L2 normalize
        Ok(l2_normalize(vec))
    }
}

/// Get or initialize the model (lazy — only loads on first call).
fn get_model(model_dir: &std::path::Path) -> Result<&'static Mutex<EmbeddingModel>> {
    MODEL.get_or_try_init(|| EmbeddingModel::load(model_dir).map(Mutex::new))
}

/// Default model directory: ~/.holly-db/models/all-MiniLM-L6-v2/
pub fn default_model_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".holly-db")
        .join("models")
        .join("all-MiniLM-L6-v2")
}

/// Generate a 384-dim embedding for the given text.
/// Lazy-loads the model on first call.
pub fn generate_embedding(text: &str) -> Result<Vec<f32>> {
    generate_embedding_with_dir(text, &default_model_dir())
}

pub fn generate_embedding_with_dir(text: &str, model_dir: &std::path::Path) -> Result<Vec<f32>> {
    let model_mutex = get_model(model_dir)?;
    let model = model_mutex
        .lock()
        .map_err(|_| HollyError::Embedding("model lock poisoned".into()))?;
    model.encode(text)
}

/// Download model files from HuggingFace Hub.
pub fn download_model(model_dir: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(model_dir)?;

    // Use hf-hub to download tokenizer and model weights
    let rt = tokio::runtime::Runtime::new().map_err(|e| HollyError::Embedding(e.to_string()))?;

    rt.block_on(async { download_model_async(model_dir).await })
}

async fn download_model_async(model_dir: &std::path::Path) -> Result<()> {
    use hf_hub::{api::tokio::Api, Repo, RepoType};

    let api = Api::new().map_err(|e| HollyError::Embedding(e.to_string()))?;
    let repo = api.repo(Repo::new(MODEL_ID.to_string(), RepoType::Model));

    for filename in &["tokenizer.json", "model.safetensors", "config.json"] {
        let dest = model_dir.join(filename);
        if dest.exists() {
            continue;
        }
        eprintln!("Downloading {}...", filename);
        let path = repo
            .get(filename)
            .await
            .map_err(|e| HollyError::Embedding(e.to_string()))?;
        std::fs::copy(&path, &dest)?;
    }

    Ok(())
}

/// Check if the model is available locally.
pub fn model_available(model_dir: &std::path::Path) -> bool {
    model_dir.join("tokenizer.json").exists()
        && model_dir.join("model.safetensors").exists()
        && model_dir.join("config.json").exists()
}

fn l2_normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}
