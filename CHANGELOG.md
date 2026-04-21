# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

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

-  [**breaking**]remove cfg attributes from macro to prevent check-cfg warnings - ([25df425](https://github.com/chikof/forgeconf/commit/25df425caaa1f15a7f6d85a4a5acbe8a88b38db5)) - Chiko

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

