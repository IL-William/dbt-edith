-- Added for dbt-edith: a singular test with one parent, beside the one above.
{{ config(tags=['recon']) }}

select payment_id
from {{ ref('stg_payments') }}
where amount < 0
