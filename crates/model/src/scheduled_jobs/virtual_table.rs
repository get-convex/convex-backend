use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::LazyLock,
};

use async_trait::async_trait;
use common::{
    document::{
        timestamp_to_ms,
        DeveloperDocument,
        ParseDocument,
        ParsedDocument,
        ResolvedDocument,
        CREATION_TIME_FIELD,
        ID_FIELD,
    },
    virtual_system_mapping::{
        GetDocument,
        VirtualSystemDocMapper,
        VirtualSystemMapping,
    },
};
use semver::Version;
use sync_types::CanonicalizedUdfPath;
use value::{
    ConvexArray,
    ConvexObject,
    ConvexValue,
    FieldName,
    TableMapping,
};

use super::{
    types::{
        ScheduledJob,
        ScheduledJobState,
    },
    SCHEDULED_JOBS_TABLE,
};

static MIN_NPM_VERSION_SCHEDULED_JOBS_V1: LazyLock<Version> =
    LazyLock::new(|| Version::parse("1.6.1").unwrap());

pub struct ScheduledJobsDocMapper;

#[async_trait]
impl VirtualSystemDocMapper for ScheduledJobsDocMapper {
    async fn system_to_virtual_doc(
        &self,
        _tx: &mut dyn GetDocument,
        virtual_system_mapping: &VirtualSystemMapping,
        doc: ResolvedDocument,
        table_mapping: &TableMapping,
        version: Version,
    ) -> anyhow::Result<DeveloperDocument> {
        // Note: in the future we may support different versions of our virtual table
        // APIs, which we determine based on the NPM client version
        let system_table_name = table_mapping.tablet_name(doc.id().tablet_id)?;
        if system_table_name == SCHEDULED_JOBS_TABLE.clone()
            && version < *MIN_NPM_VERSION_SCHEDULED_JOBS_V1
        {
            anyhow::bail!("System document cannot be converted to a virtual document")
        }

        let job: ParsedDocument<ScheduledJob> = (&doc).parse()?;
        let job: ScheduledJob = job.into_value();
        let udf_args = job.udf_args()?;
        let public_job = PublicScheduledJob {
            // TODO(ENG-6920) include component (job.path.component) in virtual table.
            name: job.path.udf_path,
            args: udf_args,
            state: job.state,
            scheduled_time: timestamp_to_ms(job.original_scheduled_ts)?,
            completed_time: match job.completed_ts {
                Some(ts) => Some(timestamp_to_ms(ts)?),
                None => None,
            },
        };
        let mut public_job_resolved: ConvexObject = public_job.try_into()?;

        let virtual_developer_id =
            virtual_system_mapping.system_resolved_id_to_virtual_developer_id(doc.id())?;

        let mut fields: BTreeMap<_, _> = public_job_resolved.into();
        fields.insert(ID_FIELD.to_owned().into(), virtual_developer_id.into());
        fields.insert(
            CREATION_TIME_FIELD.to_owned().into(),
            ConvexValue::from(f64::from(doc.creation_time())),
        );
        public_job_resolved = fields.try_into()?;

        let public_doc = DeveloperDocument::new(
            virtual_developer_id,
            doc.creation_time(),
            public_job_resolved,
        );
        Ok(public_doc)
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct PublicScheduledJob {
    pub name: CanonicalizedUdfPath,
    pub args: ConvexArray,
    pub state: ScheduledJobState,
    pub scheduled_time: f64,
    pub completed_time: Option<f64>,
}

impl TryFrom<PublicScheduledJob> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(job: PublicScheduledJob) -> anyhow::Result<Self> {
        let mut obj: BTreeMap<FieldName, ConvexValue> = BTreeMap::new();
        obj.insert(
            "name".parse()?,
            ConvexValue::try_from(String::from(job.name))?,
        );
        obj.insert("args".parse()?, ConvexValue::Array(job.args));

        // Rename `type` -> `kind` in the scheduled job state
        let system_state: ConvexObject = job.state.try_into()?;
        let mut fields: BTreeMap<_, _> = system_state.into();
        match fields.remove("type") {
            Some(value) => fields.insert(FieldName::from_str("kind")?, value),
            None => anyhow::bail!("Missing `type` field in ScheduledJobState"),
        };
        let public_state = fields.try_into()?;

        obj.insert("state".parse()?, ConvexValue::Object(public_state));
        obj.insert(
            "scheduledTime".parse()?,
            ConvexValue::Float64(job.scheduled_time),
        );
        if let Some(completed_time) = job.completed_time {
            obj.insert(
                "completedTime".parse()?,
                ConvexValue::Float64(completed_time),
            );
        }
        ConvexObject::try_from(obj)
    }
}

impl TryFrom<ConvexObject> for PublicScheduledJob {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<Self> {
        let mut fields = BTreeMap::from(obj);
        let name = match fields.remove("name") {
            Some(ConvexValue::String(name)) => String::from(name).parse()?,
            name => {
                anyhow::bail!("Missing or invalid `name` field for PublicScheduledJob: {name:?}")
            },
        };
        let args = match fields.remove("args") {
            Some(ConvexValue::Array(args)) => args,
            args => {
                anyhow::bail!("Missing or invalid `args` field for PublicScheduledJob: {args:?}")
            },
        };
        let public_state = match fields.remove("state") {
            Some(ConvexValue::Object(state)) => state,
            state => {
                anyhow::bail!("Missing or invalid `state` field for PublicScheduledJob: {state:?}")
            },
        };
        // Rename `kind` -> `type` in the scheduled job state
        let mut state_fields: BTreeMap<_, _> = public_state.into();
        match state_fields.remove("kind") {
            Some(value) => state_fields.insert(FieldName::from_str("type")?, value),
            None => anyhow::bail!("Missing `kind` field in ScheduledJobState"),
        };
        let system_state = ConvexObject::try_from(state_fields)?;
        let state = system_state.try_into()?;
        let scheduled_time = match fields.remove("scheduledTime") {
            Some(ConvexValue::Float64(scheduled_time)) => scheduled_time,
            scheduled_time => anyhow::bail!(
                "Missing or invalid `scheduledTime` field for PublicScheduledJob: \
                 {scheduled_time:?}"
            ),
        };
        let completed_time = match fields.remove("completedTime") {
            None => None,
            Some(ConvexValue::Float64(completed_time)) => Some(completed_time),
            completed_time => anyhow::bail!(
                "Invalid `completedTime` field for PublicScheduledJob: {completed_time:?}"
            ),
        };
        Ok(PublicScheduledJob {
            name,
            args,
            state,
            scheduled_time,
            completed_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use value::{
        testing::assert_roundtrips,
        ConvexObject,
    };

    use crate::scheduled_jobs::virtual_table::PublicScheduledJob;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_public_scheduled_job_roundtrips(v in any::<PublicScheduledJob>()) {
            assert_roundtrips::<PublicScheduledJob, ConvexObject>(v);
        }
    }
}
