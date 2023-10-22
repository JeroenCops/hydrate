pub use super::*;
use std::path::{Path};

use demo_types::mesh_adv::*;
use hydrate_base::BuiltObjectMetadata;
use hydrate_model::{BuilderRegistryBuilder, DataContainer, DataContainerMut, DataSet, Enum, HashMap, ImporterRegistryBuilder, ObjectId, Record, SchemaLinker, SchemaSet, SingleObject};
use hydrate_model::pipeline::{AssetPlugin, Builder, BuiltAsset};
use hydrate_model::pipeline::{ImportedImportable, ScannedImportable, Importer};
use serde::{Deserialize, Serialize};
use type_uuid::{TypeUuid, TypeUuidDynamic};

use demo_types::generated::{MeshAdvMaterialImportedDataRecord, MeshAdvMaterialAssetRecord, MeshAdvBlendMethodEnum, MeshAdvShadowMethodEnum};



#[derive(TypeUuid, Default)]
#[uuid = "02f17f4e-8df2-4b79-95cf-d2ee62e92a01"]
pub struct MeshAdvMaterialBuilder {}

impl Builder for MeshAdvMaterialBuilder {
    fn asset_type(&self) -> &'static str {
        MeshAdvMaterialAssetRecord::schema_name()
    }

    fn enumerate_dependencies(
        &self,
        asset_id: ObjectId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
    ) -> Vec<ObjectId> {
        vec![]
    }

    fn build_asset(
        &self,
        asset_id: ObjectId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<ObjectId, SingleObject>,
    ) -> BuiltAsset {
        //
        // Read asset data
        //
        let data_container = DataContainer::new_dataset(data_set, schema_set, asset_id);
        let x = MeshAdvMaterialAssetRecord::default();

        let base_color_factor = x.base_color_factor().get_vec4(&data_container).unwrap();
        let emissive_factor = x.emissive_factor().get_vec3(&data_container).unwrap();

        let metallic_factor = x.metallic_factor().get(&data_container).unwrap();
        let roughness_factor = x.roughness_factor().get(&data_container).unwrap();
        let normal_texture_scale = x.normal_texture_scale().get(&data_container).unwrap();

        let color_texture = x.color_texture().get(&data_container).unwrap();
        let metallic_roughness_texture = x.metallic_roughness_texture().get(&data_container).unwrap();
        let normal_texture = x.normal_texture().get(&data_container).unwrap();
        let emissive_texture = x.emissive_texture().get(&data_container).unwrap();
        let shadow_method = x.shadow_method().get(&data_container).unwrap();
        let blend_method = x.blend_method().get(&data_container).unwrap();

        let alpha_threshold = x.alpha_threshold().get(&data_container).unwrap();
        let backface_culling = x.backface_culling().get(&data_container).unwrap();
        let color_texture_has_alpha_channel = x.color_texture_has_alpha_channel().get(&data_container).unwrap();

        //
        // Create the processed data
        //
        let processed_data = MeshAdvMaterialData {
            base_color_factor,
            emissive_factor,
            metallic_factor,
            roughness_factor,
            normal_texture_scale,
            has_base_color_texture: !color_texture.is_empty(),
            base_color_texture_has_alpha_channel: color_texture_has_alpha_channel,
            has_metallic_roughness_texture: !metallic_roughness_texture.is_empty(),
            has_normal_texture: !normal_texture.is_empty(),
            has_emissive_texture: !emissive_texture.is_empty(),
            shadow_method: shadow_method.into(),
            blend_method: blend_method.into(),
            alpha_threshold,
            backface_culling,
        };

        //
        // Serialize and return
        //
        let serialized = bincode::serialize(&processed_data).unwrap();
        BuiltAsset {
            metadata: BuiltObjectMetadata {
                dependencies: vec![],
                subresource_count: 0,
                asset_type: uuid::Uuid::from_bytes(processed_data.uuid())
            },
            data: serialized
        }
    }
}


pub struct MeshAdvMaterialAssetPlugin;

impl AssetPlugin for MeshAdvMaterialAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
    ) {
        builder_registry.register_handler::<MeshAdvMaterialBuilder>(schema_linker);
    }
}
