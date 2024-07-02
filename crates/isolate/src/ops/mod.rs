//! This module contains the implementation of both synchronous and
//! async ops. Unlike syscalls, these functions are present in *every*
//! environment, but the environment may decide not to implement their
//! functionality, causing a runtime error.

mod blob;
mod console;
mod crypto;
mod database;
mod environment_variables;
mod errors;
mod http;
mod random;
mod storage;
mod stream;
mod text;
mod time;
mod validate_args;

use std::{
    collections::BTreeMap,
    ops::DerefMut,
};

use ::errors::ErrorMetadata;
use anyhow::anyhow;
use bytes::Bytes;
use common::{
    log_lines::LogLevel,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        EnvVarName,
        EnvVarValue,
    },
};
use deno_core::{
    v8,
    ModuleSpecifier,
};
use rand_chacha::ChaCha12Rng;
use sourcemap::SourceMap;
use uuid::Uuid;
use value::{
    heap_size::WithHeapSize,
    NamespacedTableMapping,
    NamespacedVirtualTableMapping,
    TableMappingValue,
};

use self::{
    blob::{
        op_blob_create_part,
        op_blob_read_part,
        op_blob_slice_part,
    },
    console::{
        op_console_message,
        op_console_time_end,
        op_console_time_log,
        op_console_time_start,
        op_console_trace,
    },
    crypto::{
        op_crypto_base64_url_decode,
        op_crypto_base64_url_encode,
        op_crypto_derive_bits,
        op_crypto_digest,
        op_crypto_export_key,
        op_crypto_export_pkcs8_ed25519,
        op_crypto_export_pkcs8_x25519,
        op_crypto_export_spki_ed25519,
        op_crypto_export_spki_x25519,
        op_crypto_get_random_values,
        op_crypto_import_key,
        op_crypto_import_pkcs8_ed25519,
        op_crypto_import_pkcs8_x25519,
        op_crypto_import_spki_ed25519,
        op_crypto_import_spki_x25519,
        op_crypto_jwk_x_ed25519,
        op_crypto_random_uuid,
        op_crypto_sign,
        op_crypto_sign_ed25519,
        op_crypto_verify,
        op_crypto_verify_ed25519,
    },
    database::op_get_table_mapping_without_system_tables,
    environment_variables::op_environment_variables_get,
    errors::{
        op_error_stack,
        op_throw_uncatchable_developer_error,
    },
    http::{
        async_op_fetch,
        async_op_parse_multi_part,
        op_headers_get_mime_type,
        op_headers_normalize_name,
        op_url_get_url_info,
        op_url_get_url_search_param_pairs,
        op_url_stringify_url_search_params,
        op_url_update_url_info,
    },
    storage::{
        async_op_storage_get,
        async_op_storage_store,
    },
    stream::{
        async_op_stream_read_part,
        op_stream_create,
        op_stream_extend,
    },
    text::{
        op_atob,
        op_btoa,
        op_text_encoder_cleanup,
        op_text_encoder_decode,
        op_text_encoder_decode_single,
        op_text_encoder_encode,
        op_text_encoder_encode_into,
        op_text_encoder_new_decoder,
        op_text_encoder_normalize_label,
    },
    time::{
        async_op_sleep,
        op_now,
    },
    validate_args::op_validate_args,
};
pub use self::{
    crypto::CryptoOps,
    random::op_random,
};
use crate::{
    environment::{
        AsyncOpRequest,
        IsolateEnvironment,
    },
    execution_scope::ExecutionScope,
    helpers::to_rust_string,
    metrics,
    request_scope::{
        ReadableStream,
        StreamListener,
        TextDecoderResource,
    },
};

