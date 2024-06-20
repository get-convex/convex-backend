#![allow(non_snake_case)]

use anyhow::Context;
use common::{
    query::Query,
    runtime::Runtime,
    static_span,
    version::Version,
};
use database::{
    query::TableFilter,
    DeveloperQuery,
};
use errors::ErrorMetadata;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use value::{
    id_v6::DeveloperDocumentId,
    InternalId,
    TableName,
    TableNamespace,
    TableNumber,
    TabletIdAndTableNumber,
};

use super::{
    async_syscall::AsyncSyscallProvider,
    DatabaseUdfEnvironment,
};
use crate::environment::helpers::{
    parse_version,
    with_argument_error,
    ArgName,
};

pub trait SyscallProvider<RT: Runtime> {
    fn table_filter(&self) -> TableFilter;

    fn lookup_table(&mut self, name: &TableName) -> anyhow::Result<Option<TabletIdAndTableNumber>>;
    fn lookup_virtual_table(&mut self, name: &TableName) -> anyhow::Result<Option<TableNumber>>;

    fn start_query(&mut self, query: Query, version: Option<Version>) -> anyhow::Result<u32>;
    fn cleanup_query(&mut self, query_id: u32) -> bool;
}

impl<RT: Runtime> SyscallProvider<RT> for DatabaseUdfEnvironment<RT> {
    fn table_filter(&self) -> TableFilter {
        if self.path.udf_path.is_system() {
            TableFilter::IncludePrivateSystemTables
        } else {
            TableFilter::ExcludePrivateSystemTables
        }
    }

    fn lookup_table(&mut self, name: &TableName) -> anyhow::Result<Option<TabletIdAndTableNumber>> {
        let table_mapping = self
            .phase
            .tx()?
            .table_mapping()
            .namespace(TableNamespace::by_component_TODO());
        Ok(table_mapping.id_and_number_if_exists(name))
    }

    fn lookup_virtual_table(&mut self, name: &TableName) -> anyhow::Result<Option<TableNumber>> {
        let virtual_table_mapping = self.phase.tx()?.virtual_table_mapping();
        Ok(virtual_table_mapping.number_if_exists(name))
    }

    fn start_query(&mut self, query: Query, version: Option<Version>) -> anyhow::Result<u32> {
        let table_filter = SyscallProvider::<RT>::table_filter(self);
        let component = self.component()?;
        let tx = self.phase.tx()?;
        // TODO: Are all invalid query pipelines developer errors? These could be bugs
        // in convex/server.
        let compiled_query = {
            DeveloperQuery::new_with_version(tx, component.into(), query, version, table_filter)?
        };
        let query_id = self.query_manager.put_developer(compiled_query);
        Ok(query_id)
    }

    fn cleanup_query(&mut self, query_id: u32) -> bool {
        self.query_manager.cleanup_developer(query_id)
    }
}

pub fn syscall_impl<RT: Runtime, P: SyscallProvider<RT>>(
    provider: &mut P,
    name: &str,
    args: JsonValue,
) -> anyhow::Result<JsonValue> {
    match name {
        "1.0/queryCleanup" => syscall_query_cleanup(provider, args),
        "1.0/queryStream" => syscall_query_stream(provider, args),
        "1.0/db/normalizeId" => syscall_normalize_id(provider, args),

        #[cfg(test)]
        "throwSystemError" => anyhow::bail!("I can't go for that."),
        "throwOcc" => anyhow::bail!(ErrorMetadata::user_occ(None, None)),
        "throwOverloaded" => {
            anyhow::bail!(ErrorMetadata::overloaded("Busy", "I'm a bit busy."))
        },
        #[cfg(test)]
        "slowSyscall" => {
            std::thread::sleep(std::time::Duration::from_secs(1));
            Ok(JsonValue::Number(1017.into()))
        },
        #[cfg(test)]
        "reallySlowSyscall" => {
            std::thread::sleep(std::time::Duration::from_secs(3));
            Ok(JsonValue::Number(1017.into()))
        },

        _ => {
            anyhow::bail!(ErrorMetadata::bad_request(
                "UnknownOperation",
                format!("Unknown operation {name}")
            ));
        },
    }
}

fn syscall_normalize_id<RT: Runtime, P: SyscallProvider<RT>>(
    provider: &mut P,
    args: JsonValue,
) -> anyhow::Result<JsonValue> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct NormalizeIdArgs {
        table: String,
        id_string: String,
    }
    let (table_name, id_string) = with_argument_error("db.normalizeId", || {
        let args: NormalizeIdArgs = serde_json::from_value(args)?;
        let table_name: TableName = args.table.parse().context(ArgName("table"))?;
        Ok((table_name, args.id_string))
    })?;
    let virtual_table_number = provider.lookup_virtual_table(&table_name)?;
    let table_number = match virtual_table_number {
        Some(table_number) => Some(table_number),
        None => {
            let physical_table_number = provider.lookup_table(&table_name)?.map(|t| t.table_number);
            match provider.table_filter() {
                TableFilter::IncludePrivateSystemTables => physical_table_number,
                TableFilter::ExcludePrivateSystemTables if table_name.is_system() => None,
                TableFilter::ExcludePrivateSystemTables => physical_table_number,
            }
        },
    };
    let normalized_id = match table_number {
        Some(table_number) => {
            if let Ok(id_v6) = DeveloperDocumentId::decode(&id_string)
                && id_v6.table() == table_number
            {
                Some(id_v6)
            } else if let Ok(internal_id) = InternalId::from_developer_str(&id_string) {
                let id_v6 = DeveloperDocumentId::new(table_number, internal_id);
                Some(id_v6)
            } else {
                None
            }
        },
        None => None,
    };
    match normalized_id {
        Some(id_v6) => Ok(json!({ "id": id_v6.encode() })),
        None => Ok(json!({ "id": JsonValue::Null })),
    }
}

fn syscall_query_stream<RT: Runtime, P: SyscallProvider<RT>>(
    provider: &mut P,
    args: JsonValue,
) -> anyhow::Result<JsonValue> {
    let _s = static_span!();

    #[derive(Deserialize)]
    struct QueryStreamArgs {
        query: JsonValue,
        version: Option<String>,
    }
    let (parsed_query, version) = with_argument_error("queryStream", || {
        let args: QueryStreamArgs = serde_json::from_value(args)?;
        let parsed_query = Query::try_from(args.query).context(ArgName("query"))?;
        let version = parse_version(args.version)?;
        Ok((parsed_query, version))
    })?;
    let query_id = provider.start_query(parsed_query, version)?;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct QueryStreamResult {
        query_id: u32,
    }
    Ok(serde_json::to_value(QueryStreamResult { query_id })?)
}

fn syscall_query_cleanup<RT: Runtime, P: SyscallProvider<RT>>(
    provider: &mut P,
    args: JsonValue,
) -> anyhow::Result<JsonValue> {
    let _s = static_span!();

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct QueryCleanupArgs {
        query_id: u32,
    }
    let args: QueryCleanupArgs =
        with_argument_error("queryCleanup", || Ok(serde_json::from_value(args)?))?;
    let cleaned_up = provider.cleanup_query(args.query_id);
    Ok(serde_json::to_value(cleaned_up)?)
}
