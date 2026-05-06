use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use scene_schema::{
    BlendMode, JsonObject, SceneFile, SceneNode, Transform, ValidationIssue, parse_scene_str,
    validate_scene,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PELICAN_BICYCLE: &str = include_str!("../../../examples/pelican_bicycle.vsd.json");
const BASIC_POSTER: &str = include_str!("../../../examples/basic_poster.vsd.json");
const HYBRID_SCENE: &str = include_str!("../../../examples/hybrid_scene.vsd.json");
const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_GEMINI_FALLBACK_MODEL: &str = "gemini-2.5-flash-lite";
const DEFAULT_GEMMA_FALLBACK_MODEL: &str = "gemma-4-31b-it";

pub const DEFAULT_PROVIDER_ENV_VAR: &str = "TWEAKY_AI_PROVIDER";
pub const DEFAULT_MODEL_ENV_VAR: &str = "TWEAKY_AI_MODEL";
pub const DEFAULT_API_KEY_ENV_VAR: &str = "TWEAKY_AI_API_KEY_ENV";
pub const DEFAULT_BASE_URL_ENV_VAR: &str = "TWEAKY_AI_BASE_URL";
pub const DEFAULT_FALLBACK_MODELS_ENV_VAR: &str = "TWEAKY_AI_FALLBACK_MODELS";
pub const DISABLE_FALLBACK_ENV_VAR: &str = "TWEAKY_AI_DISABLE_FALLBACK";
pub const DEFAULT_TRACE_DIR_ENV_VAR: &str = "TWEAKY_AI_TRACE_DIR";
pub const DEFAULT_PLAN_CACHE_DIR_ENV_VAR: &str = "TWEAKY_AI_PLAN_CACHE_DIR";

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);
static GEMINI_RATE_LIMITER: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenePlan {
    pub summary: String,
    pub canvas: ScenePlanCanvas,
    pub style_keywords: Vec<String>,
    pub major_nodes: Vec<ScenePlanNode>,
    pub composition_notes: Vec<String>,
    #[serde(default = "default_plan_hierarchy_root")]
    pub hierarchy: ScenePlanHierarchyNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenePlanCanvas {
    pub width: f64,
    pub height: f64,
    pub background: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenePlanNode {
    pub id: String,
    pub node_type: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenePlanHierarchyNode {
    pub id: String,
    pub role: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub children: Vec<ScenePlanHierarchyNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneCritique {
    pub satisfactory: bool,
    pub summary: String,
    pub strengths: Vec<String>,
    pub issues: Vec<String>,
    pub revision_goals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneOperationBatch {
    pub summary: String,
    pub operations: Vec<SceneOperation>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SceneOperation {
    UpsertImageResource {
        image_ref: String,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        prompt: Option<String>,
        #[serde(default)]
        width: Option<f64>,
        #[serde(default)]
        height: Option<f64>,
        #[serde(default)]
        alpha_mode: Option<String>,
        #[serde(default)]
        generation_mode: Option<String>,
        #[serde(default)]
        group_id: Option<String>,
    },
    CreateGroup {
        node_id: String,
        parent_id: String,
        name: String,
        x: f64,
        y: f64,
    },
    CreateRectangle {
        node_id: String,
        parent_id: String,
        name: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        #[serde(default)]
        corner_radius: Option<f64>,
        #[serde(default)]
        fill: Option<String>,
    },
    CreateEllipse {
        node_id: String,
        parent_id: String,
        name: String,
        x: f64,
        y: f64,
        radius_x: f64,
        radius_y: f64,
        #[serde(default)]
        fill: Option<String>,
    },
    CreatePath {
        node_id: String,
        parent_id: String,
        name: String,
        x: f64,
        y: f64,
        points: Vec<SceneOpPoint>,
        #[serde(default)]
        closed: Option<bool>,
        #[serde(default)]
        fill: Option<String>,
    },
    CreateText {
        node_id: String,
        parent_id: String,
        name: String,
        x: f64,
        y: f64,
        text: String,
        font_size: f64,
        #[serde(default)]
        font_family: Option<String>,
        #[serde(default)]
        line_height: Option<f64>,
        #[serde(default)]
        max_width: Option<f64>,
        #[serde(default)]
        align: Option<String>,
        #[serde(default)]
        fill: Option<String>,
    },
    CreateImageLayer {
        node_id: String,
        parent_id: String,
        name: String,
        x: f64,
        y: f64,
        image_ref: String,
        display_width: f64,
        display_height: f64,
    },
    SetTransform {
        node_id: String,
        #[serde(default)]
        x: Option<f64>,
        #[serde(default)]
        y: Option<f64>,
        #[serde(default)]
        scale_x: Option<f64>,
        #[serde(default)]
        scale_y: Option<f64>,
        #[serde(default)]
        rotation: Option<f64>,
        #[serde(default)]
        opacity: Option<f64>,
    },
    ReplaceParams {
        node_id: String,
        params: JsonObject,
    },
    ReplaceStyle {
        node_id: String,
        style: JsonObject,
    },
    DeleteNode {
        node_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneOpPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneTemplateKind {
    Poster,
    Shapes,
    Hybrid,
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

        Ok(config.apply_env_toggles())
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

    pub fn apply_env_toggles(mut self) -> Self {
        if env_var_truthy(DISABLE_FALLBACK_ENV_VAR) {
            self.fallback_models.clear();
        }
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
            message: "generated scene must contain at least one child node under the root"
                .to_string(),
        });
    }

    if scene.document.root.children.is_empty() {
        let suspicious_resource_keys = suspicious_resource_keys(scene);
        if !suspicious_resource_keys.is_empty() {
            issues.push(ValidationIssue {
                path: "document.resources".to_string(),
                message: format!(
                    concat!(
                        "generated scene appears to have placed drawable object ids in resources ",
                        "instead of instantiating scene nodes under document.root.children. ",
                        "Suspicious resource keys: {:?}"
                    ),
                    suspicious_resource_keys
                ),
            });
        }
    }

    issues
}

fn suspicious_resource_keys(scene: &SceneFile) -> Vec<String> {
    let mut keys = Vec::new();

    for (key, value) in &scene.document.resources.images {
        if resource_key_looks_like_scene_node(key, value) {
            keys.push(format!("images.{key}"));
        }
    }

    for (key, value) in &scene.document.resources.fonts {
        if resource_key_looks_like_scene_node(key, value) {
            keys.push(format!("fonts.{key}"));
        }
    }

    for (key, value) in &scene.document.resources.palettes {
        if resource_key_looks_like_scene_node(key, value) {
            keys.push(format!("palettes.{key}"));
        }
    }

    keys
}

fn resource_key_looks_like_scene_node(key: &str, value: &serde_json::Value) -> bool {
    let normalized = key.to_lowercase();
    let name_looks_like_node = normalized.contains("pelican")
        || normalized.contains("bicycle")
        || normalized.contains("wheel")
        || normalized.contains("frame")
        || normalized.contains("head")
        || normalized.contains("body")
        || normalized.contains("beak")
        || normalized.contains("headline")
        || normalized.contains("tagline");

    if !name_looks_like_node {
        return false;
    }

    value.as_object().is_some_and(|object| object.is_empty())
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

fn template_for_prompt(prompt: &str) -> SceneTemplateKind {
    let normalized = normalize_prompt(prompt);
    if normalized.contains("paint") || normalized.contains("brush") || normalized.contains("hybrid")
    {
        return SceneTemplateKind::Hybrid;
    }

    if normalized.contains("study")
        || normalized.contains("geometric")
        || normalized.contains("shape")
    {
        return SceneTemplateKind::Shapes;
    }

    SceneTemplateKind::Poster
}

fn template_scene_json(kind: SceneTemplateKind) -> &'static str {
    match kind {
        SceneTemplateKind::Poster => BASIC_POSTER,
        SceneTemplateKind::Shapes => include_str!("../../../examples/shapes_study.vsd.json"),
        SceneTemplateKind::Hybrid => HYBRID_SCENE,
    }
}

fn template_name(kind: SceneTemplateKind) -> &'static str {
    match kind {
        SceneTemplateKind::Poster => "poster",
        SceneTemplateKind::Shapes => "shapes",
        SceneTemplateKind::Hybrid => "hybrid",
    }
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

fn default_plan_hierarchy_root() -> ScenePlanHierarchyNode {
    ScenePlanHierarchyNode {
        id: "root".to_string(),
        role: "group".to_string(),
        label: Some("Root".to_string()),
        purpose: Some("Top-level scene hierarchy root".to_string()),
        children: Vec::new(),
    }
}

fn generate_gemini_scene_with_fallback(
    config: &ProviderConfig,
    api_key: &str,
    prompt: &str,
) -> Result<GeneratedScene, AiAdapterError> {
    let template_kind = template_for_prompt(prompt);
    let template_scene = template_scene_json(template_kind);

    if let Some(generated) =
        try_staged_template_generation(config, api_key, prompt, template_kind, template_scene)?
    {
        return Ok(generated);
    }

    Err(AiAdapterError::ApiResponseFailed(
        "Staged slot generation exhausted every configured model".to_string(),
    ))
}

fn try_staged_template_generation(
    config: &ProviderConfig,
    api_key: &str,
    prompt: &str,
    template_kind: SceneTemplateKind,
    template_scene: &str,
) -> Result<Option<GeneratedScene>, AiAdapterError> {
    if template_kind != SceneTemplateKind::Poster {
        return Ok(None);
    }
    let scaffold = build_poster_scaffold_scene(template_scene)?;
    for model in gemini_model_attempts(config) {
        let plan = match request_gemini_scene_plan(
            config,
            api_key,
            prompt,
            template_kind,
            template_scene,
            &model,
        ) {
            Ok(plan) => plan,
            Err(error) if is_retryable_gemini_error(&error) => continue,
            Err(error) => return Err(error),
        };

        let mut working_scene = scaffold.clone();
        let mut completed_stages = Vec::new();
        apply_plan_canvas_to_scene(&mut working_scene, &plan);
        materialize_plan_hierarchy(&mut working_scene, &plan);
        let stages = derive_stages_from_plan(&plan);
        let mut stage_failed = false;

        for stage in &stages {
            let mut repair_feedback = None;
            let mut stage_complete = false;
            let mut best_effort_candidate = None;

            for _ in 0..2 {
                match request_stage_operations(
                    config,
                    api_key,
                    prompt,
                    model.as_str(),
                    stage,
                    &working_scene,
                    &plan,
                    repair_feedback.as_deref(),
                ) {
                    Ok(stage_response) => {
                        match validate_stage_operations(&working_scene, &stage_response, stage) {
                            Ok(()) => {
                                let mut candidate_scene = working_scene.clone();
                                let apply_issues = apply_scene_operations_to_scene(
                                    &mut candidate_scene,
                                    &stage_response.operations,
                                );
                                if !apply_issues.is_empty() {
                                    let error = AiAdapterError::InvalidDocument(apply_issues);
                                    if should_retry_same_model_with_feedback(
                                        &error,
                                        &repair_feedback,
                                    ) {
                                        repair_feedback = Some(build_repair_feedback(&error));
                                        continue;
                                    }
                                    if is_retryable_gemini_error(&error) {
                                        stage_failed = true;
                                        break;
                                    }
                                    return Err(error);
                                }
                                best_effort_candidate = Some(candidate_scene.clone());
                                match critique_stage_and_maybe_request_retry(
                                    config,
                                    api_key,
                                    prompt,
                                    &candidate_scene,
                                    &plan,
                                    stage,
                                    &model,
                                ) {
                                    Ok(Some(feedback)) => {
                                        repair_feedback = Some(feedback);
                                    }
                                    Ok(None) => {
                                        working_scene = candidate_scene;
                                        completed_stages.push(stage.clone());
                                        stage_complete = true;
                                        break;
                                    }
                                    Err(error)
                                        if should_retry_same_model_with_feedback(
                                            &error,
                                            &repair_feedback,
                                        ) =>
                                    {
                                        repair_feedback = Some(build_repair_feedback(&error));
                                    }
                                    Err(error) if is_retryable_gemini_error(&error) => {
                                        if let Some(candidate_scene) = best_effort_candidate.take()
                                        {
                                            working_scene = candidate_scene;
                                            completed_stages.push(stage.clone());
                                            stage_complete = true;
                                        } else {
                                            stage_failed = true;
                                        }
                                        break;
                                    }
                                    Err(error) => return Err(error),
                                }
                            }
                            Err(error)
                                if should_retry_same_model_with_feedback(
                                    &error,
                                    &repair_feedback,
                                ) =>
                            {
                                repair_feedback = Some(build_repair_feedback(&error));
                            }
                            Err(error) if is_retryable_gemini_error(&error) => {
                                stage_failed = true;
                                break;
                            }
                            Err(_) => {
                                stage_failed = true;
                                break;
                            }
                        }
                    }
                    Err(error)
                        if should_retry_same_model_with_feedback(&error, &repair_feedback) =>
                    {
                        repair_feedback = Some(build_repair_feedback(&error));
                    }
                    Err(error) if is_retryable_gemini_error(&error) => {
                        stage_failed = true;
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }

            if !stage_complete && let Some(candidate_scene) = best_effort_candidate.take() {
                working_scene = candidate_scene;
                completed_stages.push(stage.clone());
                stage_complete = true;
            }

            if !stage_complete {
                stage_failed = true;
                break;
            }
        }

        if stage_failed {
            if let Some(generated) =
                finalize_staged_scene(prompt, &working_scene, &completed_stages, true)?
            {
                return Ok(Some(generated));
            }
            continue;
        }

        if let Some(generated) =
            finalize_staged_scene(prompt, &working_scene, &completed_stages, false)?
        {
            return Ok(Some(generated));
        }
    }

    Ok(None)
}

fn build_poster_scaffold_scene(template_scene: &str) -> Result<SceneFile, AiAdapterError> {
    let mut scene = parse_scene_str(template_scene)
        .map_err(|error| AiAdapterError::ParseFailed(error.to_string()))?;
    scene.document.id = "doc_staged_poster_scaffold".to_string();
    scene.document.name = "Staged Poster Scaffold".to_string();
    scene.document.background.color = "#f7f1df".to_string();
    scene
        .document
        .root
        .children
        .retain(|child| child.id == "bg_rect");
    if let Some(background) = scene.document.root.children.first_mut() {
        background.name = "Background Card".to_string();
    }
    Ok(scene)
}

fn materialize_plan_hierarchy(scene: &mut SceneFile, plan: &ScenePlan) {
    let hierarchy = normalized_plan_hierarchy(plan);
    scene
        .document
        .root
        .children
        .retain(|child| child.id == "bg_rect");
    for child in &hierarchy.children {
        scene
            .document
            .root
            .children
            .push(plan_hierarchy_node_to_scene_group(child));
    }
}

fn plan_hierarchy_node_to_scene_group(node: &ScenePlanHierarchyNode) -> SceneNode {
    let mut meta = JsonObject::new();
    meta.insert(
        "planRole".to_string(),
        serde_json::Value::String(node.role.clone()),
    );

    SceneNode {
        id: node.id.clone(),
        node_type: scene_schema::NodeType::Group,
        name: node
            .label
            .clone()
            .or_else(|| node.purpose.clone())
            .unwrap_or_else(|| node.id.clone()),
        visible: true,
        locked: false,
        blend_mode: BlendMode::Normal,
        transform: default_node_transform(),
        params: JsonObject::new(),
        style: JsonObject::new(),
        children: node
            .children
            .iter()
            .map(plan_hierarchy_node_to_scene_group)
            .collect(),
        meta,
    }
}

fn default_node_transform() -> Transform {
    Transform {
        x: 0.0,
        y: 0.0,
        scale_x: 1.0,
        scale_y: 1.0,
        rotation: 0.0,
        opacity: 1.0,
    }
}

fn finalize_staged_scene(
    prompt: &str,
    scene: &SceneFile,
    completed_stages: &[StageSpec],
    partial: bool,
) -> Result<Option<GeneratedScene>, AiAdapterError> {
    if !can_finalize_staged_scene(completed_stages) {
        return Ok(None);
    }

    let mut notes = vec!["Generated via staged subtree pipeline".to_string()];
    notes.push(format!(
        "Completed stages: {}",
        completed_stages
            .iter()
            .map(|stage| stage.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));

    if partial {
        notes.push("Returned a partial but editable scene after a later stage failed.".to_string());
    }

    let response = AiSceneResponse {
        mode: ResponseMode::FullDocument,
        summary: if partial {
            format!("Partially completed staged poster scene for prompt: {prompt}")
        } else {
            format!("Staged poster scene for prompt: {prompt}")
        },
        document: Some(scene.clone()),
        notes,
    };

    match validate_generated_response(response) {
        Ok(generated) => Ok(Some(generated)),
        Err(AiAdapterError::InvalidDocument(_)) if partial => Ok(None),
        Err(error) => Err(error),
    }
}

fn can_finalize_staged_scene(completed_stages: &[StageSpec]) -> bool {
    completed_stages
        .iter()
        .any(|stage| !matches!(stage.kind, StageKind::Support))
}

#[derive(Debug, Clone)]
enum StageKind {
    Support,
    Subject,
    Text,
    Assembly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StageOutputMode {
    Structured,
    RasterPreferred,
}

#[derive(Debug, Clone)]
struct StageSpec {
    kind: StageKind,
    output_mode: StageOutputMode,
    id: String,
    slot_id: String,
    purpose: String,
    target_node_ids: Vec<String>,
    composition_hints: Vec<String>,
    focus_group_id: Option<String>,
}

fn derive_stages_from_plan(plan: &ScenePlan) -> Vec<StageSpec> {
    let major_node_lookup = plan
        .major_nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();
    let hierarchy = normalized_plan_hierarchy(plan);
    let mut stages = Vec::new();
    collect_stages_from_hierarchy(
        &hierarchy,
        &major_node_lookup,
        &plan.composition_notes,
        &mut stages,
    );

    if stages.is_empty() {
        stages.push(StageSpec {
            kind: StageKind::Subject,
            output_mode: StageOutputMode::Structured,
            id: "primary_subject".to_string(),
            slot_id: "root".to_string(),
            purpose: format!(
                "Add the main editable subject nodes needed to realize this planned scene: {}",
                plan.summary
            ),
            target_node_ids: plan
                .major_nodes
                .iter()
                .map(|node| node.id.clone())
                .collect(),
            composition_hints: plan.composition_notes.iter().take(3).cloned().collect(),
            focus_group_id: None,
        });
    }

    stages.sort_by_key(stage_priority_key);
    stages
}

fn stage_priority_key(stage: &StageSpec) -> (u8, String) {
    let tier = match stage.kind {
        StageKind::Subject => 0,
        StageKind::Assembly => 1,
        StageKind::Text => 2,
        StageKind::Support => 3,
    };
    (tier, stage.id.clone())
}

fn normalized_plan_hierarchy(plan: &ScenePlan) -> ScenePlanHierarchyNode {
    if !plan.hierarchy.children.is_empty() {
        return plan.hierarchy.clone();
    }

    let mut root = default_plan_hierarchy_root();
    let mut support = ScenePlanHierarchyNode {
        id: "support_layer".to_string(),
        role: "group".to_string(),
        label: Some("Support Layer".to_string()),
        purpose: Some("Background and grounding elements".to_string()),
        children: Vec::new(),
    };
    let mut subject = ScenePlanHierarchyNode {
        id: "subject_layer".to_string(),
        role: "group".to_string(),
        label: Some("Subject Layer".to_string()),
        purpose: Some("Primary editable scene subjects".to_string()),
        children: Vec::new(),
    };
    let mut text = ScenePlanHierarchyNode {
        id: "text_layer".to_string(),
        role: "group".to_string(),
        label: Some("Text Layer".to_string()),
        purpose: Some("Text and caption elements".to_string()),
        children: Vec::new(),
    };

    for node in &plan.major_nodes {
        let kind = classify_stage_kind(&node.id, &node.node_type, &node.purpose, &[]);
        let slot = ScenePlanHierarchyNode {
            id: node.id.clone(),
            role: "slot".to_string(),
            label: Some(node.id.clone()),
            purpose: Some(node.purpose.clone()),
            children: Vec::new(),
        };
        match kind {
            StageKind::Support => support.children.push(slot),
            StageKind::Subject | StageKind::Assembly => subject.children.push(slot),
            StageKind::Text => text.children.push(slot),
        }
    }

    if !support.children.is_empty() {
        root.children.push(support);
    }
    if !subject.children.is_empty() {
        root.children.push(subject);
    }
    if !text.children.is_empty() {
        root.children.push(text);
    }

    root
}

fn collect_stages_from_hierarchy(
    node: &ScenePlanHierarchyNode,
    major_nodes: &HashMap<&str, &ScenePlanNode>,
    composition_notes: &[String],
    stages: &mut Vec<StageSpec>,
) {
    if node.role.eq_ignore_ascii_case("slot") {
        let major_node = major_nodes.get(node.id.as_str()).copied();
        let node_type = major_node
            .map(|entry| entry.node_type.as_str())
            .unwrap_or("Group");
        let purpose = node
            .purpose
            .clone()
            .or_else(|| major_node.map(|entry| entry.purpose.clone()))
            .unwrap_or_else(|| format!("Fill slot '{}'", node.id));
        let target_node_ids = vec![node.id.clone()];
        let kind = classify_stage_kind(
            &node.id,
            node_type,
            &purpose,
            &[node.label.clone().unwrap_or_default()],
        );
        let output_mode = classify_stage_output_mode(
            kind.clone(),
            &node.id,
            &purpose,
            &[node.label.clone().unwrap_or_default()],
            None,
        );
        stages.push(StageSpec {
            kind,
            output_mode,
            id: slugify(&node.id),
            slot_id: node.id.clone(),
            purpose: format!("Fill the planned slot '{}'. {}", node.id, purpose),
            target_node_ids,
            composition_hints: composition_notes.iter().take(3).cloned().collect(),
            focus_group_id: None,
        });
        return;
    }

    for child in &node.children {
        collect_stages_from_hierarchy(child, major_nodes, composition_notes, stages);
    }

    if let Some(stage) = group_assembly_stage(node, composition_notes) {
        stages.push(stage);
    }
}

fn classify_stage_kind(
    id: &str,
    node_type: &str,
    purpose: &str,
    extra_descriptors: &[String],
) -> StageKind {
    let descriptor = format!(
        "{} {} {}",
        id.to_lowercase(),
        node_type.to_lowercase(),
        purpose.to_lowercase()
    );
    let extras = extra_descriptors
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let descriptor = format!("{descriptor} {extras}");

    if descriptor.contains("text")
        || descriptor.contains("headline")
        || descriptor.contains("title")
        || descriptor.contains("caption")
        || descriptor.contains("tagline")
    {
        StageKind::Text
    } else if descriptor.contains("background")
        || descriptor.contains("ground")
        || descriptor.contains("panel")
        || descriptor.contains("backdrop")
        || descriptor.contains("sky")
        || descriptor.contains("support")
        || descriptor.contains("shadow")
    {
        StageKind::Support
    } else {
        StageKind::Subject
    }
}

fn classify_stage_output_mode(
    kind: StageKind,
    id: &str,
    purpose: &str,
    extra_descriptors: &[String],
    focus_group_id: Option<&str>,
) -> StageOutputMode {
    if matches!(kind, StageKind::Support | StageKind::Text) {
        return StageOutputMode::Structured;
    }

    let extras = extra_descriptors
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let focus = focus_group_id.unwrap_or_default().to_lowercase();
    let descriptor = format!(
        "{} {} {} {}",
        id.to_lowercase(),
        purpose.to_lowercase(),
        extras,
        focus
    );

    if descriptor.contains("pelican")
        || descriptor.contains("bird")
        || descriptor.contains("character")
        || descriptor.contains("creature")
        || descriptor.contains("portrait")
        || descriptor.contains("head")
        || descriptor.contains("face")
        || descriptor.contains("beak")
        || descriptor.contains("wing")
        || descriptor.contains("body")
        || descriptor.contains("feather")
        || descriptor.contains("fur")
        || descriptor.contains("hair")
    {
        StageOutputMode::RasterPreferred
    } else {
        StageOutputMode::Structured
    }
}

fn group_assembly_stage(
    node: &ScenePlanHierarchyNode,
    composition_notes: &[String],
) -> Option<StageSpec> {
    if node.id == "root" || !node.role.eq_ignore_ascii_case("group") {
        return None;
    }

    let descendant_slot_ids = collect_descendant_slot_ids(node);
    if descendant_slot_ids.len() < 2 {
        return None;
    }

    let descriptors = [node.label.clone().unwrap_or_default()];
    let kind = classify_stage_kind(
        &node.id,
        "Group",
        node.purpose.as_deref().unwrap_or(""),
        &descriptors,
    );

    if matches!(kind, StageKind::Support) {
        return None;
    }

    let label = node.label.clone().unwrap_or_else(|| node.id.clone());
    Some(StageSpec {
        kind: StageKind::Assembly,
        output_mode: classify_stage_output_mode(
            StageKind::Assembly,
            &node.id,
            node.purpose.as_deref().unwrap_or(""),
            &descriptors,
            Some(node.id.as_str()),
        ),
        id: format!("{}_assembly", slugify(&node.id)),
        slot_id: node.id.clone(),
        purpose: format!(
            "Assemble and refine the grouped subject '{}'. Keep the existing child parts, add only the connective or clarifying nodes needed to make the group read as one coherent editable subject.",
            label
        ),
        target_node_ids: descendant_slot_ids,
        composition_hints: composition_notes.iter().take(4).cloned().collect(),
        focus_group_id: Some(node.id.clone()),
    })
}

fn collect_descendant_slot_ids(node: &ScenePlanHierarchyNode) -> Vec<String> {
    let mut ids = Vec::new();
    collect_descendant_slot_ids_into(node, &mut ids);
    ids
}

fn collect_descendant_slot_ids_into(node: &ScenePlanHierarchyNode, ids: &mut Vec<String>) {
    if node.role.eq_ignore_ascii_case("slot") {
        ids.push(node.id.clone());
        return;
    }

    for child in &node.children {
        collect_descendant_slot_ids_into(child, ids);
    }
}

fn slugify(value: &str) -> String {
    let slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    slug.split('_')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn apply_plan_canvas_to_scene(scene: &mut SceneFile, plan: &ScenePlan) {
    scene.document.width = plan.canvas.width.max(1.0);
    scene.document.height = plan.canvas.height.max(1.0);
    scene.document.background.color = normalize_plan_background(&plan.canvas.background);
}

fn normalize_plan_background(background: &str) -> String {
    let trimmed = background.trim();
    if trimmed.starts_with('#') && (trimmed.len() == 7 || trimmed.len() == 9) {
        return trimmed.to_string();
    }

    "#f7f1df".to_string()
}

fn request_stage_operations(
    config: &ProviderConfig,
    api_key: &str,
    prompt: &str,
    model: &str,
    stage: &StageSpec,
    scene: &SceneFile,
    plan: &ScenePlan,
    repair_feedback: Option<&str>,
) -> Result<SceneOperationBatch, AiAdapterError> {
    let endpoint = gemini_endpoint(config, model);
    let request = GeminiGenerateContentRequest {
        system_instruction: GeminiContent {
            parts: vec![GeminiPart::text(gemini_stage_system_instruction(model))],
        },
        contents: vec![GeminiContent {
            parts: vec![GeminiPart::text(gemini_stage_user_prompt(
                prompt,
                stage,
                scene,
                plan,
                repair_feedback,
            ))],
        }],
        generation_config: GeminiGenerationConfig {
            response_mime_type: "application/json".to_string(),
            response_json_schema: scene_operations_schema(),
            temperature: Some(0.35),
        },
    };

    let json_text = send_gemini_request(
        config,
        api_key,
        model,
        &format!("stage_{}", stage.id),
        endpoint,
        &request,
    )?;
    serde_json::from_str::<SceneOperationBatch>(&json_text)
        .map_err(|error| AiAdapterError::ParseFailed(error.to_string()))
}

fn validate_stage_operations(
    scene: &SceneFile,
    batch: &SceneOperationBatch,
    stage: &StageSpec,
) -> Result<(), AiAdapterError> {
    if batch.operations.is_empty() {
        return Err(AiAdapterError::InvalidDocument(vec![ValidationIssue {
            path: "stage.operations".to_string(),
            message: "stage must return at least one operation".to_string(),
        }]));
    }

    let mut issues = validate_scene_operations(&batch.operations);
    issues.extend(validate_stage_image_references(scene, &batch.operations));
    issues.extend(validate_stage_output_mode_expectations(
        stage,
        &batch.operations,
    ));
    if !stage.operations_can_target_existing_nodes() {
        issues.extend(validate_stage_create_targets(
            &batch.operations,
            &stage.slot_id,
        ));
    }

    let mut candidate = scene.clone();
    issues.extend(apply_scene_operations_to_scene(
        &mut candidate,
        &batch.operations,
    ));
    issues.extend(validate_scene(&candidate));

    if issues.is_empty() {
        Ok(())
    } else {
        Err(AiAdapterError::InvalidDocument(issues))
    }
}

impl StageSpec {
    fn operations_can_target_existing_nodes(&self) -> bool {
        matches!(self.kind, StageKind::Assembly)
    }
}

fn validate_stage_create_targets(
    operations: &[SceneOperation],
    default_parent_id: &str,
) -> Vec<ValidationIssue> {
    operations
        .iter()
        .filter_map(|operation| match operation_parent_id(operation) {
            Some(parent_id) if parent_id != default_parent_id => Some(ValidationIssue {
                path: "stage.operations".to_string(),
                message: format!(
                    "stage create operations must target the planned slot '{}' but found parent '{}'",
                    default_parent_id, parent_id
                ),
            }),
            _ => None,
        })
        .collect()
}

fn validate_stage_output_mode_expectations(
    stage: &StageSpec,
    operations: &[SceneOperation],
) -> Vec<ValidationIssue> {
    if !matches!(stage.output_mode, StageOutputMode::RasterPreferred) {
        return Vec::new();
    }

    let uses_raster = operations.iter().any(|operation| {
        matches!(
            operation,
            SceneOperation::UpsertImageResource { .. } | SceneOperation::CreateImageLayer { .. }
        )
    });

    if uses_raster {
        Vec::new()
    } else {
        vec![ValidationIssue {
            path: "stage.operations".to_string(),
            message: format!(
                "stage '{}' is raster-preferred and should include an upsert_image_resource/create_image_layer pair",
                stage.id
            ),
        }]
    }
}

fn validate_scene_operations(operations: &[SceneOperation]) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut created_ids = std::collections::HashSet::new();

    for (index, operation) in operations.iter().enumerate() {
        let issue_path = format!("stage.operations[{index}]");
        match operation {
            SceneOperation::UpsertImageResource {
                image_ref,
                path,
                prompt,
                width,
                height,
                ..
            } => {
                if image_ref.trim().is_empty() {
                    issues.push(ValidationIssue {
                        path: issue_path.clone(),
                        message: "image_ref must not be empty".to_string(),
                    });
                }
                if path.as_deref().is_none() && prompt.as_deref().is_none() {
                    issues.push(ValidationIssue {
                        path: issue_path.clone(),
                        message:
                            "upsert_image_resource must include either path or prompt metadata"
                                .to_string(),
                    });
                }
                if width.as_ref().is_some_and(|value| *value <= 0.0)
                    || height.as_ref().is_some_and(|value| *value <= 0.0)
                {
                    issues.push(ValidationIssue {
                        path: issue_path,
                        message: "image resource width and height must be positive when provided"
                            .to_string(),
                    });
                }
            }
            SceneOperation::CreateGroup { node_id, .. }
            | SceneOperation::CreateRectangle { node_id, .. }
            | SceneOperation::CreateEllipse { node_id, .. }
            | SceneOperation::CreateText { node_id, .. }
            | SceneOperation::CreateImageLayer { node_id, .. } => {
                if !created_ids.insert(node_id.clone()) {
                    issues.push(ValidationIssue {
                        path: issue_path.clone(),
                        message: format!("duplicate created node id '{}'", node_id),
                    });
                }
            }
            SceneOperation::CreatePath {
                node_id, points, ..
            } => {
                if !created_ids.insert(node_id.clone()) {
                    issues.push(ValidationIssue {
                        path: issue_path.clone(),
                        message: format!("duplicate created node id '{}'", node_id),
                    });
                }
                if points.is_empty() {
                    issues.push(ValidationIssue {
                        path: issue_path,
                        message: "create_path must include a non-empty points array".to_string(),
                    });
                }
            }
            SceneOperation::SetTransform { opacity, .. } => {
                if let Some(opacity) = opacity
                    && !(0.0..=1.0).contains(opacity)
                {
                    issues.push(ValidationIssue {
                        path: issue_path,
                        message: "opacity must be between 0 and 1".to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    issues
}

fn validate_stage_image_references(
    scene: &SceneFile,
    operations: &[SceneOperation],
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut available_refs = scene
        .document
        .resources
        .images
        .keys()
        .cloned()
        .collect::<std::collections::HashSet<_>>();

    for (index, operation) in operations.iter().enumerate() {
        match operation {
            SceneOperation::UpsertImageResource { image_ref, .. } => {
                available_refs.insert(image_ref.clone());
            }
            SceneOperation::CreateImageLayer { image_ref, .. } => {
                if !available_refs.contains(image_ref) {
                    issues.push(ValidationIssue {
                        path: format!("stage.operations[{index}]"),
                        message: format!(
                            "create_image_layer references unknown image resource '{}'",
                            image_ref
                        ),
                    });
                }
            }
            _ => {}
        }
    }

    issues
}

fn operation_parent_id(operation: &SceneOperation) -> Option<&str> {
    match operation {
        SceneOperation::CreateGroup { parent_id, .. }
        | SceneOperation::CreateRectangle { parent_id, .. }
        | SceneOperation::CreateEllipse { parent_id, .. }
        | SceneOperation::CreatePath { parent_id, .. }
        | SceneOperation::CreateText { parent_id, .. }
        | SceneOperation::CreateImageLayer { parent_id, .. } => Some(parent_id.as_str()),
        SceneOperation::UpsertImageResource { .. }
        | SceneOperation::SetTransform { .. }
        | SceneOperation::ReplaceParams { .. }
        | SceneOperation::ReplaceStyle { .. }
        | SceneOperation::DeleteNode { .. } => None,
    }
}

fn apply_scene_operations_to_scene(
    scene: &mut SceneFile,
    operations: &[SceneOperation],
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for (index, operation) in operations.iter().enumerate() {
        if let Some(issue) = apply_scene_operation(scene, operation, index) {
            issues.push(issue);
        }
    }
    issues
}

fn apply_scene_operation(
    scene: &mut SceneFile,
    operation: &SceneOperation,
    index: usize,
) -> Option<ValidationIssue> {
    let path = format!("stage.operations[{index}]");
    match operation {
        SceneOperation::UpsertImageResource {
            image_ref,
            path,
            prompt,
            width,
            height,
            alpha_mode,
            generation_mode,
            group_id,
        } => {
            let resource = build_image_resource_value(
                path.as_deref(),
                prompt.as_deref(),
                *width,
                *height,
                alpha_mode.as_deref(),
                generation_mode.as_deref(),
                group_id.as_deref(),
            );
            scene
                .document
                .resources
                .images
                .insert(image_ref.clone(), resource);
            None
        }
        SceneOperation::CreateGroup {
            node_id,
            parent_id,
            name,
            x,
            y,
        } => insert_created_node(
            scene,
            parent_id,
            build_group_node(node_id, name, *x, *y),
            &path,
        ),
        SceneOperation::CreateRectangle {
            node_id,
            parent_id,
            name,
            x,
            y,
            width,
            height,
            corner_radius,
            fill,
        } => insert_created_node(
            scene,
            parent_id,
            build_rectangle_node(node_id, name, *x, *y, *width, *height, *corner_radius, fill),
            &path,
        ),
        SceneOperation::CreateEllipse {
            node_id,
            parent_id,
            name,
            x,
            y,
            radius_x,
            radius_y,
            fill,
        } => insert_created_node(
            scene,
            parent_id,
            build_ellipse_node(node_id, name, *x, *y, *radius_x, *radius_y, fill),
            &path,
        ),
        SceneOperation::CreatePath {
            node_id,
            parent_id,
            name,
            x,
            y,
            points,
            closed,
            fill,
        } => insert_created_node(
            scene,
            parent_id,
            build_path_node(node_id, name, *x, *y, points, closed.unwrap_or(true), fill),
            &path,
        ),
        SceneOperation::CreateText {
            node_id,
            parent_id,
            name,
            x,
            y,
            text,
            font_size,
            font_family,
            line_height,
            max_width,
            align,
            fill,
        } => insert_created_node(
            scene,
            parent_id,
            build_text_node(
                node_id,
                name,
                *x,
                *y,
                text,
                *font_size,
                font_family.clone(),
                *line_height,
                *max_width,
                align.clone(),
                fill,
            ),
            &path,
        ),
        SceneOperation::CreateImageLayer {
            node_id,
            parent_id,
            name,
            x,
            y,
            image_ref,
            display_width,
            display_height,
        } => insert_created_node(
            scene,
            parent_id,
            build_image_layer_node(
                node_id,
                name,
                *x,
                *y,
                image_ref,
                *display_width,
                *display_height,
            ),
            &path,
        ),
        SceneOperation::SetTransform {
            node_id,
            x,
            y,
            scale_x,
            scale_y,
            rotation,
            opacity,
        } => {
            let Some(node) = find_scene_node_mut(&mut scene.document.root, node_id) else {
                return Some(ValidationIssue {
                    path,
                    message: format!("could not resolve node '{}' for set_transform", node_id),
                });
            };
            if let Some(x) = x {
                node.transform.x = *x;
            }
            if let Some(y) = y {
                node.transform.y = *y;
            }
            if let Some(scale_x) = scale_x {
                node.transform.scale_x = *scale_x;
            }
            if let Some(scale_y) = scale_y {
                node.transform.scale_y = *scale_y;
            }
            if let Some(rotation) = rotation {
                node.transform.rotation = *rotation;
            }
            if let Some(opacity) = opacity {
                node.transform.opacity = *opacity;
            }
            None
        }
        SceneOperation::ReplaceParams { node_id, params } => {
            let Some(node) = find_scene_node_mut(&mut scene.document.root, node_id) else {
                return Some(ValidationIssue {
                    path,
                    message: format!("could not resolve node '{}' for replace_params", node_id),
                });
            };
            node.params = params.clone();
            None
        }
        SceneOperation::ReplaceStyle { node_id, style } => {
            let Some(node) = find_scene_node_mut(&mut scene.document.root, node_id) else {
                return Some(ValidationIssue {
                    path,
                    message: format!("could not resolve node '{}' for replace_style", node_id),
                });
            };
            node.style = style.clone();
            None
        }
        SceneOperation::DeleteNode { node_id } => {
            if scene.document.root.id == *node_id {
                return Some(ValidationIssue {
                    path,
                    message: "cannot delete the root node".to_string(),
                });
            }
            if remove_scene_node(&mut scene.document.root, node_id).is_some() {
                None
            } else {
                Some(ValidationIssue {
                    path,
                    message: format!("could not resolve node '{}' for delete_node", node_id),
                })
            }
        }
    }
}

fn insert_created_node(
    scene: &mut SceneFile,
    parent_id: &str,
    node: SceneNode,
    path: &str,
) -> Option<ValidationIssue> {
    if find_scene_node(&scene.document.root, &node.id).is_some() {
        return Some(ValidationIssue {
            path: path.to_string(),
            message: format!("node id '{}' already exists", node.id),
        });
    }

    if insert_node_under_parent(&mut scene.document.root, parent_id, node) {
        None
    } else {
        Some(ValidationIssue {
            path: path.to_string(),
            message: format!(
                "could not resolve parent '{}' for create operation",
                parent_id
            ),
        })
    }
}

fn insert_node_under_parent(current: &mut SceneNode, parent_id: &str, node: SceneNode) -> bool {
    if current.id == parent_id {
        current.children.push(node);
        return true;
    }

    for child in &mut current.children {
        if insert_node_under_parent(child, parent_id, node.clone()) {
            return true;
        }
    }

    false
}

fn find_scene_node<'a>(node: &'a SceneNode, id: &str) -> Option<&'a SceneNode> {
    if node.id == id {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_scene_node(child, id))
}

fn find_scene_node_mut<'a>(node: &'a mut SceneNode, id: &str) -> Option<&'a mut SceneNode> {
    if node.id == id {
        return Some(node);
    }
    for child in &mut node.children {
        if let Some(found) = find_scene_node_mut(child, id) {
            return Some(found);
        }
    }
    None
}

fn remove_scene_node(node: &mut SceneNode, id: &str) -> Option<SceneNode> {
    if let Some(index) = node.children.iter().position(|child| child.id == id) {
        return Some(node.children.remove(index));
    }
    for child in &mut node.children {
        if let Some(removed) = remove_scene_node(child, id) {
            return Some(removed);
        }
    }
    None
}

fn build_group_node(node_id: &str, name: &str, x: f64, y: f64) -> SceneNode {
    SceneNode {
        id: node_id.to_string(),
        node_type: scene_schema::NodeType::Group,
        name: name.to_string(),
        visible: true,
        locked: false,
        blend_mode: BlendMode::Normal,
        transform: Transform {
            x,
            y,
            ..default_node_transform()
        },
        params: JsonObject::new(),
        style: JsonObject::new(),
        children: Vec::new(),
        meta: JsonObject::new(),
    }
}

fn build_rectangle_node(
    node_id: &str,
    name: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corner_radius: Option<f64>,
    fill: &Option<String>,
) -> SceneNode {
    let mut params = JsonObject::new();
    params.insert("width".to_string(), serde_json::Value::from(width));
    params.insert("height".to_string(), serde_json::Value::from(height));
    if let Some(corner_radius) = corner_radius {
        params.insert(
            "cornerRadius".to_string(),
            serde_json::Value::from(corner_radius),
        );
    }
    build_leaf_node(
        node_id,
        scene_schema::NodeType::Rectangle,
        name,
        x,
        y,
        params,
        style_with_fill(fill),
    )
}

fn build_ellipse_node(
    node_id: &str,
    name: &str,
    x: f64,
    y: f64,
    radius_x: f64,
    radius_y: f64,
    fill: &Option<String>,
) -> SceneNode {
    let mut params = JsonObject::new();
    params.insert("radiusX".to_string(), serde_json::Value::from(radius_x));
    params.insert("radiusY".to_string(), serde_json::Value::from(radius_y));
    build_leaf_node(
        node_id,
        scene_schema::NodeType::Ellipse,
        name,
        x,
        y,
        params,
        style_with_fill(fill),
    )
}

fn build_path_node(
    node_id: &str,
    name: &str,
    x: f64,
    y: f64,
    points: &[SceneOpPoint],
    closed: bool,
    fill: &Option<String>,
) -> SceneNode {
    let mut params = JsonObject::new();
    params.insert(
        "points".to_string(),
        serde_json::Value::Array(
            points
                .iter()
                .map(|point| {
                    serde_json::json!({
                        "x": point.x,
                        "y": point.y
                    })
                })
                .collect(),
        ),
    );
    params.insert("closed".to_string(), serde_json::Value::Bool(closed));
    build_leaf_node(
        node_id,
        scene_schema::NodeType::Path,
        name,
        x,
        y,
        params,
        style_with_fill(fill),
    )
}

#[allow(clippy::too_many_arguments)]
fn build_text_node(
    node_id: &str,
    name: &str,
    x: f64,
    y: f64,
    text: &str,
    font_size: f64,
    font_family: Option<String>,
    line_height: Option<f64>,
    max_width: Option<f64>,
    align: Option<String>,
    fill: &Option<String>,
) -> SceneNode {
    let mut params = JsonObject::new();
    params.insert(
        "text".to_string(),
        serde_json::Value::String(text.to_string()),
    );
    params.insert("fontSize".to_string(), serde_json::Value::from(font_size));
    if let Some(font_family) = font_family {
        params.insert(
            "fontFamily".to_string(),
            serde_json::Value::String(font_family),
        );
    }
    if let Some(line_height) = line_height {
        params.insert(
            "lineHeight".to_string(),
            serde_json::Value::from(line_height),
        );
    }
    if let Some(max_width) = max_width {
        params.insert("maxWidth".to_string(), serde_json::Value::from(max_width));
    }
    if let Some(align) = align {
        params.insert("align".to_string(), serde_json::Value::String(align));
    }
    build_leaf_node(
        node_id,
        scene_schema::NodeType::Text,
        name,
        x,
        y,
        params,
        style_with_fill(fill),
    )
}

fn build_image_layer_node(
    node_id: &str,
    name: &str,
    x: f64,
    y: f64,
    image_ref: &str,
    display_width: f64,
    display_height: f64,
) -> SceneNode {
    let mut params = JsonObject::new();
    params.insert(
        "imageRef".to_string(),
        serde_json::Value::String(image_ref.to_string()),
    );
    params.insert(
        "displayWidth".to_string(),
        serde_json::Value::from(display_width),
    );
    params.insert(
        "displayHeight".to_string(),
        serde_json::Value::from(display_height),
    );
    build_leaf_node(
        node_id,
        scene_schema::NodeType::ImageLayer,
        name,
        x,
        y,
        params,
        JsonObject::new(),
    )
}

fn build_image_resource_value(
    path: Option<&str>,
    prompt: Option<&str>,
    width: Option<f64>,
    height: Option<f64>,
    alpha_mode: Option<&str>,
    generation_mode: Option<&str>,
    group_id: Option<&str>,
) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert(
        "kind".to_string(),
        serde_json::Value::String("generated_patch".to_string()),
    );
    object.insert(
        "status".to_string(),
        serde_json::Value::String(if path.is_some() { "ready" } else { "planned" }.to_string()),
    );

    if let Some(path) = path {
        object.insert(
            "path".to_string(),
            serde_json::Value::String(path.to_string()),
        );
    }
    if let Some(prompt) = prompt {
        object.insert(
            "prompt".to_string(),
            serde_json::Value::String(prompt.to_string()),
        );
    }
    if let Some(width) = width {
        object.insert("width".to_string(), serde_json::Value::from(width));
    }
    if let Some(height) = height {
        object.insert("height".to_string(), serde_json::Value::from(height));
    }
    if let Some(alpha_mode) = alpha_mode {
        object.insert(
            "alphaMode".to_string(),
            serde_json::Value::String(alpha_mode.to_string()),
        );
    }
    if let Some(generation_mode) = generation_mode {
        object.insert(
            "generationMode".to_string(),
            serde_json::Value::String(generation_mode.to_string()),
        );
    }
    if let Some(group_id) = group_id {
        object.insert(
            "groupId".to_string(),
            serde_json::Value::String(group_id.to_string()),
        );
    }

    serde_json::Value::Object(object)
}

fn build_leaf_node(
    node_id: &str,
    node_type: scene_schema::NodeType,
    name: &str,
    x: f64,
    y: f64,
    params: JsonObject,
    style: JsonObject,
) -> SceneNode {
    SceneNode {
        id: node_id.to_string(),
        node_type,
        name: name.to_string(),
        visible: true,
        locked: false,
        blend_mode: BlendMode::Normal,
        transform: Transform {
            x,
            y,
            ..default_node_transform()
        },
        params,
        style,
        children: Vec::new(),
        meta: JsonObject::new(),
    }
}

fn style_with_fill(fill: &Option<String>) -> JsonObject {
    let mut style = JsonObject::new();
    if let Some(fill) = fill {
        style.insert("fill".to_string(), serde_json::Value::String(fill.clone()));
    }
    style
}

fn critique_stage_and_maybe_request_retry(
    config: &ProviderConfig,
    api_key: &str,
    prompt: &str,
    scene: &SceneFile,
    plan: &ScenePlan,
    stage: &StageSpec,
    model: &str,
) -> Result<Option<String>, AiAdapterError> {
    if !should_visually_critique_stage(stage) {
        return Ok(None);
    }

    let rendered_png = render_scene_png(scene)?;
    let critique = request_gemini_stage_critique(
        config,
        api_key,
        prompt,
        scene,
        plan,
        stage,
        &rendered_png,
        model,
    )?;

    if critique.satisfactory || critique.revision_goals.is_empty() {
        return Ok(None);
    }

    Ok(Some(build_stage_critique_feedback(stage, &critique)))
}

fn should_visually_critique_stage(stage: &StageSpec) -> bool {
    !matches!(stage.kind, StageKind::Support)
}

fn build_stage_critique_feedback(stage: &StageSpec, critique: &SceneCritique) -> String {
    let focus_group = stage
        .focus_group_id
        .as_ref()
        .map(|group_id| format!(" Focus on the assembled group '{}'.", group_id))
        .unwrap_or_default();

    format!(
        concat!(
            "The rendered image shows problems with stage '{}'. ",
            "Stage summary: {} ",
            "Issues: {} ",
            "Revision goals: {} ",
            "Regenerate only this stage's nodes and keep the rest of the scene intact.{}"
        ),
        stage.id,
        critique.summary,
        critique.issues.join(" | "),
        critique.revision_goals.join(" | "),
        focus_group,
    )
}

fn render_scene_png(scene: &SceneFile) -> Result<Vec<u8>, AiAdapterError> {
    let plan = renderer::build_render_plan(scene);
    renderer::skia_backend::render_plan_to_png(
        &plan,
        scene.document.width.round() as u32,
        scene.document.height.round() as u32,
    )
    .map_err(|error| {
        AiAdapterError::InvalidProviderConfig(format!("failed to render critique PNG: {error}"))
    })
}

fn request_gemini_scene_plan(
    config: &ProviderConfig,
    api_key: &str,
    prompt: &str,
    template_kind: SceneTemplateKind,
    template_scene: &str,
    model: &str,
) -> Result<ScenePlan, AiAdapterError> {
    if let Some(cached_plan) = read_cached_scene_plan(prompt, template_kind) {
        return Ok(cached_plan);
    }

    let endpoint = gemini_endpoint(config, model);
    let request = GeminiGenerateContentRequest {
        system_instruction: GeminiContent {
            parts: vec![GeminiPart::text(gemini_plan_system_instruction(model))],
        },
        contents: vec![GeminiContent {
            parts: vec![GeminiPart::text(gemini_plan_user_prompt(
                prompt,
                template_kind,
                template_scene,
            ))],
        }],
        generation_config: GeminiGenerationConfig {
            response_mime_type: "application/json".to_string(),
            response_json_schema: scene_plan_schema(),
            temperature: Some(0.5),
        },
    };

    let json_text = send_gemini_request(config, api_key, model, "plan", endpoint, &request)?;
    let plan = serde_json::from_str::<ScenePlan>(&json_text)
        .map_err(|error| AiAdapterError::ParseFailed(error.to_string()))?;
    write_cached_scene_plan(prompt, template_kind, &plan);
    Ok(plan)
}

fn request_gemini_stage_critique(
    config: &ProviderConfig,
    api_key: &str,
    prompt: &str,
    scene: &SceneFile,
    plan: &ScenePlan,
    stage: &StageSpec,
    rendered_png: &[u8],
    model: &str,
) -> Result<SceneCritique, AiAdapterError> {
    let endpoint = gemini_endpoint(config, model);
    let request = GeminiGenerateContentRequest {
        system_instruction: GeminiContent {
            parts: vec![GeminiPart::text(gemini_critique_system_instruction(model))],
        },
        contents: vec![GeminiContent {
            parts: vec![
                GeminiPart::text(gemini_stage_critique_user_prompt(
                    prompt, scene, plan, stage,
                )),
                GeminiPart::inline_png(rendered_png),
            ],
        }],
        generation_config: GeminiGenerationConfig {
            response_mime_type: "application/json".to_string(),
            response_json_schema: scene_critique_schema(),
            temperature: Some(0.2),
        },
    };

    let json_text = send_gemini_request(
        config,
        api_key,
        model,
        &format!("stage_critique_{}", stage.id),
        endpoint,
        &request,
    )?;
    serde_json::from_str::<SceneCritique>(&json_text)
        .map_err(|error| AiAdapterError::ParseFailed(error.to_string()))
}

fn send_gemini_request(
    config: &ProviderConfig,
    api_key: &str,
    model: &str,
    phase: &str,
    endpoint: String,
    request: &GeminiGenerateContentRequest,
) -> Result<String, AiAdapterError> {
    let max_attempts = gemini_request_max_attempts(model);
    let mut last_error = None;

    for attempt in 0..max_attempts {
        respect_gemini_rate_limit(model);
        match send_gemini_request_once(config, api_key, model, phase, &endpoint, request) {
            Ok(response) => return Ok(response),
            Err(error) if attempt + 1 < max_attempts && is_retryable_gemini_error(&error) => {
                last_error = Some(error);
                std::thread::sleep(gemini_retry_backoff(attempt));
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        AiAdapterError::ApiResponseFailed(format!(
            "Gemini request failed without a final error via {model}"
        ))
    }))
}

fn send_gemini_request_once(
    config: &ProviderConfig,
    api_key: &str,
    model: &str,
    phase: &str,
    endpoint: &str,
    request: &GeminiGenerateContentRequest,
) -> Result<String, AiAdapterError> {
    let request_trace =
        sanitize_trace_value(serde_json::to_value(request).unwrap_or_else(
            |_| serde_json::json!({ "trace_error": "failed to serialize request" }),
        ));
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|error| AiAdapterError::HttpFailed(error.to_string()))?;

    let http_response = client
        .post(endpoint)
        .header("x-goog-api-key", api_key)
        .json(&request)
        .send()
        .map_err(|error| {
            write_trace_bundle(
                config,
                phase,
                model,
                &request_trace,
                None,
                Some(&format!("HTTP request failed: {error}")),
            );
            AiAdapterError::HttpFailed(error.to_string())
        })?;

    let status = http_response.status();
    let response_text = http_response.text().map_err(|error| {
        write_trace_bundle(
            config,
            phase,
            model,
            &request_trace,
            None,
            Some(&format!("failed to read response body: {error}")),
        );
        AiAdapterError::ParseFailed(error.to_string())
    })?;
    let response_trace = sanitize_trace_value(
        serde_json::from_str::<serde_json::Value>(&response_text)
            .unwrap_or_else(|_| serde_json::json!({ "raw_text": response_text.clone() })),
    );
    let response: GeminiGenerateContentResponse =
        serde_json::from_str(&response_text).map_err(|error| {
            write_trace_bundle(
                config,
                phase,
                model,
                &request_trace,
                Some(&response_trace),
                Some(&format!(
                    "failed to parse Gemini response envelope: {error}"
                )),
            );
            AiAdapterError::ParseFailed(error.to_string())
        })?;

    if !status.is_success() {
        if let Some(error) = response.error {
            write_trace_bundle(
                config,
                phase,
                model,
                &request_trace,
                Some(&response_trace),
                Some(&format!(
                    "provider error: {} ({})",
                    error.message, error.status
                )),
            );
            return Err(AiAdapterError::ApiResponseFailed(format!(
                "{} ({}) via {}",
                error.message, error.status, model
            )));
        }
        write_trace_bundle(
            config,
            phase,
            model,
            &request_trace,
            Some(&response_trace),
            Some(&format!("provider returned HTTP status {status}")),
        );
        return Err(AiAdapterError::ApiResponseFailed(format!(
            "Gemini returned HTTP status {status} via {model}"
        )));
    }

    if let Some(error) = response.error {
        write_trace_bundle(
            config,
            phase,
            model,
            &request_trace,
            Some(&response_trace),
            Some(&format!(
                "provider error: {} ({})",
                error.message, error.status
            )),
        );
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
            write_trace_bundle(
                config,
                phase,
                model,
                &request_trace,
                Some(&response_trace),
                Some("Gemini response did not include any JSON text parts"),
            );
            AiAdapterError::ApiResponseFailed(format!(
                "Gemini response did not include any JSON text parts via {model}"
            ))
        })?;

    write_trace_bundle(
        config,
        phase,
        model,
        &request_trace,
        Some(&response_trace),
        None,
    );
    Ok(json_text)
}

fn gemini_request_max_attempts(model: &str) -> usize {
    if model.contains("gemma-4") { 4 } else { 1 }
}

fn gemini_retry_backoff(attempt: usize) -> Duration {
    match attempt {
        0 => Duration::from_secs(2),
        1 => Duration::from_secs(5),
        _ => Duration::from_secs(10),
    }
}

fn respect_gemini_rate_limit(model: &str) {
    let Some(min_spacing) = gemini_min_spacing_for_model(model) else {
        return;
    };

    let limiter = GEMINI_RATE_LIMITER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut state = match limiter.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let now = Instant::now();
    if let Some(last_request_at) = state.get(model).copied() {
        let elapsed = now.saturating_duration_since(last_request_at);
        if elapsed < min_spacing {
            std::thread::sleep(min_spacing - elapsed);
        }
    }

    state.insert(model.to_string(), Instant::now());
}

fn read_cached_scene_plan(prompt: &str, template_kind: SceneTemplateKind) -> Option<ScenePlan> {
    let path = plan_cache_file_path(prompt, template_kind)?;
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str::<ScenePlan>(&contents).ok()
}

fn write_cached_scene_plan(prompt: &str, template_kind: SceneTemplateKind, plan: &ScenePlan) {
    let Some(path) = plan_cache_file_path(prompt, template_kind) else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(serialized) = serde_json::to_string_pretty(plan) else {
        return;
    };
    let _ = fs::write(path, serialized);
}

fn plan_cache_file_path(
    prompt: &str,
    template_kind: SceneTemplateKind,
) -> Option<std::path::PathBuf> {
    let cache_root = env::var(DEFAULT_PLAN_CACHE_DIR_ENV_VAR)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| ".tweaky-ai-cache/plans".to_string());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    prompt.hash(&mut hasher);
    template_name(template_kind).hash(&mut hasher);
    let cache_key = format!("{:016x}", hasher.finish());
    Some(Path::new(&cache_root).join(format!("{cache_key}.json")))
}

fn gemini_min_spacing_for_model(model: &str) -> Option<Duration> {
    let normalized = model.trim().to_lowercase();

    if normalized.contains("2.5-flash-lite") {
        return Some(Duration::from_millis(4_200));
    }

    if normalized.contains("2.5-flash") {
        return Some(Duration::from_millis(6_200));
    }

    None
}

fn write_trace_bundle(
    config: &ProviderConfig,
    phase: &str,
    model: &str,
    request: &serde_json::Value,
    response: Option<&serde_json::Value>,
    error: Option<&str>,
) {
    let Some(trace_dir) = env::var(DEFAULT_TRACE_DIR_ENV_VAR)
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return;
    };

    let counter = TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let payload = serde_json::json!({
        "phase": phase,
        "provider": format!("{}", config.provider),
        "model": model,
        "timestamp_ms": timestamp_ms,
        "request": request,
        "response": response.cloned(),
        "error": error,
    });

    let path = Path::new(&trace_dir);
    if fs::create_dir_all(path).is_err() {
        return;
    }

    let filename = format!("{timestamp_ms:013}_{counter:04}_{phase}_{model}.json");
    let output_path = path.join(filename);
    let _ = fs::write(
        output_path,
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
            "{\"trace_error\":\"failed to serialize trace payload\"}\n".to_string()
        }),
    );
}

fn sanitize_trace_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(mut map) => {
            if let Some(inline_data) = map.get_mut("inlineData")
                && let Some(inline_map) = inline_data.as_object_mut()
            {
                if let Some(data) = inline_map.get("data").and_then(|value| value.as_str()) {
                    inline_map.insert(
                        "data".to_string(),
                        serde_json::Value::String(format!("<base64:{} bytes>", data.len())),
                    );
                }
            }

            let sanitized = map
                .into_iter()
                .map(|(key, value)| (key, sanitize_trace_value(value)))
                .collect();
            serde_json::Value::Object(sanitized)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sanitize_trace_value).collect())
        }
        other => other,
    }
}

fn gemini_endpoint(config: &ProviderConfig, model: &str) -> String {
    let base = config
        .base_url
        .as_deref()
        .unwrap_or(DEFAULT_GEMINI_BASE_URL)
        .trim_end_matches('/');
    format!("{base}/models/{model}:generateContent")
}

fn gemini_plan_system_instruction(model: &str) -> String {
    format!(
        concat!(
            "Target model: {}.\n",
            "You are planning a tweaky scene before emitting final JSON.\n",
            "Return JSON only.\n",
            "Produce a compact scene plan with canvas, style keywords, major nodes, and composition notes.\n",
            "Do not write final tweaky scene JSON in this step.\n",
            "The plan must be concrete and detailed enough to drive a later structured scene generation pass.\n"
        ),
        model
    )
}

fn gemini_stage_system_instruction(model: &str) -> String {
    format!(
        concat!(
            "Target model: {}.\n",
            "You are generating one stage of a tweaky scene as editable scene operations.\n",
            "Return JSON only.\n",
            "Return an object with keys summary, operations, and notes.\n",
            "operations must be an array of valid tweaky scene operations.\n",
            "Do not return a full document in this step.\n",
            "Do not return empty operations arrays.\n",
            "Prefer concrete create/update operations over prose.\n",
            "Use create operations for new nodes and set/replace operations for refinement.\n",
            "Follow the exact tweaky operation and param conventions in the guide below.\n",
            "Schema for the operation batch:\n{}\n"
        ),
        model,
        serde_json::to_string_pretty(&scene_operations_schema())
            .unwrap_or_else(|_| "{}".to_string())
    )
}

fn stage_output_mode_label(mode: StageOutputMode) -> &'static str {
    match mode {
        StageOutputMode::Structured => "structured",
        StageOutputMode::RasterPreferred => "raster_preferred",
    }
}

