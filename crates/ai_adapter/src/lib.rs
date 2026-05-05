use scene_schema::{SceneFile, ValidationIssue, parse_scene_str, validate_scene};
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::str::FromStr;

const PELICAN_BICYCLE: &str = include_str!("../../../examples/pelican_bicycle.vsd.json");
const BASIC_POSTER: &str = include_str!("../../../examples/basic_poster.vsd.json");
const HYBRID_SCENE: &str = include_str!("../../../examples/hybrid_scene.vsd.json");
const SCENE_DOCUMENT_SCHEMA: &str = include_str!("../../../schemas/scene-document.schema.json");
const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_GEMINI_FALLBACK_MODEL: &str = "gemini-3.1-flash-lite-preview";

pub const DEFAULT_PROVIDER_ENV_VAR: &str = "TWEAKY_AI_PROVIDER";
pub const DEFAULT_MODEL_ENV_VAR: &str = "TWEAKY_AI_MODEL";
pub const DEFAULT_API_KEY_ENV_VAR: &str = "TWEAKY_AI_API_KEY_ENV";
pub const DEFAULT_BASE_URL_ENV_VAR: &str = "TWEAKY_AI_BASE_URL";
pub const DEFAULT_FALLBACK_MODELS_ENV_VAR: &str = "TWEAKY_AI_FALLBACK_MODELS";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    FullDocument,
    Patch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiSceneResponse {
    pub mode: ResponseMode,
    pub summary: String,
    #[serde(default)]
    pub document: Option<SceneFile>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedScene {
    pub response: AiSceneResponse,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Deserialize)]
struct RawAiSceneResponse {
    mode: ResponseMode,
    summary: String,
    #[serde(default)]
    document: Option<serde_json::Value>,
    #[serde(default)]
    notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Mock,
    Gemini,
    OpenAiCompatible,
}

impl ProviderKind {
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Mock => "mock-scene-generator",
            Self::Gemini => "gemini-2.5-flash",
            Self::OpenAiCompatible => "gpt-4o-mini",
        }
    }

    pub fn default_api_key_env_var(self) -> Option<&'static str> {
        match self {
            Self::Mock => None,
            Self::Gemini => Some("GEMINI_API_KEY"),
            Self::OpenAiCompatible => Some("OPENAI_API_KEY"),
        }
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mock => write!(f, "mock"),
            Self::Gemini => write!(f, "gemini"),
            Self::OpenAiCompatible => write!(f, "openai-compatible"),
        }
    }
}

