use crate::edit_context::EditContext;
use crate::editor::undo::UndoStack;
use crate::{DataSet, DataSource, FileSystemIdBasedDataSource, FileSystemPathBasedDataSource, HashMap, HashSet, ImporterRegistry, LocationTree, ObjectId, ObjectPath, ObjectSourceId, PathNode, PathNodeRoot, SchemaNamedType, SchemaSet};
use slotmap::DenseSlotMap;
use std::path::PathBuf;
use std::sync::Arc;
use hydrate_data::{ObjectLocation, ObjectName};
use hydrate_schema::SchemaFingerprint;
use crate::import_util::ImportToQueue;
slotmap::new_key_type! { pub struct EditContextKey; }

pub struct EditorModel {
    schema_set: Arc<SchemaSet>,
    undo_stack: UndoStack,
    root_edit_context_key: EditContextKey,
    edit_contexts: DenseSlotMap<EditContextKey, EditContext>,
    //TODO: slot_map?
    data_sources: HashMap<ObjectSourceId, Box<dyn DataSource>>,

    path_node_id_to_path: HashMap<ObjectId, ObjectPath>,
    //path_to_object_id: HashMap<ObjectPath, ObjectId>,
    location_tree: LocationTree,

    path_node_schema: SchemaNamedType,
    path_node_root_schema: SchemaNamedType,
}

impl EditorModel {
    pub fn new(schema_set: Arc<SchemaSet>) -> Self {
        let undo_stack = UndoStack::default();
        let mut edit_contexts: DenseSlotMap<EditContextKey, EditContext> = Default::default();

        let root_edit_context_key = edit_contexts
            .insert_with_key(|key| EditContext::new(key, schema_set.clone(), &undo_stack));

        let path_node_root_schema = schema_set
            .find_named_type(PathNodeRoot::schema_name())
            .unwrap()
            .clone();

        let path_node_schema = schema_set
            .find_named_type(PathNode::schema_name())
            .unwrap()
            .clone();

        EditorModel {
            schema_set,
            undo_stack,
            root_edit_context_key,
            edit_contexts,
            data_sources: Default::default(),
            location_tree: Default::default(),
            path_node_id_to_path: Default::default(),
            path_node_root_schema,
            path_node_schema,
        }
    }

    pub fn is_path_node_or_root(&self, fingerprint: SchemaFingerprint) -> bool {
        self.path_node_schema.fingerprint() == fingerprint || self.path_node_root_schema.fingerprint() == fingerprint
    }

    // pub fn object_symbol_name(&self, object_id: ObjectId) -> Option<String> {
    //     for data_source in self.data_sources.values() {
    //         if let Some(symbol_name) = data_source.object_symbol_name(object_id) {
    //             Some(symbol_name)
    //         }
    //     }
    //
    //     None
    // }

    pub fn is_generated_asset(&self, object_id: ObjectId) -> bool {
        for data_source in self.data_sources.values() {
            if data_source.is_generated_asset(object_id) {
                return true;
            }
        }

        false
    }

    pub fn persist_generated_asset(&mut self, object_id: ObjectId) {
        for (_, data_source) in &mut self.data_sources {
            let root_edit_context = self.edit_contexts
                .get_mut(self.root_edit_context_key)
                .unwrap();

            data_source.persist_generated_asset(root_edit_context, object_id);
        }
    }

    pub fn commit_all_pending_undo_contexts(&mut self) {
        for (_, context) in &mut self.edit_contexts {
            context.commit_pending_undo_context();
        }
    }

    pub fn cancel_all_pending_undo_contexts(&mut self) {
        for (_, context) in &mut self.edit_contexts {
            context.commit_pending_undo_context();
        }
    }

    pub fn any_edit_context_has_unsaved_changes(&self) -> bool {
        for (_key, context) in &self.edit_contexts {
            if context.has_changes() {
                return true;
            }
        }

        false
    }

    pub fn schema_set(&self) -> &SchemaSet {
        &*self.schema_set
    }

