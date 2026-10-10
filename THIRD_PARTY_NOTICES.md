# Third-party notices

dbt-edith embeds the following libraries, unmodified, under `web/vendor/`.

| Library | Version | License | Copyright |
| --- | --- | --- | --- |
| [CodeMirror](https://github.com/codemirror/codemirror5), including the `sql`, `yaml`, `markdown`, `jinja2` modes and the `searchcursor` and `merge` addons | 5.65.21 | MIT | Marijn Haverbeke and others |
| [xterm.js](https://github.com/xtermjs/xterm.js) and `@xterm/addon-fit` | 5.5.0 and 0.10.0 | MIT | The xterm.js authors |
| [diff-match-patch](https://github.com/google/diff-match-patch) | upstream `javascript/` build | Apache-2.0 | 2018 The diff-match-patch Authors |

The full license texts are at the links above. The minified xterm.js build
carries no header of its own, so its notice lives here.

## Test fixtures

The repository also carries, for its tests only and never in the binary:

| Project | Version | License | Copyright |
| --- | --- | --- | --- |
| [jaffle_shop_duckdb](https://github.com/dbt-labs/jaffle_shop_duckdb), in `tests/fixtures/jaffle_shop/` with the additions its README lists | commit `20cc904` | Apache-2.0, its `LICENSE` beside it | dbt Labs |
