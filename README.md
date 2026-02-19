# bec_log_ingestor

[![Linting](https://github.com/bec-project/bec_log_ingestor/actions/workflows/check-and-lint.yaml/badge.svg)](https://github.com/bec-project/bec_log_ingestor/actions/workflows/check-and-lint.yaml) [![codecov](https://codecov.io/gh/bec-project/bec_log_ingestor/graph/badge.svg?token=B7Mzj4EhzH)](https://codecov.io/gh/bec-project/bec_log_ingestor)

Small service to pull BEC logs and metrics from Redis and push them to Loki and Mimir.

## Configuration

There is an example config file in `./install/example_config.toml`.
Logging or metrics can be temporarily disabled individually with `enable_logging = false` or `enable_logging = false` at
the top level. The config still needs to be syntactically valid for the disabled component, but the service will not try
to connect to it.

### Metric intervals

Builtin metrics can have their polling frequency defined in the config file, under `[metrics.intervals]`, for example:
```toml
[metrics.intervals]
cpu_usage_percent = { Secondly = 15 }
ram_usage_bytes = { Daily = 1 }
```

Options for time periods are `Millis`, `Secondly`, `Minutely`, `Hourly`, `Daily`, and `Weekly`. `{ Hourly = 2 }` 
corresponds to every two hours, not twice an hour.


### Dynamically defined metrics

The config can be updated to look for metric values at specific keys in Redis without redeploying. These can be PubSub
or polling key-value entries. For example:

```toml
[metrics.dynamic]
dynamic_metric = { read_type = { Poll = { Minutely = 15 } }, key = "/user/dynamicmetrics/1", dtype = "Float" }
dynamic_pubsub_metric = { read_type = "PubSub", key = "/user/dynamicmetrics/2", dtype = "Float" }
```

The `dtype` parameter represents how to parse the information obtained from Redis.

### Deployment

To use the systemd service as-is, a config should be created at `/etc/bec_log_ingestor.toml`, following the example in `install/example_config.toml`. For SLS deployments this is managed in puppet already.

Published RPM packages are signed by PGP, the corresponding public key is:

```
-----BEGIN PGP PUBLIC KEY BLOCK-----

mDMEaONxxBYJKwYBBAHaRw8BAQdA/Y2oM4wF9kWCHh871tRVkpXD43Kwcv+4hawF
KXSU8wW0NkJFQyBUZWFtIChCRUMgUlBNIHNpZ25pbmcga2V5KSA8YmVjX2NpX3N0
YWdpbmdAcHNpLmNoPoiWBBMWCgA+FiEEzj2vq2b33XAPoA97EO1Yp4CfPg4FAmjj
ccQCGwMFCQHhM4AFCwkIBwIGFQoJCAsCBBYCAwECHgECF4AACgkQEO1Yp4CfPg5H
SQEAxH518nANNuBhiNaWQrcfG0M8MsNv0MPmw8KIu2av3wQBAOTnnx+KSMz3G3FV
pXWNXqodA3hevPmlEGK3p9246DQHuDgEaONxxBIKKwYBBAGXVQEFAQEHQCD0FdIe
KMARufv8f1q1UpHe/VezH5ws0FWyea6b7ow5AwEIB4h+BBgWCgAmFiEEzj2vq2b3
3XAPoA97EO1Yp4CfPg4FAmjjccQCGwwFCQHhM4AACgkQEO1Yp4CfPg4mkwD9GR4+
hFUtIZhCUdQB3ttwcW7TRKF98zthyA+LU+/YDsMBAMXjE/ZlZVgTmKk6tMamLF1R
cX0DclDjhPJgOxT6NZwB
=ATUU
-----END PGP PUBLIC KEY BLOCK-----
```