fn gemini_plan_user_prompt(
    prompt: &str,
    template_kind: SceneTemplateKind,
    template_scene: &str,
) -> String {
    format!(
        concat!(
            "Create a scene plan for this request:\n",
            "{}\n\n",
            "Use this scaffold family as a structural prior: {}.\n",
            "Template scene:\n{}\n\n",
            "Requirements:\n",
            "- choose a concrete canvas size and background color\n",
            "- list the major editable nodes needed to draw the scene\n",
            "- provide a hierarchy tree rooted at id 'root' with role 'group'\n",
            "- use role 'group' for container layers and role 'slot' for leaf insertion targets\n",
            "- when a subject is naturally composed of parts, introduce an intermediate named group for that subject and place part slots beneath it\n",
            "- when two subjects should later be treated as one composition, introduce a higher-level group that contains them\n",
            "- make each major editable object correspond to a leaf slot id in the hierarchy\n",
            "- make the scene funny and compositionally clear\n",
            "- prefer native structured nodes over raster fallback when possible\n"
        ),
        prompt,
        template_name(template_kind),
        template_scene
    )
}

fn gemini_stage_user_prompt(
    prompt: &str,
    stage: &StageSpec,
    scene: &SceneFile,
    plan: &ScenePlan,
    repair_feedback: Option<&str>,
) -> String {
    let repair_block = repair_feedback
        .map(|feedback| {
            format!(
                concat!(
                    "\nRepair feedback from the previous stage attempt:\n",
                    "{}\n",
                    "Fix these exact issues in this retry.\n"
                ),
                feedback
            )
        })
        .unwrap_or_default();
    let raster_block = match stage.output_mode {
        StageOutputMode::Structured => String::new(),
        StageOutputMode::RasterPreferred => format!(
            concat!(
                "\nRaster preference for this stage:\n",
                "- This stage is style-heavy and raster-preferred.\n",
                "- Prefer returning an upsert_image_resource operation plus one create_image_layer operation for the subject patch.\n",
                "- The image resource should capture a transparent raster patch for this stage's subject detail, not a full scene render.\n",
                "- Use image_ref names derived from the stage id such as '{}_patch'.\n",
                "- Include a concise prompt in upsert_image_resource.prompt describing the patch content and style.\n",
                "- Include approximate native width and height in the image resource metadata.\n",
                "- Parent the resulting create_image_layer under the target slot/group so it can be moved and critiqued with the rest of the scene.\n",
                "- Only fall back to primitive geometry if a raster patch would be actively worse for editability.\n"
            ),
            stage.id
        ),
    };

    format!(
        concat!(
            "User request:\n{}\n\n",
            "Current working scene JSON:\n{}\n\n",
            "Overall plan summary: {}\n\n",
            "Current stage id: {}\n",
            "Target slot id: {}\n",
            "Stage purpose: {}\n\n",
            "Preferred output mode: {}\n",
            "Focus group id: {}\n",
            "Target node ids for this stage: {}\n",
            "Relevant composition hints:\n{}\n\n",
            "Instructions:\n",
            "- return only the scene operations needed for this stage\n",
            "- for slot-filling stages, create new nodes under the target slot group using parent_id equal to the target slot id\n",
            "- respect the existing composition and avoid duplicating existing nodes\n",
            "- make the result editable and structurally clear\n",
            "- use multiple operations when that helps readability\n",
            "- do not return an empty operations array\n",
            "- if this is an assembly/refinement stage, preserve the existing part nodes already in the target group and use create/update operations only for the connective, clarifying, or compositional edits needed to make the group read correctly as a whole\n",
            "- Ellipse params use radiusX and radiusY, not width/height\n",
            "- Path params use points: [{{\"x\":..,\"y\":..}}] and optional closed, not SVG path data\n",
            "- Text params use text, fontSize, optional fontFamily, and optional lineHeight\n",
            "- Rectangle params use width, height, optional cornerRadius\n",
            "- Keep the output focused on this stage's target node ids and avoid duplicating nodes from other stages\n\n",
            "{}\n",
            "Quick operation and param guide:\n{}\n",
            "{}"
        ),
        prompt,
        serde_json::to_string_pretty(scene).unwrap_or_else(|_| "{}".to_string()),
        plan.summary,
        stage.id,
        stage.slot_id,
        stage.purpose,
        stage_output_mode_label(stage.output_mode),
        stage.focus_group_id.as_deref().unwrap_or("-"),
        stage.target_node_ids.join(", "),
        stage.composition_hints.join("\n"),
        raster_block,
        stage_param_guide(),
        repair_block
    )
}

