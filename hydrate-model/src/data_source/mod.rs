mod file_system_id_based;

pub use file_system_id_based::*;

use crate::edit_context::EditContext;
use crate::AssetId;

mod file_system_path_based;
pub use file_system_path_based::*;
use hydrate_pipeline::{HydrateProjectConfiguration, ImportToQueue};

pub trait DataSource {
    // Replace memory with storage state
    // Reset memory to storage
    // Load storage state to memory
    fn load_from_storage(
        &mut self,
        project_config: &HydrateProjectConfiguration,
        edit_context: &mut EditContext,
        imports_to_queue: &mut Vec<ImportToQueue>,
    );

    // Replace storage state with memory state
    // Flush memory to storage
    fn flush_to_storage(
        &mut self,
        edit_context: &mut EditContext,
    );

    fn is_generated_asset(
        &self,
        asset_id: AssetId,
    ) -> bool;

    // fn asset_symbol_name(
    //     &self,
    //     asset_id: AssetId
    // ) -> Option<String>;

    fn persist_generated_asset(
        &mut self,
        edit_context: &mut EditContext,
        asset_id: AssetId,
    );
    // fn revert_all_modified(
    //     &mut self,
    //     edit_context: &mut EditContext,
    //     imports_to_queue: &mut Vec<ImportToQueue>,
    // );

    // fn get_file_operations_required_to_save();
    //
    //
    //
    // fn save_assets(objects: &[ObjectId]);
}