impl FromStr for ProviderKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "mock" => Ok(Self::Mock),
            "gemini" => Ok(Self::Gemini),
            "openai-compatible" | "openai_compatible" | "openai" => Ok(Self::OpenAiCompatible),
            other => Err(format!("unsupported provider: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub provider: ProviderKind,
    pub model: String,
    pub fallback_models: Vec<String>,
    pub api_key_env_var: Option<String>,
    pub base_url: Option<String>,
}

impl ProviderConfig {
    pub fn for_provider(provider: ProviderKind) -> Self {
        Self {
            provider,
            model: provider.default_model().to_string(),
            fallback_models: default_fallback_models(provider),
            api_key_env_var: provider.default_api_key_env_var().map(str::to_string),
            base_url: None,
        }
    }

    pub fn from_env() -> Result<Self, AiAdapterError> {
        let provider = match env::var(DEFAULT_PROVIDER_ENV_VAR) {
            Ok(value) => ProviderKind::from_str(&value)
                .map_err(|error| AiAdapterError::InvalidProviderConfig(error.to_string()))?,
            Err(_) => ProviderKind::Mock,
        };
        let mut config = Self::for_provider(provider);

        if let Ok(model) = env::var(DEFAULT_MODEL_ENV_VAR)
            && !model.trim().is_empty()
        {
            config.model = model;
        }

        if let Ok(env_var) = env::var(DEFAULT_API_KEY_ENV_VAR)
            && !env_var.trim().is_empty()
        {
            config.api_key_env_var = Some(env_var);
        }

        if let Ok(base_url) = env::var(DEFAULT_BASE_URL_ENV_VAR)
            && !base_url.trim().is_empty()
        {
            config.base_url = Some(base_url);
        }

        if let Ok(fallback_models) = env::var(DEFAULT_FALLBACK_MODELS_ENV_VAR) {
            let parsed = parse_fallback_models(&fallback_models);
            if !parsed.is_empty() {
                config.fallback_models = parsed;
            }
        }

        Ok(config)
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_fallback_models(mut self, models: Vec<String>) -> Self {
        self.fallback_models = models;
        self
    }

    pub fn with_api_key_env_var(mut self, env_var: impl Into<String>) -> Self {
        self.api_key_env_var = Some(env_var.into());
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn resolved_api_key(&self) -> Result<Option<String>, AiAdapterError> {
        let Some(env_var) = &self.api_key_env_var else {
            return Ok(None);
        };

        match env::var(env_var) {
            Ok(value) if !value.trim().is_empty() => Ok(Some(value)),
            Ok(_) | Err(env::VarError::NotPresent) => Err(AiAdapterError::MissingApiKey {
                provider: self.provider,
                env_var: env_var.clone(),
            }),
            Err(env::VarError::NotUnicode(_)) => Err(AiAdapterError::InvalidProviderConfig(
                format!("environment variable {env_var} contains invalid unicode"),
            )),
        }
    }
}

pub trait SceneGenerator {
    fn generate_scene_from_prompt(&self, prompt: &str) -> Result<GeneratedScene, AiAdapterError>;
}

pub fn generate_scene_from_prompt(prompt: &str) -> Result<GeneratedScene, AiAdapterError> {
    let config = ProviderConfig::from_env()?;
    generate_scene_from_prompt_with_config(&config, prompt)
}

pub fn generate_scene_from_prompt_with_config(
    config: &ProviderConfig,
    prompt: &str,
) -> Result<GeneratedScene, AiAdapterError> {
    match config.provider {
        ProviderKind::Mock => MockProvider.generate_scene_from_prompt(prompt),
        ProviderKind::Gemini => {
            GeminiProvider::new(config.clone())?.generate_scene_from_prompt(prompt)
        }
        ProviderKind::OpenAiCompatible => {
            OpenAiCompatibleProvider::new(config.clone())?.generate_scene_from_prompt(prompt)
        }
    }
}

pub fn generate_scene_with_provider(
    provider: &dyn SceneGenerator,
    prompt: &str,
) -> Result<GeneratedScene, AiAdapterError> {
    provider.generate_scene_from_prompt(prompt)
}

#[derive(Debug, Clone, PartialEq)]
pub enum AiAdapterError {
    UnsupportedPrompt(String),
    ParseFailed(String),
    MissingDocument,
    InvalidDocument(Vec<ValidationIssue>),
    InvalidProviderConfig(String),
    MissingApiKey {
        provider: ProviderKind,
        env_var: String,
    },
    HttpFailed(String),
    ApiResponseFailed(String),
    ProviderNotImplemented {
        provider: ProviderKind,
        details: String,
    },
}

impl fmt::Display for AiAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPrompt(prompt) => {
                write!(f, "no AI response is configured for prompt: {prompt}")
            }
            Self::ParseFailed(error) => write!(f, "failed to parse AI response: {error}"),
            Self::MissingDocument => write!(f, "AI response did not include a document"),
            Self::InvalidDocument(issues) => write!(f, "AI document was invalid: {issues:?}"),
            Self::InvalidProviderConfig(error) => write!(f, "invalid AI provider config: {error}"),
            Self::MissingApiKey { provider, env_var } => {
                write!(
                    f,
                    "{provider} requires an API key in environment variable {env_var}"
                )
            }
            Self::HttpFailed(error) => write!(f, "AI HTTP request failed: {error}"),
            Self::ApiResponseFailed(error) => write!(f, "AI provider returned an error: {error}"),
            Self::ProviderNotImplemented { provider, details } => {
                write!(f, "{provider} provider is not implemented yet: {details}")
            }
        }
    }
}

