
mod dynamic_array;
pub use dynamic_array::*;

mod r#enum;
pub use r#enum::*;

mod fixed;
pub use fixed::*;

mod interface;
pub use interface::*;

mod map;
pub use map::*;

mod record;
pub use record::*;

mod ref_constraint;
pub use ref_constraint::*;

mod static_array;
pub use static_array::*;

use std::hash::{Hash, Hasher};
use siphasher::sip128::Hasher128;
use crate::{SchemaFingerprint, Value};
use crate::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SchemaId(u128);

// Defines a unique number for each variant for hashing/fingerprinting purposes, the number is
// completely arbitrary
#[derive(Copy, Clone)]
enum SchemaTypeIndex {
    Nullable = 0,
    Boolean = 1,
    I32 = 2,
    I64 = 3,
    U32 = 4,
    U64 = 5,
    F32 = 6,
    F64 = 7,
    Bytes = 8,
    Buffer = 9,
    String = 10,
    StaticArray = 11,
    DynamicArray = 12,
    Map = 13,
    RecordRef = 14,
    Record = 15,
    Enum = 16,
    Fixed = 17,
}

impl SchemaTypeIndex {
    pub(crate) fn fingerprint_hash<T: Hasher>(&self, hasher: &mut T) {
        (*self as u32).hash(hasher);
    }
}

#[derive(Clone, Debug)]
pub enum SchemaNamedType {
    Record(SchemaRecord),
    Enum(SchemaEnum),
    Fixed(SchemaFixed),
    // Union?
}

impl SchemaNamedType {
    pub fn fingerprint(&self) -> SchemaFingerprint {
        match self {
            SchemaNamedType::Record(x) => x.fingerprint(),
            SchemaNamedType::Enum(x) => x.fingerprint(),
            SchemaNamedType::Fixed(x) => x.fingerprint(),
        }
    }

