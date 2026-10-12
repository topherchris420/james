# Standard license texts

`tools/notices` writes `THIRD-PARTY-NOTICES.txt` for every package. Most
crates publish their own license files, which are reproduced as they are. A
few publish none; for those, the notices reproduce the standard text of each
license the crate declares, from this folder.

The texts are copied unchanged from the `text/` folder of the
[SPDX License List data](https://github.com/spdx/license-list-data), release
v3.29.0 (commit `31ba1a50e5397e00a304dbadc76531740e89ee48`), which is
published under CC0-1.0. If a new dependency declares a license without a text
here, the notices tool stops with an error naming it; add `<SPDX-ID>.txt`
from the same source.