impl std::error::Error for AiAdapterError {}

struct MockProvider;

impl SceneGenerator for MockProvider {
    fn generate_scene_from_prompt(&self, prompt: &str) -> Result<GeneratedScene, AiAdapterError> {
        let response = mock_response_for_prompt(prompt)?;
        validate_generated_response(response)
    }
}

struct GeminiProvider {
    config: ProviderConfig,
}

impl GeminiProvider {
    fn new(config: ProviderConfig) -> Result<Self, AiAdapterError> {
        if config.model.trim().is_empty() {
            return Err(AiAdapterError::InvalidProviderConfig(
                "gemini model name cannot be empty".to_string(),
            ));
        }
        Ok(Self { config })
    }
}

impl SceneGenerator for GeminiProvider {
    fn generate_scene_from_prompt(&self, prompt: &str) -> Result<GeneratedScene, AiAdapterError> {
        let api_key = self.config.resolved_api_key()?.ok_or_else(|| {
            AiAdapterError::InvalidProviderConfig("gemini API key resolution failed".to_string())
        })?;
        generate_gemini_scene_with_fallback(&self.config, &api_key, prompt)
    }
}

struct OpenAiCompatibleProvider {
    config: ProviderConfig,
}

impl OpenAiCompatibleProvider {
    fn new(config: ProviderConfig) -> Result<Self, AiAdapterError> {
        if config.model.trim().is_empty() {
            return Err(AiAdapterError::InvalidProviderConfig(
                "openai-compatible model name cannot be empty".to_string(),
            ));
        }
        Ok(Self { config })
    }
}

impl SceneGenerator for OpenAiCompatibleProvider {
    fn generate_scene_from_prompt(&self, _prompt: &str) -> Result<GeneratedScene, AiAdapterError> {
        let _api_key = self.config.resolved_api_key()?;
        Err(AiAdapterError::ProviderNotImplemented {
            provider: ProviderKind::OpenAiCompatible,
            details: format!(
                "configured for model {}. This is the extension seam for OpenAI-compatible providers.",
                self.config.model
            ),
        })
    }
}

fn validate_generated_response(
    response: AiSceneResponse,
) -> Result<GeneratedScene, AiAdapterError> {
    let document = response
        .document
        .clone()
        .ok_or(AiAdapterError::MissingDocument)?;
    let mut issues = validate_scene(&document);
    issues.extend(validate_generated_scene_quality(&document));
    if !issues.is_empty() {
        return Err(AiAdapterError::InvalidDocument(issues));
    }

    Ok(GeneratedScene { response, issues })
}

fn validate_generated_scene_quality(scene: &SceneFile) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if scene.document.root.children.is_empty() {
        issues.push(ValidationIssue {
            path: "document.root.children".to_string(),
            message: "generated scene must contain at least one child node under the root".to_string(),
        });
    }

    issues
}

fn mock_response_for_prompt(prompt: &str) -> Result<AiSceneResponse, AiAdapterError> {
    let normalized = normalize_prompt(prompt);
    if normalized.contains("pelican") && normalized.contains("bicycle") {
        return response_from_scene_json(
            PELICAN_BICYCLE,
            "A playful poster-like scene of a pelican riding a bicycle.",
            vec![
                "Uses native ellipses and paths for the bike and bird body masses.",
                "Keeps text as editable scene nodes instead of rasterizing the title.",
            ],
        );
    }

    if normalized.contains("poster") {
        return response_from_scene_json(
            BASIC_POSTER,
            "A graphic poster scene with a bold title and soft background card.",
            vec!["Uses a text node for the headline and a rectangle for the main field."],
        );
    }

    Err(AiAdapterError::UnsupportedPrompt(prompt.to_string()))
}

fn response_from_scene_json(
    scene_json: &str,
    summary: &str,
    notes: Vec<&str>,
) -> Result<AiSceneResponse, AiAdapterError> {
    let document = parse_scene_str(scene_json)
        .map_err(|error| AiAdapterError::ParseFailed(error.to_string()))?;

    Ok(AiSceneResponse {
        mode: ResponseMode::FullDocument,
        summary: summary.to_string(),
        document: Some(document),
        notes: notes.into_iter().map(str::to_string).collect(),
    })
}

