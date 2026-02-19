# Advanced Configuration and Tuning

There is a large number of detailed configuration options in
[knobs.rs](/crates/common/src/knobs.rs). These options are configurable via
environment variables. In order to tune your Convex instance at scale for your
workload, you may need to adjust these knobs. You will have to set these
environment variables by adding them to your `docker-compose.yml` file. Commonly
overriden knobs are listed in the `env` section of the
[`docker-compose.yml`](../docker/docker-compose.yml)

## `APPLICATION_MAX_CONCURRENT_*` knobs

You can increase the max concurrency on your self-hosted instance with these
environment variables. Note that increasing concurrency will increase load on
your system and after a certain threshold, performance will degrade. You will
have to tune parameters based on your own hardware and workload.
