## [1.3.4](https://github.com/bec-project/bec_log_ingestor/compare/v1.3.3...v1.3.4) (2025-10-13)


### Bug Fixes

* make systemd service type exec ([cbce082](https://github.com/bec-project/bec_log_ingestor/commit/cbce08244313c5cd95174688780cbc3ca587c960))

## [1.3.3](https://github.com/bec-project/bec_log_ingestor/compare/v1.3.2...v1.3.3) (2025-10-13)


### Bug Fixes

* make user without home ([346401b](https://github.com/bec-project/bec_log_ingestor/commit/346401bf00ab03c566bc41c087322833603407c0))

## [1.3.2](https://github.com/bec-project/bec_log_ingestor/compare/v1.3.1...v1.3.2) (2025-10-13)


### Bug Fixes

* reduce verbosity ([1cd19f6](https://github.com/bec-project/bec_log_ingestor/commit/1cd19f698acb321d17d046ce4a3a57409beefb25))

## [1.3.1](https://github.com/bec-project/bec_log_ingestor/compare/v1.3.0...v1.3.1) (2025-10-12)


### Bug Fixes

* correct section for service keys ([7beb651](https://github.com/bec-project/bec_log_ingestor/commit/7beb651e7d661e2efed37d934c5bfa9c2a49d2d1))
* don't print secrets ([a98e0ff](https://github.com/bec-project/bec_log_ingestor/commit/a98e0ff06c7006914ad0f9f7c86d7137f004fb78))

# [1.3.0](https://github.com/bec-project/bec_log_ingestor/compare/v1.2.0...v1.3.0) (2025-10-07)


### Features

* add systemd service and user creation ([ca6f978](https://github.com/bec-project/bec_log_ingestor/commit/ca6f9782ba1e4fa8e6497247684568a823235aac))

# [1.2.0](https://github.com/bec-project/bec_log_ingestor/compare/v1.1.0...v1.2.0) (2025-10-07)


### Features

* add beamline name to config and doc ([7dc9a41](https://github.com/bec-project/bec_log_ingestor/commit/7dc9a4108cb6906ee530fd2687fab1e34481f173))

# [1.1.0](https://github.com/bec-project/bec_log_ingestor/compare/v1.0.3...v1.1.0) (2025-10-06)


### Features

* sign packaged RPM ([0f9fd23](https://github.com/bec-project/bec_log_ingestor/commit/0f9fd23c2a49ef0eb9b9f9a88a93944b2642f7c0))

## [1.0.3](https://github.com/bec-project/bec_log_ingestor/compare/v1.0.2...v1.0.3) (2025-08-27)


### Bug Fixes

* refresh repo to get version for rpm build ([a0957a9](https://github.com/bec-project/bec_log_ingestor/commit/a0957a9c580f13750eb2543686f261671642e18d))

## [1.0.2](https://github.com/bec-project/bec_log_ingestor/compare/v1.0.1...v1.0.2) (2025-08-27)


### Bug Fixes

* push RPM to PSI gitea ([518d001](https://github.com/bec-project/bec_log_ingestor/commit/518d0017e2e895445d53e4166f10651a1c7ca5c8))

## [1.0.1](https://github.com/bec-project/bec_log_ingestor/compare/v1.0.0...v1.0.1) (2025-08-26)


### Bug Fixes

* remove unneccessary import ([9c05d43](https://github.com/bec-project/bec_log_ingestor/commit/9c05d43f26b86de1f37dd3b946644f0e587e832c))

# 1.0.0 (2025-08-26)


### Bug Fixes

* adjust CI permissions ([c012d3c](https://github.com/bec-project/bec_log_ingestor/commit/c012d3c5eab16b279e9566c50a6aeff7451352da))
* don't check certs ([8cc640b](https://github.com/bec-project/bec_log_ingestor/commit/8cc640b7651ac44ec9f5cffb02c8613388c30032))
* make async consumer work ([b9912f1](https://github.com/bec-project/bec_log_ingestor/commit/b9912f12b060f8c4f13a028e9795a22147059dc9))
* remove unneccessary into() ([7c90c5c](https://github.com/bec-project/bec_log_ingestor/commit/7c90c5cdc14c67e5e964e2b6b8b49c979aba3a1a))
* unpack error directly ([4610094](https://github.com/bec-project/bec_log_ingestor/commit/4610094ea0d27fbd8fabdbeb391773c400ec6972))


### Features

* add blocking time to config ([3a119fa](https://github.com/bec-project/bec_log_ingestor/commit/3a119facee5d94c09fa0eea6edead7ce6d740bcb))
* add index to config ([b9041d2](https://github.com/bec-project/bec_log_ingestor/commit/b9041d2b637f3f39e03d97e8cb619d977f8827ee))
* bulk push to elastic ([33f1986](https://github.com/bec-project/bec_log_ingestor/commit/33f19866505d250e768ce7cbd41e8f1594087f71))
* convert timestamp to rfc3339 ([01614ec](https://github.com/bec-project/bec_log_ingestor/commit/01614ec430e1aa8915fe6e559d867db577b4e1ba))
* get config from file ([ce3d808](https://github.com/bec-project/bec_log_ingestor/commit/ce3d808a6df49c36d214b294c75a201582321ef5))
* option to use basic auth for elastic ([6b33900](https://github.com/bec-project/bec_log_ingestor/commit/6b33900af5d6d48aebc5a7b3e7ca029ddf089ef5))
* parse raw msgpack ([fa75cc7](https://github.com/bec-project/bec_log_ingestor/commit/fa75cc759e7ce63cf815083fce1e5fde6d75655f))
* read log messages from redis ([d0363c2](https://github.com/bec-project/bec_log_ingestor/commit/d0363c2422e5c9e3212e3174a35fbb9cc145f2ec))
* split to producer consumer ([e38984f](https://github.com/bec-project/bec_log_ingestor/commit/e38984fbacf0b975d201eb51d6dfde126c15fdbf))
* unpack to struct ([5d5c755](https://github.com/bec-project/bec_log_ingestor/commit/5d5c7557238e104d2d608a5ef49ca2b12f4b5653))
* use config in connection setups ([1a9ff5c](https://github.com/bec-project/bec_log_ingestor/commit/1a9ff5c42cc12d888305fef125bc594e561a83b9))