fn normalize_prompt(prompt: &str) -> String {
    prompt
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character.is_whitespace() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
}

fn generate_gemini_scene_with_fallback(
    config: &ProviderConfig,
    api_key: &str,
    prompt: &str,
) -> Result<GeneratedScene, AiAdapterError> {
    let mut last_error = None;
    for model in gemini_model_attempts(config) {
        let mut repair_feedback = None;

        for _ in 0..2 {
            match request_gemini_scene(config, api_key, prompt, &model, repair_feedback.as_deref())
            {
                Ok(response) => match validate_generated_response(response) {
                    Ok(generated) => return Ok(generated),
                    Err(error) if should_retry_same_model_with_feedback(&error, &repair_feedback) => {
                        repair_feedback = Some(build_repair_feedback(&error));
                        last_error = Some(error);
                    }
                    Err(error) if is_retryable_gemini_error(&error) => {
                        last_error = Some(error);
                        break;
                    }
                    Err(error) => return Err(error),
                },
                Err(error) if should_retry_same_model_with_feedback(&error, &repair_feedback) => {
                    repair_feedback = Some(build_repair_feedback(&error));
                    last_error = Some(error);
                }
                Err(error) if is_retryable_gemini_error(&error) => {
                    last_error = Some(error);
                    break;
                }
                Err(error) => return Err(error),
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        AiAdapterError::ApiResponseFailed("Gemini fallback chain exhausted".to_string())
    }))
}

fn request_gemini_scene(
    config: &ProviderConfig,
    api_key: &str,
    prompt: &str,
    model: &str,
    repair_feedback: Option<&str>,
) -> Result<AiSceneResponse, AiAdapterError> {
    let endpoint = gemini_endpoint(config, model);
    let request = GeminiGenerateContentRequest {
        system_instruction: GeminiContent {
            parts: vec![GeminiTextPart {
                text: gemini_system_instruction(model),
            }],
        },
        contents: vec![GeminiContent {
            parts: vec![GeminiTextPart {
                text: gemini_user_prompt(prompt, repair_feedback),
            }],
        }],
        generation_config: GeminiGenerationConfig {
            response_mime_type: "application/json".to_string(),
            response_json_schema: response_envelope_schema(),
            temperature: Some(0.4),
        },
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|error| AiAdapterError::HttpFailed(error.to_string()))?;

    let http_response = client
        .post(endpoint)
        .header("x-goog-api-key", api_key)
        .json(&request)
        .send()
        .map_err(|error| AiAdapterError::HttpFailed(error.to_string()))?;

    let status = http_response.status();
    let response: GeminiGenerateContentResponse = http_response
        .json()
        .map_err(|error| AiAdapterError::ParseFailed(error.to_string()))?;

    if !status.is_success() {
        if let Some(error) = response.error {
            return Err(AiAdapterError::ApiResponseFailed(format!(
                "{} ({}) via {}",
                error.message, error.status, model
            )));
        }
        return Err(AiAdapterError::ApiResponseFailed(format!(
            "Gemini returned HTTP status {status} via {model}"
        )));
    }

    if let Some(error) = response.error {
        return Err(AiAdapterError::ApiResponseFailed(format!(
            "{} ({}) via {}",
            error.message, error.status, model
        )));
    }

    let json_text = response
        .candidates
        .into_iter()
        .find_map(|candidate| candidate.content)
        .and_then(|content| {
            let combined = content
                .parts
                .into_iter()
                .filter_map(|part| part.text)
                .collect::<String>();
            if combined.trim().is_empty() {
                None
            } else {
                Some(combined)
            }
        })
        .ok_or_else(|| {
            AiAdapterError::ApiResponseFailed(
                format!("Gemini response did not include any JSON text parts via {model}"),
            )
        })?;

    parse_ai_scene_response(&json_text)
}