pub trait OpProvider<'b> {
    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng>;
    fn scope(&mut self) -> &mut v8::HandleScope<'b>;
    fn lookup_source_map(
        &mut self,
        specifier: &ModuleSpecifier,
    ) -> anyhow::Result<Option<SourceMap>>;
    fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()>;
    fn console_timers(
        &mut self,
    ) -> anyhow::Result<&mut WithHeapSize<BTreeMap<String, UnixTimestamp>>>;
    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp>;
    fn unix_timestamp_non_deterministic(&mut self) -> anyhow::Result<UnixTimestamp>;

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()>;

    fn create_blob_part(&mut self, bytes: Bytes) -> anyhow::Result<Uuid>;
    fn get_blob_part(&mut self, uuid: &Uuid) -> anyhow::Result<Option<Bytes>>;

    fn create_stream(&mut self) -> anyhow::Result<Uuid>;
    fn extend_stream(
        &mut self,
        id: Uuid,
        bytes: Option<Bytes>,
        new_done: bool,
    ) -> anyhow::Result<()>;
    fn new_stream_listener(
        &mut self,
        stream_id: Uuid,
        listener: StreamListener,
    ) -> anyhow::Result<()>;

    fn create_text_decoder(&mut self, decoder: TextDecoderResource) -> anyhow::Result<Uuid>;
    fn get_text_decoder(&mut self, uuid: &Uuid) -> anyhow::Result<&mut TextDecoderResource>;
    fn remove_text_decoder(&mut self, uuid: &Uuid) -> anyhow::Result<TextDecoderResource>;

    fn get_environment_variable(&mut self, name: EnvVarName)
        -> anyhow::Result<Option<EnvVarValue>>;

    fn get_all_table_mappings(
        &mut self,
    ) -> anyhow::Result<(NamespacedTableMapping, NamespacedVirtualTableMapping)>;
    fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue>;
}

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> OpProvider<'b>
    for ExecutionScope<'a, 'b, RT, E>
{
    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
        let state = self.state_mut()?;
        state.environment.rng()
    }

    fn lookup_source_map(
        &mut self,
        specifier: &ModuleSpecifier,
    ) -> anyhow::Result<Option<SourceMap>> {
        ExecutionScope::lookup_source_map(self, specifier)
    }

    fn scope(&mut self) -> &mut v8::HandleScope<'b> {
        self.deref_mut()
    }

    fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        state.environment.trace(level, messages)?;
        Ok(())
    }

    fn console_timers(
        &mut self,
    ) -> anyhow::Result<&mut WithHeapSize<BTreeMap<String, UnixTimestamp>>> {
        let state = self.state_mut()?;
        Ok(&mut state.console_timers)
    }

    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp> {
        let state = self.state_mut()?;
        state.environment.unix_timestamp()
    }

    fn unix_timestamp_non_deterministic(&mut self) -> anyhow::Result<UnixTimestamp> {
        let state = self.state_mut()?;
        Ok(state.unix_timestamp_non_deterministic())
    }

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        state.environment.start_async_op(request, resolver)
    }

    fn create_blob_part(&mut self, bytes: Bytes) -> anyhow::Result<Uuid> {
        let state = self.state_mut()?;
        state.create_blob_part(bytes)
    }

    fn get_blob_part(&mut self, uuid: &Uuid) -> anyhow::Result<Option<Bytes>> {
        let state = self.state_mut()?;
        Ok(state.blob_parts.get(uuid).cloned())
    }

    fn create_stream(&mut self) -> anyhow::Result<Uuid> {
        self.state_mut()?.create_stream()
    }

    fn extend_stream(
        &mut self,
        id: Uuid,
        bytes: Option<Bytes>,
        new_done: bool,
    ) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        let new_part_id = match bytes {
            Some(bytes) => Some(state.create_blob_part(bytes)?),
            None => None,
        };
        state.streams.mutate(&id, |stream| -> anyhow::Result<()> {
            let Some(Ok(ReadableStream { parts, done })) = stream else {
                anyhow::bail!("unrecognized stream id {id}");
            };
            if *done {
                anyhow::bail!("stream {id} is already done");
            }
            if let Some(new_part_id) = new_part_id {
                parts.push_back(new_part_id);
            }
            if new_done {
                *done = true;
            }
            Ok(())
        })?;
        self.update_stream_listeners()?;
        Ok(())
    }

    fn new_stream_listener(
        &mut self,
        stream_id: Uuid,
        listener: StreamListener,
    ) -> anyhow::Result<()> {
        if self
            .state_mut()?
            .stream_listeners
            .insert(stream_id, listener)
            .is_some()
        {
            anyhow::bail!("cannot read from the same stream twice");
        }
        self.update_stream_listeners()
    }

    fn create_text_decoder(&mut self, decoder: TextDecoderResource) -> anyhow::Result<Uuid> {
        self.state_mut()?.create_text_decoder(decoder)
    }

    fn get_text_decoder(&mut self, uuid: &Uuid) -> anyhow::Result<&mut TextDecoderResource> {
        self.state_mut()?.get_text_decoder(uuid)
    }

    fn remove_text_decoder(&mut self, uuid: &Uuid) -> anyhow::Result<TextDecoderResource> {
        self.state_mut()?.remove_text_decoder(uuid)
    }

    fn get_environment_variable(
        &mut self,
        name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        let state = self.state_mut()?;
        state.environment.get_environment_variable(name)
    }

    fn get_all_table_mappings(
        &mut self,
    ) -> anyhow::Result<(NamespacedTableMapping, NamespacedVirtualTableMapping)> {
        let state = self.state_mut()?;
        state.environment.get_all_table_mappings()
    }

    fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue> {
        let state = self.state_mut()?;
        state.environment.get_table_mapping_without_system_tables()
    }
}