fn gemini_critique_system_instruction(model: &str) -> String {
    format!(
        concat!(
            "Target model: {}.\n",
            "You are critiquing a rendered tweaky scene against the user's prompt.\n",
            "Look at the image, compare it to the prompt and scene JSON, and return JSON only.\n",
            "Be specific about visual problems and concrete revision goals.\n"
        ),
        model
    )
}

fn gemini_stage_critique_user_prompt(
    prompt: &str,
    scene: &SceneFile,
    plan: &ScenePlan,
    stage: &StageSpec,
) -> String {
    format!(
        concat!(
            "User prompt:\n{}\n\n",
            "Overall plan summary:\n{}\n\n",
            "Current stage id: {}\n",
            "Target slot id: {}\n",
            "Stage purpose: {}\n",
            "Focus group id: {}\n",
            "Target node ids: {}\n",
            "Relevant composition hints:\n{}\n\n",
            "Current scene JSON:\n{}\n\n",
            "Look at the rendered image and judge whether this stage's contribution is visually successful.\n",
            "Focus on the newest stage output, but consider how it fits into the whole scene.\n",
            "If a focus group id is present, evaluate whether that assembled group reads clearly as one coherent subject or composition.\n",
            "If this stage is acceptable, mark satisfactory true.\n",
            "If not, give concise issues and revision goals for regenerating only this stage."
        ),
        prompt,
        plan.summary,
        stage.id,
        stage.slot_id,
        stage.purpose,
        stage.focus_group_id.as_deref().unwrap_or("-"),
        stage.target_node_ids.join(", "),
        stage.composition_hints.join("\n"),
        serde_json::to_string_pretty(scene).unwrap_or_else(|_| "{}".to_string())
    )
}