fn parse_ai_scene_response(json_text: &str) -> Result<AiSceneResponse, AiAdapterError> {
    let mut raw: RawAiSceneResponse =
        serde_json::from_str(json_text).map_err(|error| AiAdapterError::ParseFailed(error.to_string()))?;
    let document = raw
        .document
        .take()
        .map(repair_scene_document_value)
        .map(parse_scene_value)
        .transpose()?;

    Ok(AiSceneResponse {
        mode: raw.mode,
        summary: raw.summary,
        document,
        notes: raw.notes,
    })
}

fn parse_scene_value(value: serde_json::Value) -> Result<SceneFile, AiAdapterError> {
    serde_json::from_value(value).map_err(|error| AiAdapterError::ParseFailed(error.to_string()))
}

fn repair_scene_document_value(value: serde_json::Value) -> serde_json::Value {
    let mut root = match value {
        serde_json::Value::Object(map) => map,
        other => return other,
    };

    if !root.contains_key("version") {
        root.insert(
            "version".to_string(),
            serde_json::Value::String("0.1".to_string()),
        );
    }

    serde_json::Value::Object(root)
}

fn gemini_endpoint(config: &ProviderConfig, model: &str) -> String {
    let base = config
        .base_url
        .as_deref()
        .unwrap_or(DEFAULT_GEMINI_BASE_URL)
        .trim_end_matches('/');
    format!("{base}/models/{model}:generateContent")
}

fn gemini_system_instruction(model: &str) -> String {
    format!(
        concat!(
            "Target model: {}.\n",
            "You generate tweaky scene documents.\n",
            "Return JSON only.\n",
            "Return an object with keys mode, summary, document, and notes.\n",
            "mode must be full_document.\n",
            "document must be a complete tweaky scene JSON document that follows this schema.\n",
            "Do not return placeholders, empty documents, or empty root children arrays.\n",
            "The root group must contain multiple meaningful drawable child nodes.\n",
            "Do not include markdown fences or prose outside JSON.\n",
            "Prefer the node types Group, Rectangle, Ellipse, Path, Text, and ImageLayer.\n",
            "Use named nodes, stable ids, explicit transforms, and editable structure.\n",
            "If painterly detail is difficult to represent structurally, use ImageLayer only when necessary.\n",
            "Schema:\n{}\n"
        ),
        model,
        SCENE_DOCUMENT_SCHEMA
    )
}

fn gemini_user_prompt(prompt: &str, repair_feedback: Option<&str>) -> String {
    let repair_block = repair_feedback
        .map(|feedback| {
            format!(
                concat!(
                    "\nRepair feedback from the previous attempt:\n",
                    "{}\n",
                    "Fix that issue directly in this attempt.\n"
                ),
                feedback
            )
        })
        .unwrap_or_default();

    format!(
        concat!(
            "Create a new tweaky scene document for this request:\n",
            "{}\n\n",
            "Canvas guidance:\n",
            "- use a reasonable poster-like canvas size\n",
            "- include a complete root hierarchy\n",
            "- the root must include multiple non-empty drawable child nodes\n",
            "- do not leave `children` empty and do not return a placeholder scene\n",
            "- keep the result funny, editable, and visually readable\n",
            "- notes should briefly explain key scene construction choices\n\n",
            "Here are valid example tweaky scenes. Match their structural completeness and naming quality.\n\n",
            "Example 1: playful structured poster\n{}\n\n",
            "Example 2: hybrid structured plus raster scene\n{}\n",
            "{}"
        ),
        prompt
        , PELICAN_BICYCLE
        , HYBRID_SCENE
        , repair_block
    )
}

fn response_envelope_schema() -> serde_json::Value {
    let document_schema: serde_json::Value = serde_json::from_str(SCENE_DOCUMENT_SCHEMA)
        .unwrap_or_else(|_| serde_json::json!({ "type": "object" }));
    serde_json::json!({
        "type": "object",
        "properties": {
            "mode": {
                "type": "string",
                "enum": ["full_document"]
            },
            "summary": {
                "type": "string"
            },
            "document": document_schema,
            "notes": {
                "type": "array",
                "items": {
                    "type": "string"
                }
            }
        },
        "required": ["mode", "summary", "document", "notes"]
    })
}

