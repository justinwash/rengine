use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::assets::{AudioClip, Color, SpriteSheet};
use crate::renderer::TextureId;
use crate::renderer3d::{MeshId, Vertex3D};
use crate::text::FontId;

#[derive(Debug)]
pub enum AssetError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Utf8 {
        path: PathBuf,
        source: std::string::FromUtf8Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    Image {
        path: PathBuf,
        source: image::ImageError,
    },
    Mesh {
        path: PathBuf,
        message: String,
    },
    Manifest {
        path: PathBuf,
        message: String,
    },
    Scene {
        path: PathBuf,
        message: String,
    },
    Audio {
        path: PathBuf,
        message: String,
    },
    InvalidSpriteSheet {
        path: PathBuf,
        texture_width: u32,
        texture_height: u32,
        cell_width: u32,
        cell_height: u32,
    },
}

impl AssetError {
    pub(crate) fn audio_message(path: &Path, message: impl Into<String>) -> Self {
        Self::Audio {
            path: path.to_path_buf(),
            message: message.into(),
        }
    }

    pub(crate) fn manifest_message(path: &Path, message: impl Into<String>) -> Self {
        Self::Manifest {
            path: path.to_path_buf(),
            message: message.into(),
        }
    }

    pub(crate) fn scene_message(path: &Path, message: impl Into<String>) -> Self {
        Self::Scene {
            path: path.to_path_buf(),
            message: message.into(),
        }
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed to read asset '{}': {source}", path.display())
            }
            Self::Utf8 { path, source } => {
                write!(f, "asset '{}' is not valid UTF-8: {source}", path.display())
            }
            Self::Json { path, source } => {
                write!(
                    f,
                    "asset '{}' contains invalid JSON: {source}",
                    path.display()
                )
            }
            Self::Image { path, source } => {
                write!(f, "failed to decode image '{}': {source}", path.display())
            }
            Self::Mesh { path, message } => {
                write!(f, "failed to load mesh '{}': {message}", path.display())
            }
            Self::Manifest { path, message } => {
                write!(f, "failed to load manifest '{}': {message}", path.display())
            }
            Self::Scene { path, message } => {
                write!(f, "failed to load scene '{}': {message}", path.display())
            }
            Self::Audio { path, message } => {
                write!(
                    f,
                    "failed to use audio asset '{}': {message}",
                    path.display()
                )
            }
            Self::InvalidSpriteSheet {
                path,
                texture_width,
                texture_height,
                cell_width,
                cell_height,
            } => write!(
                f,
                "sprite sheet '{}' with size {}x{} is not evenly divisible by cell size {}x{}",
                path.display(),
                texture_width,
                texture_height,
                cell_width,
                cell_height
            ),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Utf8 { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Image { source, .. } => Some(source),
            Self::Mesh { .. }
            | Self::Manifest { .. }
            | Self::Scene { .. }
            | Self::Audio { .. }
            | Self::InvalidSpriteSheet { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextureAsset {
    pub id: TextureId,
    pub width: u32,
    pub height: u32,
    pub path: PathBuf,
}

impl TextureAsset {
    pub fn texture(&self) -> TextureId {
        self.id
    }

    pub fn size(&self) -> glam::UVec2 {
        glam::UVec2::new(self.width, self.height)
    }
}

#[derive(Debug, Clone)]
pub struct MeshAsset {
    pub id: MeshId,
    pub vertex_count: usize,
    pub index_count: usize,
    pub path: PathBuf,
}

impl MeshAsset {
    pub fn mesh(&self) -> MeshId {
        self.id
    }
}

#[derive(Debug, Clone)]
pub struct FontAsset {
    pub id: FontId,
    pub path: PathBuf,
}

impl FontAsset {
    pub fn font(&self) -> FontId {
        self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteSheetAssetDef {
    pub path: String,
    pub cell_width: u32,
    pub cell_height: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetManifest {
    #[serde(default)]
    pub bytes: HashMap<String, String>,
    #[serde(default)]
    pub text: HashMap<String, String>,
    #[serde(default)]
    pub fonts: HashMap<String, String>,
    #[serde(default)]
    pub textures: HashMap<String, String>,
    #[serde(default)]
    pub sprite_sheets: HashMap<String, SpriteSheetAssetDef>,
    #[serde(default)]
    pub meshes: HashMap<String, String>,
    #[serde(default)]
    pub audio: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AssetSummary {
    pub bytes_count: usize,
    pub text_count: usize,
    pub font_count: usize,
    pub texture_count: usize,
    pub sprite_sheet_count: usize,
    pub mesh_count: usize,
    pub manifest_count: usize,
    pub bytes_paths: Vec<PathBuf>,
    pub text_paths: Vec<PathBuf>,
    pub font_paths: Vec<PathBuf>,
    pub texture_paths: Vec<PathBuf>,
    pub sprite_sheet_paths: Vec<PathBuf>,
    pub mesh_paths: Vec<PathBuf>,
    pub manifest_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct AssetPack {
    bytes: HashMap<String, Arc<[u8]>>,
    text: HashMap<String, Arc<str>>,
    fonts: HashMap<String, FontAsset>,
    textures: HashMap<String, TextureAsset>,
    sprite_sheets: HashMap<String, SpriteSheet>,
    meshes: HashMap<String, MeshAsset>,
    audio: HashMap<String, AudioClip>,
}

impl AssetPack {
    pub fn bytes(&self, alias: &str) -> Option<&Arc<[u8]>> {
        self.bytes.get(alias)
    }

    pub fn text(&self, alias: &str) -> Option<&Arc<str>> {
        self.text.get(alias)
    }

    pub fn font(&self, alias: &str) -> Option<&FontAsset> {
        self.fonts.get(alias)
    }

    pub fn font_id(&self, alias: &str) -> Option<FontId> {
        self.fonts.get(alias).map(|asset| asset.id)
    }

    pub fn texture(&self, alias: &str) -> Option<&TextureAsset> {
        self.textures.get(alias)
    }

    pub fn sprite_sheet(&self, alias: &str) -> Option<&SpriteSheet> {
        self.sprite_sheets.get(alias)
    }

    pub fn mesh(&self, alias: &str) -> Option<&MeshAsset> {
        self.meshes.get(alias)
    }

    pub fn audio(&self, alias: &str) -> Option<&AudioClip> {
        self.audio.get(alias)
    }

    pub fn texture_id(&self, alias: &str) -> Option<TextureId> {
        self.textures
            .get(alias)
            .map(|asset| asset.id)
            .or_else(|| self.sprite_sheets.get(alias).map(|sheet| sheet.texture))
    }

    pub(crate) fn insert_bytes(&mut self, alias: String, bytes: Arc<[u8]>) {
        self.bytes.insert(alias, bytes);
    }

    pub(crate) fn insert_text(&mut self, alias: String, text: Arc<str>) {
        self.text.insert(alias, text);
    }

    pub(crate) fn insert_font(&mut self, alias: String, font: FontAsset) {
        self.fonts.insert(alias, font);
    }

    pub(crate) fn insert_texture(&mut self, alias: String, texture: TextureAsset) {
        self.textures.insert(alias, texture);
    }

    pub(crate) fn insert_sprite_sheet(&mut self, alias: String, sheet: SpriteSheet) {
        self.sprite_sheets.insert(alias, sheet);
    }

    pub(crate) fn insert_mesh(&mut self, alias: String, mesh: MeshAsset) {
        self.meshes.insert(alias, mesh);
    }

    pub(crate) fn insert_audio(&mut self, alias: String, clip: AudioClip) {
        self.audio.insert(alias, clip);
    }
}

#[derive(Debug, Clone)]
pub struct AssetBundle {
    manifest_path: PathBuf,
    dependencies: Vec<PathBuf>,
    pack: AssetPack,
}

impl AssetBundle {
    pub(crate) fn new(manifest_path: PathBuf, dependencies: Vec<PathBuf>, pack: AssetPack) -> Self {
        Self {
            manifest_path,
            dependencies,
            pack,
        }
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn dependencies(&self) -> &[PathBuf] {
        &self.dependencies
    }

    pub fn assets(&self) -> &AssetPack {
        &self.pack
    }

    pub fn into_inner(self) -> AssetPack {
        self.pack
    }
}

impl AsRef<AssetPack> for AssetBundle {
    fn as_ref(&self) -> &AssetPack {
        &self.pack
    }
}

impl std::ops::Deref for AssetBundle {
    type Target = AssetPack;

    fn deref(&self) -> &Self::Target {
        &self.pack
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SpriteSheetKey {
    path: PathBuf,
    cell_width: u32,
    cell_height: u32,
}

pub(crate) struct AssetPipeline {
    root: PathBuf,
    bytes: HashMap<PathBuf, Arc<[u8]>>,
    text: HashMap<PathBuf, Arc<str>>,
    manifests: HashMap<PathBuf, AssetManifest>,
    fonts: HashMap<PathBuf, FontAsset>,
    textures: HashMap<PathBuf, TextureAsset>,
    sprite_sheets: HashMap<SpriteSheetKey, SpriteSheet>,
    meshes: HashMap<PathBuf, MeshAsset>,
    texture_timestamps: HashMap<PathBuf, SystemTime>,
    mesh_timestamps: HashMap<PathBuf, SystemTime>,
    manifest_timestamps: HashMap<PathBuf, SystemTime>,
    manifest_deps: HashMap<PathBuf, Vec<PathBuf>>,
    retained_paths: HashMap<PathBuf, usize>,
}

impl AssetPipeline {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            bytes: HashMap::new(),
            text: HashMap::new(),
            manifests: HashMap::new(),
            fonts: HashMap::new(),
            textures: HashMap::new(),
            sprite_sheets: HashMap::new(),
            meshes: HashMap::new(),
            texture_timestamps: HashMap::new(),
            mesh_timestamps: HashMap::new(),
            manifest_timestamps: HashMap::new(),
            manifest_deps: HashMap::new(),
            retained_paths: HashMap::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn set_root(&mut self, root: impl Into<PathBuf>) {
        self.root = root.into();
    }

    pub fn resolve_path(&self, path: &Path) -> PathBuf {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        candidate.canonicalize().unwrap_or(candidate)
    }

    pub fn loaded_texture<P: AsRef<Path>>(&self, path: P) -> Option<TextureAsset> {
        let resolved = self.resolve_path(path.as_ref());
        self.textures.get(&resolved).cloned()
    }

    pub fn load_bytes<P: AsRef<Path>>(&mut self, path: P) -> Result<Arc<[u8]>, AssetError> {
        let resolved = self.resolve_path(path.as_ref());
        if let Some(bytes) = self.bytes.get(&resolved) {
            return Ok(bytes.clone());
        }

        let bytes = fs::read(&resolved).map_err(|source| AssetError::Io {
            path: resolved.clone(),
            source,
        })?;
        let bytes: Arc<[u8]> = Arc::from(bytes.into_boxed_slice());
        self.bytes.insert(resolved, bytes.clone());
        Ok(bytes)
    }

    pub fn load_text<P: AsRef<Path>>(&mut self, path: P) -> Result<Arc<str>, AssetError> {
        let resolved = self.resolve_path(path.as_ref());
        if let Some(text) = self.text.get(&resolved) {
            return Ok(text.clone());
        }

        let bytes = self.load_bytes(&resolved)?;
        let text = String::from_utf8(bytes.to_vec()).map_err(|source| AssetError::Utf8 {
            path: resolved.clone(),
            source,
        })?;
        let text: Arc<str> = Arc::from(text);
        self.text.insert(resolved, text.clone());
        Ok(text)
    }

    pub fn load_font<P, F>(&mut self, path: P, create_font: F) -> Result<FontAsset, AssetError>
    where
        P: AsRef<Path>,
        F: FnOnce(&[u8]) -> FontId,
    {
        let resolved = self.resolve_path(path.as_ref());
        if let Some(font) = self.fonts.get(&resolved) {
            return Ok(font.clone());
        }

        let bytes = self.load_bytes(&resolved)?;
        let asset = FontAsset {
            id: create_font(bytes.as_ref()),
            path: resolved.clone(),
        };
        self.fonts.insert(resolved, asset.clone());
        Ok(asset)
    }

    pub fn load_manifest<P: AsRef<Path>>(&mut self, path: P) -> Result<AssetManifest, AssetError> {
        let resolved = self.resolve_path(path.as_ref());
        if let Some(manifest) = self.manifests.get(&resolved) {
            return Ok(manifest.clone());
        }

        let text = fs::read_to_string(&resolved).map_err(|source| AssetError::Io {
            path: resolved.clone(),
            source,
        })?;
        let manifest: AssetManifest =
            serde_json::from_str(&text).map_err(|source| AssetError::Json {
                path: resolved.clone(),
                source,
            })?;
        if let Ok(modified) = file_modified_time(&resolved) {
            self.manifest_timestamps.insert(resolved.clone(), modified);
        }
        self.manifests.insert(resolved, manifest.clone());
        Ok(manifest)
    }

    pub fn load_texture<P, F>(
        &mut self,
        path: P,
        create_texture: F,
    ) -> Result<TextureAsset, AssetError>
    where
        P: AsRef<Path>,
        F: FnOnce(u32, u32, &[u8]) -> TextureId,
    {
        let resolved = self.resolve_path(path.as_ref());
        if let Some(texture) = self.textures.get(&resolved) {
            return Ok(texture.clone());
        }

        let (width, height, rgba) = self.read_image_rgba(&resolved)?;
        let id = create_texture(width, height, &rgba);
        let asset = TextureAsset {
            id,
            width,
            height,
            path: resolved.clone(),
        };
        if let Ok(modified) = file_modified_time(&resolved) {
            self.texture_timestamps.insert(resolved.clone(), modified);
        }
        self.textures.insert(resolved, asset.clone());
        Ok(asset)
    }

    pub fn load_sprite_sheet<P, F>(
        &mut self,
        path: P,
        cell_width: u32,
        cell_height: u32,
        create_texture: F,
    ) -> Result<SpriteSheet, AssetError>
    where
        P: AsRef<Path>,
        F: FnOnce(u32, u32, &[u8]) -> TextureId,
    {
        let texture = self.load_texture(path.as_ref(), create_texture)?;
        let key = SpriteSheetKey {
            path: texture.path.clone(),
            cell_width,
            cell_height,
        };

        if let Some(sheet) = self.sprite_sheets.get(&key) {
            return Ok(sheet.clone());
        }

        if cell_width == 0
            || cell_height == 0
            || texture.width % cell_width != 0
            || texture.height % cell_height != 0
        {
            return Err(AssetError::InvalidSpriteSheet {
                path: texture.path.clone(),
                texture_width: texture.width,
                texture_height: texture.height,
                cell_width,
                cell_height,
            });
        }

        let sheet = SpriteSheet::new(
            texture.id,
            texture.width,
            texture.height,
            cell_width,
            cell_height,
        );
        self.sprite_sheets.insert(key, sheet.clone());
        Ok(sheet)
    }

    pub fn load_obj_mesh<P, F>(&mut self, path: P, create_mesh: F) -> Result<MeshAsset, AssetError>
    where
        P: AsRef<Path>,
        F: FnOnce(Vec<Vertex3D>, Vec<u32>) -> MeshId,
    {
        self.load_mesh(path, create_mesh)
    }

    pub fn load_gltf_mesh<P, F>(&mut self, path: P, create_mesh: F) -> Result<MeshAsset, AssetError>
    where
        P: AsRef<Path>,
        F: FnOnce(Vec<Vertex3D>, Vec<u32>) -> MeshId,
    {
        self.load_mesh(path, create_mesh)
    }

    pub fn load_mesh<P, F>(&mut self, path: P, create_mesh: F) -> Result<MeshAsset, AssetError>
    where
        P: AsRef<Path>,
        F: FnOnce(Vec<Vertex3D>, Vec<u32>) -> MeshId,
    {
        let resolved = self.resolve_path(path.as_ref());
        if let Some(mesh) = self.meshes.get(&resolved) {
            return Ok(mesh.clone());
        }

        let (mut vertices, mut indices) = read_mesh(&resolved)?;
        fix_winding_from_normals(&vertices, &mut indices);
        if vertices
            .iter()
            .all(|vertex| vertex.normal == [0.0, 0.0, 0.0])
        {
            compute_flat_normals(&mut vertices, &indices);
        }

        let vertex_count = vertices.len();
        let index_count = indices.len();
        let id = create_mesh(vertices, indices);
        let asset = MeshAsset {
            id,
            vertex_count,
            index_count,
            path: resolved.clone(),
        };
        if let Ok(modified) = file_modified_time(&resolved) {
            self.mesh_timestamps.insert(resolved.clone(), modified);
        }
        self.meshes.insert(resolved, asset.clone());
        Ok(asset)
    }

    pub fn reload_changed_textures<F>(
        &mut self,
        mut replace_texture: F,
    ) -> Vec<Result<PathBuf, AssetError>>
    where
        F: FnMut(TextureId, u32, u32, &[u8]),
    {
        let watched: Vec<(PathBuf, TextureAsset)> = self
            .textures
            .iter()
            .map(|(path, texture)| (path.clone(), texture.clone()))
            .collect();
        let mut results = Vec::new();

        for (path, existing) in watched {
            let Ok(modified) = file_modified_time(&path) else {
                continue;
            };
            let changed = self
                .texture_timestamps
                .get(&path)
                .map(|known| modified > *known)
                .unwrap_or(true);
            if !changed {
                continue;
            }

            match self.read_image_rgba(&path) {
                Ok((width, height, rgba)) => {
                    replace_texture(existing.id, width, height, &rgba);
                    if let Some(texture) = self.textures.get_mut(&path) {
                        texture.width = width;
                        texture.height = height;
                    }
                    self.update_sprite_sheet_dimensions(&path, width, height);
                    self.texture_timestamps.insert(path.clone(), modified);
                    results.push(Ok(path));
                }
                Err(error) => results.push(Err(error)),
            }
        }

        results
    }

    pub fn reload_changed_meshes<F>(
        &mut self,
        mut replace_mesh: F,
    ) -> Vec<Result<PathBuf, AssetError>>
    where
        F: FnMut(MeshId, Vec<Vertex3D>, Vec<u32>),
    {
        let watched: Vec<(PathBuf, MeshAsset)> = self
            .meshes
            .iter()
            .map(|(path, mesh)| (path.clone(), mesh.clone()))
            .collect();
        let mut results = Vec::new();

        for (path, existing) in watched {
            let Ok(modified) = file_modified_time(&path) else {
                continue;
            };
            let changed = self
                .mesh_timestamps
                .get(&path)
                .map(|known| modified > *known)
                .unwrap_or(true);
            if !changed {
                continue;
            }

            match read_mesh(&path) {
                Ok((mut vertices, mut indices)) => {
                    fix_winding_from_normals(&vertices, &mut indices);
                    if vertices
                        .iter()
                        .all(|vertex| vertex.normal == [0.0, 0.0, 0.0])
                    {
                        compute_flat_normals(&mut vertices, &indices);
                    }
                    let vertex_count = vertices.len();
                    let index_count = indices.len();
                    replace_mesh(existing.id, vertices, indices);
                    if let Some(mesh) = self.meshes.get_mut(&path) {
                        mesh.vertex_count = vertex_count;
                        mesh.index_count = index_count;
                    }
                    self.mesh_timestamps.insert(path.clone(), modified);
                    results.push(Ok(path));
                }
                Err(error) => results.push(Err(error)),
            }
        }

        results
    }

    pub fn invalidate_changed_manifests(&mut self) -> Vec<PathBuf> {
        let watched: Vec<PathBuf> = self.manifests.keys().cloned().collect();
        let mut invalidated = Vec::new();

        for path in watched {
            let Ok(modified) = file_modified_time(&path) else {
                continue;
            };
            let changed = self
                .manifest_timestamps
                .get(&path)
                .map(|known| modified > *known)
                .unwrap_or(true);
            if changed {
                self.manifests.remove(&path);
                self.manifest_timestamps.insert(path.clone(), modified);
                invalidated.push(path);
            }
        }

        invalidated
    }

    pub fn validate_manifest<P: AsRef<Path>>(&self, path: P) -> Vec<AssetError> {
        let resolved = self.resolve_path(path.as_ref());
        let text = match fs::read_to_string(&resolved) {
            Ok(t) => t,
            Err(source) => {
                return vec![AssetError::Io {
                    path: resolved,
                    source,
                }];
            }
        };
        let manifest: AssetManifest = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(source) => {
                return vec![AssetError::Json {
                    path: resolved,
                    source,
                }];
            }
        };

        let mut errors = Vec::new();

        let all_paths: Vec<(&str, &str)> = manifest
            .bytes
            .iter()
            .map(|(alias, p)| (alias.as_str(), p.as_str()))
            .chain(
                manifest
                    .text
                    .iter()
                    .map(|(alias, p)| (alias.as_str(), p.as_str())),
            )
            .chain(
                manifest
                    .fonts
                    .iter()
                    .map(|(alias, p)| (alias.as_str(), p.as_str())),
            )
            .chain(
                manifest
                    .textures
                    .iter()
                    .map(|(alias, p)| (alias.as_str(), p.as_str())),
            )
            .chain(
                manifest
                    .meshes
                    .iter()
                    .map(|(alias, p)| (alias.as_str(), p.as_str())),
            )
            .chain(
                manifest
                    .audio
                    .iter()
                    .map(|(alias, p)| (alias.as_str(), p.as_str())),
            )
            .collect();

        for (_alias, rel_path) in &all_paths {
            let file_path = self.resolve_path(Path::new(rel_path));
            if !file_path.exists() {
                errors.push(AssetError::Io {
                    path: file_path,
                    source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
                });
            }
        }

        for (_alias, sheet_def) in &manifest.sprite_sheets {
            let file_path = self.resolve_path(Path::new(&sheet_def.path));
            if !file_path.exists() {
                errors.push(AssetError::Io {
                    path: file_path.clone(),
                    source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
                });
            } else if sheet_def.cell_width == 0 || sheet_def.cell_height == 0 {
                errors.push(AssetError::InvalidSpriteSheet {
                    path: file_path,
                    texture_width: 0,
                    texture_height: 0,
                    cell_width: sheet_def.cell_width,
                    cell_height: sheet_def.cell_height,
                });
            } else {
                match self.read_image_rgba(&file_path) {
                    Ok((w, h, _)) => {
                        if w % sheet_def.cell_width != 0 || h % sheet_def.cell_height != 0 {
                            errors.push(AssetError::InvalidSpriteSheet {
                                path: file_path,
                                texture_width: w,
                                texture_height: h,
                                cell_width: sheet_def.cell_width,
                                cell_height: sheet_def.cell_height,
                            });
                        }
                    }
                    Err(e) => {
                        errors.push(e);
                    }
                }
            }
        }

        errors
    }

    pub(crate) fn peek_manifest<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<AssetManifest, AssetError> {
        let resolved = self.resolve_path(path.as_ref());
        let text = fs::read_to_string(&resolved).map_err(|source| AssetError::Io {
            path: resolved.clone(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| AssetError::Json {
            path: resolved,
            source,
        })
    }

    pub(crate) fn retain_bundle(&mut self, manifest_path: &Path, dependencies: &[PathBuf]) {
        let manifest_path = self.resolve_path(manifest_path);
        self.retain_path(manifest_path);
        for dependency in dependencies {
            self.retain_path(dependency.clone());
        }
    }

    pub(crate) fn sync_retained_bundle(
        &mut self,
        _manifest_path: &Path,
        old_dependencies: &[PathBuf],
        new_dependencies: &[PathBuf],
    ) -> Vec<PathBuf> {
        for dependency in new_dependencies {
            if !old_dependencies.contains(dependency) {
                self.retain_path(dependency.clone());
            }
        }

        let mut released = Vec::new();
        for dependency in old_dependencies {
            if !new_dependencies.contains(dependency) {
                if let Some(path) = self.release_path(dependency.clone()) {
                    released.push(path);
                }
            }
        }
        released.sort();
        released.dedup();
        released
    }

    pub(crate) fn release_bundle(
        &mut self,
        manifest_path: &Path,
        dependencies: &[PathBuf],
    ) -> Vec<PathBuf> {
        let mut released = Vec::new();
        if let Some(path) = self.release_path(self.resolve_path(manifest_path)) {
            released.push(path);
        }
        for dependency in dependencies {
            if let Some(path) = self.release_path(dependency.clone()) {
                released.push(path);
            }
        }
        released.sort();
        released.dedup();
        released
    }

    pub(crate) fn record_manifest_deps(&mut self, manifest_path: PathBuf, mut deps: Vec<PathBuf>) {
        deps.push(manifest_path.clone());
        deps.sort();
        deps.dedup();
        self.manifest_deps.insert(manifest_path, deps);
    }

    fn retain_path(&mut self, path: PathBuf) {
        *self.retained_paths.entry(path).or_insert(0) += 1;
    }

    fn release_path(&mut self, path: PathBuf) -> Option<PathBuf> {
        let Some(count) = self.retained_paths.get_mut(&path) else {
            return None;
        };

        if *count > 1 {
            *count -= 1;
            return None;
        }

        self.retained_paths.remove(&path);
        Some(path)
    }

    pub fn manifest_dependencies<P: AsRef<Path>>(&self, path: P) -> Option<&[PathBuf]> {
        let resolved = self.resolve_path(path.as_ref());
        self.manifest_deps.get(&resolved).map(|v| v.as_slice())
    }

    pub fn loaded_asset_summary(&self) -> AssetSummary {
        AssetSummary {
            bytes_count: self.bytes.len(),
            text_count: self.text.len(),
            font_count: self.fonts.len(),
            texture_count: self.textures.len(),
            sprite_sheet_count: self.sprite_sheets.len(),
            mesh_count: self.meshes.len(),
            manifest_count: self.manifests.len(),
            bytes_paths: self.bytes.keys().cloned().collect(),
            text_paths: self.text.keys().cloned().collect(),
            font_paths: self.fonts.keys().cloned().collect(),
            texture_paths: self.textures.keys().cloned().collect(),
            sprite_sheet_paths: self.sprite_sheets.keys().map(|k| k.path.clone()).collect(),
            mesh_paths: self.meshes.keys().cloned().collect(),
            manifest_paths: self.manifests.keys().cloned().collect(),
        }
    }

    pub fn unload_texture<P: AsRef<Path>>(&mut self, path: P) {
        let resolved = self.resolve_path(path.as_ref());
        self.textures.remove(&resolved);
        self.texture_timestamps.remove(&resolved);
        self.sprite_sheets.retain(|key, _| key.path != resolved);
    }

    pub fn unload_manifest<P: AsRef<Path>>(&mut self, path: P) {
        let resolved = self.resolve_path(path.as_ref());
        self.manifests.remove(&resolved);
        self.manifest_timestamps.remove(&resolved);
        self.manifest_deps.remove(&resolved);
    }

    pub fn unload_mesh<P: AsRef<Path>>(&mut self, path: P) {
        let resolved = self.resolve_path(path.as_ref());
        self.meshes.remove(&resolved);
        self.mesh_timestamps.remove(&resolved);
    }

    pub fn unload_data<P: AsRef<Path>>(&mut self, path: P) {
        let resolved = self.resolve_path(path.as_ref());
        self.bytes.remove(&resolved);
        self.text.remove(&resolved);
    }

    fn read_image_rgba(&self, path: &Path) -> Result<(u32, u32, Vec<u8>), AssetError> {
        let bytes = fs::read(path).map_err(|source| AssetError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let image = image::load_from_memory(&bytes).map_err(|source| AssetError::Image {
            path: path.to_path_buf(),
            source,
        })?;
        let rgba = image.to_rgba8();
        Ok((rgba.width(), rgba.height(), rgba.into_raw()))
    }

    fn update_sprite_sheet_dimensions(&mut self, path: &Path, width: u32, height: u32) {
        let keys: Vec<SpriteSheetKey> = self
            .sprite_sheets
            .keys()
            .filter(|key| key.path == path)
            .cloned()
            .collect();

        for key in keys {
            if width % key.cell_width != 0 || height % key.cell_height != 0 {
                continue;
            }
            if let Some(sheet) = self.sprite_sheets.get_mut(&key) {
                sheet.texture_width = width;
                sheet.texture_height = height;
            }
        }
    }
}

impl Default for AssetPipeline {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

fn read_mesh(path: &Path) -> Result<(Vec<Vertex3D>, Vec<u32>), AssetError> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("obj") => read_obj_mesh(path),
        Some("gltf") | Some("glb") => read_gltf_mesh(path),
        Some(other) => Err(AssetError::Mesh {
            path: path.to_path_buf(),
            message: format!("unsupported mesh format '.{other}'"),
        }),
        None => Err(AssetError::Mesh {
            path: path.to_path_buf(),
            message: "mesh path has no file extension".into(),
        }),
    }
}

fn read_obj_mesh(path: &Path) -> Result<(Vec<Vertex3D>, Vec<u32>), AssetError> {
    let options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ..Default::default()
    };
    let (models, _) = tobj::load_obj(path, &options).map_err(|error| AssetError::Mesh {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    if models.is_empty() {
        return Err(AssetError::Mesh {
            path: path.to_path_buf(),
            message: "file did not contain any meshes".into(),
        });
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for model in models {
        let mesh = model.mesh;
        if mesh.positions.len() % 3 != 0 {
            return Err(AssetError::Mesh {
                path: path.to_path_buf(),
                message: format!("mesh '{}' has malformed vertex positions", model.name),
            });
        }

        let base = vertices.len() as u32;
        let vertex_count = mesh.positions.len() / 3;
        let has_normals = mesh.normals.len() == mesh.positions.len();

        for index in 0..vertex_count {
            let position = [
                mesh.positions[index * 3],
                mesh.positions[index * 3 + 1],
                mesh.positions[index * 3 + 2],
            ];
            let normal = if has_normals {
                [
                    mesh.normals[index * 3],
                    mesh.normals[index * 3 + 1],
                    mesh.normals[index * 3 + 2],
                ]
            } else {
                [0.0, 0.0, 0.0]
            };
            vertices.push(Vertex3D::new(position, normal, Color::WHITE));
        }

        indices.extend(mesh.indices.iter().map(|index| base + *index));
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err(AssetError::Mesh {
            path: path.to_path_buf(),
            message: "mesh contained no drawable geometry".into(),
        });
    }

    Ok((vertices, indices))
}

fn read_gltf_mesh(path: &Path) -> Result<(Vec<Vertex3D>, Vec<u32>), AssetError> {
    let (document, buffers, _) = gltf::import(path).map_err(|error| AssetError::Mesh {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
            let Some(positions) = reader.read_positions() else {
                continue;
            };
            let normals: Option<Vec<[f32; 3]>> = reader.read_normals().map(|iter| iter.collect());

            let base = vertices.len() as u32;
            for (index, position) in positions.enumerate() {
                let normal = normals
                    .as_ref()
                    .and_then(|data| data.get(index).copied())
                    .unwrap_or([0.0, 0.0, 0.0]);
                vertices.push(Vertex3D::new(position, normal, Color::WHITE));
            }

            if let Some(read_indices) = reader.read_indices() {
                indices.extend(read_indices.into_u32().map(|index| base + index));
            } else {
                let vertex_count = vertices.len() as u32 - base;
                if vertex_count % 3 != 0 {
                    return Err(AssetError::Mesh {
                        path: path.to_path_buf(),
                        message: format!(
                            "primitive '{}' omitted indices and is not triangulated",
                            mesh.name().unwrap_or("unnamed")
                        ),
                    });
                }
                indices.extend((0..vertex_count).map(|index| base + index));
            }
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err(AssetError::Mesh {
            path: path.to_path_buf(),
            message: "file did not contain any readable mesh primitives".into(),
        });
    }

    Ok((vertices, indices))
}

fn compute_flat_normals(vertices: &mut [Vertex3D], indices: &[u32]) {
    for triangle in indices.chunks_exact(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        let p0 = glam::Vec3::from_array(vertices[i0].position);
        let p1 = glam::Vec3::from_array(vertices[i1].position);
        let p2 = glam::Vec3::from_array(vertices[i2].position);

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let face = edge1.cross(edge2);
        if face.length_squared() == 0.0 {
            continue;
        }

        let normal = face.normalize();
        for &index in triangle {
            let vertex = &mut vertices[index as usize];
            let accum = glam::Vec3::from_array(vertex.normal) + normal;
            vertex.normal = accum.to_array();
        }
    }

    for vertex in vertices.iter_mut() {
        let normal = glam::Vec3::from_array(vertex.normal);
        vertex.normal = if normal.length_squared() > 0.0 {
            normal.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };
    }
}

fn file_modified_time(path: &Path) -> Result<SystemTime, std::io::Error> {
    fs::metadata(path)?.modified()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{compute_flat_normals, AssetBundle, AssetPack, AssetPipeline, FontAsset};
    use crate::assets::Color;
    use crate::renderer3d::Vertex3D;
    use crate::text::FontId;

    #[test]
    fn computes_normals_for_triangle() {
        let mut vertices = vec![
            Vertex3D::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], Color::WHITE),
            Vertex3D::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], Color::WHITE),
            Vertex3D::new([0.0, 1.0, 0.0], [0.0, 0.0, 0.0], Color::WHITE),
        ];
        let indices = vec![0, 1, 2];

        compute_flat_normals(&mut vertices, &indices);

        for vertex in vertices {
            assert_eq!(vertex.normal, [0.0, 0.0, 1.0]);
        }
    }

    #[test]
    fn asset_pack_returns_manifest_font_handles() {
        let mut pack = AssetPack::default();
        pack.insert_font(
            "mono".into(),
            FontAsset {
                id: FontId(1),
                path: PathBuf::from("mono.ttf"),
            },
        );

        assert_eq!(pack.font_id("mono"), Some(FontId(1)));
        assert_eq!(pack.font("mono").map(|font| font.font()), Some(FontId(1)));
    }

    #[test]
    fn asset_bundle_exposes_pack_and_metadata() {
        let mut pack = AssetPack::default();
        pack.insert_font(
            "body".into(),
            FontAsset {
                id: FontId(2),
                path: PathBuf::from("body.ttf"),
            },
        );

        let manifest_path = PathBuf::from("ui.assets.json");
        let deps = vec![PathBuf::from("body.ttf"), PathBuf::from("panel.png")];
        let bundle = AssetBundle::new(manifest_path.clone(), deps.clone(), pack);

        assert_eq!(bundle.manifest_path(), manifest_path.as_path());
        assert_eq!(bundle.dependencies(), deps.as_slice());
        assert_eq!(bundle.font_id("body"), Some(FontId(2)));
        assert_eq!(bundle.assets().font_id("body"), Some(FontId(2)));
    }

    #[test]
    fn retained_bundle_release_waits_for_last_owner() {
        let mut pipeline = AssetPipeline::default();
        let shared = pipeline.resolve_path(Path::new("shared.png"));
        let menu_only = pipeline.resolve_path(Path::new("menu.txt"));
        let menu_manifest = pipeline.resolve_path(Path::new("menu.assets.json"));
        let hud_manifest = pipeline.resolve_path(Path::new("hud.assets.json"));

        pipeline.retain_bundle(&menu_manifest, &[shared.clone(), menu_only.clone()]);
        pipeline.retain_bundle(&hud_manifest, std::slice::from_ref(&shared));

        assert_eq!(
            pipeline.release_bundle(&menu_manifest, &[shared.clone(), menu_only.clone()]),
            vec![menu_manifest.clone(), menu_only.clone()]
        );
        assert_eq!(
            pipeline.release_bundle(&hud_manifest, std::slice::from_ref(&shared)),
            vec![hud_manifest.clone(), shared]
        );
    }

    #[test]
    fn retained_bundle_reload_only_releases_removed_dependencies() {
        let mut pipeline = AssetPipeline::default();
        let manifest = pipeline.resolve_path(Path::new("garage.assets.json"));
        let old_shared = pipeline.resolve_path(Path::new("old_shared.png"));
        let old_only = pipeline.resolve_path(Path::new("old_only.txt"));
        let new_shared = pipeline.resolve_path(Path::new("new_shared.png"));

        pipeline.retain_bundle(&manifest, &[old_shared.clone(), old_only.clone()]);

        assert_eq!(
            pipeline.sync_retained_bundle(
                &manifest,
                &[old_shared.clone(), old_only.clone()],
                &[old_shared.clone(), new_shared.clone()],
            ),
            vec![old_only.clone()]
        );
        assert_eq!(
            pipeline.release_bundle(&manifest, &[old_shared.clone(), new_shared.clone()]),
            vec![manifest, new_shared, old_shared]
        );
    }
}

fn fix_winding_from_normals(vertices: &[Vertex3D], indices: &mut [u32]) {
    let mut agreement = 0i32;

    for triangle in indices.chunks_exact(3).take(128) {
        let p0 = glam::Vec3::from_array(vertices[triangle[0] as usize].position);
        let p1 = glam::Vec3::from_array(vertices[triangle[1] as usize].position);
        let p2 = glam::Vec3::from_array(vertices[triangle[2] as usize].position);
        let face = (p1 - p0).cross(p2 - p0);
        if face.length_squared() <= f32::EPSILON {
            continue;
        }

        let normal_sum = glam::Vec3::from_array(vertices[triangle[0] as usize].normal)
            + glam::Vec3::from_array(vertices[triangle[1] as usize].normal)
            + glam::Vec3::from_array(vertices[triangle[2] as usize].normal);
        if normal_sum.length_squared() <= f32::EPSILON {
            continue;
        }

        if face.dot(normal_sum) >= 0.0 {
            agreement += 1;
        } else {
            agreement -= 1;
        }
    }

    if agreement < 0 {
        for triangle in indices.chunks_exact_mut(3) {
            triangle.swap(1, 2);
        }
    }
}
