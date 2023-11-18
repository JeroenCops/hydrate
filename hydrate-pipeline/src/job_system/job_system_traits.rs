use super::{JobId, JobTypeId};
use crate::import_jobs;
use crate::{AssetArtifactIdPair, BuiltArtifact, ImportData, ImportJobs};
use hydrate_base::handle::DummySerdeContextHandle;
use hydrate_base::hashing::HashMap;
use hydrate_base::{ArtifactId, AssetId, BuiltArtifactMetadata, Handle};
use hydrate_data::{DataContainer, DataSet, FieldReader, PropertyPath, SchemaSet, SingleObject};
use serde::{Deserialize, Serialize};
use siphasher::sip128::Hasher128;
use std::hash::Hash;
use type_uuid::{TypeUuid, TypeUuidDynamic};

pub trait ImportDataProvider {
    fn clone_import_data_metadata_hashes(&self) -> HashMap<AssetId, u64>;

    fn load_import_data(
        &self,
        schema_set: &SchemaSet,
        asset_id: AssetId,
    ) -> ImportData;
}

impl ImportDataProvider for ImportJobs {
    fn clone_import_data_metadata_hashes(&self) -> HashMap<AssetId, u64> {
        self.clone_import_data_metadata_hashes()
    }

    fn load_import_data(
        &self,
        schema_set: &SchemaSet,
        asset_id: AssetId,
    ) -> ImportData {
        import_jobs::load_import_data(self.import_data_root_path(), schema_set, asset_id)
    }
}

pub struct NewJob {
    pub job_type: JobTypeId,
    pub input_hash: u128,
    pub input_data: Vec<u8>,
}

fn create_artifact_id<T: Hash>(
    asset_id: AssetId,
    artifact_key: Option<T>,
) -> ArtifactId {
    if let Some(artifact_key) = artifact_key {
        let mut hasher = siphasher::sip128::SipHasher::default();
        asset_id.hash(&mut hasher);
        artifact_key.hash(&mut hasher);
        let input_hash = hasher.finish128().as_u128();
        ArtifactId::from_u128(input_hash)
    } else {
        ArtifactId::from_uuid(asset_id.as_uuid())
    }
}

//
// API Design
//
pub trait JobApi: Send + Sync {
    fn enqueue_job(
        &self,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job: NewJob,
        debug_name: String,
    ) -> JobId;

    fn artifact_handle_created(
        &self,
        asset_id: AssetId,
        artifact_id: ArtifactId,
    );

    fn produce_artifact(
        &self,
        artifact: BuiltArtifact,
    );
}

//
// Job Traits
//
pub trait JobInput: Hash + Serialize + for<'a> Deserialize<'a> {}

pub trait JobOutput: Serialize + for<'a> Deserialize<'a> {}

#[derive(Default, Clone)]
pub struct JobEnumeratedDependencies {
    // The contents of assets can affect the output so we need to include a hash of the contents of
    // the asset. But assets can ref other assets, task needs to list all assets that are touched
    // (including prototypes of those assets).
    //
    // We could do it at asset type granularity? (i.e. if you change an asset of type X all jobs that
    // read an asset of type X have to rerun.
    //
    // What if we provide a data_set reader that keeps track of what was read? When we run the task
    // the first time we don't know what we will touch or how to hash it but we can store it. Second
    // build we can check if anything that was read last time was modified.
    //
    // Alternatively, jobs that read assets must always copy data out of the data set into a hashable
    // form and pass it as input to a job.
    pub import_data: Vec<AssetId>,
    //pub built_data: Vec<ArtifactId>,
    pub upstream_jobs: Vec<JobId>,
}

pub trait JobProcessorAbstract: Send + Sync {
    fn version_inner(&self) -> u32;

    fn enumerate_dependencies_inner(
        &self,
        input: &Vec<u8>,
        data_set: &DataSet,
        schema_set: &SchemaSet,
    ) -> JobEnumeratedDependencies;

    fn run_inner(
        &self,
        input: &Vec<u8>,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<AssetId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> Vec<u8>;
}

pub struct EnumerateDependenciesContext<'a, InputT> {
    pub input: &'a InputT,
    pub data_set: &'a DataSet,
    pub schema_set: &'a SchemaSet,
}

pub struct RunContext<'a, InputT> {
    pub input: &'a InputT,
    pub data_set: &'a DataSet,
    pub schema_set: &'a SchemaSet,
    pub dependency_data: &'a HashMap<AssetId, SingleObject>,
    pub(super) job_api: &'a dyn JobApi,
}

