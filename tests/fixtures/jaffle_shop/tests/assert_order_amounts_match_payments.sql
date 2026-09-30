-- Added for dbt-edith: a singular test reading a model and one of its own
-- ancestors, the case where buildable keeps a test that cautious leaves out.
select orders.order_id
from {{ ref('orders') }} as orders
left join (
    select order_id, sum(amount) as total
    from {{ ref('stg_payments') }}
    group by order_id
) as paid on paid.order_id = orders.order_id
where coalesce(paid.total, 0) <> orders.amount
