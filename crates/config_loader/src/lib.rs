//! Tools for dynamically loading a configuration file from disk upon receiving
//! a signal

#![feature(try_blocks)]
#![feature(trait_alias)]

use std::{
    future,
    path::PathBuf,
};

use anyhow::Context;
use common::{
    errors::report_error,
    runtime::{
        Runtime,
        SpawnHandle,
    },
};
use decoding::ConfigDecoder;
use futures::{
    Stream,
    StreamExt,
};
use tokio::{
    signal::unix::SignalKind,
    sync::watch,
};
use tokio_stream::wrappers::{
    ReceiverStream,
    SignalStream,
    WatchStream,
};

pub mod decoding;
pub mod encoding;
mod metrics;

pub struct ImmediateMode;
pub struct LazyMode;

mod mode {
    pub trait ConfigLoaderMode<T> {
        type Maybe: From<T> + Into<Option<T>> + PartialEq + Clone + Send + Sync + 'static;
        const LAZY: bool;
    }

    impl<T> ConfigLoaderMode<T> for super::ImmediateMode
    where
        T: PartialEq + Clone + Send + Sync + 'static,
    {
        type Maybe = T;

        const LAZY: bool = false;
    }

    impl<T> ConfigLoaderMode<T> for super::LazyMode
    where
        T: PartialEq + Clone + Send + Sync + 'static,
    {
        type Maybe = Option<T>;

        const LAZY: bool = true;
    }
}

/// This struct loads a file from disk upon creation and then sets up a signal
/// handler to load it again whenever a user-defined signal is received.
/// Users can subscribe to changes with the [`ConfigLoader::subscribe`] method,
/// which passes the config through a [`ConfigDecoder`] implementation to allow
/// converting from various formats (e.g. TextProto format to protobuf message
/// via [TextProtoDecoder](`decoding::TextProtoDecoder`)).
/// Multiple subscribers can share the same [`ConfigLoader`] and decoding will
/// still only be performed once per update, and the output is not unnecessarily
/// cloned unless the caller desires ownership.
///
/// Requires running inside a Tokio runtime due to usage of [`tokio::fs`].
pub struct ConfigLoader<D: ConfigDecoder + Send + 'static, M: mode::ConfigLoaderMode<D::Output>> {
    config_rx: watch::Receiver<M::Maybe>,
    reload_tx: tokio::sync::mpsc::Sender<()>,
    handle: Box<dyn SpawnHandle>,
}

impl<D: ConfigDecoder + Send + 'static> ConfigLoader<D, ImmediateMode> {
    /// Create a ConfigLoader and decode the current value of the config,
    /// returning an error if it can't be decoded.
    pub async fn new_immediate<RT: Runtime>(
        rt: RT,
        signal_kind: SignalKind,
        config_path: PathBuf,
        decoder: D,
    ) -> anyhow::Result<Self> {
        Self::new(
            rt,
            signal_kind,
            config_path,
            decoder,
            async |config_path, decoder| decoder.decode(tokio::fs::read(&config_path).await?),
        )
        .await
    }
}

impl<D: ConfigDecoder + Send + 'static> ConfigLoader<D, LazyMode> {
    /// Create a ConfigLoader that decodes the config in the background. This is
    /// useful if you don't want decode errors to block startup.
    pub async fn new_lazy<RT: Runtime>(
        rt: RT,
        signal_kind: SignalKind,
        config_path: PathBuf,
        decoder: D,
    ) -> anyhow::Result<Self> {
        Self::new(rt, signal_kind, config_path, decoder, async |_, _| Ok(None)).await
    }
}

impl<D: ConfigDecoder + Send + 'static, M: mode::ConfigLoaderMode<D::Output>> ConfigLoader<D, M> {
    async fn new<RT: Runtime>(
        rt: RT,
        signal_kind: SignalKind,
        config_path: PathBuf,
        decoder: D,
        init: impl AsyncFnOnce(&PathBuf, &D) -> anyhow::Result<M::Maybe>,
    ) -> anyhow::Result<Self> {
        // Make sure we set up the signal handler before spawning the listener task so
        // there's no chance of missing signals after the initial read of the config
        // file.
        let signal_fut =
            tokio::signal::unix::signal(signal_kind).context("Couldn't set up signal handler")?;
        let initial_value = init(&config_path, &decoder).await?;
        let (config_tx, mut config_rx) = watch::channel(initial_value);
        config_rx.mark_changed();
        let (reload_tx, reload_rx) = tokio::sync::mpsc::channel(1);
        if M::LAZY {
            reload_tx.try_send(())?;
        }
        let handle = rt.spawn("config_loader", async move {
            let config_path = config_path;
            tracing::info!("Starting config loader thread for {config_path:?}");
            let mut stream = futures::stream::select(
                SignalStream::new(signal_fut),
                ReceiverStream::new(reload_rx),
            );
            let mut invalid_config_gauge = None;
            loop {
                let () = stream.select_next_some().await;
                match tokio::fs::read(&config_path)
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|s| decoder.decode(s))
                    .with_context(|| format!("Failed to reload config from {config_path:?}"))
                {
                    Ok(config) => {
                        invalid_config_gauge = None;
                        tracing::info!("Reloading config from {config_path:?}");
                        let config = <M::Maybe>::from(config);
                        config_tx.send_if_modified(|old_config| {
                            if old_config != &config {
                                *old_config = config;
                                return true;
                            }
                            false
                        });
                    },
                    Err(mut e) => {
                        invalid_config_gauge
                            .get_or_insert_with(|| metrics::invalid_config_gauge(&config_path))
                            .set(1);
                        report_error(&mut e).await;
                        continue;
                    },
                }
            }
        });
        Ok(ConfigLoader {
            handle,
            config_rx,
            reload_tx,
        })
    }

    /// Returns a stream of updates to the config file. This stream only emits a
    /// new value when the result of decoding the file is different.
    ///
    /// If `include_current` is true, the stream will include the current value
    /// (assuming one has successfully been decoded); otherwise it only returns
    /// future changes.
    pub fn subscribe(
        &self,
        include_current: bool,
    ) -> impl Stream<Item = D::Output> + Unpin + use<D, M> {
        let mut rx = self.config_rx.clone();
        if include_current {
            rx.mark_changed();
        } else {
            rx.mark_unchanged();
        }
        // Omit `None` from the resulting stream. For LazyMode this means the
        // stream will block if there is an error reading the config.
        WatchStream::from_changes(rx).filter_map(|v| future::ready(v.into()))
    }

    /// Returns the current decoded config, if available.
    pub fn get_config(&self) -> M::Maybe {
        self.config_rx.borrow().clone()
    }

    /// Manually trigger a reload of the configuration file on disk.
    pub fn reload(&self) {
        let _ = self.reload_tx.try_send(());
    }
}

impl<D: ConfigDecoder + Send, M: mode::ConfigLoaderMode<D::Output>> Drop for ConfigLoader<D, M> {
    fn drop(&mut self) {
        self.handle.shutdown()
    }
}