impl<'a, InputT> RunContext<'a, InputT> {
    pub fn imported_data<T: FieldReader<'a>>(&'a self, asset_id: AssetId) -> Option<T> {
        Some(T::new(PropertyPath::default(), DataContainer::from_single_object(self.dependency_data.get(&asset_id)?, self.schema_set)))
    }

    pub fn enqueue_job<JobProcessorT: JobProcessor>(
        &self,
        input: <JobProcessorT as JobProcessor>::InputT,
    ) -> JobId {
        enqueue_job::<JobProcessorT>(self.data_set, self.schema_set, self.job_api, input)
    }

    pub fn produce_artifact<KeyT: Hash + std::fmt::Display, ArtifactT: TypeUuid + Serialize>(
        &self,
        asset_id: AssetId,
        artifact_key: Option<KeyT>,
        asset: ArtifactT,
    ) -> AssetArtifactIdPair {
        produce_artifact(self.job_api, asset_id, artifact_key, asset)
    }

    pub fn produce_artifact_with_handles<
        KeyT: Hash + std::fmt::Display,
        ArtifactT: TypeUuid + Serialize,
        F: FnOnce(HandleFactory) -> ArtifactT,
    >(
        &self,
        asset_id: AssetId,
        artifact_key: Option<KeyT>,
        asset_fn: F,
    ) -> ArtifactId {
        produce_artifact_with_handles(self.job_api, asset_id, artifact_key, asset_fn)
    }

    pub fn produce_default_artifact<AssetT: TypeUuid + Serialize>(
        &self,
        asset_id: AssetId,
        asset: AssetT,
    ) {
        produce_default_artifact(self.job_api, asset_id, asset)
    }

    pub fn produce_default_artifact_with_handles<AssetT: TypeUuid + Serialize, F: FnOnce(HandleFactory) -> AssetT>(
        &self,
        asset_id: AssetId,
        asset_fn: F,
    ) {
        produce_default_artifact_with_handles(self.job_api, asset_id, asset_fn)
    }
}

pub trait JobProcessor: TypeUuid {
    type InputT: JobInput + 'static;
    type OutputT: JobOutput + 'static;

    fn version(&self) -> u32;

    fn enumerate_dependencies(
        &self,
        context: EnumerateDependenciesContext<Self::InputT>,
    ) -> JobEnumeratedDependencies;

    fn run(
        &self,
        context: RunContext<Self::InputT>,
    ) -> Self::OutputT;
}

pub(crate) fn enqueue_job<T: JobProcessor>(
    data_set: &DataSet,
    schema_set: &SchemaSet,
    job_api: &dyn JobApi,
    input: <T as JobProcessor>::InputT,
) -> JobId {
    let mut hasher = siphasher::sip128::SipHasher::default();
    input.hash(&mut hasher);
    let input_hash = hasher.finish128().as_u128();

    let input_data = bincode::serialize(&input).unwrap();

    let queued_job = NewJob {
        job_type: JobTypeId::from_bytes(T::UUID),
        input_hash,
        input_data,
    };

    let debug_name = format!("{}", std::any::type_name::<T>());
    job_api.enqueue_job(data_set, schema_set, queued_job, debug_name)
}

fn produce_default_artifact<T: TypeUuid + Serialize>(
    job_api: &dyn JobApi,
    asset_id: AssetId,
    asset: T,
) {
    //produce_asset_with_handles(job_api, asset_id, || asset);
    produce_artifact_with_handles(job_api, asset_id, None::<u32>, |handle_factory| asset);
}

fn produce_default_artifact_with_handles<T: TypeUuid + Serialize, F: FnOnce(HandleFactory) -> T>(
    job_api: &dyn JobApi,
    asset_id: AssetId,
    asset_fn: F,
) {
    produce_artifact_with_handles(job_api, asset_id, None::<u32>, asset_fn);
    // let mut ctx = DummySerdeContextHandle::default();
    // ctx.begin_serialize_asset(AssetId(*asset_id.as_uuid().as_bytes()));
    //
    // let (built_data, asset_type) = ctx.scope(|| {
    //     let asset = (asset_fn)();
    //     let built_data = bincode::serialize(&asset).unwrap();
    //     (built_data, asset.uuid())
    // });
    //
    // let referenced_assets = ctx.end_serialize_asset(AssetId(*asset_id.as_uuid().as_bytes()));
    //
    // job_api.produce_asset(BuiltAsset {
    //     asset_id,
    //     metadata: BuiltArtifactMetadata {
    //         dependencies: referenced_assets.into_iter().map(|x| ArtifactId::from_uuid(Uuid::from_bytes(x.0.0))).collect(),
    //         subresource_count: 0,
    //         asset_type: uuid::Uuid::from_bytes(asset_type)
    //     },
    //     data: built_data
    // });
}

fn produce_artifact<T: TypeUuid + Serialize, U: Hash + std::fmt::Display>(
    job_api: &dyn JobApi,
    asset_id: AssetId,
    artifact_key: Option<U>,
    asset: T,
) -> AssetArtifactIdPair {
    let artifact_id = produce_artifact_with_handles(job_api, asset_id, artifact_key, |handle_factory| asset);
    AssetArtifactIdPair {
        asset_id,
        artifact_id,
    }
}

