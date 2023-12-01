use crate::{ImporterRegistry, ImportType};
use crate::{DynEditContext, PipelineResult};
use crate::{Importer, ScanContext, ScannedImportable};
use hydrate_base::hashing::HashSet;
use hydrate_data::{AssetId, AssetLocation, AssetName, HashMap, ImportInfo, ImporterId};
use hydrate_data::{ImportableName, PathReference};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
use hydrate_schema::SchemaRecord;

#[derive(Debug, Clone)]
pub struct RequestedImportable {
    pub asset_id: AssetId,
    pub schema: SchemaRecord,
    pub asset_name: AssetName,
    pub asset_location: AssetLocation,
    //pub importer_id: ImporterId,
    pub source_file: PathReference,
    pub path_references: HashMap<PathReference, AssetId>,
    pub replace_with_default_asset: bool,
}

#[derive(Debug)]
pub struct ImportToQueue {
    pub source_file_path: PathBuf,
    pub importer_id: ImporterId,
    pub requested_importables: HashMap<ImportableName, RequestedImportable>,
    pub import_type: ImportType,
}

// pub fn create_import_info(
//     source_file_path: &Path,
//     importer: &Arc<dyn Importer>,
//     scanned_importable: &ScannedImportable,
// ) -> ImportInfo {
//     let mut file_references = Vec::default();
//     for file_reference in &scanned_importable.referenced_source_files {
//         file_references.push(file_reference.path_reference.clone());
//     }
//
//     let source_file = PathReference {
//         path: source_file_path.to_string_lossy().to_string(),
//         importable_name: scanned_importable.name.clone(),
//     };
//
//     //
//     // When we import, set the import info so we track where the import comes from
//     //
//     ImportInfo::new(importer.importer_id(), source_file, file_references, 0, 0, 0)
// }

pub fn create_asset_name(
    source_file_path: &Path,
    scanned_importable: &ScannedImportable,
) -> AssetName {
    if let Some(file_name) = source_file_path.file_name() {
        let file_name = file_name.to_string_lossy();
        if let Some(importable_name) = &scanned_importable.name.name() {
            AssetName::new(format!("{}.{}", file_name, importable_name))
        } else {
            AssetName::new(file_name.to_string())
        }
    } else {
        AssetName::empty()
    }
}