fn scene_plan_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "$defs": {
            "hierarchy_node": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "role": { "type": "string", "enum": ["group", "slot"] },
                    "label": { "type": "string" },
                    "purpose": { "type": "string" },
                    "children": {
                        "type": "array",
                        "items": { "$ref": "#/$defs/hierarchy_node" }
                    }
                },
                "required": ["id", "role", "children"]
            }
        },
        "properties": {
            "summary": { "type": "string" },
            "canvas": {
                "type": "object",
                "properties": {
                    "width": { "type": "number" },
                    "height": { "type": "number" },
                    "background": { "type": "string" }
                },
                "required": ["width", "height", "background"]
            },
            "style_keywords": {
                "type": "array",
                "items": { "type": "string" }
            },
            "major_nodes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "node_type": { "type": "string" },
                        "purpose": { "type": "string" }
                    },
                    "required": ["id", "node_type", "purpose"]
                }
            },
            "composition_notes": {
                "type": "array",
                "items": { "type": "string" }
            },
            "hierarchy": {
                "$ref": "#/$defs/hierarchy_node"
            }
        },
        "required": ["summary", "canvas", "style_keywords", "major_nodes", "composition_notes", "hierarchy"]
    })
}

fn scene_critique_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "satisfactory": { "type": "boolean" },
            "summary": { "type": "string" },
            "strengths": {
                "type": "array",
                "items": { "type": "string" }
            },
            "issues": {
                "type": "array",
                "items": { "type": "string" }
            },
            "revision_goals": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["satisfactory", "summary", "strengths", "issues", "revision_goals"]
    })
}

