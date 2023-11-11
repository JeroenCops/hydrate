mod file_system_id_based;

pub use file_system_id_based::*;

use crate::edit_context::EditContext;
use crate::AssetId;

mod file_system_path_based;
use crate::import_util::ImportToQueue;
pub use file_system_path_based::*;

trait SourceFileHandler {
    fn supported_file_extensions(&self) -> &[&'static str];

    fn generate_default_asset(
        &self,
        importable_name: Option<String>,
        edit_context: &EditContext,
    ) -> Vec<AssetId>;

    // importer also implements scan file
}

// impl SourceFileHandler {
//     fn handles_file(path: &Path) -> bool {
//
//     }
// }

pub trait DataSource {
    // Replace memory with storage state
    // Reset memory to storage
    // Load storage state to memory
    fn load_from_storage(
        &mut self,
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
        object_id: AssetId,
    ) -> bool;

    // fn object_symbol_name(
    //     &self,
    //     object_id: ObjectId
    // ) -> Option<String>;

    fn persist_generated_asset(
        &mut self,
        edit_context: &mut EditContext,
        object_id: AssetId,
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
    // fn save_objects(objects: &[ObjectId]);
}