    pub fn as_record(&self) -> Option<&SchemaRecord> {
        match self {
            SchemaNamedType::Record(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_enum(&self) -> Option<&SchemaEnum> {
        match self {
            SchemaNamedType::Enum(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_fixed(&self) -> Option<&SchemaFixed> {
        match self {
            SchemaNamedType::Fixed(x) => Some(x),
            _ => None,
        }
    }
}

/// Describes format of data, either a single primitive value or complex layout comprised of
/// potentially many values
#[derive(Clone, Debug)]
pub enum Schema {
    //
    // Anonymous Types
    //

    /// Marks the field as possible to be null
    Nullable(Box<Schema>),
    Boolean,
    I32,
    I64,
    U32,
    U64,
    F32,
    F64,
    /// Variable amount of bytes stored within the object, intended to be relatively small
    Bytes,
    /// A variable amount of bytes stored on a reference-counted heap and shared. Can be large (MBs)
    Buffer,
    /// Variable-length UTF-8 String
    String,
    /// Fixed-size array of values
    StaticArray(SchemaStaticArray),
    DynamicArray(SchemaDynamicArray),
    Map(SchemaMap),
    RecordRef(SchemaRefConstraint),

    //
    // Named Types
    //

    /// An object or inlined struct within an object
    // Record(SchemaRecord),
    // Enum(SchemaEnum),
    // Fixed(SchemaFixed),
    // union?
    NamedType(SchemaFingerprint)
}

impl Schema {
    // pub(crate) fn fingerprint_hash<T: Hasher>(&self, hasher: &mut T) {
    //     match self {
    //         Schema::Nullable(inner) => {
    //             SchemaTypeIndex::Nullable.fingerprint_hash(hasher);
    //             inner.fingerprint_hash(hasher)
    //         },
    //         Schema::Boolean => SchemaTypeIndex::Boolean.fingerprint_hash(hasher),
    //         Schema::I32 => SchemaTypeIndex::I32.fingerprint_hash(hasher),
    //         Schema::I64 => SchemaTypeIndex::I64.fingerprint_hash(hasher),
    //         Schema::U32 => SchemaTypeIndex::U32.fingerprint_hash(hasher),
    //         Schema::U64 => SchemaTypeIndex::U64.fingerprint_hash(hasher),
    //         Schema::F32 => SchemaTypeIndex::F32.fingerprint_hash(hasher),
    //         Schema::F64 => SchemaTypeIndex::F64.fingerprint_hash(hasher),
    //         Schema::Bytes => SchemaTypeIndex::Bytes.fingerprint_hash(hasher),
    //         Schema::Buffer => SchemaTypeIndex::Buffer.fingerprint_hash(hasher),
    //         Schema::String => SchemaTypeIndex::String.fingerprint_hash(hasher),
    //         Schema::StaticArray(inner) => inner.fingerprint_hash(hasher),
    //         Schema::DynamicArray(inner) => inner.fingerprint_hash(hasher),
    //         Schema::Map(inner) => inner.fingerprint_hash(hasher),
    //         Schema::RecordRef(inner) => inner.fingerprint_hash(hasher),
    //         Schema::Record(inner) => inner.fingerprint_hash(hasher),
    //         Schema::Enum(inner) => inner.fingerprint_hash(hasher),
    //         Schema::Fixed(inner) => inner.fingerprint_hash(hasher),
    //         //TODO: Union?
    //     }
    // }

    // pub fn fingerprint(&self) -> SchemaFingerprint {
    //     let mut hasher = siphasher::sip128::SipHasher::default();
    //     self.fingerprint_hash(&mut hasher);
    //     SchemaFingerprint(hasher.finish128().as_u128())
    // }

    pub fn is_nullable(&self) -> bool {
        match self {
            Schema::Nullable(_) => true,
            _ => false
        }
    }

    pub fn is_boolean(&self) -> bool {
        match self {
            Schema::Boolean => true,
            _ => false
        }
    }

    pub fn is_i32(&self) -> bool {
        match self {
            Schema::I32 => true,
            _ => false
        }
    }

    pub fn is_u32(&self) -> bool {
        match self {
            Schema::U32 => true,
            _ => false
        }
    }

    pub fn is_i64(&self) -> bool {
        match self {
            Schema::I64 => true,
            _ => false
        }
    }

    pub fn is_u64(&self) -> bool {
        match self {
            Schema::U64 => true,
            _ => false
        }
    }

    pub fn is_f32(&self) -> bool {
        match self {
            Schema::F32 => true,
            _ => false
        }
    }

    pub fn is_f64(&self) -> bool {
        match self {
            Schema::F64 => true,
            _ => false
        }
    }

    pub fn is_bytes(&self) -> bool {
        match self {
            Schema::Bytes => true,
            _ => false
        }
    }

    pub fn is_buffer(&self) -> bool {
        match self {
            Schema::Buffer => true,
            _ => false
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            Schema::String => true,
            _ => false
        }
    }

    pub fn is_static_array(&self) -> bool {
        match self {
            Schema::StaticArray(_) => true,
            _ => false
        }
    }

    pub fn is_dynamic_array(&self) -> bool {
        match self {
            Schema::DynamicArray(_) => true,
            _ => false
        }
    }

    pub fn find_property_schema<'a>(&'a self, name: impl AsRef<str>, named_types: &'a HashMap<SchemaFingerprint, SchemaNamedType>) -> Option<&'a Schema> {
        match self {
            Schema::Nullable(x) => {
                if name.as_ref() == "value" {
                    Some(&*x)
                } else {
                     None
                }
            }
            Schema::NamedType(named_type_id) => {
                let named_type = named_types.get(named_type_id).unwrap();
                match named_type {
                    SchemaNamedType::Record(x) => x.field_schema(name),
                    SchemaNamedType::Enum(_) => None,
                    SchemaNamedType::Fixed(_) => None,
                }
            },
            Schema::StaticArray(x) => {
                if name.as_ref().parse::<u32>().is_ok() {
                    Some(x.item_type())
                } else {
                    None
                }
            },
            Schema::DynamicArray(x) => {
                // We are not picky about the index being a number as the Object DB/property
                // handling uses UUIDs to ID each object, we just don't show the IDs to users
                Some(x.item_type())
            },
            Schema::Map(x) => {
                Some(x.value_type())
            },
            _ => None
        }
    }

    //
    // pub fn find_property_path_schema<T: AsRef<str>>(&self, path: &[T]) -> Option<&Schema> {
    //     let mut schema = self;
    //
    //     for path_element in path {
    //         let s = schema.find_property_schema(path_element);
    //         if let Some(s) = s {
    //             schema = s;
    //         } else {
    //             return None;
    //         }
    //     }
    //
    //     Some(schema)
    // }
    //
    // pub fn as_record(&self) -> Option<&SchemaRecord> {
    //     if let Schema::Record(x) = self {
    //         Some(x)
    //     } else {
    //         None
    //     }
    // }
}
