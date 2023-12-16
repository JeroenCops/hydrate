use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use eframe::epaint::Color32;
use egui::{ColorImage, Context, SizeHint, TextureHandle, TextureOptions};
use egui::ImageData::Color;
use egui::load::{ImageLoader, ImageLoadResult, ImagePoll, LoadError, SizedTexture, TextureLoader, TextureLoadResult, TexturePoll};
use uuid::Uuid;
use hydrate_model::{AssetId, HashMap, SchemaFingerprint, SchemaSet};
use hydrate_model::pipeline::{AssetEngine, ThumbnailImage, ThumbnailProviderRegistry, ThumbnailSystemState};
use hydrate_base::lru_cache::LruCache;

const THUMBNAIL_ASSET_URI_PREFIX: &str = "thumbnail-asset://";
const THUMBNAIL_ASSET_TYPE_URI_PREFIX: &str = "thumbnail-asset-type://";
const THUMBNAIL_SPECIAL_PREFIX: &str = "thumbnail-special://";
const THUMBNAIL_CACHE_SIZE: u32 = 64;

#[derive(PartialEq)]
enum LoadState {
    Requesting,
    Loaded(Arc<ColorImage>)
}

struct ThumbnailInfo {
    asset_id: AssetId,
    count: usize,
    load_state: LoadState,
}

pub struct AssetThumbnailImageLoader {
    dummy_image: Arc<ColorImage>,
    thumbnail_cache: Mutex<LruCache<AssetId, Arc<ColorImage>>>,
    thumbnail_system_state: ThumbnailSystemState,
    thumbnail_provider_registry: ThumbnailProviderRegistry,
    default_thumbnails: HashMap<SchemaFingerprint, Arc<ColorImage>>,
    schema_set: SchemaSet,
}

impl AssetThumbnailImageLoader {
    pub fn new(
        schema_set: &SchemaSet,
        thumbnail_provider_registry: &ThumbnailProviderRegistry,
        thumbnail_system_state: &ThumbnailSystemState
    ) -> Self {
        let dummy_image = ColorImage::example();
        let mut loaded_images = HashMap::<PathBuf, Arc<ColorImage>>::default();
        let mut default_thumbnails = HashMap::default();

        for (k, v) in schema_set.schemas() {
            if let Some(record) = v.try_as_record() {
                if let Some(path) = &record.markup().default_thumbnail {
                    if let Some(loaded_image) = loaded_images.get(path) {
                        default_thumbnails.insert(*k, loaded_image.clone());
                    } else {
                        println!("open path {:?}", path);
                        let image = image::open(path).unwrap().into_rgba8();
                        let image = Arc::new(ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], &image.into_raw()));
                        loaded_images.insert(path.clone(), image.clone());
                        default_thumbnails.insert(*k, image);
                    }


                }
            }
        }

        AssetThumbnailImageLoader {
            schema_set: schema_set.clone(),
            dummy_image: Arc::new(dummy_image),
            thumbnail_cache: Mutex::new(LruCache::new(THUMBNAIL_CACHE_SIZE)),
            thumbnail_system_state: thumbnail_system_state.clone(),
            thumbnail_provider_registry: thumbnail_provider_registry.clone(),
            default_thumbnails,
        }
    }

    pub fn thumbnail_uri_for_asset(&self, schema_fingerprint: SchemaFingerprint, asset_id: AssetId) -> String {
        if self.thumbnail_provider_registry.has_provider_for_asset(schema_fingerprint) {
            format!("thumbnail-asset://{}", asset_id.as_uuid().to_string())
        } else if self.default_thumbnails.contains_key(&schema_fingerprint) {
            format!("thumbnail-asset-type://{}", schema_fingerprint.as_uuid().to_string())
        } else {
            "thumbnail-special://unknown".to_string()
        }
    }
}

impl ImageLoader for AssetThumbnailImageLoader {
    fn id(&self) -> &str {
        "hydrate_editor::AssetThumbnailImageLoader"
    }