pub fn scene_operations_schema() -> serde_json::Value {
    serde_json::json!({
        "$defs": {
            "point": {
                "type": "object",
                "properties": {
                    "x": { "type": "number" },
                    "y": { "type": "number" }
                },
                "required": ["x", "y"],
                "additionalProperties": false
            },
            "scene_operation": {
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "upsert_image_resource" },
                            "image_ref": { "type": "string" },
                            "path": { "type": "string" },
                            "prompt": { "type": "string" },
                            "width": { "type": "number" },
                            "height": { "type": "number" },
                            "alpha_mode": { "type": "string" },
                            "generation_mode": { "type": "string" },
                            "group_id": { "type": "string" }
                        },
                        "required": ["op", "image_ref"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "create_group" },
                            "node_id": { "type": "string" },
                            "parent_id": { "type": "string" },
                            "name": { "type": "string" },
                            "x": { "type": "number" },
                            "y": { "type": "number" }
                        },
                        "required": ["op", "node_id", "parent_id", "name", "x", "y"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "create_rectangle" },
                            "node_id": { "type": "string" },
                            "parent_id": { "type": "string" },
                            "name": { "type": "string" },
                            "x": { "type": "number" },
                            "y": { "type": "number" },
                            "width": { "type": "number" },
                            "height": { "type": "number" },
                            "corner_radius": { "type": "number" },
                            "fill": { "type": "string" }
                        },
                        "required": ["op", "node_id", "parent_id", "name", "x", "y", "width", "height"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "create_ellipse" },
                            "node_id": { "type": "string" },
                            "parent_id": { "type": "string" },
                            "name": { "type": "string" },
                            "x": { "type": "number" },
                            "y": { "type": "number" },
                            "radius_x": { "type": "number" },
                            "radius_y": { "type": "number" },
                            "fill": { "type": "string" }
                        },
                        "required": ["op", "node_id", "parent_id", "name", "x", "y", "radius_x", "radius_y"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "create_path" },
                            "node_id": { "type": "string" },
                            "parent_id": { "type": "string" },
                            "name": { "type": "string" },
                            "x": { "type": "number" },
                            "y": { "type": "number" },
                            "points": {
                                "type": "array",
                                "items": { "$ref": "#/$defs/point" }
                            },
                            "closed": { "type": "boolean" },
                            "fill": { "type": "string" }
                        },
                        "required": ["op", "node_id", "parent_id", "name", "x", "y", "points"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "create_text" },
                            "node_id": { "type": "string" },
                            "parent_id": { "type": "string" },
                            "name": { "type": "string" },
                            "x": { "type": "number" },
                            "y": { "type": "number" },
                            "text": { "type": "string" },
                            "font_size": { "type": "number" },
                            "font_family": { "type": "string" },
                            "line_height": { "type": "number" },
                            "max_width": { "type": "number" },
                            "align": { "type": "string", "enum": ["left", "center", "right"] },
                            "fill": { "type": "string" }
                        },
                        "required": ["op", "node_id", "parent_id", "name", "x", "y", "text", "font_size"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "create_image_layer" },
                            "node_id": { "type": "string" },
                            "parent_id": { "type": "string" },
                            "name": { "type": "string" },
                            "x": { "type": "number" },
                            "y": { "type": "number" },
                            "image_ref": { "type": "string" },
                            "display_width": { "type": "number" },
                            "display_height": { "type": "number" }
                        },
                        "required": ["op", "node_id", "parent_id", "name", "x", "y", "image_ref", "display_width", "display_height"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "set_transform" },
                            "node_id": { "type": "string" },
                            "x": { "type": "number" },
                            "y": { "type": "number" },
                            "scale_x": { "type": "number" },
                            "scale_y": { "type": "number" },
                            "rotation": { "type": "number" },
                            "opacity": { "type": "number" }
                        },
                        "required": ["op", "node_id"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "replace_params" },
                            "node_id": { "type": "string" },
                            "params": { "type": "object" }
                        },
                        "required": ["op", "node_id", "params"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "replace_style" },
                            "node_id": { "type": "string" },
                            "style": { "type": "object" }
                        },
                        "required": ["op", "node_id", "style"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "op": { "const": "delete_node" },
                            "node_id": { "type": "string" }
                        },
                        "required": ["op", "node_id"],
                        "additionalProperties": false
                    }
                ]
            }
        },
        "type": "object",
        "properties": {
            "summary": { "type": "string" },
            "operations": {
                "type": "array",
                "items": { "$ref": "#/$defs/scene_operation" }
            },
            "notes": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["summary", "operations", "notes"],
        "additionalProperties": false
    })
}