    pub fn clone_schema_set(&self) -> Arc<SchemaSet> {
        self.schema_set.clone()
    }

    pub fn root_edit_context(&self) -> &EditContext {
        self.edit_contexts.get(self.root_edit_context_key).unwrap()
    }

    pub fn root_edit_context_mut(&mut self) -> &mut EditContext {
        self.edit_contexts
            .get_mut(self.root_edit_context_key)
            .unwrap()
    }

    pub fn path_node_id_to_path(
        &self,
        object_id: ObjectId,
    ) -> Option<&ObjectPath> {
        self.path_node_id_to_path.get(&object_id)
    }

    pub fn object_display_name_long(
        &self,
        object_id: ObjectId,
    ) -> String {
        let root_data_set = &self.root_edit_context().data_set;
        let location = root_data_set.object_location(object_id);

        // Look up the location, if we don't find it just assume the object is at the root. This
        // allows some degree of robustness even when data is in a bad state (like cyclical references)
        let path = location
            .map(|x| self.path_node_id_to_path(x.path_node_id()))
            .flatten()
            .cloned()
            .unwrap_or_else(ObjectPath::root);

        let name = root_data_set.object_name(object_id);
        if let Some(name) = name.as_string() {
            path.join(name).as_str().to_string()
        } else {
            path.join(&format!("{}", object_id.as_uuid()))
                .as_str()
                .to_string()
        }
    }

    pub fn data_source(
        &mut self,
        object_source_id: ObjectSourceId,
    ) -> Option<&dyn DataSource> {
        self.data_sources.get(&object_source_id).map(|x| &**x)
    }

    pub fn is_a_root_object(
        &self,
        object_id: ObjectId
    ) -> bool {
        for source in self.data_sources.keys() {
            if *source.uuid() == object_id.as_uuid() {
                return true;
            }
        }

        false
    }

    pub fn add_file_system_id_based_data_source<RootPathT: Into<PathBuf>>(
        &mut self,
        data_source_name: &str,
        file_system_root_path: RootPathT,
        imports_to_queue: &mut Vec<ImportToQueue>,
    ) -> ObjectSourceId {
        let path_node_root_schema = self.path_node_root_schema.as_record().unwrap().clone();
        let root_edit_context = self.root_edit_context_mut();
        let file_system_root_path = file_system_root_path.into();

        // Commit any pending changes so we have a clean change tracking state
        root_edit_context.commit_pending_undo_context();

        //
        // Create the PathNodeRoot object that acts as the root location for all objects in this DS
        //
        let object_source_id = ObjectSourceId::new();
        let root_object_id = ObjectId::from_uuid(*object_source_id.uuid());
        root_edit_context.new_object_with_id(
            root_object_id,
            &ObjectName::new(data_source_name),
            &ObjectLocation::null(),
            &path_node_root_schema,
        ).unwrap();

        // Clear change tracking so that the new root object we just added doesn't appear as a unsaved change.
        // (It should never serialize)
        root_edit_context.clear_change_tracking();

        //
        // Create the data source and force full reload of it
        //
        let mut fs = FileSystemIdBasedDataSource::new(file_system_root_path.clone(), root_edit_context, object_source_id);
        fs.load_from_storage(root_edit_context, imports_to_queue);

        self.data_sources.insert(object_source_id, Box::new(fs));

        object_source_id
    }

