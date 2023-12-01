use crate::app_state::{ActionQueueSender, ModalAction, ModalActionControlFlow};
use crate::db_state::DbState;
use crate::ui_state::UiState;
use hydrate_model::pipeline::import_util::ImportToQueue;
use hydrate_model::pipeline::{AssetEngine, ImporterRegistry, ImportType};
use hydrate_model::pipeline::{Importer, PipelineResult};
use hydrate_model::{AssetId, AssetLocation, AssetPathCache, HashMap, HashSet, ImportableName, LocationTree, LocationTreeNode};
use imgui::sys::ImVec2;
use imgui::{im_str, PopupModal, TreeNodeFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ImportFilesModal {
    finished_first_draw: bool,
    files_to_import: Vec<PathBuf>,
    selected_import_location: AssetLocation,
}

impl ImportFilesModal {
    pub fn new(
        files_to_import: Vec<PathBuf>,
        importer_registry: &ImporterRegistry,
    ) -> Self {
        println!("show ImportFilesModal {:?}", files_to_import);

        let mut all_files_to_import: HashSet<PathBuf> = files_to_import.iter().cloned().collect();

        for file in &files_to_import {
            // Recursively look for files
            if file.is_dir() {
                let walker = globwalk::GlobWalkerBuilder::from_patterns(file, &["**"])
                    .file_type(globwalk::FileType::FILE)
                    .build()
                    .unwrap();

                for file in walker {
                    if let Ok(file) = file {
                        let file = dunce::canonicalize(&file.path()).unwrap();
                        if let Some(extension) = file.extension() {
                            if !importer_registry
                                .importers_for_file_extension(&*extension.to_string_lossy())
                                .is_empty()
                            {
                                all_files_to_import.insert(file.to_path_buf());
                                println!("import {:?}", file);
                            }
                        }
                    }
                }
            }
        }

        ImportFilesModal {
            finished_first_draw: false,
            files_to_import: all_files_to_import.into_iter().collect(),
            selected_import_location: AssetLocation::null(),
        }
    }
}

fn default_flags() -> imgui::TreeNodeFlags {
    imgui::TreeNodeFlags::OPEN_ON_DOUBLE_CLICK
        | imgui::TreeNodeFlags::OPEN_ON_ARROW
        | imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH
}

fn leaf_flags() -> imgui::TreeNodeFlags {
    imgui::TreeNodeFlags::LEAF | default_flags()
}

pub fn path_tree_node(
    ui: &imgui::Ui,
    db_state: &DbState,
    ui_state: &mut UiState,
    child_name: &str,
    tree_node: &LocationTreeNode,
    selected_import_location: &mut AssetLocation,
) {
    let id = im_str!("{}", tree_node.location.path_node_id().as_uuid());
    let is_selected = *selected_import_location == tree_node.location;

    let label = im_str!("{}", child_name);

    let mut flags = if tree_node.children.is_empty() {
        leaf_flags()
    } else {
        default_flags()
    };

    if is_selected {
        flags |= TreeNodeFlags::SELECTED;
    }

    let ds_tree_node = imgui::TreeNode::new(&id).label(&label).flags(flags);
    let token = ds_tree_node.push(ui);
    //style.pop();

    //try_select_tree_node(ui, ui_state, &tree_node.location);
    if ui.is_item_clicked() && !ui.is_item_toggled_open() {
        *selected_import_location = tree_node.location.clone();
    }

    if let Some(_token) = token {
        // Draw nodes with children first
        for (child_name, child) in &tree_node.children {
            if !child.children.is_empty() {
                path_tree_node(
                    ui,
                    db_state,
                    ui_state,
                    child_name.name(),
                    child,
                    selected_import_location,
                );
            }
        }

        // Then draw nodes without children
        for (child_name, child) in &tree_node.children {
            if child.children.is_empty() {
                path_tree_node(
                    ui,
                    db_state,
                    ui_state,
                    child_name.name(),
                    child,
                    selected_import_location,
                );
            }
        }
    }
}

pub fn path_tree(
    ui: &imgui::Ui,
    db_state: &mut DbState,
    ui_state: &mut UiState,
    selected_import_location: &mut AssetLocation,
) {
    db_state.asset_path_cache = AssetPathCache::build(&db_state.editor_model);
    db_state.location_tree = LocationTree::build(&db_state.editor_model, &db_state.asset_path_cache);

    for (child_name, child) in &db_state.location_tree.root_nodes {
        path_tree_node(
            ui,
            db_state,
            ui_state,
            child_name.name(),
            child,
            selected_import_location,
        );
    }
}

fn recursively_gather_import_operations_and_create_assets(
    file: &Path,
    importer: &Arc<dyn Importer>,
    db_state: &mut DbState,
    asset_engine: &AssetEngine,
    selected_import_location: &AssetLocation,
    imports_to_queue: &mut Vec<ImportToQueue>,
) -> PipelineResult<HashMap<ImportableName, AssetId>> {
    hydrate_model::pipeline::import_util::recursively_gather_import_operations_and_create_assets(
        file,
        importer,
        db_state.editor_model.root_edit_context_mut(),
        asset_engine.importer_registry(),
        selected_import_location,
        imports_to_queue,
    )
}

impl ModalAction for ImportFilesModal {
    fn draw_imgui(
        &mut self,
        ui: &mut imgui::Ui,
        _imnodes_context: &mut imnodes::Context,
        db_state: &mut DbState,
        ui_state: &mut UiState,
        asset_engine: &mut AssetEngine,
        _action_queue: ActionQueueSender,
    ) -> ModalActionControlFlow {
        if !self.finished_first_draw {
            ui.open_popup(imgui::im_str!("Import Files"));
        }

        unsafe {
            imgui::sys::igSetNextWindowSize(
                ImVec2::new(600.0, 400.0),
                imgui::sys::ImGuiCond__ImGuiCond_Appearing as _,
            );
        }

        let result = PopupModal::new(imgui::im_str!("Import Files")).build(ui, || {
            ui.text("Files to be imported:");

            imgui::ChildWindow::new("child1")
                .size([0.0, 100.0])
                .build(ui, || {
                    for file in &self.files_to_import {
                        ui.text(file.to_str().unwrap());
                    }
                });

            ui.separator();
            ui.text("Where to import the files");

            imgui::ChildWindow::new("child2")
                .size([0.0, 180.0])
                .build(ui, || {
                    path_tree(ui, db_state, ui_state, &mut self.selected_import_location);
                });

            if ui.button(imgui::im_str!("Cancel")) {
                ui.close_current_popup();

                return ModalActionControlFlow::End;
            }

            ui.same_line();
            if ui.button(imgui::im_str!("Import")) {
                //let mut files_to_import: HashSet<PathBuf> = self.files_to_import.iter().cloned().collect();

                // for file in &self.files_to_import {
                //     // Recursively look for files
                //     if file.is_dir() {
                //         let walker = globwalk::GlobWalkerBuilder::from_patterns(file, &["**"])
                //             .file_type(globwalk::FileType::FILE)
                //             .build()
                //             .unwrap();
                //
                //         for file in walker {
                //             if let Ok(file) = file {
                //                 let file = dunce::canonicalize(&file.path()).unwrap();
                //                 if let Some(extension) = file.path().extension() {
                //                     if !asset_engine.importer_registry().importers_for_file_extension(&*extension.to_string_lossy()).is_empty() {
                //                         files_to_import.insert(file.path().to_path_buf());
                //                         println!("import {:?}", file);
                //                     }
                //                 }
                //             }
                //         }
                //     }
                // }

                for file in &self.files_to_import {
                    let extension = file.extension();
                    if let Some(extension) = extension {
                        let extension = extension.to_string_lossy().to_string();
                        let handlers = asset_engine.importers_for_file_extension(&extension);

                        if !handlers.is_empty() {
                            //
                            // Find the importer to use on the file
                            //
                            let importer = asset_engine.importer(handlers[0]).unwrap();

                            let mut imports_to_queue = Vec::default();
                            recursively_gather_import_operations_and_create_assets(
                                file,
                                importer,
                                db_state,
                                asset_engine,
                                &self.selected_import_location,
                                &mut imports_to_queue,
                            )
                            .unwrap();

                            for import_to_queue in imports_to_queue {
                                asset_engine.queue_import_operation(
                                    import_to_queue.requested_importables,
                                    import_to_queue.importer_id,
                                    import_to_queue.source_file_path,
                                    ImportType::ImportIfImportDataStale,
                                );
                            }

                            // //
                            // // When we import, set the import info so we track where the import comes from
                            // //
                            // let import_info = ImportInfo::new(importer.importer_id(), file.clone());
                            //
                            // //
                            // // We now build a list of things we will be importing from the file.
                            // // 1. Scan the file to see what's available
                            // // 2. Create/Find assets for all the things we want to import
                            // // 3. Enqueue the import operation
                            // //
                            // let mut asset_ids = HashMap::default();
                            //
                            // let scanned_importables = importer.scan_file(file, db_state.editor_model.schema_set());
                            // for scanned_importable in &scanned_importables {
                            //     //
                            //     // Pick name for the asset for this file
                            //     //
                            //     let asset_name = if let Some(file_name) = file.file_name() {
                            //         let file_name =  file_name.to_string_lossy();
                            //         if let Some(importable_name) = &scanned_importable.name {
                            //             AssetName::new(format!("{}.{}", file_name, importable_name))
                            //         } else {
                            //             AssetName::new(file_name.to_string())
                            //         }
                            //     } else {
                            //         AssetName::empty()
                            //     };
                            //
                            //     //TODO: Check referenced source files to find existing imported assets or import referenced files
                            //     for referenced_source_file in &scanned_importable.referenced_source_files {
                            //         referenced_source_file.path
                            //     }
                            //
                            //     let asset_id = db_state.editor_model.root_edit_context_mut().new_asset(&asset_name, &self.selected_import_location, &scanned_importable.asset_type);
                            //     db_state.editor_model.root_edit_context_mut().set_import_info(asset_id, import_info.clone());
                            //     asset_ids.insert(scanned_importable.name.clone(), asset_id);
                            // }
                            //
                            // //
                            // // Trigger transition to modal waiting for imports to complete
                            // //
                            // asset_engine.queue_import_operation(asset_ids, importer.importer_id(), file.clone());
                        }
                    }
                }

                ui.close_current_popup();

                // do import?

                return ModalActionControlFlow::End;
            }

            ModalActionControlFlow::Continue
        });

        self.finished_first_draw = true;
        result.unwrap_or(ModalActionControlFlow::End)
    }
}
