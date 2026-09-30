-- Added for dbt-edith: a singular test reading two models, neither of them
-- upstream of the other, which is what tells dbt's indirect selection modes apart.
{{ config(tags=['recon']) }}

select customers.customer_id
from {{ ref('customers') }} as customers
left join (
    select customer_id, sum(amount) as total
    from {{ ref('orders') }}
    group by customer_id
) as totals on totals.customer_id = customers.customer_id
where coalesce(totals.total, 0) <> coalesce(customers.customer_lifetime_value, 0)