    pub fn add_file_system_path_based_data_source<RootPathT: Into<PathBuf>>(
        &mut self,
        data_source_name: &str,
        file_system_root_path: RootPathT,
        importer_registry: &ImporterRegistry,
        imports_to_queue: &mut Vec<ImportToQueue>,
    ) -> ObjectSourceId {
        let path_node_root_schema = self.path_node_root_schema.as_record().unwrap().clone();
        let root_edit_context = self.root_edit_context_mut();
        let file_system_root_path = file_system_root_path.into();

        // Commit any pending changes so we have a clean change tracking state
        root_edit_context.commit_pending_undo_context();

        //
        // Create the PathNodeRoot object that acts as the root location for all objects in this DS
        //
        let object_source_id = ObjectSourceId::new();
        let root_object_id = ObjectId::from_uuid(*object_source_id.uuid());
        root_edit_context.new_object_with_id(
            root_object_id,
            &ObjectName::new(data_source_name),
            &ObjectLocation::null(),
            &path_node_root_schema,
        ).unwrap();

        // Clear change tracking so that the new root object we just added doesn't appear as a unsaved change.
        // (It should never serialize)
        root_edit_context.clear_change_tracking();

        //
        // Create the data source and force full reload of it
        //
        let mut fs = FileSystemPathBasedDataSource::new(file_system_root_path.clone(), root_edit_context, object_source_id, importer_registry);
        fs.load_from_storage(root_edit_context, imports_to_queue);

        self.data_sources.insert(object_source_id, Box::new(fs));

        object_source_id
    }

    pub fn save_root_edit_context(&mut self) {
        //
        // Ensure pending edits are flushed to the data set so that our modified objects list is fully up to date
        //
        let root_edit_context = self
            .edit_contexts
            .get_mut(self.root_edit_context_key)
            .unwrap();
        root_edit_context.commit_pending_undo_context();

        for (_id, data_source) in &mut self.data_sources {
            data_source.flush_to_storage(root_edit_context);
        }

        //
        // Clear modified objects list since we saved everything to disk
        //
        root_edit_context.clear_change_tracking();
    }

    pub fn revert_root_edit_context(
        &mut self,
        imports_to_queue: &mut Vec<ImportToQueue>,
    ) {
        //
        // Ensure pending edits are cleared
        //
        let root_edit_context = self
            .edit_contexts
            .get_mut(self.root_edit_context_key)
            .unwrap();
        root_edit_context.cancel_pending_undo_context();

        //
        // Take the contents of the modified object list, leaving the edit context with a cleared list
        //
        let (modified_objects, modified_locations) =
            root_edit_context.take_modified_objects_and_locations();
        println!(
            "Revert:\nObjects: {:?}\nLocations: {:?}",
            modified_objects, modified_locations
        );

        for (_id, data_source) in &mut self.data_sources {
            data_source.load_from_storage(root_edit_context, imports_to_queue);
        }

        //
        // Clear modified objects list since we reloaded everything from disk.
        //
        root_edit_context.clear_change_tracking();
        //root_edit_context.cancel_pending_undo_context();

        println!("stuff");
        //self.refresh_object_path_lookups();
        //self.refresh_location_tree();
    }

    pub fn close_file_system_source(
        &mut self,
        _object_source_id: ObjectSourceId,
    ) {
        unimplemented!();
        // kill edit contexts or fail

        // clear root_edit_context of data from this source

        // drop the source
        //let old = self.data_sources.remove(&object_source_id);
        //assert!(old.is_some());
    }

    // Spawns a separate edit context with copies of the given objects. The undo stack will be shared
    // globally, but changes will not be visible on the root context. The edit context will be flushed
    // to the root context in a single operation. Generally, we don't expect objects opened in a
    // separate edit context to change in the root context, but there is nothing that prevents it.
    pub fn open_edit_context(
        &mut self,
        objects: &[ObjectId],
    ) -> EditContextKey {
        let new_edit_context_key = self.edit_contexts.insert_with_key(|key| {
            EditContext::new_with_data(key, self.schema_set.clone(), &self.undo_stack)
        });

        let [root_edit_context, new_edit_context] = self
            .edit_contexts
            .get_disjoint_mut([self.root_edit_context_key, new_edit_context_key])
            .unwrap();

        for &object_id in objects {
            new_edit_context
                .data_set
                .copy_from(root_edit_context.data_set(), object_id);
        }

        new_edit_context_key
    }

    pub fn flush_edit_context_to_root(
        &mut self,
        edit_context: EditContextKey,
    ) {
        assert_ne!(edit_context, self.root_edit_context_key);
        let [root_context, context_to_flush] = self
            .edit_contexts
            .get_disjoint_mut([self.root_edit_context_key, edit_context])
            .unwrap();

        for &object_id in context_to_flush.modified_objects() {
            root_context
                .data_set
                .copy_from(&context_to_flush.data_set, object_id);
        }

        context_to_flush.clear_change_tracking();
    }