pub fn run_op<'b, P: OpProvider<'b>>(
    provider: &mut P,
    args: v8::FunctionCallbackArguments,
    rv: v8::ReturnValue,
) -> anyhow::Result<()> {
    if args.length() < 1 {
        // This must be a bug in our `udf-runtime` code, not a developer error.
        anyhow::bail!("op(op_name, ...) takes at least one argument");
    }
    let op_name: v8::Local<v8::String> = args.get(0).try_into()?;
    let op_name = to_rust_string(provider.scope(), &op_name)?;

    let timer = metrics::op_timer(&op_name);
    match &op_name[..] {
        "throwUncatchableDeveloperError" => {
            op_throw_uncatchable_developer_error(provider, args, rv)?
        },
        "console/message" => op_console_message(provider, args, rv)?,
        "console/trace" => op_console_trace(provider, args, rv)?,
        "console/timeStart" => op_console_time_start(provider, args, rv)?,
        "console/timeLog" => op_console_time_log(provider, args, rv)?,
        "console/timeEnd" => op_console_time_end(provider, args, rv)?,
        "error/stack" => op_error_stack(provider, args, rv)?,
        "random" => op_random(provider, args, rv)?,
        "now" => op_now(provider, args, rv)?,
        "url/getUrlInfo" => op_url_get_url_info(provider, args, rv)?,
        "url/getUrlSearchParamPairs" => op_url_get_url_search_param_pairs(provider, args, rv)?,
        "url/stringifyUrlSearchParams" => op_url_stringify_url_search_params(provider, args, rv)?,
        "url/updateUrlInfo" => op_url_update_url_info(provider, args, rv)?,
        "blob/createPart" => op_blob_create_part(provider, args, rv)?,
        "blob/slicePart" => op_blob_slice_part(provider, args, rv)?,
        "blob/readPart" => op_blob_read_part(provider, args, rv)?,
        "headers/getMimeType" => op_headers_get_mime_type(provider, args, rv)?,
        "headers/normalizeName" => op_headers_normalize_name(provider, args, rv)?,
        "stream/create" => op_stream_create(provider, args, rv)?,
        "stream/extend" => op_stream_extend(provider, args, rv)?,
        "textEncoder/encode" => op_text_encoder_encode(provider, args, rv)?,
        "textEncoder/encodeInto" => op_text_encoder_encode_into(provider, args, rv)?,
        "textEncoder/decodeSingle" => op_text_encoder_decode_single(provider, args, rv)?,
        "textEncoder/decode" => op_text_encoder_decode(provider, args, rv)?,
        "textEncoder/newDecoder" => op_text_encoder_new_decoder(provider, args, rv)?,
        "textEncoder/cleanup" => op_text_encoder_cleanup(provider, args, rv)?,
        "textEncoder/normalizeLabel" => op_text_encoder_normalize_label(provider, args, rv)?,
        "atob" => op_atob(provider, args, rv)?,
        "btoa" => op_btoa(provider, args, rv)?,
        "environmentVariables/get" => op_environment_variables_get(provider, args, rv)?,
        "getTableMappingWithoutSystemTables" => {
            op_get_table_mapping_without_system_tables(provider, args, rv)?
        },
        "validateArgs" => op_validate_args(provider, args, rv)?,

        "crypto/randomUUID" => op_crypto_random_uuid(provider, args, rv)?,
        "crypto/getRandomValues" => op_crypto_get_random_values(provider, args, rv)?,
        "crypto/sign" => op_crypto_sign(provider, args, rv)?,
        "crypto/signEd25519" => op_crypto_sign_ed25519(provider, args, rv)?,
        "crypto/verify" => op_crypto_verify(provider, args, rv)?,
        "crypto/verifyEd25519" => op_crypto_verify_ed25519(provider, args, rv)?,
        "crypto/deriveBits" => op_crypto_derive_bits(provider, args, rv)?,
        "crypto/digest" => op_crypto_digest(provider, args, rv)?,
        "crypto/importKey" => op_crypto_import_key(provider, args, rv)?,
        "crypto/importSpkiEd25519" => op_crypto_import_spki_ed25519(provider, args, rv)?,
        "crypto/importPkcs8Ed25519" => op_crypto_import_pkcs8_ed25519(provider, args, rv)?,
        "crypto/importSpkiX25519" => op_crypto_import_spki_x25519(provider, args, rv)?,
        "crypto/importPkcs8X25519" => op_crypto_import_pkcs8_x25519(provider, args, rv)?,
        "crypto/base64UrlEncode" => op_crypto_base64_url_encode(provider, args, rv)?,
        "crypto/base64UrlDecode" => op_crypto_base64_url_decode(provider, args, rv)?,
        "crypto/exportKey" => op_crypto_export_key(provider, args, rv)?,
        "crypto/exportSpkiEd25519" => op_crypto_export_spki_ed25519(provider, args, rv)?,
        "crypto/exportPkcs8Ed25519" => op_crypto_export_pkcs8_ed25519(provider, args, rv)?,
        "crypto/JwkXEd25519" => op_crypto_jwk_x_ed25519(provider, args, rv)?,
        "crypto/exportSpkiX25519" => op_crypto_export_spki_x25519(provider, args, rv)?,
        "crypto/exportPkcs8X25519" => op_crypto_export_pkcs8_x25519(provider, args, rv)?,
        _ => {
            anyhow::bail!(ErrorMetadata::bad_request(
                "UnknownOperation",
                format!("Unknown operation {op_name}")
            ));
        },
    }
    timer.finish();
    Ok(())
}