fn stage_param_guide() -> &'static str {
    concat!(
        "- upsert_image_resource = {\"op\":\"upsert_image_resource\", \"image_ref\": string, optional \"path\": string, optional \"prompt\": string, optional \"width\": number, optional \"height\": number, optional \"alpha_mode\": string, optional \"generation_mode\": string, optional \"group_id\": string}\n",
        "- create_group = {\"op\":\"create_group\",\"node_id\": string, \"parent_id\": string, \"name\": string, \"x\": number, \"y\": number}\n",
        "- create_rectangle = {\"op\":\"create_rectangle\", ..., \"width\": number, \"height\": number, optional \"corner_radius\": number}\n",
        "- create_ellipse = {\"op\":\"create_ellipse\", ..., \"radius_x\": number, \"radius_y\": number}\n",
        "- create_path = {\"op\":\"create_path\", ..., \"points\": [{\"x\": number, \"y\": number}, ...], optional \"closed\": boolean}\n",
        "- create_text = {\"op\":\"create_text\", ..., \"text\": string, \"font_size\": number, optional \"font_family\": string, optional \"line_height\": number}\n",
        "- create_image_layer = {\"op\":\"create_image_layer\", ..., \"image_ref\": string, \"display_width\": number, \"display_height\": number}\n",
        "- set_transform = {\"op\":\"set_transform\", \"node_id\": string, optional x/y/scale_x/scale_y/rotation/opacity}\n",
        "- replace_params = {\"op\":\"replace_params\", \"node_id\": string, \"params\": {...}}\n",
        "- replace_style = {\"op\":\"replace_style\", \"node_id\": string, \"style\": {...}}\n",
        "- delete_node = {\"op\":\"delete_node\", \"node_id\": string}\n",
        "- Rectangle.params = {\"width\": number, \"height\": number, optional \"cornerRadius\": number}\n",
        "- Ellipse.params = {\"radiusX\": number, \"radiusY\": number}\n",
        "- Path.params = {\"points\": [{\"x\": number, \"y\": number}, ...], optional \"closed\": boolean}\n",
        "- Text.params = {\"text\": string, \"fontSize\": number, optional \"fontFamily\": string, optional \"lineHeight\": number}\n",
        "- ImageLayer.params = {\"imageRef\": string, \"displayWidth\": number, \"displayHeight\": number}\n",
        "- style.fill should be a color like #ffffff when needed\n"
    )
}

