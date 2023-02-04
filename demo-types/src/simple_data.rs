use hydrate_model::{DataSet, DataSetEntry, ObjectId, SchemaSet};
use serde::{Deserialize, Serialize};
use type_uuid::TypeUuid;
use hydrate_base::{AssetUuid, Handle};

#[derive(Serialize, Deserialize, TypeUuid, Debug)]
#[uuid = "7132d33e-9bbc-4fb1-b857-17962afd44b8"]
pub struct TransformRef {
    pub transform: Handle<Transform>
}

impl DataSetEntry for TransformRef {
    fn from_data_set(
        object_id: ObjectId,
        data_set: &DataSet,
        schema: &SchemaSet,
    ) -> Self {
        let object_id = data_set.resolve_property(schema, object_id, "transform").unwrap().as_object_ref().unwrap();

        let asset_id = AssetUuid(*object_id.as_uuid().as_bytes());

        //TODO: Verify type?
        let handle = hydrate_base::handle::make_handle::<Transform>(asset_id);

        TransformRef {
            transform: handle
        }
    }
}


#[derive(Serialize, Deserialize, TypeUuid, Debug)]
#[uuid = "da334afa-7af9-4894-8b7e-29defe202e90"]
pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl DataSetEntry for Transform {
    fn from_data_set(
        object_id: ObjectId,
        data_set: &DataSet,
        schema: &SchemaSet,
    ) -> Self {
        let position = [
            data_set
                .resolve_property(schema, object_id, "position.x")
                .unwrap()
                .as_f32()
                .unwrap(),
            data_set
                .resolve_property(schema, object_id, "position.y")
                .unwrap()
                .as_f32()
                .unwrap(),
            data_set
                .resolve_property(schema, object_id, "position.z")
                .unwrap()
                .as_f32()
                .unwrap(),
        ];

        let rotation = [
            data_set
                .resolve_property(schema, object_id, "rotation.x")
                .unwrap()
                .as_f32()
                .unwrap(),
            data_set
                .resolve_property(schema, object_id, "rotation.y")
                .unwrap()
                .as_f32()
                .unwrap(),
            data_set
                .resolve_property(schema, object_id, "rotation.z")
                .unwrap()
                .as_f32()
                .unwrap(),
            data_set
                .resolve_property(schema, object_id, "rotation.w")
                .unwrap()
                .as_f32()
                .unwrap(),
        ];

        let scale = [
            data_set
                .resolve_property(schema, object_id, "scale.x")
                .unwrap()
                .as_f32()
                .unwrap(),
            data_set
                .resolve_property(schema, object_id, "scale.y")
                .unwrap()
                .as_f32()
                .unwrap(),
            data_set
                .resolve_property(schema, object_id, "scale.z")
                .unwrap()
                .as_f32()
                .unwrap(),
        ];

        //let test_field = data_set.resolve_property(schema, object_id, "test_ref").unwrap().as_object_ref().unwrap();
        // Create handle passing the ObjectId?

        Transform {
            position,
            rotation,
            scale,
        }
    }
}

#[derive(Serialize, Deserialize, TypeUuid)]
#[uuid = "df64f515-7e2f-47c2-b4d3-17ec7f2e63c7"]
pub struct AllFields {
    pub boolean: bool,
    pub int32: i32,
    pub int64: i64,
}

impl DataSetEntry for AllFields {
    fn from_data_set(
        object_id: ObjectId,
        data_set: &DataSet,
        schema: &SchemaSet,
    ) -> Self {
        let boolean = data_set
            .resolve_property(schema, object_id, "boolean")
            .unwrap()
            .as_boolean()
            .unwrap();
        let int32 = data_set
            .resolve_property(schema, object_id, "int32")
            .unwrap()
            .as_i32()
            .unwrap();
        let int64 = data_set
            .resolve_property(schema, object_id, "int64")
            .unwrap()
            .as_i64()
            .unwrap();

        AllFields {
            boolean,
            int32,
            int64,
        }
    }
}