pub fn start_async_op<'b, P: OpProvider<'b>>(
    provider: &mut P,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) -> anyhow::Result<()> {
    if args.length() < 1 {
        anyhow::bail!("asyncOp(op, ...args) takes at least one argument");
    }
    let op_name: v8::Local<v8::String> = args.get(0).try_into()?;
    let op_name = to_rust_string(provider.scope(), &op_name)?;

    let resolver = v8::PromiseResolver::new(provider.scope())
        .ok_or_else(|| anyhow!("Failed to create PromiseResolver"))?;
    let promise = resolver.get_promise(provider.scope());
    let resolver = v8::Global::new(provider.scope(), resolver);

    match &op_name[..] {
        "fetch" => async_op_fetch(provider, args, resolver)?,
        "form/parseMultiPart" => async_op_parse_multi_part(provider, args, resolver)?,
        "sleep" => async_op_sleep(provider, args, resolver)?,
        "storage/store" => async_op_storage_store(provider, args, resolver)?,
        "storage/get" => async_op_storage_get(provider, args, resolver)?,
        "stream/readPart" => async_op_stream_read_part(provider, args, resolver)?,
        _ => {
            anyhow::bail!(ErrorMetadata::bad_request(
                "UnknownAsyncOperation",
                format!("Unknown async operation {op_name}")
            ));
        },
    };

    rv.set(promise.into());
    Ok(())
}
