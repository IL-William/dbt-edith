// Links into Snowsight: a query's page, and a relation's object, schema and
// database, all on the account the session reports (0048, 0051).
// Run from the repository root: jsc web/tests/snowsight.js
var app = read('web/app.js');
eval(app.slice(app.indexOf('function snowsightHome'), app.indexOf('function historyRoles')));
eval(app.slice(app.indexOf('function splitRelation'), app.indexOf('function resolvedRelation')));

function check(label, got, want) {
  print((got === want ? 'PASS  ' : 'FAIL  ') + label + (got === want ? '' : '\n        expected ' + want + '\n        got      ' + got));
}

var session = { org: 'MYORG', account: 'MY_ACCOUNT', user: 'SOMEONE', role: 'TRANSFORMER' };
var home = 'https://app.snowflake.com/myorg/my_account/#';

print('--- a query ---');
check('organization and account name the page, in lower case',
      snowsightUrl(session, '01b7a2c4-0000-003b'), home + '/compute/history/queries/01b7a2c4-0000-003b/detail');
check('no organization, no link', snowsightUrl({ account: 'MY_ACCOUNT' }, '01b7'), '');
check('no session yet, no link', snowsightUrl(null, '01b7'), '');
check('every part is escaped', snowsightUrl({ org: 'a/b', account: 'c?d' }, 'e#f'),
      'https://app.snowflake.com/a%2Fb/c%3Fd/#/compute/history/queries/e%23f/detail');

print('\n--- a relation ---');
function places(relation, materialized, who) {
  return snowsightPlaces(who === undefined ? session : who, splitRelation(relation), materialized);
}
var view = places('analytics.dbt_someone.dim_customers', 'view');
check('the object first, then its schema and its database',
      view.map(function (p) { return p.what + ' ' + p.name; }).join(' | '),
      'view DIM_CUSTOMERS | schema DBT_SOMEONE | database ANALYTICS');
check('a view', view[0].url, home + '/data/databases/ANALYTICS/schemas/DBT_SOMEONE/view/DIM_CUSTOMERS');
check('its schema', view[1].url, home + '/data/databases/ANALYTICS/schemas/DBT_SOMEONE');
check('its database', view[2].url, home + '/data/databases/ANALYTICS');
check('a table', places('analytics.marts.orders', 'table')[0].url, home + '/data/databases/ANALYTICS/schemas/MARTS/table/ORDERS');
check('an incremental model is a table', places('analytics.marts.orders', 'incremental')[0].what, 'table');
check('so are a seed and a snapshot', places('a.b.c', 'seed')[0].what + ' ' + places('a.b.c', 'snapshot')[0].what, 'table table');
check('a dynamic table has its own page', places('a.b.c', 'dynamic_table')[0].url, home + '/data/databases/A/schemas/B/dynamic-table/C');
check('a materialized view is a view', places('a.b.c', 'materialized_view')[0].url, home + '/data/databases/A/schemas/B/view/C');
var source = places('raw.crm.customers', 'source')[0];
check('a source is asked for as a table', source.url, home + '/data/databases/RAW/schemas/CRM/table/CUSTOMERS');
check('and says it is a guess', source.guess.indexOf('view') > 0, true);
check('a model is no guess', view[0].guess, '');
check('quoted parts keep their case, the others fold to upper case',
      places('"Raw"."crm"."Customers"', 'source')[0].url, home + '/data/databases/Raw/schemas/crm/table/Customers');
check('mixed', places('raw."Crm".customers', 'source')[0].url, home + '/data/databases/RAW/schemas/Crm/table/CUSTOMERS');
check('a name that needs it is escaped', places('"a b"."c/d"."e#f"', 'table')[0].url, home + '/data/databases/a%20b/schemas/c%2Fd/table/e%23f');
check('a quoted dot stays inside its part', places('"a.b".c.d', 'view')[2].url, home + '/data/databases/a.b');
check('no organization, no link', places('a.b.c', 'view', { account: 'MY_ACCOUNT' }).length, 0);
check('no session yet, no link', places('a.b.c', 'view', null).length, 0);
check('two parts are not a relation', places('b.c', 'view').length, 0);
check('nor is an empty part', places('a..c', 'view').length, 0);