    pub fn close_edit_context(
        &mut self,
        edit_context: EditContextKey,
    ) {
        assert_ne!(edit_context, self.root_edit_context_key);
        self.edit_contexts.remove(edit_context);
    }

    pub fn undo(&mut self) {
        self.undo_stack.undo(&mut self.edit_contexts)
    }

    pub fn redo(&mut self) {
        self.undo_stack.redo(&mut self.edit_contexts)
    }

    fn do_populate_path(
        data_set: &DataSet,
        path_stack: &mut HashSet<ObjectId>,
        paths: &mut HashMap<ObjectId, ObjectPath>,
        path_node: ObjectId,
    ) -> ObjectPath {
        if path_node.is_null() {
            return ObjectPath::root();
        }

        // If we already know the path for the tree node, just return it
        if let Some(parent_path) = paths.get(&path_node) {
            return parent_path.clone();
        }

        // To detect cyclical references, we accumulate visited objects into a set
        let is_cyclical_reference = !path_stack.insert(path_node);
        let source_id_and_path = if is_cyclical_reference {
            // If we detect a cycle, bail and return root path
            ObjectPath::root()
        } else {
            if let Some(object) = data_set.objects().get(&path_node) {
                if let Some(name) = object.object_name().as_string() {
                    // Parent is found, named, and not a cyclical reference
                    let parent = Self::do_populate_path(
                        data_set,
                        path_stack,
                        paths,
                        object.object_location().path_node_id(),
                    );
                    let path = parent.join(name);
                    path
                } else {
                    // Parent is unnamed, just treat as being at root path
                    ObjectPath::root()
                }
            } else {
                // Can't find parent, just treat as being at root path
                ObjectPath::root()
            }
        };

        paths.insert(path_node, source_id_and_path.clone());

        if !is_cyclical_reference {
            path_stack.remove(&path_node);
        }

        source_id_and_path
    }

    fn populate_paths(
        data_set: &DataSet,
        path_node_type: &SchemaNamedType,
        path_node_root_type: &SchemaNamedType,
    ) -> HashMap<ObjectId, ObjectPath> {
        let mut path_stack = HashSet::default();
        let mut paths = HashMap::<ObjectId, ObjectPath>::default();
        for (object_id, info) in data_set.objects() {
            // For objects that *are* path nodes, use their ID directly. For objects that aren't
            // path nodes, use their location object ID
            let path_node_id = if info.schema().fingerprint() == path_node_type.fingerprint() || info.schema().fingerprint() == path_node_root_type.fingerprint() {
                *object_id
            } else {
                // We could process objects so that if for some reason the parent nodes don't exist, we can still
                // generate path lookups for them. Instead we will consider a parent not being found as
                // the object being at the root level. We could also have a "lost and found" UI.
                //info.object_location().path_node_id()
                continue;
            };

            // We will walk up the location chain and cache the path_node_id/path pairs. (We resolve
            // the parents recursively going all the way up to the root, and then appending the
            // current node to it's parent's resolved path.)
            Self::do_populate_path(data_set, &mut path_stack, &mut paths, path_node_id);
        }

        paths
    }

    pub fn refresh_tree_node_cache(&mut self) {
        // Build lookup of object ID to paths. This should only include objects of type
        // PathNode or PathNodeRoot
        let root_edit_context = self.edit_contexts.get(self.root_edit_context_key).unwrap();
        let path_node_id_to_path =
            Self::populate_paths(&root_edit_context.data_set, &self.path_node_schema, &self.path_node_root_schema);

        self.path_node_id_to_path = path_node_id_to_path;

        // Build a tree structure of all paths
        self.location_tree =
            LocationTree::build(&self.data_sources, &root_edit_context.data_set, &self.path_node_id_to_path);
    }

    pub fn cached_location_tree(&self) -> &LocationTree {
        &self.location_tree
    }
}
