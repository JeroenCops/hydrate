use std::cmp::Ordering;
use std::collections::BTreeMap;
use uuid::Uuid;
use crate::{HashSet, ObjectLocation, ObjectPath, ObjectSourceId};

#[derive(Debug, PartialEq, Eq)]
pub struct LocationTreeNodeKey {
    name: String,
    source: ObjectSourceId
}

impl LocationTreeNodeKey {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> ObjectSourceId {
        self.source
    }
}

impl PartialOrd<Self> for LocationTreeNodeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LocationTreeNodeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.source.cmp(&other.source) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.name.cmp(&other.name),
            Ordering::Greater => Ordering::Greater,
        }
    }
}


#[derive(Debug)]
pub struct LocationTreeNode {
    pub location: ObjectLocation,
    pub children: BTreeMap<LocationTreeNodeKey, LocationTreeNode>,
    pub has_changes: bool,
}

#[derive(Debug)]
pub struct LocationTree {
    pub root_node: LocationTreeNode,
}

impl Default for LocationTree {
    fn default() -> Self {
        LocationTree {
            root_node: LocationTreeNode {
                location: ObjectLocation::new(ObjectSourceId::new_with_uuid(Uuid::nil()), ObjectPath::root()),
                children: Default::default(),
                has_changes: false
            }
        }
    }
}

impl LocationTree {
    fn get_or_create_path(&mut self, source: ObjectSourceId, path_components: &[&str], unsaved_paths: &HashSet<ObjectLocation>) -> &mut LocationTreeNode {
        let mut tree_node = &mut self.root_node;

        let mut node_path = ObjectPath::root();
        for path_component in path_components {
            println!("  path component {:?}", path_component);
            node_path = node_path.join(path_component);
            let node_key = LocationTreeNodeKey {
                name: path_component.to_string(),
                source
            };
            tree_node = tree_node.children.entry(node_key).or_insert_with(|| {
                let node_location = ObjectLocation::new(source, node_path.clone());
                let has_changes = unsaved_paths.contains(&node_location);
                LocationTreeNode {
                    location: node_location,
                    children: Default::default(),
                    has_changes
                }
            });

            //assert!(tree_node.is_directory);
        }

        tree_node
    }

    pub fn build(object_locations: &HashSet<ObjectLocation>, unsaved_paths: &HashSet<ObjectLocation>) -> Self {
        let root_node = LocationTreeNode {
            location: ObjectLocation::new(ObjectSourceId::new_with_uuid(Uuid::nil()), ObjectPath::root()),
            children: Default::default(),
            has_changes: false
        };

        let mut tree = LocationTree {
            root_node
        };

        for object_location in object_locations {
            let components = object_location.path().split_components();
            if !components.is_empty() {
                println!("source {:?}", object_location.path());

                // Skip the root component since it is our root node
                tree.get_or_create_path(object_location.source(), &components, unsaved_paths);
            }
        }

        tree
    }
}