fn default_fallback_models(provider: ProviderKind) -> Vec<String> {
    match provider {
        ProviderKind::Gemini => vec![DEFAULT_GEMINI_FALLBACK_MODEL.to_string()],
        ProviderKind::Mock | ProviderKind::OpenAiCompatible => Vec::new(),
    }
}

fn parse_fallback_models(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
        .collect()
}

fn gemini_model_attempts(config: &ProviderConfig) -> Vec<String> {
    let mut models = vec![config.model.clone()];
    for fallback in &config.fallback_models {
        if !models.iter().any(|existing| existing == fallback) {
            models.push(fallback.clone());
        }
    }
    models
}

fn is_retryable_gemini_error(error: &AiAdapterError) -> bool {
    match error {
        AiAdapterError::ApiResponseFailed(message) => {
            message.contains("UNAVAILABLE")
                || message.contains("RESOURCE_EXHAUSTED")
                || message.contains("DEADLINE_EXCEEDED")
        }
        AiAdapterError::HttpFailed(_) => true,
        AiAdapterError::ParseFailed(_) => true,
        AiAdapterError::InvalidDocument(_) => true,
        _ => false,
    }
}

fn should_retry_same_model_with_feedback(
    error: &AiAdapterError,
    repair_feedback: &Option<String>,
) -> bool {
    if repair_feedback.is_some() {
        return false;
    }

    matches!(
        error,
        AiAdapterError::ParseFailed(_)
            | AiAdapterError::MissingDocument
            | AiAdapterError::InvalidDocument(_)
    )
}

fn build_repair_feedback(error: &AiAdapterError) -> String {
    match error {
        AiAdapterError::ParseFailed(message) => format!(
            concat!(
                "Your previous JSON could not be parsed into a valid tweaky response. ",
                "Return strict JSON only and ensure the `document` field is a complete scene document. ",
                "Parser message: {}"
            ),
            message
        ),
        AiAdapterError::MissingDocument => {
            "Your previous response omitted the `document` field. Return a full scene document in the `document` key.".to_string()
        }
        AiAdapterError::InvalidDocument(issues) => format!(
            concat!(
                "Your previous scene document failed validation. ",
                "Return a complete valid tweaky scene document with all required fields and compatible node properties. ",
                "Validation issues: {:?}"
            ),
            issues
        ),
        other => format!("Repair the previous attempt. Error: {other}"),
    }
}

#[derive(Debug, Serialize)]
struct GeminiGenerateContentRequest {
    system_instruction: GeminiContent,
    contents: Vec<GeminiContent>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiTextPart>,
}

