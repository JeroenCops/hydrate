use demo_types::image::GpuImageBuiltData;
use hydrate::loader::Handle;
use std::path::PathBuf;
use demo_types::gpu_buffer::GpuBufferBuiltData;
use demo_types::mesh_adv::{MeshAdvBufferAssetData, MeshAdvMaterialAssetData, MeshAdvMaterialData, MeshAdvMeshAssetData};
use demo_types::simple_data::{Transform, TransformRef};
use hydrate::base::ArtifactId;

pub fn build_data_source_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../demo-editor/data/build_data"
    ))
}

fn main() {
    // Setup logging
    env_logger::Builder::default()
        .write_style(env_logger::WriteStyle::Always)
        .filter_level(log::LevelFilter::Debug)
        .init();

    let mut loader = hydrate::loader::AssetManager::new(build_data_source_path()).unwrap();
    loader.add_storage::<GpuImageBuiltData>();
    loader.add_storage::<GpuBufferBuiltData>();
    loader.add_storage::<Transform>();
    loader.add_storage::<TransformRef>();
    loader.add_storage::<MeshAdvMeshAssetData>();
    loader.add_storage::<MeshAdvBufferAssetData>();
    loader.add_storage::<MeshAdvMaterialAssetData>();
    loader.add_storage::<MeshAdvMaterialData>();

    // let load_handle_image: Handle<ImageBuiltData> = loader.load_asset(ObjectId(
    //     uuid::Uuid::parse_str("df737bdbfc014fc5929a5e7a0d0f1281")
    //         .unwrap()
    //         .as_u128(),
    // ));
    //
    //
    // let load_handle_mesh: Handle<GltfBuiltMeshData> = loader.load_asset(ObjectId(
    //     uuid::Uuid::parse_str("ced7b55b693240b281feed577fcc4cba")
    //         .unwrap()
    //         .as_u128(),
    // ));
    //
    //
    // let load_handle_material: Handle<GltfBuiltMaterialData> = loader.load_asset(ObjectId(
    //     uuid::Uuid::parse_str("ccd1f453d6224b2fab9bc8021a6c7dde")
    //         .unwrap()
    //         .as_u128(),
    // ));
    //
    //
    //
    // let load_handle_material2: Handle<GltfBuiltMaterialData> = loader.load_asset(ObjectId(
    //     uuid::Uuid::parse_str("ccd1f453d6224b2fab9bc8021a6c7dde")
    //         .unwrap()
    //         .as_u128(),
    // ));
    //
    //
    // let load_handle_transform: Handle<Transform> = loader.load_asset(ObjectId(
    //     uuid::Uuid::parse_str("dece7fdfc3fc4691b93101c0b25cb822")
    //         .unwrap()
    //         .as_u128(),
    // ));

    let load_handle_transform_ref: Handle<TransformRef> = loader.load_asset(ArtifactId(
        uuid::Uuid::parse_str("798bd93be6d14f459d31d7e689c28c03")
            .unwrap()
            .as_u128(),
    ));


    let load_handle_mesh_ref: Handle<MeshAdvMeshAssetData> = loader.load_asset(ArtifactId(
        uuid::Uuid::parse_str("522aaf98-5dc3-4578-a4cc-411ca6c0a826")
            .unwrap()
            .as_u128(),
    ));


    loop {
        std::thread::sleep(std::time::Duration::from_millis(15));
        loader.update();

        // let data = load_handle_image.asset(loader.storage());
        // if let Some(data) = data {
        //     //println!("{} {}", data.width, data.height);
        // } else {
        //     println!("not loaded");
        // }
        //
        // let data = load_handle_mesh.asset(loader.storage());
        // if let Some(data) = data {
        //     //println!("mesh loaded");
        // } else {
        //     println!("mesh not loaded");
        // }
        //
        // let data = load_handle_material.asset(loader.storage());
        // if let Some(data) = data {
        //     //println!("material loaded");
        // } else {
        //     println!("material not loaded");
        // }
        //
        // let data = load_handle_material2.asset(loader.storage());
        // if let Some(data) = data {
        //     //println!("material loaded");
        // } else {
        //     println!("material not loaded");
        // }
        //
        // let data = load_handle_transform.asset(loader.storage());
        // if let Some(data) = data {
        //     //println!("transform loaded {:?}", data);
        // } else {
        //     println!("material not loaded");
        // }

        let data = load_handle_transform_ref.asset(loader.storage());
        if let Some(data) = data {
            let data_inner = data.transform.asset(loader.storage());
            println!("transform loaded {:?}", data);
            println!("transform loaded {:?}", data_inner);

        } else {
            println!("material not loaded");
        }

        let data = load_handle_mesh_ref.asset(loader.storage());
        if let Some(data) = data {
            let data_full_vb = data.vertex_position_buffer.as_ref().map(|x| x.asset(loader.storage()).unwrap());
            let data_position_vb = data.vertex_position_buffer.as_ref().map(|x| x.asset(loader.storage()).unwrap());
            println!("mesh loaded {:?}", data.mesh_parts);
            if let Some(data_full_vb) = data_full_vb {
                println!("full vb {:?}", data_full_vb.data.len());
            }

            if let Some(data_position_vb) = data_position_vb {
                println!("position vb {:?}", data_position_vb.data.len());
            }

        } else {
            println!("material not loaded");
        }
    }
}
