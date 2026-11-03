# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---
## [1.0.0](https://github.com/chikof/forgeconf/compare/v0.5.0..v1.0.0) - 2026-11-03

### Bug Fixes

- **(macros)** show default expressions in help as written - ([7958c80](https://github.com/chikof/forgeconf/commit/7958c801917cad5288d6e9e8ff42a422368fa2bd)) - chiko
- debloat pending todo - ([09b1b4a](https://github.com/chikof/forgeconf/commit/09b1b4a93896e098b26e46ce6b07625964514c94)) - chiko
- supress clippy warnings - ([6759614](https://github.com/chikof/forgeconf/commit/675961414f9a7f5e930e9b46a0f341a2efff9913)) - chiko

### Documentation

- **(benchmarks)** record versions and the JSON limiter measurement - ([b7eb62b](https://github.com/chikof/forgeconf/commit/b7eb62b58da1a702ed718ffe4e636a96dadcb846)) - chiko
- **(benchmarks)** record results after the performance work - ([c622f55](https://github.com/chikof/forgeconf/commit/c622f5542a497d74bb88655cf5daa0d4006464cc)) - chiko
- **(benchmarks)** record results with inline strings - ([018da70](https://github.com/chikof/forgeconf/commit/018da70739f10cd20e629b0874f1b735e06b7198)) - chiko
- **(benchmarks)** refresh one-job build times and the layered subtitle - ([3794f72](https://github.com/chikof/forgeconf/commit/3794f7292f361a2c375c26d7f9a6c0c64680c1bb)) - chiko
- add performance tips and refresh benchmark charts - ([49b32e9](https://github.com/chikof/forgeconf/commit/49b32e9fd3b6b290a36554be7c91877947311b5f)) - chiko

### Features

- **(cli)** [**breaking**] parse command-line flags from the config shape with generated help - ([8e594ee](https://github.com/chikof/forgeconf/commit/8e594ee68a615b0f1e22b4243ea696a89934557c)) - chiko
- **(core)** add DotenvSource behind dotenv feature flag - ([90d8cd7](https://github.com/chikof/forgeconf/commit/90d8cd7d8d37542876d2955488eb5c787cc64a63)) - chiko
- **(core)** add WatchSource for notify-based hot-reload - ([78fe412](https://github.com/chikof/forgeconf/commit/78fe4123661a061524a1d67600e5c5fd2c48dbc2)) - chiko
- **(env)** [**breaking**] load environment variables and .env files from the config shape - ([7826a59](https://github.com/chikof/forgeconf/commit/7826a59713ac1bac5fe5c85aebb23a8583a2aa1b)) - chiko
- **(format)** [**breaking**] parse JSON in-house with line and column errors - ([59c9dd8](https://github.com/chikof/forgeconf/commit/59c9dd805a30853d678f4cbc750eb04c0f35ecc0)) - chiko
- **(format)** [**breaking**] parse a documented YAML subset in-house - ([fd5d72f](https://github.com/chikof/forgeconf/commit/fd5d72f8938f8ac3c7c68fc5783ec542b0e6a016)) - chiko
- **(format)** [**breaking**] parse TOML 1.0 in-house - ([7e98dd5](https://github.com/chikof/forgeconf/commit/7e98dd592c39b618c96753841f7c0ebff746655e)) - chiko
- **(macros)** split render into modules; add prefix-aware clap trait - ([5b3d114](https://github.com/chikof/forgeconf/commit/5b3d114fcc0b1683a560e2d4af509ddc74397bb9)) - chiko
- **(macros)** add env_prefix attribute to forgeconf macro - ([55511f4](https://github.com/chikof/forgeconf/commit/55511f4a1be57c85c0144d8ce6120385d8125b81)) - chiko
- **(macros)** add deprecated field attribute with runtime warning - ([7b655fa](https://github.com/chikof/forgeconf/commit/7b655fa76dac782d4144a2473c6dcec59999e800)) - chiko
- **(macros)** add sensitive field attribute with masked Debug output - ([daaa630](https://github.com/chikof/forgeconf/commit/daaa6309b52549af4582467ee9171a579bee1b9f)) - chiko
- **(macros)** add alias field attribute for multi-key config lookup - ([3a68495](https://github.com/chikof/forgeconf/commit/3a6849515671cca48ad195245fc92fba17fb6909)) - chiko
- **(watch)** [**breaking**] poll watched files and reload without notify - ([abd283e](https://github.com/chikof/forgeconf/commit/abd283e64006779dfb8bd4eb8a43def35eb7b8c8)) - chiko
- add clap support for cli - ([6709cdf](https://github.com/chikof/forgeconf/commit/6709cdf587657fbe217d6b202809137ed3d7329d)) - chiko
- [**breaking**] load maps, sets, paths, durations and addresses; panic-free validators - ([aacc346](https://github.com/chikof/forgeconf/commit/aacc346b6c2a32b8b11b3df6c6c9c943860fcbd6)) - chiko
- [**breaking**] replace #[forgeconf] with #[derive(Config)] and Loader - ([5adf0be](https://github.com/chikof/forgeconf/commit/5adf0beae20544d645146a7c64aa1b7d6b0832ac)) - chiko
- [**breaking**] store short strings inline with the new Text type - ([9f804be](https://github.com/chikof/forgeconf/commit/9f804bea8154222c811f99daaaa0e150ba070f0b)) - chiko

### Miscellaneous Chores

- fix clippy warning - ([2aed23b](https://github.com/chikof/forgeconf/commit/2aed23b7a83a730def4f6672647cbbf0e22797a4)) - chiko
- adopt devenv and project tooling - ([779411d](https://github.com/chikof/forgeconf/commit/779411def548040b538ade6ea8dbe9ab29324ee4)) - chiko
- fix the spacing of unscoped breaking entries in the changelog - ([31a6972](https://github.com/chikof/forgeconf/commit/31a69725b0c7c2d3157ea8a069ffaf2a76fad08b)) - chiko

### Performance

- **(convert)** move scalars and children out of the owned tree - ([aef921d](https://github.com/chikof/forgeconf/commit/aef921dc137daaed13d59787c65a8da210a2699e)) - chiko
- **(convert)** parse scalars before trimming and match bool words by length - ([9cf0c27](https://github.com/chikof/forgeconf/commit/9cf0c27e3f02a0f57eeb8ce278fb3358bc5c6d63)) - chiko
- **(env)** walk leaves and deprecations without allocating - ([fbc55f5](https://github.com/chikof/forgeconf/commit/fbc55f58fbb17a1d53e65e714beb7336937572be)) - chiko
- **(error)** write Display directly into the formatter - ([c529e1a](https://github.com/chikof/forgeconf/commit/c529e1a869073b588240b03d1301c0305d01e759)) - chiko
- **(error)** format origin and counts without fmt::Arguments - ([4166b24](https://github.com/chikof/forgeconf/commit/4166b246c4343d101d8b9b3b2f01ab8d13c0062b)) - chiko
- **(error)** push single chars and digits without write_str - ([1b327eb](https://github.com/chikof/forgeconf/commit/1b327eb68e8037438c563083c6173885ad6e0207)) - chiko
- **(errors)** mark parser error builders cold - ([8f91258](https://github.com/chikof/forgeconf/commit/8f91258b02f1c853ca5b62fb99b4b2e6c8a21211)) - chiko
- **(format)** decode ASCII bytes directly in the cursor - ([9d9c202](https://github.com/chikof/forgeconf/commit/9d9c202c13c58c67217fc922f1074f3ee946ecc7)) - chiko
- **(format)** name the file origin without formatting the path - ([794b61f](https://github.com/chikof/forgeconf/commit/794b61fcd627ec104553fd6825fcbdc92d9d7f6d)) - chiko
- **(inline)** inline small accessors and derive helpers across the crate boundary - ([bb666fe](https://github.com/chikof/forgeconf/commit/bb666fe1a372c3f3efe1024d72dfc8645b4da72c)) - chiko
- **(json)** bulk-build object tables and scan strings and numbers by slice - ([db9593b](https://github.com/chikof/forgeconf/commit/db9593b1cff060ae17e1490951084db2ad5f8b97)) - chiko
- **(json)** insert small objects directly and scan strings eight bytes at a time - ([7cfdf6a](https://github.com/chikof/forgeconf/commit/7cfdf6a917d03574c6683a2d99df1ce3d793449b)) - chiko
- **(json)** share one member stack across all objects of a document - ([6390d4d](https://github.com/chikof/forgeconf/commit/6390d4d5132bad4e5ecd6f17c882861b71319491)) - chiko
- **(json)** advance string and number text without line tracking - ([3a67420](https://github.com/chikof/forgeconf/commit/3a674208a6996818eaa2daaa4870ae80ef7960be)) - chiko
- **(json)** step over punctuation and blanks without char decoding - ([87a0590](https://github.com/chikof/forgeconf/commit/87a0590f17f39eef16e2d8310f6714c438e8c589)) - chiko
- **(json)** presize the member stack from the input length - ([a60e403](https://github.com/chikof/forgeconf/commit/a60e4035499183002043a3eb498240b91ad3e1e0)) - chiko
- **(json)** split the escape-free string path from the general one - ([fed83e5](https://github.com/chikof/forgeconf/commit/fed83e5f718243a0d17da1775278d06c5526b375)) - chiko
- **(json)** move the column by byte count over ASCII text - ([ab1fe69](https://github.com/chikof/forgeconf/commit/ab1fe694fbd2531ce7c953eae3386da65ff04140)) - chiko
- **(json)** detect repeated keys after the bulk build - ([f5d6128](https://github.com/chikof/forgeconf/commit/f5d6128a3a5ac8c1057afbea520d13447280885f)) - chiko
- **(loader)** move into an empty base, skip plain fields in the deprecation walk - ([73399d5](https://github.com/chikof/forgeconf/commit/73399d5b56108210363dc17978a8f3c0417b412c)) - chiko
- **(macros)** share the borrowed field reader across fields without validators - ([1780a5c](https://github.com/chikof/forgeconf/commit/1780a5c3b6997c9ce4d5871925e4ae0d7813fbb3)) - chiko
- **(macros)** read unvalidated fields without the validator branches - ([e6ce3cb](https://github.com/chikof/forgeconf/commit/e6ce3cb4024c595a9dc16518c6f1c26bf4132207)) - chiko
- **(macros)** record field errors inside the shared reader - ([a19b575](https://github.com/chikof/forgeconf/commit/a19b575fdaad72df958ed1c3c6cdff7ea6c759d7)) - chiko
- **(macros)** instantiate validated field readers once per type - ([9d0380c](https://github.com/chikof/forgeconf/commit/9d0380cc0f32bb228e71f1131000eccf9d21703a)) - chiko
- **(suggest)** reuse one row buffer and prune by length and bound - ([724a8d7](https://github.com/chikof/forgeconf/commit/724a8d78bc02c174d644bfb7332a3c19118569d7)) - chiko
- **(suggest)** ASCII fast path and table-free check for one edit - ([f18d4ca](https://github.com/chikof/forgeconf/commit/f18d4caf9b0a22d078e1f436f1f264fd633fb9f5)) - chiko
- **(toml)** scan strings, keys and comments by byte runs - ([cf3cbf2](https://github.com/chikof/forgeconf/commit/cf3cbf25549de6bf47fb3537eb3a91c0cb267499)) - chiko
- **(toml)** fill the current table directly and keep leaves out of the builder tree - ([04fb4f1](https://github.com/chikof/forgeconf/commit/04fb4f152c9dce1e47931d15c98e1ca0aff2d456)) - chiko
- **(toml)** take plain decimal integers verbatim - ([2a5d64c](https://github.com/chikof/forgeconf/commit/2a5d64ca20a658e0551f72c223fe8b67cb9af5de)) - chiko
- **(toml)** test single bytes instead of matching prefixes - ([e36122e](https://github.com/chikof/forgeconf/commit/e36122e390ee82ff4efb92a05d53a16ee42d4b26)) - chiko
- **(toml)** scan keys, tokens and strings with byte tables and flat column steps - ([d1da97c](https://github.com/chikof/forgeconf/commit/d1da97c2abd133da7f62e218f238b2be1ae9381b)) - chiko
- **(toml)** build small tables in a map directly and keep the sorted list for large ones - ([6038a0b](https://github.com/chikof/forgeconf/commit/6038a0bc9f41430edd59c9c51d511b81360560dd)) - chiko
- **(yaml)** build single-line plain scalars from the borrowed slice - ([4fa10b1](https://github.com/chikof/forgeconf/commit/4fa10b196fc08b060db843a52985b0c3717c164e)) - chiko

### Refactoring

- **(format)** pass the origin source to every parser - ([836554e](https://github.com/chikof/forgeconf/commit/836554e9b61bd4a7f18b7bff8b97ddf5b5f71af7)) - chiko
- **(format)** compile the cursor only with a format enabled - ([0031545](https://github.com/chikof/forgeconf/commit/0031545beb044245d3af76686c7b872c401774be)) - chiko
- **(source)** share the settable-leaf walk for env and CLI - ([f9cc4f0](https://github.com/chikof/forgeconf/commit/f9cc4f0200482984e94608c4c99603d7bee7d91e)) - chiko
- [**breaking**] merge forgeconf_core into forgeconf and drop dead dependencies - ([98e5a99](https://github.com/chikof/forgeconf/commit/98e5a997957fedce671092679081d9cc768f40ce)) - chiko
- [**breaking**] rebuild errors, nodes and FromNode on a std-only core - ([b0c0898](https://github.com/chikof/forgeconf/commit/b0c089871cd057d273a8119eec862dca6f3b64d5)) - chiko
- satisfy the library lint table - ([764877f](https://github.com/chikof/forgeconf/commit/764877fc2ff9c0b8c70772650c4c2d8848d07de3)) - chiko

### Style

- **(format)** wrap a cursor doc comment - ([b9ba213](https://github.com/chikof/forgeconf/commit/b9ba2133b76c9da17bf16c1eba29684317cd0ff9)) - chiko

### Tests

- **(benchmarks)** count instructions per scenario with callgrind - ([2339c7f](https://github.com/chikof/forgeconf/commit/2339c7fac45d6c5a0ba67a70ec89460bcf586cd2)) - chiko
- **(benchmarks)** give confique the same nine env variables as forgeconf - ([bddce7d](https://github.com/chikof/forgeconf/commit/bddce7ddc0efd9f997160c97069acbc6bbb6f603)) - chiko
- **(benchmarks)** draw the results as SVG charts - ([18a51bc](https://github.com/chikof/forgeconf/commit/18a51bc60122cec2476cd7c6afa0dd14120d5bed)) - chiko
- **(text)** check ordering against str around the inline limit - ([45c622b](https://github.com/chikof/forgeconf/commit/45c622b946a2dd7e7dbc9daec8735df9bbabd3f7)) - chiko
- **(yaml)** pin the cases the subset rejects as Unsupported - ([fd80984](https://github.com/chikof/forgeconf/commit/fd80984873b36b55b61a0d971663b72a9417e1bd)) - chiko
- run MSRV, feature powerset and TOML/JSON/YAML conformance suites via test-all - ([4c3a840](https://github.com/chikof/forgeconf/commit/4c3a84056d5285b44e2c8fa175645abef7496258)) - chiko
- benchmark forgeconf against config, figment, confique, serde and clap - ([ad32287](https://github.com/chikof/forgeconf/commit/ad322874d758585a68ca4cb401647eea0ec75592)) - chiko
- avoid a spelling-checker false positive - ([06f8720](https://github.com/chikof/forgeconf/commit/06f87202b2570fe04cded54752f22c5d7591fb6d)) - chiko
- avoid a spelling-checker false positive - ([0bd675d](https://github.com/chikof/forgeconf/commit/0bd675d407fe4b8487b43c1321a6e96cfe0bf064)) - chiko

### Build

- **(deps)** bump toml from 1.0.6+spec-1.1.0 to 1.1.2+spec-1.1.0 ([#36](https://github.com/chikof/forgeconf/issues/36)) - ([9344f3a](https://github.com/chikof/forgeconf/commit/9344f3ae96015cf6708dd5bfe98925a045eb9e11)) - dependabot[bot]
- **(deps)** bump clap from 4.5.54 to 4.6.1 ([#38](https://github.com/chikof/forgeconf/issues/38)) - ([1cff7be](https://github.com/chikof/forgeconf/commit/1cff7be4e1b6301ba0dfd116a6a13f821d735904)) - dependabot[bot]
- **(deps)** bump config from 0.15.21 to 0.15.23 ([#37](https://github.com/chikof/forgeconf/issues/37)) - ([e0fb2ff](https://github.com/chikof/forgeconf/commit/e0fb2ff7b480d79413a43b8bcaca059d091e8d77)) - dependabot[bot]
- adopt the library lint table workspace-wide - ([f425d98](https://github.com/chikof/forgeconf/commit/f425d98c7ff615802ae2d1b5ed8877d72dc80893)) - chiko
- compile shape-walk and cursor helpers only where used - ([bfcf286](https://github.com/chikof/forgeconf/commit/bfcf2865de85bf9fbee9b516d82a4845b0c80781)) - chiko

### Ci

- run the check gate, MSRV and feature matrix - ([35bd71c](https://github.com/chikof/forgeconf/commit/35bd71c4d0bc943e85537ceb47009dff1c171f5a)) - chiko
- retry publishing forgeconf while the macro crate propagates - ([b60bee9](https://github.com/chikof/forgeconf/commit/b60bee956bc179d27068af4762eed11005dc1c48)) - chiko

---
## [0.5.0](https://github.com/chikof/forgeconf/compare/v0.4.1..v0.5.0) - 2026-04-21

### Bug Fixes

- **(ci)** allow unused_assignments cause of miette - ([f80a54e](https://github.com/chikof/forgeconf/commit/f80a54e807c88b5937f2c722a7489ef3c9c58df7)) - Chiko
- **(node)** trim whitespace before scalar parsing - ([9e74380](https://github.com/chikof/forgeconf/commit/9e74380a40055f18ff134691608cf0870f53d545)) - Chiko
- had to manually specify unsafe when setting a var with the change of rust edition 2024 - ([2cfb914](https://github.com/chikof/forgeconf/commit/2cfb91488e09945e99eba90c995d38ea043c80d5)) - Chiko
- not nightly on nix i guess - ([d788e33](https://github.com/chikof/forgeconf/commit/d788e33d429f5d9d655a6aec613b45ae156a3171)) - Chiko

### Features

- **(loader)** [**breaking**] auto-load config files and require explicit nested - ([d961871](https://github.com/chikof/forgeconf/commit/d96187139a7afffce7ff5e66948cbf4a9860cef2)) - Chiko
- allow dynamic path expressions - ([334157f](https://github.com/chikof/forgeconf/commit/334157fdbd41dec91f210c0ad829e7344533cbd1)) - Chiko

### Miscellaneous Chores

- **(cliff)** dont include release commit on releases - ([29ad583](https://github.com/chikof/forgeconf/commit/29ad583b458a550be8e276bbeef243487baf9299)) - Chiko
- somehow the format got reverted? - ([1e19683](https://github.com/chikof/forgeconf/commit/1e19683e059e7bb908d0e6e014dfd5ef58e55a92)) - Chiko
- formatting - ([420475c](https://github.com/chikof/forgeconf/commit/420475caca0ab470eec4846b5e59b30cc1cbb9e7)) - Chiko
- update readme - ([a2d64ef](https://github.com/chikof/forgeconf/commit/a2d64ef55d747a702f0bbcf0fae8802267504fa1)) - Chiko
- clean release script - ([a9509f6](https://github.com/chikof/forgeconf/commit/a9509f6d3b9087fd6560c8f7b535563d5e02b41f)) - Chiko

### Style

- run `cargo fmt` (my nvim config wont let me format) - ([a95e131](https://github.com/chikof/forgeconf/commit/a95e13166f878da1d44859e8a8786292281ccc58)) - Chiko

### Build

- **(deps)** bump regex from 1.12.2 to 1.12.3 ([#20](https://github.com/chikof/forgeconf/issues/20)) - ([296cabd](https://github.com/chikof/forgeconf/commit/296cabdb9ecad48c342d4e9bba6a9be0cc4f9f8b)) - dependabot[bot]
- **(deps)** bump tempfile from 3.24.0 to 3.25.0 ([#22](https://github.com/chikof/forgeconf/issues/22)) - ([0c4fb7e](https://github.com/chikof/forgeconf/commit/0c4fb7ed47caa54daf865677405d6ba15f2947b1)) - dependabot[bot]
- **(deps)** bump syn from 2.0.114 to 2.0.116 ([#23](https://github.com/chikof/forgeconf/issues/23)) - ([a5f2c91](https://github.com/chikof/forgeconf/commit/a5f2c912804eee94df0b712996c9e43df6d177c4)) - dependabot[bot]
- **(deps)** bump criterion from 0.8.1 to 0.8.2 ([#21](https://github.com/chikof/forgeconf/issues/21)) - ([b4b2e01](https://github.com/chikof/forgeconf/commit/b4b2e01707834cea02c19978746256b3a06814e9)) - dependabot[bot]
- **(deps)** bump syn from 2.0.116 to 2.0.117 ([#29](https://github.com/chikof/forgeconf/issues/29)) - ([2a68178](https://github.com/chikof/forgeconf/commit/2a68178af90e14780ad5c83858f61900d5e91240)) - dependabot[bot]
- **(deps)** bump config from 0.15.19 to 0.15.21 ([#34](https://github.com/chikof/forgeconf/issues/34)) - ([e92ceb2](https://github.com/chikof/forgeconf/commit/e92ceb294d628eeaa65ec168ce2f63c74015b6a3)) - dependabot[bot]
- **(deps)** bump tempfile from 3.25.0 to 3.27.0 ([#33](https://github.com/chikof/forgeconf/issues/33)) - ([d0e2895](https://github.com/chikof/forgeconf/commit/d0e2895c7afba4e5ce9707c4390d053a7b6bd070)) - dependabot[bot]
- **(deps)** bump toml from 0.9.11+spec-1.1.0 to 1.0.6+spec-1.1.0 ([#32](https://github.com/chikof/forgeconf/issues/32)) - ([fd10176](https://github.com/chikof/forgeconf/commit/fd10176986c1cf768f15dfad67d43b7c4100c7dd)) - dependabot[bot]
- **(deps)** bump quote from 1.0.44 to 1.0.45 ([#31](https://github.com/chikof/forgeconf/issues/31)) - ([e24ecec](https://github.com/chikof/forgeconf/commit/e24ecec138d86efcbc83f835925b7a99d921443b)) - dependabot[bot]

---
## [0.4.1](https://github.com/chikof/forgeconf/compare/v0.4.0..v0.4.1) - 2026-02-08

### Bug Fixes

- suppress unexpected_cfgs warnings on generated parse methods - ([3a72518](https://github.com/chikof/forgeconf/commit/3a7251880d9763271f072ea42f6f1e4d49e2d260)) - Chiko

### Miscellaneous Chores

- forgot to format - ([1067c13](https://github.com/chikof/forgeconf/commit/1067c137bdd0051d66eb8367c062b1e0c094d761)) - Chiko

---
## [0.4.0](https://github.com/chikof/forgeconf/compare/v0.3.0..v0.4.0) - 2026-02-08

### Bug Fixes

- [**breaking**] remove cfg attributes from macro to prevent check-cfg warnings - ([25df425](https://github.com/chikof/forgeconf/commit/25df425caaa1f15a7f6d85a4a5acbe8a88b38db5)) - Chiko

### Ci

- **(release)** only generate changelog for the current release - ([efe88d0](https://github.com/chikof/forgeconf/commit/efe88d09a6c66edc8c630085cbf4dc96684dffac)) - Chiko

---
## [0.3.0](https://github.com/chikof/forgeconf/compare/v0.2.0..v0.3.0) - 2026-01-28

### Bug Fixes

- forgot to format - ([f40964a](https://github.com/chikof/forgeconf/commit/f40964a08c40b887d2a16352cf77b76472c384f2)) - chikof
- forgot to format, again - ([57a018c](https://github.com/chikof/forgeconf/commit/57a018c594ec0a52846dcf02c1fe810737b90955)) - chikof

### Features

- **(miette)** add an optional feature for fancy errors - ([e6c0b9e](https://github.com/chikof/forgeconf/commit/e6c0b9e63c805f3033e9d80f95702133c9ad4d45)) - chikof
- **(parse)** parsing configs from text is now possible - ([c04bd7f](https://github.com/chikof/forgeconf/commit/c04bd7f1adde6d1f8d950bb9b77a0b3aee0cad82)) - chikof

### Miscellaneous Chores

- add benchmarks - ([ee93311](https://github.com/chikof/forgeconf/commit/ee9331124434e338f825165147b9e5ea06a7ba76)) - chikof
- formatting - ([1e67c3a](https://github.com/chikof/forgeconf/commit/1e67c3a522ac7158874e44d5c023ef789dd29988)) - Chiko

### Build

- **(deps)** bump thiserror from 2.0.12 to 2.0.17 ([#7](https://github.com/chikof/forgeconf/issues/7)) - ([a52d7a9](https://github.com/chikof/forgeconf/commit/a52d7a9b6f8c87169e2835c7ebf1686078f66389)) - dependabot[bot]
- **(deps)** bump quote from 1.0.40 to 1.0.42 ([#6](https://github.com/chikof/forgeconf/issues/6)) - ([ab3c48e](https://github.com/chikof/forgeconf/commit/ab3c48e60e8374092f45a18530a9c9251ae35a76)) - dependabot[bot]
- **(deps)** bump proc-macro2 from 1.0.103 to 1.0.104 ([#11](https://github.com/chikof/forgeconf/issues/11)) - ([4c21219](https://github.com/chikof/forgeconf/commit/4c21219860ed8c79075d3752a36f39eeab97ecfc)) - dependabot[bot]
- **(deps)** bump tempfile from 3.23.0 to 3.24.0 ([#10](https://github.com/chikof/forgeconf/issues/10)) - ([22afd78](https://github.com/chikof/forgeconf/commit/22afd78150a6973c47c06dfbfc899863cff8423f)) - dependabot[bot]
- **(deps)** bump yaml-rust2 from 0.10.4 to 0.11.0 ([#9](https://github.com/chikof/forgeconf/issues/9)) - ([071a77c](https://github.com/chikof/forgeconf/commit/071a77cc4625743156ad7190f8621a9a1785deb8)) - dependabot[bot]
- **(deps)** bump toml from 0.9.8 to 0.9.11+spec-1.1.0 ([#16](https://github.com/chikof/forgeconf/issues/16)) - ([cf1a507](https://github.com/chikof/forgeconf/commit/cf1a5077823970c84973d1429b467fdb0416456f)) - dependabot[bot]
- **(deps)** bump syn from 2.0.111 to 2.0.114 ([#15](https://github.com/chikof/forgeconf/issues/15)) - ([7aae2ea](https://github.com/chikof/forgeconf/commit/7aae2eaac2b06d0c214007c82546e14ed947bbea)) - dependabot[bot]
- **(deps)** bump proc-macro2 from 1.0.104 to 1.0.105 ([#14](https://github.com/chikof/forgeconf/issues/14)) - ([a2433d0](https://github.com/chikof/forgeconf/commit/a2433d0f177dcd415b8971a2ff59047995e7435a)) - dependabot[bot]
- **(deps)** bump proc-macro2 from 1.0.105 to 1.0.106 ([#19](https://github.com/chikof/forgeconf/issues/19)) - ([71d4e45](https://github.com/chikof/forgeconf/commit/71d4e45681d4d2f34611b64527e65362a73bce15)) - dependabot[bot]
- **(deps)** bump quote from 1.0.42 to 1.0.44 ([#18](https://github.com/chikof/forgeconf/issues/18)) - ([8498e41](https://github.com/chikof/forgeconf/commit/8498e418226db435dd8313727aeccd7024f252fc)) - dependabot[bot]
- **(deps)** bump thiserror from 2.0.17 to 2.0.18 ([#17](https://github.com/chikof/forgeconf/issues/17)) - ([65ec39e](https://github.com/chikof/forgeconf/commit/65ec39e228ead692f07f6be8eaf420e15e53185f)) - dependabot[bot]

---
## [0.2.0](https://github.com/chikof/forgeconf/compare/v0.1.0..v0.2.0) - 2025-11-30

### Bug Fixes

- clippy suggestion - ([0da11c0](https://github.com/chikof/forgeconf/commit/0da11c0c591508f52fa86f1154c4999818225e1c)) - chikof

### Features

- **(validators)** add built-in helpers and git-cliff notes - ([35db15b](https://github.com/chikof/forgeconf/commit/35db15b7eb9c129a7642fb20c8669cef2d7dcfc4)) - chikof
- couple of tests - ([ef92d13](https://github.com/chikof/forgeconf/commit/ef92d136c1cd1d9f66525f772104401401b1d7da)) - chikof

### Miscellaneous Chores

- forgot to run cargo fmt - ([be2337b](https://github.com/chikof/forgeconf/commit/be2337b1767c4be62f3bc556b955c77204f8094c)) - chikof

### Build

- **(deps)** bump toml from 0.8.23 to 0.9.8 ([#5](https://github.com/chikof/forgeconf/issues/5)) - ([5d60468](https://github.com/chikof/forgeconf/commit/5d604687f04878a2d7dfd861de1ec68242d66046)) - dependabot[bot]
- **(deps)** bump tempfile from 3.20.0 to 3.23.0 ([#4](https://github.com/chikof/forgeconf/issues/4)) - ([616cefd](https://github.com/chikof/forgeconf/commit/616cefd9c58bb78345463bf877db1a73d3684b94)) - dependabot[bot]
- **(deps)** bump yaml-rust2 from 0.10.3 to 0.10.4 ([#2](https://github.com/chikof/forgeconf/issues/2)) - ([7156024](https://github.com/chikof/forgeconf/commit/7156024d0b1aae60b8fe68436ebcf8c854704938)) - dependabot[bot]
- **(deps)** bump proc-macro2 from 1.0.95 to 1.0.103 ([#1](https://github.com/chikof/forgeconf/issues/1)) - ([ad16a23](https://github.com/chikof/forgeconf/commit/ad16a232411b7689d510451ce768fb278c48f542)) - dependabot[bot]
- **(deps)** bump syn from 2.0.104 to 2.0.111 ([#3](https://github.com/chikof/forgeconf/issues/3)) - ([6c41423](https://github.com/chikof/forgeconf/commit/6c4142364bc7a0f1b303cebef40ff5f9f0c64767)) - dependabot[bot]

---
## [0.1.0] - 2025-11-29

### Bug Fixes

- **(ci)** install rustfmt (nightly) - ([e3d6652](https://github.com/chikof/forgeconf/commit/e3d665202ab4b9ccedfd523c168b4ad2d2f4d3ea)) - chikof
- **(ci)** maybe nightly clippy - ([f440ac1](https://github.com/chikof/forgeconf/commit/f440ac179f00aebdb93cb66c889f4ef79a4dd425)) - chikof

### Features

- hello github - ([511af9e](https://github.com/chikof/forgeconf/commit/511af9efbb89866adbd7d9dbd3789351432792ed)) - chikof

