## Log Streaming

### Concepts

`LogManager`: the main router/aggregation point for logging events.

`LogSource`: a source that creates and buffers logs for the `LogManager` to
query.

`LogSink`: a dynamically-attachable log receiver. Subscribes to `LogManager`.

### Adding a new LogSink

See `sinks/datadog.rs` for an example of a simple sink. The following steps are
necessary:

1. Write the sink implementation which implements `LogSink` in `sinks/`
2. Provide a `start` method that configures your sink and returns a
   `LogSinkClient`. This is how the `LogManager` communicates with your
   `LogSink` and is able to send it events.
3. Create a backend endpoint similar to `/api/datadog_integration` which accepts
   a `post` method to create the integration. This should collect the necessary
   configuration parameters and update the `_log_sinks` table.
4. Add a new `LogSinkType` and `LogSinkConfig` with an associated configuration
   with an associated config which can be stores in the DB model.
5. Update the switch in `LogManager::config_to_log_sink_client` to call `start`
   on your `LogSink` with the necessary configuration params.
6. If configured correctly, now the `LogManager` will forward logs to your sink
   once the sink is configured and subscribed to!