#[derive(Debug, Serialize)]
struct GeminiTextPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    response_mime_type: String,
    response_json_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct GeminiGenerateContentResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    error: Option<GeminiErrorPayload>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiCandidateContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidateContent {
    #[serde(default)]
    parts: Vec<GeminiCandidatePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidatePart {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorPayload {
    #[serde(default)]
    message: String,
    #[serde(default)]
    status: String,
}

#[cfg(test)]
mod tests {
    use super::{
        AiAdapterError, DEFAULT_GEMINI_BASE_URL, DEFAULT_GEMINI_FALLBACK_MODEL, GeneratedScene,
        ProviderConfig, ProviderKind, ResponseMode, gemini_endpoint, gemini_model_attempts,
        gemini_user_prompt, generate_scene_from_prompt_with_config, parse_ai_scene_response,
    };
    use std::env;

    fn assert_pelican_scene(generated: GeneratedScene) {
        assert_eq!(generated.response.mode, ResponseMode::FullDocument);
        assert!(generated.issues.is_empty());
        assert_eq!(
            generated
                .response
                .document
                .as_ref()
                .expect("document should exist")
                .document
                .name,
            "Pelican Riding a Bicycle"
        );
    }

    #[test]
    fn mock_provider_generates_pelican_bicycle_scene() {
        let generated = generate_scene_from_prompt_with_config(
            &ProviderConfig::for_provider(ProviderKind::Mock),
            "a drawing of a pelican riding a bicycle",
        )
        .expect("mock generation should work");

        assert_pelican_scene(generated);
    }

    #[test]
    fn rejects_unknown_prompt() {
        let error = generate_scene_from_prompt_with_config(
            &ProviderConfig::for_provider(ProviderKind::Mock),
            "mysterious crab cathedral",
        )
        .expect_err("unknown prompt should fail");
        assert!(error.to_string().contains("no AI response"));
    }

    #[test]
    fn provider_defaults_match_expected_models() {
        let gemini = ProviderConfig::for_provider(ProviderKind::Gemini);
        assert_eq!(gemini.model, "gemini-2.5-flash");
        assert_eq!(
            gemini.fallback_models,
            vec![DEFAULT_GEMINI_FALLBACK_MODEL.to_string()]
        );
        assert_eq!(gemini.api_key_env_var.as_deref(), Some("GEMINI_API_KEY"));
    }

    #[test]
    fn gemini_provider_requires_api_key() {
        let config = ProviderConfig::for_provider(ProviderKind::Gemini);
        if let Some(env_var) = &config.api_key_env_var {
            unsafe {
                env::remove_var(env_var);
            }
        }

        let error = generate_scene_from_prompt_with_config(&config, "hello")
            .expect_err("gemini should require an API key");

        assert!(matches!(error, AiAdapterError::MissingApiKey { .. }));
    }

    #[test]
    fn gemini_endpoint_uses_default_base_url() {
        let config = ProviderConfig::for_provider(ProviderKind::Gemini);
        assert_eq!(
            gemini_endpoint(&config, &config.model),
            format!("{DEFAULT_GEMINI_BASE_URL}/models/gemini-2.5-flash:generateContent")
        );
    }

    #[test]
    fn gemini_attempts_primary_then_fallback() {
        let config = ProviderConfig::for_provider(ProviderKind::Gemini)
            .with_model("gemini-2.5-flash".to_string())
            .with_fallback_models(vec![
                "gemini-3.1-flash-lite-preview".to_string(),
                "gemini-3.1-flash-lite-preview".to_string(),
            ]);
        assert_eq!(
            gemini_model_attempts(&config),
            vec![
                "gemini-2.5-flash".to_string(),
                "gemini-3.1-flash-lite-preview".to_string()
            ]
        );
    }

    #[test]
    fn repairs_missing_scene_version_in_ai_response() {
        let response = parse_ai_scene_response(
            r##"{
              "mode": "full_document",
              "summary": "Pelican test",
              "document": {
                "document": {
                  "id": "scene_1",
                  "name": "Pelican Test",
                  "width": 1200,
                  "height": 900,
                  "background": { "type": "solid", "color": "#ffffff" },
                  "resources": { "images": {}, "fonts": {}, "palettes": {} },
                  "root": {
                    "id": "root",
                    "type": "Group",
                    "name": "Root",
                    "visible": true,
                    "locked": false,
                    "blendMode": "normal",
                    "transform": {
                      "x": 0.0,
                      "y": 0.0,
                      "scaleX": 1.0,
                      "scaleY": 1.0,
                      "rotation": 0.0,
                      "opacity": 1.0
                    },
                    "params": {},
                    "style": {},
                    "children": [],
                    "meta": {}
                  }
                }
              },
              "notes": []
            }"##,
        )
        .expect("response should parse after repair");

        assert_eq!(
            response.document.expect("document should exist").version,
            "0.1"
        );
    }

    #[test]
    fn gemini_prompt_includes_examples_and_feedback() {
        let prompt = gemini_user_prompt(
            "a drawing of a pelican riding a bicycle",
            Some("missing document"),
        );

        assert!(prompt.contains("Example 1: playful structured poster"));
        assert!(prompt.contains("Example 2: hybrid structured plus raster scene"));
        assert!(prompt.contains("missing document"));
    }
}
