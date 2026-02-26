# [1.9.0](https://github.com/bec-project/bec_log_ingestor/compare/v1.8.0...v1.9.0) (2026-02-26)


### Features

* add hostname to all metric labels ([2b7ffa1](https://github.com/bec-project/bec_log_ingestor/commit/2b7ffa1470f02260f03d81df8dc94866b7a9defe))

# [1.8.0](https://github.com/bec-project/bec_log_ingestor/compare/v1.7.1...v1.8.0) (2026-02-24)


### Bug Fixes

* enable integration tests only on x86_64 ([dd269d3](https://github.com/bec-project/bec_log_ingestor/commit/dd269d3e4721f8cc254acd743b03d1e4f5579cf8))


### Features

* stop flag ([81f59e7](https://github.com/bec-project/bec_log_ingestor/commit/81f59e799d7487f39171fd8ef8d3c9d07f8299ef))

## [1.7.1](https://github.com/bec-project/bec_log_ingestor/compare/v1.7.0...v1.7.1) (2026-02-20)


### Bug Fixes

* add test for definitions ([01b6f5b](https://github.com/bec-project/bec_log_ingestor/commit/01b6f5b06bfa0a658fb60bc007685155d1990160))
* rename main loop ([f9ab20d](https://github.com/bec-project/bec_log_ingestor/commit/f9ab20de704c09ec1f1fa252f0a88dd34203f12e))

# [1.7.0](https://github.com/bec-project/bec_log_ingestor/compare/v1.6.0...v1.7.0) (2026-02-20)


### Features

* add dynamic metrics to config ([74ed870](https://github.com/bec-project/bec_log_ingestor/commit/74ed8708bd422a32e6d81264fd31c23f83cbf9a7))
* polling dynamic metric ([4e0493d](https://github.com/bec-project/bec_log_ingestor/commit/4e0493d46c9dfb95a84a27c6a865f75373ed5f9f))
* propagate pubsub metrics to mimir ([55315b6](https://github.com/bec-project/bec_log_ingestor/commit/55315b6f9d18166e329f8c45a03b268c06179c9b))
* refactor task spawner for dynamic metrics ([9dd4406](https://github.com/bec-project/bec_log_ingestor/commit/9dd4406d83527fe03348da967da4225fb26d596a))
* update redis and use async client ([22b756b](https://github.com/bec-project/bec_log_ingestor/commit/22b756b5023c0bff20eebdef0e21c7e2695510c8))

# [1.6.0](https://github.com/bec-project/bec_log_ingestor/compare/v1.5.3...v1.6.0) (2026-02-09)


### Features

* add model for service metric message ([b3bf639](https://github.com/bec-project/bec_log_ingestor/commit/b3bf63923d22d3f03261c4678e886ce034e922c0))
* add reactive error handling to metrics ([912832c](https://github.com/bec-project/bec_log_ingestor/commit/912832ca59d292cdd21f8e020afe3b5da578ada4))
* allow configuring a default interval per metric ([5ac1333](https://github.com/bec-project/bec_log_ingestor/commit/5ac1333b372a0b561583a49ecf8a232c37662dcf))
* propagate version metrics to mimir ([13bbbde](https://github.com/bec-project/bec_log_ingestor/commit/13bbbde0f2d701e3936b5e179f3e5e4580f84527))

## [1.5.3](https://github.com/bec-project/bec_log_ingestor/compare/v1.5.2...v1.5.3) (2026-01-16)


### Bug Fixes

* add detail for failed version command log ([7423153](https://github.com/bec-project/bec_log_ingestor/commit/7423153cf7309381a25937341325e4669dac5a7b))


### Performance Improvements

* box leak config to avoid clones ([2e0309b](https://github.com/bec-project/bec_log_ingestor/commit/2e0309b56f75756bcede885d9ee58abd63520c39))

## [1.5.2](https://github.com/bec-project/bec_log_ingestor/compare/v1.5.1...v1.5.2) (2026-01-15)


### Bug Fixes

* build env to use cmake for aws-lc-sys ([d676fc5](https://github.com/bec-project/bec_log_ingestor/commit/d676fc593bbf6f0a6f984cfbce4223ef22cd8ce0))

## [1.5.1](https://github.com/bec-project/bec_log_ingestor/compare/v1.5.0...v1.5.1) (2026-01-15)


### Bug Fixes

* install cross ([142254f](https://github.com/bec-project/bec_log_ingestor/commit/142254fb5cdea2cc91bd8bcc08e874af30640118))

# [1.5.0](https://github.com/bec-project/bec_log_ingestor/compare/v1.4.2...v1.5.0) (2026-01-14)


### Bug Fixes

* aarch build deps ([cd91aed](https://github.com/bec-project/bec_log_ingestor/commit/cd91aedab133e72a2eaf415fbde650adc2110854))
* add other targets ([2344cf1](https://github.com/bec-project/bec_log_ingestor/commit/2344cf14041673e708b93835d231cdcd8c523db9))
* improve build rs ([cd32b50](https://github.com/bec-project/bec_log_ingestor/commit/cd32b50480569b2495c27889e560b5a8ebd6c659))
* remove failing apple build for now ([aa7ae64](https://github.com/bec-project/bec_log_ingestor/commit/aa7ae64b588c1d76c7e19e501463f0bfbf9e5a9b))
* use updated reqwest with rustls ([993a3ed](https://github.com/bec-project/bec_log_ingestor/commit/993a3ede7b8c29b8f33edf6094fc869932fb0e15))


### Features

* add async framework for pushing metrics ([2277ca6](https://github.com/bec-project/bec_log_ingestor/commit/2277ca6ca7a9bb63faf68463e77caeff49abe233))
* add metric for deployed versions ([b85b1b7](https://github.com/bec-project/bec_log_ingestor/commit/b85b1b7dd5644870bb2419f2323566bf222dcfc9))
* add watchdog to restart failed metrics ([f971fe7](https://github.com/bec-project/bec_log_ingestor/commit/f971fe768afd5cde1f3f2fe0f62463682bfdc080))
* exit with failure after 3 tries communicating with mimir ([a3614d8](https://github.com/bec-project/bec_log_ingestor/commit/a3614d89db745a9408ca0f08690b87e6640e51c7))

## [1.4.2](https://github.com/bec-project/bec_log_ingestor/compare/v1.4.1...v1.4.2) (2026-01-09)


### Bug Fixes

* restart systemd service in RPM post-install hook ([fe0a64d](https://github.com/bec-project/bec_log_ingestor/commit/fe0a64d5b7157b3d3ed6b424d70be4f85f059d5e))

## [1.4.1](https://github.com/bec-project/bec_log_ingestor/compare/v1.4.0...v1.4.1) (2025-12-09)


### Bug Fixes

* loki json format ([22d1057](https://github.com/bec-project/bec_log_ingestor/commit/22d105727b7aa65e2f6d05ac7438c9b7f4841ad0))

# [1.4.0](https://github.com/bec-project/bec_log_ingestor/compare/v1.3.6...v1.4.0) (2025-12-05)


### Features

* replace elastic with loki ([cffcc60](https://github.com/bec-project/bec_log_ingestor/commit/cffcc60cc0cabdfffa6f99b27acdb0d52c6e9ad1))

## [1.3.6](https://github.com/bec-project/bec_log_ingestor/compare/v1.3.5...v1.3.6) (2025-11-14)


### Bug Fixes

* better logic for retrying redis connection ([dd36fa0](https://github.com/bec-project/bec_log_ingestor/commit/dd36fa063fa4605c7fc0b1b0e7eee51ddb5edaee))
* check if redis logging endpoint exists ([05c31ed](https://github.com/bec-project/bec_log_ingestor/commit/05c31ed36bae307f5f095098ce33ae532023dab7))

## [1.3.5](https://github.com/bec-project/bec_log_ingestor/compare/v1.3.4...v1.3.5) (2025-10-14)


### Bug Fixes

* update service to load credential ([05aeaa7](https://github.com/bec-project/bec_log_ingestor/commit/05aeaa70b062a0f01aad8a7c1b20ae0d533bc3be))

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
