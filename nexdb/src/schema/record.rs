use super::Schema;
use crate::schema::SchemaTypeIndex;
use crate::{SchemaFingerprint, SchemaId};
use siphasher::sip128::Hasher128;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub struct SchemaRecordField {
    name: String,
    aliases: Box<[String]>,
    field_schema: Schema,
}

impl SchemaRecordField {
    pub fn new(
        name: String,
        aliases: Box<[String]>,
        field_schema: Schema,
    ) -> Self {
        SchemaRecordField {
            name,
            aliases,
            field_schema,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn field_schema(&self) -> &Schema {
        &self.field_schema
    }
}

#[derive(Debug)]
pub struct SchemaRecordInner {
    name: String,
    fingerprint: SchemaFingerprint,
    aliases: Box<[String]>,
    fields: Box<[SchemaRecordField]>,
}

#[derive(Clone, Debug)]
pub struct SchemaRecord {
    inner: Arc<SchemaRecordInner>,
}

impl Deref for SchemaRecord {
    type Target = SchemaRecordInner;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl SchemaRecord {
    pub fn new(
        name: String,
        fingerprint: SchemaFingerprint,
        aliases: Box<[String]>,
        fields: Box<[SchemaRecordField]>,
    ) -> Self {
        // Check names are unique
        for i in 0..fields.len() {
            for j in 0..i {
                assert_ne!(fields[i].name, fields[j].name);
            }
        }

        let inner = SchemaRecordInner {
            name,
            fingerprint,
            aliases,
            fields,
        };

        SchemaRecord {
            inner: Arc::new(inner),
        }
    }

    // pub fn create_from_def(&self, &schema_def: SchemaDefNamedType, schemas_by_name: &HashMap<String, SchemaFingerprint>) -> SchemaRecord {
    //
    // }

    pub fn fingerprint(&self) -> SchemaFingerprint {
        self.fingerprint
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn fields(&self) -> &[SchemaRecordField] {
        &*self.fields
    }

    pub fn field_schema(
        &self,
        field_name: impl AsRef<str>,
    ) -> Option<&Schema> {
        for field in &*self.fields {
            if field.name == field_name.as_ref() {
                return Some(&field.field_schema);
            }
        }

        None
    }
}