fn produce_artifact_with_handles<
    T: TypeUuid + Serialize,
    U: Hash + std::fmt::Display,
    F: FnOnce(HandleFactory) -> T,
>(
    job_api: &dyn JobApi,
    asset_id: AssetId,
    artifact_key: Option<U>,
    asset_fn: F,
) -> ArtifactId {
    let artifact_key_debug_name = artifact_key.as_ref().map(|x| format!("{}", x));
    let artifact_id = create_artifact_id(asset_id, artifact_key);

    let mut ctx = DummySerdeContextHandle::default();
    ctx.begin_serialize_artifact(artifact_id);

    let (built_data, asset_type) = ctx.scope(|| {
        let asset = (asset_fn)(HandleFactory {
            job_api
        });
        let built_data = bincode::serialize(&asset).unwrap();
        (built_data, asset.uuid())
    });

    let referenced_assets = ctx.end_serialize_artifact(artifact_id);

    log::trace!(
        "produce_artifact {:?} {:?} {:?}",
        asset_id,
        artifact_id,
        artifact_key_debug_name
    );
    job_api.produce_artifact(BuiltArtifact {
        asset_id,
        artifact_id,
        metadata: BuiltArtifactMetadata {
            dependencies: referenced_assets
                .into_iter()
                .map(|x| ArtifactId::from_uuid(x.0.as_uuid()))
                .collect(),
            asset_type: uuid::Uuid::from_bytes(asset_type),
        },
        data: built_data,
        artifact_key_debug_name,
    });

    artifact_id
}

#[derive(Copy, Clone)]
pub struct HandleFactory<'a> {
    job_api: &'a dyn JobApi,
}

impl<'a> HandleFactory<'a> {
    pub fn make_handle_to_default_artifact<T>(
        &self,
        asset_id: AssetId,
    ) -> Handle<T> {
        self.make_handle_to_artifact_key(asset_id, None::<u32>)
    }

    pub fn make_handle_to_artifact<T>(
        &self,
        asset_artifact_id_pair: AssetArtifactIdPair,
    ) -> Handle<T> {
        self.job_api.artifact_handle_created(
            asset_artifact_id_pair.asset_id,
            asset_artifact_id_pair.artifact_id,
        );
        hydrate_base::handle::make_handle_within_serde_context::<T>(asset_artifact_id_pair.artifact_id)
    }

    pub fn make_handle_to_artifact_raw<T>(
        &self,
        asset_id: AssetId,
        artifact_id: ArtifactId,
    ) -> Handle<T> {
        self.job_api.artifact_handle_created(asset_id, artifact_id);
        hydrate_base::handle::make_handle_within_serde_context::<T>(artifact_id)
    }

    pub fn make_handle_to_artifact_key<T, K: Hash>(
        &self,
        asset_id: AssetId,
        artifact_key: Option<K>,
    ) -> Handle<T> {
        let artifact_id = create_artifact_id(asset_id, artifact_key);
        self.job_api.artifact_handle_created(asset_id, artifact_id);
        hydrate_base::handle::make_handle_within_serde_context::<T>(artifact_id)
    }

}

/*
fn make_handle_to_default_artifact<T>(
    job_api: &dyn JobApi,
    asset_id: AssetId,
) -> Handle<T> {
    make_handle_to_artifact_key(job_api, asset_id, None::<u32>)
}

// pub fn make_handle_to_artifact<T>(
//     job_api: &dyn JobApi,
//     asset_artifact_id_pair: AssetArtifactIdPair,
// ) -> Handle<T> {
//     job_api.artifact_handle_created(
//         asset_artifact_id_pair.asset_id,
//         asset_artifact_id_pair.artifact_id,
//     );
//     hydrate_base::handle::make_handle_within_serde_context::<T>(asset_artifact_id_pair.artifact_id)
// }

fn make_handle_to_artifact_raw<T>(
    job_api: &dyn JobApi,
    asset_id: AssetId,
    artifact_id: ArtifactId,
) -> Handle<T> {
    job_api.artifact_handle_created(asset_id, artifact_id);
    hydrate_base::handle::make_handle_within_serde_context::<T>(artifact_id)
}

fn make_handle_to_artifact_key<T, K: Hash>(
    job_api: &dyn JobApi,
    asset_id: AssetId,
    artifact_key: Option<K>,
) -> Handle<T> {
    let artifact_id = create_artifact_id(asset_id, artifact_key);
    job_api.artifact_handle_created(asset_id, artifact_id);
    hydrate_base::handle::make_handle_within_serde_context::<T>(artifact_id)
}
*/