fn default_fallback_models(provider: ProviderKind) -> Vec<String> {
    match provider {
        ProviderKind::Gemini => vec![
            DEFAULT_GEMINI_FALLBACK_MODEL.to_string(),
            DEFAULT_GEMMA_FALLBACK_MODEL.to_string(),
        ],
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

fn env_var_truthy(name: &str) -> bool {
    matches!(
        env::var(name).ok().as_deref().map(str::trim),
        Some("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}

fn gemini_model_attempts(config: &ProviderConfig) -> Vec<String> {
    if env_var_truthy(DISABLE_FALLBACK_ENV_VAR) {
        return vec![config.model.clone()];
    }

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
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    inline_data: Option<GeminiInlineData>,
}

impl GeminiPart {
    fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            inline_data: None,
        }
    }

    fn inline_png(bytes: &[u8]) -> Self {
        Self {
            text: None,
            inline_data: Some(GeminiInlineData {
                mime_type: "image/png".to_string(),
                data: BASE64_STANDARD.encode(bytes),
            }),
        }
    }
}

#[derive(Debug, Serialize)]
struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
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
        AiAdapterError, DEFAULT_GEMINI_BASE_URL, DEFAULT_GEMINI_FALLBACK_MODEL,
        DEFAULT_GEMMA_FALLBACK_MODEL, GeneratedScene, ProviderConfig, ProviderKind, ResponseMode,
        SceneOperation, SceneOperationBatch, ScenePlan, ScenePlanCanvas, ScenePlanHierarchyNode,
        ScenePlanNode, StageKind, StageOutputMode, StageSpec, can_finalize_staged_scene,
        gemini_endpoint, gemini_model_attempts, generate_scene_from_prompt_with_config,
        scene_operations_schema, scene_plan_schema,
    };
    use std::env;
    use std::sync::{Mutex, OnceLock};

    static ENV_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn lock_env_test() -> std::sync::MutexGuard<'static, ()> {
        ENV_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env test lock should not be poisoned")
    }

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
            vec![
                DEFAULT_GEMINI_FALLBACK_MODEL.to_string(),
                DEFAULT_GEMMA_FALLBACK_MODEL.to_string()
            ]
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
        let _guard = lock_env_test();
        let previous_disable_fallback = env::var(crate::DISABLE_FALLBACK_ENV_VAR).ok();
        unsafe {
            env::remove_var(crate::DISABLE_FALLBACK_ENV_VAR);
        }

        let config = ProviderConfig::for_provider(ProviderKind::Gemini)
            .with_model("gemini-2.5-flash".to_string())
            .with_fallback_models(vec![
                "gemini-2.5-flash-lite".to_string(),
                "gemini-2.5-flash-lite".to_string(),
                "gemma-4-31b-it".to_string(),
                "gemma-4-31b-it".to_string(),
            ]);
        assert_eq!(
            gemini_model_attempts(&config),
            vec![
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-flash-lite".to_string(),
                "gemma-4-31b-it".to_string()
            ]
        );

        match previous_disable_fallback {
            Some(value) => unsafe {
                env::set_var(crate::DISABLE_FALLBACK_ENV_VAR, value);
            },
            None => unsafe {
                env::remove_var(crate::DISABLE_FALLBACK_ENV_VAR);
            },
        }
    }

    #[test]
    fn disables_fallback_models_via_env() {
        let _guard = lock_env_test();
        unsafe {
            env::set_var(crate::DEFAULT_PROVIDER_ENV_VAR, "gemini");
            env::set_var(crate::DISABLE_FALLBACK_ENV_VAR, "1");
        }

        let config = ProviderConfig::from_env().expect("env config should load");
        assert!(config.fallback_models.is_empty());

        unsafe {
            env::remove_var(crate::DEFAULT_PROVIDER_ENV_VAR);
            env::remove_var(crate::DISABLE_FALLBACK_ENV_VAR);
        }
    }

    #[test]
    fn partial_staged_scene_requires_non_support_progress() {
        let support_only = vec![StageSpec {
            kind: StageKind::Support,
            output_mode: StageOutputMode::Structured,
            id: "support".to_string(),
            slot_id: "support".to_string(),
            purpose: "support".to_string(),
            target_node_ids: vec!["ground".to_string()],
            composition_hints: vec![],
            focus_group_id: None,
        }];
        assert!(!can_finalize_staged_scene(&support_only));

        let with_subject = vec![
            support_only[0].clone(),
            StageSpec {
                kind: StageKind::Subject,
                output_mode: StageOutputMode::Structured,
                id: "pelican".to_string(),
                slot_id: "pelican".to_string(),
                purpose: "subject".to_string(),
                target_node_ids: vec!["pelican".to_string()],
                composition_hints: vec![],
                focus_group_id: None,
            },
        ];
        assert!(can_finalize_staged_scene(&with_subject));
    }

    #[test]
    fn derive_stages_adds_subject_group_assembly_stage() {
        let plan = ScenePlan {
            summary: "Pelican on a bicycle".to_string(),
            canvas: ScenePlanCanvas {
                width: 1200.0,
                height: 900.0,
                background: "#f7f1df".to_string(),
            },
            style_keywords: vec!["playful".to_string()],
            major_nodes: vec![
                ScenePlanNode {
                    id: "pelican_body".to_string(),
                    node_type: "Ellipse".to_string(),
                    purpose: "Pelican body".to_string(),
                },
                ScenePlanNode {
                    id: "pelican_beak".to_string(),
                    node_type: "Path".to_string(),
                    purpose: "Pelican beak".to_string(),
                },
            ],
            composition_notes: vec!["Keep the pelican centered.".to_string()],
            hierarchy: ScenePlanHierarchyNode {
                id: "root".to_string(),
                role: "group".to_string(),
                label: Some("Root".to_string()),
                purpose: Some("Scene root".to_string()),
                children: vec![ScenePlanHierarchyNode {
                    id: "pelican_group".to_string(),
                    role: "group".to_string(),
                    label: Some("Pelican Group".to_string()),
                    purpose: Some("Grouped pelican subject".to_string()),
                    children: vec![
                        ScenePlanHierarchyNode {
                            id: "pelican_body".to_string(),
                            role: "slot".to_string(),
                            label: Some("Body".to_string()),
                            purpose: Some("Pelican body".to_string()),
                            children: vec![],
                        },
                        ScenePlanHierarchyNode {
                            id: "pelican_beak".to_string(),
                            role: "slot".to_string(),
                            label: Some("Beak".to_string()),
                            purpose: Some("Pelican beak".to_string()),
                            children: vec![],
                        },
                    ],
                }],
            },
        };

        let stages = super::derive_stages_from_plan(&plan);
        assert_eq!(stages.len(), 3);
        assert!(stages.iter().any(|stage| {
            matches!(stage.kind, StageKind::Assembly)
                && stage.slot_id == "pelican_group"
                && stage.focus_group_id.as_deref() == Some("pelican_group")
        }));
    }

    #[test]
    fn derive_stages_does_not_add_support_group_assembly_stage() {
        let plan = ScenePlan {
            summary: "Poster".to_string(),
            canvas: ScenePlanCanvas {
                width: 1200.0,
                height: 900.0,
                background: "#f7f1df".to_string(),
            },
            style_keywords: vec!["poster".to_string()],
            major_nodes: vec![
                ScenePlanNode {
                    id: "sky_slot".to_string(),
                    node_type: "Rectangle".to_string(),
                    purpose: "Background sky".to_string(),
                },
                ScenePlanNode {
                    id: "ground_slot".to_string(),
                    node_type: "Rectangle".to_string(),
                    purpose: "Ground plane".to_string(),
                },
            ],
            composition_notes: vec![],
            hierarchy: ScenePlanHierarchyNode {
                id: "root".to_string(),
                role: "group".to_string(),
                label: Some("Root".to_string()),
                purpose: Some("Scene root".to_string()),
                children: vec![ScenePlanHierarchyNode {
                    id: "background_group".to_string(),
                    role: "group".to_string(),
                    label: Some("Background Group".to_string()),
                    purpose: Some("Background support".to_string()),
                    children: vec![
                        ScenePlanHierarchyNode {
                            id: "sky_slot".to_string(),
                            role: "slot".to_string(),
                            label: Some("Sky".to_string()),
                            purpose: Some("Background sky".to_string()),
                            children: vec![],
                        },
                        ScenePlanHierarchyNode {
                            id: "ground_slot".to_string(),
                            role: "slot".to_string(),
                            label: Some("Ground".to_string()),
                            purpose: Some("Ground plane".to_string()),
                            children: vec![],
                        },
                    ],
                }],
            },
        };

        let stages = super::derive_stages_from_plan(&plan);
        assert_eq!(stages.len(), 2);
        assert!(
            !stages
                .iter()
                .any(|stage| matches!(stage.kind, StageKind::Assembly))
        );
    }

    #[test]
    fn derive_stages_prioritizes_subjects_before_support() {
        let plan = ScenePlan {
            summary: "Pelican on bike".to_string(),
            canvas: ScenePlanCanvas {
                width: 1200.0,
                height: 900.0,
                background: "#f7f1df".to_string(),
            },
            style_keywords: vec!["playful".to_string()],
            major_nodes: vec![
                ScenePlanNode {
                    id: "background_elements".to_string(),
                    node_type: "Group".to_string(),
                    purpose: "Decorative background support.".to_string(),
                },
                ScenePlanNode {
                    id: "pelican_body".to_string(),
                    node_type: "Ellipse".to_string(),
                    purpose: "Pelican body.".to_string(),
                },
                ScenePlanNode {
                    id: "pelican_beak".to_string(),
                    node_type: "Path".to_string(),
                    purpose: "Pelican beak.".to_string(),
                },
            ],
            composition_notes: vec![],
            hierarchy: ScenePlanHierarchyNode {
                id: "root".to_string(),
                role: "group".to_string(),
                label: Some("Root".to_string()),
                purpose: Some("Scene root".to_string()),
                children: vec![
                    ScenePlanHierarchyNode {
                        id: "background_group".to_string(),
                        role: "group".to_string(),
                        label: Some("Background".to_string()),
                        purpose: Some("Background support".to_string()),
                        children: vec![ScenePlanHierarchyNode {
                            id: "background_elements".to_string(),
                            role: "slot".to_string(),
                            label: Some("Background Elements".to_string()),
                            purpose: Some("Decorative background support.".to_string()),
                            children: vec![],
                        }],
                    },
                    ScenePlanHierarchyNode {
                        id: "pelican_group".to_string(),
                        role: "group".to_string(),
                        label: Some("Pelican Group".to_string()),
                        purpose: Some("Grouped pelican subject".to_string()),
                        children: vec![
                            ScenePlanHierarchyNode {
                                id: "pelican_body".to_string(),
                                role: "slot".to_string(),
                                label: Some("Body".to_string()),
                                purpose: Some("Pelican body.".to_string()),
                                children: vec![],
                            },
                            ScenePlanHierarchyNode {
                                id: "pelican_beak".to_string(),
                                role: "slot".to_string(),
                                label: Some("Beak".to_string()),
                                purpose: Some("Pelican beak.".to_string()),
                                children: vec![],
                            },
                        ],
                    },
                ],
            },
        };

        let stages = super::derive_stages_from_plan(&plan);
        let kinds = stages.iter().map(|stage| &stage.kind).collect::<Vec<_>>();
        assert!(matches!(kinds[0], StageKind::Subject));
        assert!(matches!(kinds[1], StageKind::Subject));
        assert!(matches!(kinds[2], StageKind::Assembly));
        assert!(matches!(kinds[3], StageKind::Support));
    }

    #[test]
    fn scene_plan_prompt_embeds_plan_json() {
        let plan = ScenePlan {
            summary: "Pelican on bike".to_string(),
            canvas: ScenePlanCanvas {
                width: 1200.0,
                height: 900.0,
                background: "#f7f1df".to_string(),
            },
            style_keywords: vec!["playful".to_string(), "poster".to_string()],
            major_nodes: vec![ScenePlanNode {
                id: "pelican_body".to_string(),
                node_type: "Ellipse".to_string(),
                purpose: "Main pelican body mass".to_string(),
            }],
            composition_notes: vec!["Center the bicycle".to_string()],
            hierarchy: ScenePlanHierarchyNode {
                id: "root".to_string(),
                role: "group".to_string(),
                label: Some("Root".to_string()),
                purpose: Some("Scene root".to_string()),
                children: vec![ScenePlanHierarchyNode {
                    id: "pelican_body".to_string(),
                    role: "slot".to_string(),
                    label: Some("Pelican Slot".to_string()),
                    purpose: Some("Main pelican body mass".to_string()),
                    children: vec![],
                }],
            },
        };

        let normalized = super::normalized_plan_hierarchy(&plan);
        assert_eq!(normalized.children.len(), 1);
        assert_eq!(normalized.children[0].id, "pelican_body");
    }

    #[test]
    fn scene_plan_schema_requires_major_sections() {
        let schema = scene_plan_schema();
        let required = schema["required"]
            .as_array()
            .expect("required should be an array");
        assert!(required.iter().any(|value| value == "summary"));
        assert!(required.iter().any(|value| value == "major_nodes"));
        assert!(required.iter().any(|value| value == "hierarchy"));
    }

    #[test]
    fn scene_operations_schema_includes_core_ops() {
        let schema = scene_operations_schema();
        let serialized = serde_json::to_string(&schema).expect("schema should serialize");
        assert!(serialized.contains("upsert_image_resource"));
        assert!(serialized.contains("create_group"));
        assert!(serialized.contains("create_rectangle"));
        assert!(serialized.contains("set_transform"));
        assert!(serialized.contains("replace_style"));
    }

    #[test]
    fn scene_operation_batch_round_trips() {
        let batch = SceneOperationBatch {
            summary: "Build a pelican group".to_string(),
            operations: vec![
                SceneOperation::UpsertImageResource {
                    image_ref: "pelican_patch".to_string(),
                    path: Some("assets/pelican_patch.png".to_string()),
                    prompt: Some("Painterly pelican cutout".to_string()),
                    width: Some(768.0),
                    height: Some(768.0),
                    alpha_mode: Some("straight".to_string()),
                    generation_mode: Some("raster_patch".to_string()),
                    group_id: Some("pelican_group".to_string()),
                },
                SceneOperation::CreateGroup {
                    node_id: "pelican_group".to_string(),
                    parent_id: "main_subject_group".to_string(),
                    name: "Pelican Group".to_string(),
                    x: 240.0,
                    y: 180.0,
                },
                SceneOperation::CreateEllipse {
                    node_id: "pelican_body".to_string(),
                    parent_id: "pelican_group".to_string(),
                    name: "Pelican Body".to_string(),
                    x: 0.0,
                    y: 0.0,
                    radius_x: 90.0,
                    radius_y: 60.0,
                    fill: Some("#f2f2ea".to_string()),
                },
                SceneOperation::SetTransform {
                    node_id: "pelican_group".to_string(),
                    x: Some(260.0),
                    y: Some(190.0),
                    scale_x: None,
                    scale_y: None,
                    rotation: Some(-4.0),
                    opacity: None,
                },
            ],
            notes: vec!["Use this as a future Gemini tool-call contract.".to_string()],
        };

        let json = serde_json::to_string_pretty(&batch).expect("batch should serialize");
        let reparsed =
            serde_json::from_str::<SceneOperationBatch>(&json).expect("batch should parse");
        assert_eq!(reparsed, batch);
    }

    #[test]
    fn pelican_subject_slots_prefer_raster_output() {
        assert_eq!(
            super::classify_stage_output_mode(
                StageKind::Subject,
                "pelican_body",
                "Pelican body mass",
                &["Body".to_string()],
                None,
            ),
            StageOutputMode::RasterPreferred
        );
        assert_eq!(
            super::classify_stage_output_mode(
                StageKind::Subject,
                "bicycle_frame",
                "Bicycle frame geometry",
                &["Frame".to_string()],
                None,
            ),
            StageOutputMode::Structured
        );
    }

    #[test]
    fn image_layer_reference_can_be_introduced_by_resource_upsert() {
        let scene = scene_schema::parse_scene_str(
            r##"{
  "version": "0.1",
  "document": {
    "id": "doc",
    "name": "Hybrid",
    "width": 800,
    "height": 600,
    "background": { "type": "solid", "color": "#ffffff" },
    "resources": { "images": {}, "fonts": {}, "palettes": {} },
    "root": {
      "id": "root",
      "type": "Group",
      "name": "Root",
      "visible": true,
      "locked": false,
      "blendMode": "normal",
      "transform": { "x": 0, "y": 0, "scaleX": 1, "scaleY": 1, "rotation": 0, "opacity": 1 },
      "params": {},
      "style": {},
      "children": [
        {
          "id": "pelican_body",
          "type": "Group",
          "name": "Pelican Body Slot",
          "visible": true,
          "locked": false,
          "blendMode": "normal",
          "transform": { "x": 0, "y": 0, "scaleX": 1, "scaleY": 1, "rotation": 0, "opacity": 1 },
          "params": {},
          "style": {},
          "children": [],
          "meta": {}
        }
      ],
      "meta": {}
    }
  }
}"##,
        )
        .expect("scene should parse");

        let operations = vec![
            SceneOperation::UpsertImageResource {
                image_ref: "pelican_body_patch".to_string(),
                path: None,
                prompt: Some("Painterly pelican torso patch".to_string()),
                width: Some(640.0),
                height: Some(480.0),
                alpha_mode: Some("straight".to_string()),
                generation_mode: Some("raster_patch".to_string()),
                group_id: Some("pelican_group".to_string()),
            },
            SceneOperation::CreateImageLayer {
                node_id: "pelican_body_patch_layer".to_string(),
                parent_id: "pelican_body".to_string(),
                name: "Pelican Body Patch".to_string(),
                x: 0.0,
                y: 0.0,
                image_ref: "pelican_body_patch".to_string(),
                display_width: 420.0,
                display_height: 320.0,
            },
        ];

        let issues = super::validate_stage_image_references(&scene, &operations);
        assert!(issues.is_empty(), "unexpected issues: {issues:?}");
    }
}
