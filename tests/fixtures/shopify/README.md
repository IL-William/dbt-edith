# Shopify, as a test fixture

Fivetran's [dbt_shopify](https://github.com/fivetran/dbt_shopify) package,
Apache-2.0, at tag `v1.10.0`, commit `03e91d7`: its integration tests, a root
project that reads the package from `../` as any project reads an installed
one. The package itself is not here. `scripts/compare_with_dbt.py` clones the
tag into a temporary directory, checks the commit, copies these two files
beside its `dbt_project.yml`, and runs `dbt deps` and `dbt parse` there (0056):

- `package-lock.yml` pins what `dbt deps` installs from dbt's hub:
  fivetran_utils, spark_utils and dbt_utils. It is dbt-collin's, which reads
  the same tag, so both repositories compare against one project.
- `profiles.yml` points DuckDB at memory: the comparison only parses and
  lists, and neither opens a connection.

What it has that the Jaffle Shop copy has not: 240 models in a package under a
root project, 87 sources, 29 ephemeral and 4 incremental models, folders three
deep, and disabled models and sources. It has no tag, no exposure and no test
with two parents, which Jaffle Shop keeps.

Moving to another tag means changing the tag and its commit in the script, and
the lock to what `dbt deps` writes on that tag.