    fn load(&self, ctx: &Context, uri: &str, size_hint: SizeHint) -> ImageLoadResult {
        if uri.starts_with(THUMBNAIL_ASSET_TYPE_URI_PREFIX) {
            let schema_fingerprint = SchemaFingerprint::from_uuid(Uuid::parse_str(&uri[THUMBNAIL_ASSET_TYPE_URI_PREFIX.len()..]).unwrap());
            if let Some(default_thumbnail) = self.default_thumbnails.get(&schema_fingerprint) {
                Ok(ImagePoll::Ready {
                    image: default_thumbnail.clone()
                })
            } else {
                Ok(ImagePoll::Ready {
                    image: self.dummy_image.clone()
                })
            }
        } else if uri.starts_with(THUMBNAIL_ASSET_URI_PREFIX) {
            let asset_id = AssetId::parse_str(&uri[THUMBNAIL_ASSET_URI_PREFIX.len()..]).unwrap();
            let mut cache = self.thumbnail_cache.lock().unwrap();
            if let Some(image) = cache.get(&asset_id) {
                Ok(ImagePoll::Ready {
                    image: image.clone()
                })
            } else if let Some(cached_entry) = self.thumbnail_system_state.request(asset_id) {
                let mut image = Arc::new(ColorImage::from_rgba_unmultiplied(
                    [cached_entry.width as usize, cached_entry.height as usize], &cached_entry.pixel_data
                ));

                Ok(ImagePoll::Ready {
                    image
                })
            } else {
                Ok(ImagePoll::Pending {
                    size: None,
                })
            }
        } else {
            Err(LoadError::NotSupported)
        }
    }

    fn forget(&self, uri: &str) {
        if uri.starts_with(THUMBNAIL_ASSET_URI_PREFIX) {
            let asset_id = AssetId::parse_str(&uri[THUMBNAIL_ASSET_URI_PREFIX.len()..]).unwrap();
            self.thumbnail_system_state.forget(asset_id);
        }
        // if uri.starts_with(THUMBNAIL_URI_PREFIX) {
        //     let asset_id = AssetId::parse_str(&uri[THUMBNAIL_URI_PREFIX.len()..]).unwrap();
        //     let mut inner = self.inner.lock().unwrap();
        //     inner.cache.remove(&asset_id);
        //     inner.requested_thumbnails_list_needs_update = true;
        // }
    }

    fn forget_all(&self) {
        self.thumbnail_system_state.forget_all();
        // let mut inner = self.inner.lock().unwrap();
        // inner.cache = LruCache::new(THUMBNAIL_CACHE_SIZE);
        // inner.requested_thumbnails_list_needs_update = true;
    }

    fn byte_size(&self) -> usize {
        //TODO: Implement this
        0
    }
}




pub struct AssetThumbnailTextureLoader {
    cache: Mutex<LruCache<(String, TextureOptions), TextureHandle>>,
}

impl AssetThumbnailTextureLoader {
    pub fn new() -> Self {
        AssetThumbnailTextureLoader {
            cache: Mutex::new(LruCache::new(THUMBNAIL_CACHE_SIZE))
        }
    }
}

impl TextureLoader for AssetThumbnailTextureLoader {
    fn id(&self) -> &str {
        "hydrate_editor::AssetThumbnailTextureLoader"
    }

    fn load(
        &self,
        ctx: &Context,
        uri: &str,
        texture_options: TextureOptions,
        size_hint: SizeHint,
    ) -> TextureLoadResult {
        let mut cache = self.cache.lock().unwrap();
        if let Some(handle) = cache.get(&(uri.into(), texture_options)) {
            let texture = SizedTexture::from_handle(handle);
            Ok(TexturePoll::Ready { texture })
        } else {
            match ctx.try_load_image(uri, size_hint)? {
                ImagePoll::Pending { size } => Ok(TexturePoll::Pending { size }),
                ImagePoll::Ready { image } => {
                    let handle = ctx.load_texture(uri, image, texture_options);
                    let texture = SizedTexture::from_handle(&handle);
                    cache.insert((uri.into(), texture_options), handle);
                    Ok(TexturePoll::Ready { texture })
                }
            }
        }
    }

    fn forget(&self, uri: &str) {
        let mut pending_remove = Vec::default();

        let mut cache = self.cache.lock().unwrap();
        for (asset_id, thumbnail_info) in cache.pairs_mut().iter_mut().filter_map(|x| x.as_mut()) {
            if asset_id.0 == uri {
                pending_remove.push(asset_id.clone());
            }
        }

        for key in pending_remove {
            cache.remove(&key);
        }
    }

    fn forget_all(&self) {
        let mut cache = self.cache.lock().unwrap();
        *cache = LruCache::new(THUMBNAIL_CACHE_SIZE)
    }

    fn end_frame(&self, _: usize) {}

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .unwrap()
            .pairs()
            .iter()
            .filter_map(|x| x.as_ref())
            .map(|(k, v)| v.byte_size())
            .sum()
    }
}
