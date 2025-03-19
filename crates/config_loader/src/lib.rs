//! Tools for dynamically loading a configuration file from disk upon receiving
//! a signal

#![feature(try_blocks)]
#![feature(trait_alias)]

use std::path::PathBuf;

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
pub struct ConfigLoader<D: ConfigDecoder + Send + 'static> {
    config_rx: watch::Receiver<D::Output>,
    reload_tx: tokio::sync::mpsc::Sender<()>,
    handle: Box<dyn SpawnHandle>,
}

impl<D: ConfigDecoder + Send + 'static> ConfigLoader<D> {
    pub async fn new<RT: Runtime>(
        rt: RT,
        signal_kind: SignalKind,
        config_path: PathBuf,
        decoder: D,
    ) -> anyhow::Result<Self> {
        // Make sure we set up the signal handler before spawning the listener task so
        // there's no chance of missing signals after the initial read of the config
        // file.
        let signal_fut =
            tokio::signal::unix::signal(signal_kind).context("Couldn't set up signal handler")?;
        let initial_value = decoder.decode(tokio::fs::read(&config_path).await?)?;
        let (config_tx, config_rx) = watch::channel(initial_value);
        let _decoder = decoder.clone();
        let (reload_tx, reload_rx) = tokio::sync::mpsc::channel(1);
        let handle = rt.spawn("config_loader", async move {
            let config_path = config_path;
            let decoder = _decoder;
            tracing::info!("Starting config loader thread for {config_path:?}");
            let mut stream = futures::stream::select(
                SignalStream::new(signal_fut),
                ReceiverStream::new(reload_rx),
            );
            loop {
                let () = stream.select_next_some().await;
                match tokio::fs::read(&config_path)
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|s| decoder.decode(s))
                    .with_context(|| format!("Failed to reload config from {config_path:?}"))
                {
                    Ok(config) => {
                        tracing::info!("Reloading config from {config_path:?}");
                        config_tx.send_if_modified(|old_config| {
                            if old_config != &config {
                                *old_config = config;
                                return true;
                            }
                            false
                        });
                    },
                    Err(mut e) => {
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

    /// Returns a stream of updates to the config file. The stream is initially
    /// blocked; to get the current value, use [`ConfigLoader::get_config`]
    /// This stream only emits a new value when the result of decoding the file
    /// is different.
    pub fn subscribe(&self) -> impl Stream<Item = D::Output> + Unpin {
        let mut rx = self.config_rx.clone();
        rx.mark_unchanged();
        WatchStream::from_changes(rx)
    }

    /// Returns a reference to the current decoded config. The output can be
    /// cloned if ownership is needed.
    pub fn get_config(&self) -> D::Output {
        self.config_rx.borrow().clone()
    }

    /// Manually trigger a reload of the configuration file on disk.
    pub fn reload(&self) {
        let _ = self.reload_tx.try_send(());
    }
}

impl<D: ConfigDecoder + Send> Drop for ConfigLoader<D> {
    fn drop(&mut self) {
        self.handle.shutdown()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{
            Seek,
            Write,
        },
        sync::LazyLock,
    };

    use futures::{
        FutureExt,
        StreamExt,
    };
    use pb::{
        common::RedactedLogLines,
        DESCRIPTOR_POOL,
    };
    use prost_reflect::{
        DynamicMessage,
        MessageDescriptor,
    };
    use runtime::prod::ProdRuntime;
    use tokio::signal::unix::SignalKind;

    use crate::{
        decoding::TextProtoDecoder,
        ConfigLoader,
    };

    static CONFIG_DESCRIPTOR: LazyLock<MessageDescriptor> = LazyLock::new(|| {
        DESCRIPTOR_POOL
            .get_message_by_name("common.RedactedLogLines")
            .unwrap()
    });

    #[convex_macro::prod_rt_test]
    async fn test_config_loader(rt: ProdRuntime) -> anyhow::Result<()> {
        // Just choosing an arbitrary proto for testing.
        let initial_config = RedactedLogLines {
            log_lines: vec!["foo".to_owned()],
        };
        let mut file = tempfile::NamedTempFile::new()?;
        {
            let mut message = DynamicMessage::new(CONFIG_DESCRIPTOR.clone());
            message.transcode_from(&initial_config)?;
            file.write_all(message.to_text_format().as_bytes())?;
        }
        let config_loader = ConfigLoader::new(
            rt.clone(),
            SignalKind::user_defined1(),
            file.path().to_owned(),
            TextProtoDecoder::new(CONFIG_DESCRIPTOR.clone()),
        )
        .await?;
        assert_eq!(config_loader.get_config(), initial_config);
        let mut subscription = config_loader.subscribe().fuse();
        assert!(subscription.next().now_or_never().is_none());
        let new_config = RedactedLogLines {
            log_lines: vec!["bar".to_owned()],
        };
        {
            let mut message = DynamicMessage::new(CONFIG_DESCRIPTOR.clone());
            message.transcode_from(&new_config)?;
            file.as_file().set_len(0)?;
            file.seek(std::io::SeekFrom::Start(0))?;
            file.write_all(message.to_text_format().as_bytes())?;
        }
        config_loader.reload();
        let next_config = subscription.select_next_some().await;
        assert_eq!(new_config, next_config);
        assert_eq!(config_loader.get_config(), new_config);
        Ok(())
    }
}