pub fn recursively_gather_import_operations_and_create_assets(
    source_file_path: &Path,
    importer: &Arc<dyn Importer>,
    editor_context: &dyn DynEditContext,
    importer_registry: &ImporterRegistry,
    //asset_engine: &AssetEngine,
    selected_import_location: &AssetLocation,

    // In addition to being the imports that need to be queued, this is also the assets that were
    // created. Pre-existing but referenced assets won't be in this list
    imports_to_queue: &mut Vec<ImportToQueue>,
) -> PipelineResult<HashMap<ImportableName, AssetId>> {
    //
    // We now build a list of things we will be importing from the file.
    // 1. Scan the file to see what's available
    // 2. Create/Find assets for all the things we want to import
    // 3. Enqueue the import operation
    //
    let mut requested_importables = HashMap::<ImportableName, RequestedImportable>::default();
    let mut imported_asset_ids = HashMap::default();

    let mut scanned_importables = HashMap::default();

    importer.scan_file(ScanContext::new(
        source_file_path,
        editor_context.schema_set(),
        importer_registry,
        &mut scanned_importables,
    ))?;

    for (_, scanned_importable) in &scanned_importables {
        // let mut file_references = Vec::default();
        // for file_reference in &scanned_importable.file_references {
        //     file_references.push(file_reference.path.clone());
        // }
        //
        // //
        // // When we import, set the import info so we track where the import comes from
        // //
        // let import_info = ImportInfo::new(
        //     importer.importer_id(),
        //     source_file_path.to_path_buf(),
        //     scanned_importable.name.clone().unwrap_or_default(),
        //     file_references,
        // );
        //let import_info = create_import_info(source_file_path, importer, scanned_importable);

        //
        // Pick name for the asset for this file
        //
        let object_name = create_asset_name(source_file_path, scanned_importable);

        let mut referenced_source_file_asset_ids = Vec::default();

        //TODO: Check referenced source files to find existing imported assets or import referenced files
        for referenced_source_file in &scanned_importable.referenced_source_files {
            let referenced_file_absolute = PathReference::canonicalize_relative(
                source_file_path,
                &referenced_source_file.path_reference,
            );

            // Does it already exist?
            let mut found = None;

            // Have we already iterated over it and will be creating it later?
            for (_, requested_importable) in &requested_importables {
                if requested_importable.source_file == referenced_source_file.path_reference {
                    found = Some(requested_importable.asset_id);
                }
            }

            // Have we imported it previously?
            if found.is_none() {
                for (asset_id, _) in editor_context.data_set().assets() {
                    if let Some(import_info) = editor_context.data_set().import_info(*asset_id) {
                        if *import_info.source_file() == referenced_source_file.path_reference {
                            found = Some(*asset_id);
                        }
                    }
                }
            }

            // If we didn't find it, try to import it
            if found.is_none() {
                let importer = importer_registry
                    .importer(referenced_source_file.importer_id)
                    .unwrap();
                found = recursively_gather_import_operations_and_create_assets(
                    &Path::new(&referenced_file_absolute.path),
                    importer,
                    editor_context,
                    importer_registry,
                    selected_import_location,
                    imports_to_queue,
                )?
                .get(&referenced_file_absolute.importable_name)
                .copied();
            }

            referenced_source_file_asset_ids.push(found);
        }

        // At this point all referenced files have either been found or scanned
        assert_eq!(
            referenced_source_file_asset_ids.len(),
            scanned_importable.referenced_source_files.len()
        );

        //TODO: We should avoid writing into the dataset here, instead it should occur when we actually
        // do the import so assets don't end up in a half-initialized state
        // let asset_id = editor_context.new_asset(
        //     &object_name,
        //     selected_import_location,
        //     &scanned_importable.asset_type,
        // );
        // editor_context
        //     .set_import_info(asset_id, import_info.clone())
        //     .unwrap();

        // We create a random asset ID now so that other imported files can reference this asset later
        let asset_id = AssetId::from_uuid(Uuid::new_v4());

        let mut file_references = HashMap::default();
        for (k, v) in scanned_importable
            .referenced_source_files
            .iter()
            .zip(referenced_source_file_asset_ids)
        {
            if let Some(v) = v {
                file_references.insert(k.path_reference.clone(), v);
                //TODO: Update
                // editor_context
                //     .set_file_reference_override(asset_id, k.path_reference.clone(), v)
                //     .unwrap();
            }
        }

        let source_file = PathReference {
            path: source_file_path.to_string_lossy().to_string(),
            importable_name: scanned_importable.name.clone(),
        };

        // let mut file_references = Vec::default();
        // for file_reference in &scanned_importable.referenced_source_files {
        //     file_references.push(file_reference.path_reference.clone());
        // }

        // This is everything we will need to create the asset, set the import info, and init
        // the build info with path overrides
        let requested_importable = RequestedImportable {
            asset_id,
            schema: scanned_importable.asset_type.clone(),
            asset_name: object_name,
            asset_location: selected_import_location.clone(),
            //importer_id: importer.importer_id(),
            source_file,
            path_references: file_references,
            //TODO: A re-import of data from the source file might not want to do this
            replace_with_default_asset: true,
        };

        requested_importables.insert(scanned_importable.name.clone(), requested_importable);

        let old = imported_asset_ids.insert(scanned_importable.name.clone(), asset_id);
        assert!(old.is_none());
    }

    //asset_engine.queue_import_operation(asset_ids, importer.importer_id(), file.to_path_buf());
    //(asset_ids, importer.importer_id(), file.to_path_buf())
    imports_to_queue.push(ImportToQueue {
        source_file_path: source_file_path.to_path_buf(),
        importer_id: importer.importer_id(),
        requested_importables,
        import_type: ImportType::ImportIfImportDataStale
    });

    Ok(imported_asset_ids)
}
