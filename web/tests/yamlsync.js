// Updating a model's YAML from the catalog (0188): what the server's answer does to the text of a
// properties file, and what the button and the card say of it. Fixtures are invented.
// Run from the repository root: jsc web/tests/yamlsync.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function yamlIndent'), src.indexOf('/* ATX headings')));
eval(src.slice(src.indexOf('function yamlValueText'), src.indexOf('/* The project\'s `name:`')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function lines(a) { return a.join('\n'); }
function patch(text, sync, model) { return yamlSyncPatch(text, model || 'orders', sync); }
function said(p) {
  return { text: p.text, added: p.added, removed: p.removed, retyped: p.retyped.length, typed: p.typed.length, skipped: p.skipped.length, refused: p.refused };
}

var base = [
  'version: 2',
  '',
  'models:',
  '  - name: orders',
  '    description: Orders.',
  '    columns:',
  '      - name: order_id',
  '        data_type: number',
  '        description: The key.',
  '      - name: amount',
  '        data_type: float',
  '',
  '  - name: customers',
  '    columns:',
  '      - name: id',
].join('\n');

// A column the table has, between two the file has: after the one before it in the table.
var between = patch(base, { missing: [{ name: 'status', data_type: 'varchar', after: 'order_id' }] });
check('a missing column goes after the one before it in the table', between.text.split('\n').slice(5, 13), [
  '    columns:',
  '      - name: order_id',
  '        data_type: number',
  '        description: The key.',
  '      - name: status',
  '        data_type: varchar',
  '      - name: amount',
  '        data_type: float',
]);
check('and is said to be added, on the lines it added', [between.added, between.lines], [['status'], [9, 10]]);
check('the other model is not touched', between.text.split('\n').slice(-3), ['  - name: customers', '    columns:', '      - name: id']);

var top = patch(base, { missing: [{ name: 'region', data_type: 'varchar' }, { name: 'zone', data_type: 'varchar', after: 'region' }] });
check('with nothing before it a column goes to the top of the list, in the table\'s order',
  top.text.split('\n').slice(5, 11), ['    columns:', '      - name: region', '        data_type: varchar', '      - name: zone', '        data_type: varchar', '      - name: order_id']);

var last = patch(base, { missing: [{ name: 'note', data_type: 'varchar', after: 'amount' }] });
check('after the last column it goes before the blank line that ends the entry',
  last.text.split('\n').slice(9, 15), ['      - name: amount', '        data_type: float', '      - name: note', '        data_type: varchar', '', '  - name: customers']);

check('a column named by case alone is the one it follows',
  patch(base, { missing: [{ name: 'status', data_type: 'varchar', after: 'ORDER_ID' }] }).text, between.text);

var noType = patch(base, { missing: [{ name: 'price', after: 'amount' }] });
check('a column with no type to write is added with its name alone', noType.text.split('\n').slice(9, 13), ['      - name: amount', '        data_type: float', '      - name: price', '']);

// A file that sets its columns apart sets the new ones apart.
var spaced = lines(['models:', '  - name: orders', '    columns:', '      - name: a', '', '      - name: b', '', '  - name: other', '']);
check('a column put after the last one is set apart by a blank line', patch(spaced, { missing: [{ name: 'c', data_type: 'date', after: 'b' }] }).text,
  lines(['models:', '  - name: orders', '    columns:', '      - name: a', '', '      - name: b', '', '      - name: c', '        data_type: date', '', '  - name: other', '']));
check('one put between two is set apart on its side', patch(spaced, { missing: [{ name: 'c', data_type: 'date', after: 'a' }] }).text.split('\n').slice(3, 10),
  ['      - name: a', '', '      - name: c', '        data_type: date', '', '      - name: b', '']);
check('two at the top come before the first, apart from each other and from it',
  patch(spaced, { missing: [{ name: 'c', data_type: 'date' }, { name: 'd', data_type: 'date', after: 'c' }] }).text.split('\n').slice(2, 11),
  ['    columns:', '      - name: c', '        data_type: date', '', '      - name: d', '        data_type: date', '', '      - name: a', '']);

// A model with no columns: key.
var bare = lines(['version: 2', 'models:', '  - name: orders', '    description: Orders.', '', '  - name: customers', '    description: x', '']);
check('a model with no columns: gets the key at the end of its entry, indented under its keys',
  patch(bare, { missing: [{ name: 'id', data_type: 'number' }, { name: 'name', data_type: 'varchar', after: 'id' }] }).text.split('\n').slice(2, 11),
  ['  - name: orders', '    description: Orders.', '    columns:', '      - name: id', '        data_type: number', '      - name: name', '        data_type: varchar', '', '  - name: customers']);
check('and follows a file that writes its lists at their key\'s column',
  patch(lines(['models:', '  - name: orders', '    data_tests:', '    - unique', '']), { missing: [{ name: 'id', data_type: 'number' }] }).text,
  lines(['models:', '  - name: orders', '    data_tests:', '    - unique', '    columns:', '    - name: id', '      data_type: number', '']));

// A model that is not in the file, or a file with no models, or nothing at all.
var absent = patch(base, { missing: [{ name: 'id', data_type: 'number' }] }, 'payments');
check('a model the file lacks is added under models:, after the last entry', absent.text.split('\n').slice(-6),
  ['    columns:', '      - name: id', '  - name: payments', '    columns:', '      - name: id', '        data_type: number']);
check('the file\'s own columns are not offered to a model it does not hold',
  patch(base, { missing: [{ name: 'id', data_type: 'number' }], extra: [{ name: 'old' }] }, 'payments').skipped.map((s) => s.op), ['extra']);
check('a file with other keys than models: gets one, set apart',
  patch('version: 2\nsources:\n  - name: raw\n', { missing: [{ name: 'id', data_type: 'number' }] }).text,
  'version: 2\nsources:\n  - name: raw\n\nmodels:\n  - name: orders\n    columns:\n      - name: id\n        data_type: number\n');
check('no file at all is a new one',
  patch('', { missing: [{ name: 'id', data_type: 'number' }, { name: 'name', data_type: 'varchar', after: 'id' }] }).text,
  'version: 2\n\nmodels:\n  - name: orders\n    columns:\n      - name: id\n        data_type: number\n      - name: name\n        data_type: varchar\n');
check('nothing to add leaves the file as it is', patch(base, {}, 'payments').text, base);

// Line endings and a byte order mark stay, and so does everything untouched.
var crlf = '﻿' + base.split('\n').join('\r\n') + '\r\n';
var made = patch(crlf, { missing: [{ name: 'status', data_type: 'varchar', after: 'order_id' }] });
check('a CRLF file stays CRLF, its BOM too', made.text, '﻿' + between.text.split('\n').join('\r\n') + '\r\n');
check('a CRLF file without a final line break takes a last column with none either, and no bare CR',
  patch(base.split('\n').join('\r\n'), { missing: [{ name: 'z', data_type: 'date', after: 'id' }] }, 'customers').text,
  base.split('\n').join('\r\n') + '\r\n      - name: z\r\n        data_type: date');
check('a file mixing both keeps each line\'s own ending',
  patch('models:\r\n  - name: orders\n    columns:\r\n      - name: a\n', { missing: [{ name: 'b', data_type: 'date', after: 'a' }] }).text,
  'models:\r\n  - name: orders\n    columns:\r\n      - name: a\n      - name: b\r\n        data_type: date\r\n');

// Comments stay.
var commented = lines([
  'models:',
  '  - name: orders',
  '    columns:',
  '      # the key',
  '      - name: order_id   # kept',
  '        data_type: number',
  '      # money',
  '      - name: amount',
  '        data_type: float',
  '',
]);
check('a comment above a column stays with it when a column goes first',
  patch(commented, { missing: [{ name: 'region', data_type: 'varchar' }] }).text.split('\n').slice(2, 8),
  ['    columns:', '      - name: region', '        data_type: varchar', '      # the key', '      - name: order_id   # kept', '        data_type: number']);
check('a column put after one goes before the comment of the next',
  patch(commented, { missing: [{ name: 'status', data_type: 'varchar', after: 'order_id' }] }).text.split('\n').slice(4, 10),
  ['      - name: order_id   # kept', '        data_type: number', '      - name: status', '        data_type: varchar', '      # money', '      - name: amount']);

// Removal.
var gone = patch(commented, { extra: [{ name: 'order_id' }] });
check('a removed column takes its lines and leaves the comment of the next one', gone.text.split('\n').slice(2, 7),
  ['    columns:', '      # the key', '      # money', '      - name: amount', '        data_type: float']);
check('and is said to be removed', gone.removed, ['order_id']);
check('its indented comments go with it, the next sibling\'s stay',
  patch(lines(['models:', '  - name: orders', '    columns:', '      - name: a', '        description: x', '        # about a', '', '      # about b', '      - name: b', '']), { extra: [{ name: 'a' }] }).text,
  lines(['models:', '  - name: orders', '    columns:', '', '      # about b', '      - name: b', '']));
check('the last column of the list takes the list with it, unless one is put back',
  patch(lines(['models:', '  - name: orders', '    description: x', '    columns:', '      - name: a', '']), { extra: [{ name: 'a' }] }).text,
  lines(['models:', '  - name: orders', '    description: x', '']));
check('a column put back keeps the list',
  patch(lines(['models:', '  - name: orders', '    columns:', '      - name: a', '']), { extra: [{ name: 'a' }], missing: [{ name: 'b', data_type: 'date' }] }).text,
  lines(['models:', '  - name: orders', '    columns:', '      - name: b', '        data_type: date', '']));
check('a column whose block description has dashes and colons goes whole',
  patch(lines(['models:', '  - name: orders', '    columns:', '      - name: a', '        description: |', '          - not: a key', '      - name: b', '']), { extra: [{ name: 'a' }] }).text,
  lines(['models:', '  - name: orders', '    columns:', '      - name: b', '']));
check('a name is found whatever its case', patch(commented, { extra: [{ name: 'ORDER_ID' }] }).removed, ['ORDER_ID']);
check('a column that is not there is skipped, not invented', patch(commented, { extra: [{ name: 'nope' }] }).skipped, [{ op: 'extra', name: 'nope', why: 'it is no longer in the file' }]);

// Types.
var typed = lines(['models:', '  - name: orders', '    columns:', '      - name: id', '        description: x', '      - name: qty', '        data_type: integer  # whole', '      - name: label', '        data_type: "text"']);
var both = patch(typed, { untyped: [{ name: 'id', to: 'number' }], retype: [{ name: 'qty', from: 'integer', to: 'varchar' }, { name: 'label', from: 'text', to: 'date' }] });
var blank = lines(['models:', '  - name: orders', '    columns:', '      - name: a', '        data_type:', '      - name: b', '        data_type: ""', "      - name: c", "        data_type: ''  # none", '      - name: d', '        data_type: null', '      - name: e', '        data_type: | ', '          text', '      - name: f', '        data_type: # later', '']);
var filled = patch(blank, { untyped: ['a', 'b', 'c', 'd', 'e', 'f'].map((n) => ({ name: n, to: 'date' })) });
check('a data_type with nothing in it is typed where it stands, not skipped', filled.text.split('\n').filter((l) => l.includes('data_type')), [
  '        data_type: date', '        data_type: date', '        data_type: date  # none', '        data_type: date', '        data_type: | ', '        data_type: date # later']);
check('and a block value is a type', [filled.typed.length, filled.skipped.map((s) => s.name)], [5, ['e']]);
check('a type is written after the name line, one replaced in place', both.text.split('\n').slice(3), [
  '      - name: id', '        data_type: number', '        description: x',
  '      - name: qty', '        data_type: varchar  # whole',
  '      - name: label', '        data_type: date']);
check('what was typed and retyped is said', [both.typed, both.retyped], [[{ name: 'id', to: 'number' }], [{ name: 'qty', from: 'integer', to: 'varchar' }, { name: 'label', from: 'text', to: 'date' }]]);
check('the lines changed are the ones marked', both.lines, [4, 7, 9]);
var edited = patch(typed, { retype: [{ name: 'qty', from: 'bigint', to: 'varchar' }], untyped: [{ name: 'qty', to: 'varchar' }] });
check('a type the file no longer holds is left, and the reason said', [edited.text, edited.skipped.map((s) => s.why)], [typed, ['its type is no longer bigint', 'it has a type now']]);
check('a column that has no type to replace is skipped',
  patch(typed, { retype: [{ name: 'id', from: 'text', to: 'date' }] }).skipped.map((s) => s.why), ['it has no data_type now']);
check('a type needing quotes is quoted', patch(typed, { untyped: [{ name: 'id', to: 'yes' }] }).text.split('\n')[4], "        data_type: 'yes'");
check('number(10,2) is plain', yamlScalar('number(10,2)'), 'number(10,2)');
check('a name that reads as a number or a boolean is quoted', [yamlScalar('2023'), yamlScalar('no'), yamlScalar('a: b'), yamlScalar('order id'), yamlScalar("it's #1")], ["'2023'", "'no'", "'a: b'", 'order id', "'it''s #1'"]);

// Duplicates are listed and never merged.
var twice = lines(['models:', '  - name: orders', '    columns:', '      - name: id', '        data_type: text', '      - name: id', '        data_type: text']);
var dup = patch(twice, { retype: [{ name: 'id', from: 'text', to: 'number' }], extra: [{ name: 'id' }], missing: [{ name: 'x', data_type: 'date', after: 'id' }] });
check('a column declared twice is listed and left alone', [dup.duplicates, dup.skipped.map((s) => s.why), dup.retyped.length, dup.removed.length], [['id'], ['it is declared twice', 'it is declared twice'], 0, 0]);
check('what is put after it is still added', dup.added, ['x']);
check('two quoted names that differ by case are two columns', patch(lines(['models:', '  - name: orders', '    columns:', '      - name: id', '      - name: ID', '']), { missing: [] }).duplicates, []);

// A missing column already in the file is not added twice.
check('a column the file has by then is skipped', patch(base, { missing: [{ name: 'AMOUNT', data_type: 'float', after: 'order_id' }] }).skipped, [{ op: 'add', name: 'AMOUNT', why: 'it is already declared' }]);

// What it will not touch.
function refused(label, text, want, sync) {
  var p = patch(text, sync || { missing: [{ name: 'x', data_type: 'date' }], extra: [{ name: 'order_id' }] });
  check(label, [p.refused, p.text === text], [want, true]);
}
refused('a columns: in flow style', 'models:\n  - name: orders\n    columns: [a, b]\n', 'its columns: is written in flow style');
refused('an empty flow columns:', 'models:\n  - name: orders\n    columns: []\n', 'its columns: is written in flow style');
refused('a column in flow style', 'models:\n  - name: orders\n    columns:\n      - {name: a, data_type: text}\n', 'a column is written in flow style');
refused('a model entry in flow style', 'models:\n  - {name: orders, columns: []}\n', 'its entry is written in flow style');
refused('models: in flow style', 'models: [{name: orders}]\n', 'its models: is written in flow style');
refused('an anchor in the entry', 'models:\n  - name: orders\n    columns: &cols\n      - name: a\n', 'its entry holds an anchor, an alias or a merge key');
refused('an alias in the entry', 'models:\n  - name: orders\n    columns: *cols\n', 'its entry holds an anchor, an alias or a merge key');
refused('a merge key in the entry', 'models:\n  - name: orders\n    columns:\n      - <<: *base\n        name: a\n', 'its entry holds an anchor, an alias or a merge key');
refused('several documents', 'models:\n  - name: orders\n---\nmodels:\n  - name: other\n', 'it holds several documents');
refused('a document end', 'models:\n  - name: orders\n...\n', 'it holds several documents');
refused('a tab in the indentation', 'models:\n  - name: orders\n    columns:\n\t- name: a\n', 'it has a tab in its indentation');
refused('CR-only line breaks', 'models:\r  - name: orders\r', 'it has CR-only line breaks');
refused('a column with no name', 'models:\n  - name: orders\n    columns:\n      - description: x\n', 'a column has no name');
refused('a plain item among the columns', 'models:\n  - name: orders\n    columns:\n      - a\n', 'a column has no name');
refused('a versioned model', 'models:\n  - name: orders\n    versions:\n      - v: 1\n    columns:\n      - name: a\n', 'the model has versions');
refused('the model declared twice', 'models:\n  - name: orders\n  - name: orders\n', 'the model is declared twice in it');
check('a leading document marker is one document', patch('---\nmodels:\n  - name: orders\n', { missing: [{ name: 'a', data_type: 'date' }] }).refused, '');
check('a star or an ampersand inside a description is no alias',
  patch('models:\n  - name: orders\n    description: "a *b* & c"\n    columns:\n      - name: a\n        description: rate *per* month\n', { missing: [{ name: 'b', data_type: 'date', after: 'a' }] }).refused, '');
check('an anchor in a block description is no anchor',
  patch('models:\n  - name: orders\n    description: |\n      &x\n      *y\n    columns:\n      - name: a\n', { missing: [{ name: 'b', data_type: 'date', after: 'a' }] }).refused, '');
check('another model\'s anchor does not refuse this one',
  patch('models:\n  - name: other\n    columns: &c\n      - name: a\n  - name: orders\n    columns:\n      - name: a\n', { missing: [{ name: 'b', data_type: 'date', after: 'a' }] }).refused, '');
check('a refused file is given back whole', said(patch('models: [{name: orders}]\n', { missing: [{ name: 'a' }] })),
  { text: 'models: [{name: orders}]\n', added: [], removed: [], retyped: 0, typed: 0, skipped: 0, refused: 'its models: is written in flow style' });

// A model among several in a file, names with quotes, a package's model is the project's by its name.
check('a quoted model name and a quoted column name are read as they are',
  patch('models:\n  - name: "orders"\n    columns:\n      - name: "Order Id"\n', { missing: [{ name: 'b', data_type: 'date', after: 'order id' }] }).text,
  'models:\n  - name: "orders"\n    columns:\n      - name: "Order Id"\n      - name: b\n        data_type: date\n');

// The button and the card.
check('the button says what it would do', yamlSyncTip({ missing: [1, 2, 3], extra: [1], retype: [1], untyped: [1] }), '+3 · −1 · 2 types');
check('and leaves out what it would not', yamlSyncTip({ untyped: [1] }), '1 type');
check('a button with nothing to write says what it holds', yamlSyncTip({ held: [{ name: 'a' }] }), '1 held');
var when = (d) => 'DATE' + d.getTime() / 1000;
check('a catalog older than the run says its date', yamlSyncBehind({ catalog_behind_run: true }, 5, when),
  "catalog.json was written DATE5, before this model's last run, so its columns may be the old table's.");
check('a table older than its code says to run it', yamlSyncBehind({ table_behind_code: true }, 5, when),
  "This model's table is older than its code: run it first, or the columns catalog.json lists are not the ones its code builds.");
check('both are two lines, and neither is nothing', [yamlSyncBehind({ catalog_behind_run: true, table_behind_code: true }, 5, when).split('\n').length, yamlSyncBehind({}, 5, when)], [2, '']);
check('a model with no YAML gets one beside its SQL', [yamlSyncNewFile('models/marts/orders.sql', 'orders'), yamlSyncNewFile('orders.sql', 'orders')], ['models/marts/orders.yml', 'orders.yml']);
var card = yamlSyncSaid({
  extra: [{ name: 'old', described: true, tests: 2 }, { name: 'plain' }],
  held: [{ name: 'price', reason: 'its precision is not in catalog.json' }, { name: 'cost', reason: 'its precision is not in catalog.json' }],
}, {
  added: ['a'], removed: ['old', 'plain'], retyped: [], typed: [{ name: 'q', to: 'number' }],
  skipped: [{ op: 'retype', name: 'z', why: 'it has no data_type now' }, { op: 'retype', name: 'y', why: 'it has no data_type now' }, { op: 'extra', name: 'x', why: 'it is declared twice' }],
  duplicates: ['id'],
}, 'models/schema.yml');
check('the card is a line a reason, and says what was done, what removing cost, what was left and that nothing is saved', card, [
  'Added 1: a',
  'Removed 2: old, plain',
  'Lost with them: old (a description and 2 tests)',
  'Typed 1: q number',
  'Left (retype) z, y: it has no data_type now',
  'Left (extra) x: it is declared twice',
  'Not typed, price, cost: its precision is not in catalog.json',
  'Declared twice, not touched: id',
  'models/schema.yml is changed in the editor, not saved: save it to keep it.',
]);
check('a model of hundreds of left columns is still few lines', yamlSyncSaid({}, { added: [], removed: [], retyped: [], typed: [], skipped: Array.from({ length: 300 }, (_, i) => ({ op: 'extra', name: 'c' + i, why: 'it is no longer in the file' })), duplicates: [] }, '').length, 2);
check('a model of types alone says so', yamlSyncTip({ status: 'untyped', untyped: [1, 2] }), '2 types, types only');
// A catalog of a deployed environment says so, and says more when the code is newer.
check('a deployed catalog is always said', yamlSyncBehind({ deployed: true, catalog_env: '.env.prod' }, 5, when).split('\n').length, 1);
check('and says more when the code is newer', yamlSyncBehind({ deployed: true, catalog_env: '.env.prod', code_after_catalog: true }, 5, when).split('\n').length, 2);
check('and names the file, the removals and the SQL', /\.env\.prod.*removal.*SQL/.test(yamlSyncBehind({ deployed: true, catalog_env: '.env.prod' }, 5, when)), true);

// Where the columns are read from: catalog.json, or Snowflake for the pill's environment.
check('the source is named in a switch, a tooltip and a head', [
  yamlSyncSourceWords('catalog', 'manifest'),
  yamlSyncSourceWords('snowflake', 'manifest'),
  yamlSyncSourceWords('snowflake', ''),
  yamlSyncSourceWords('snowflake', '.env.prod'),
], ['catalog.json', "Snowflake, the manifest's target", "Snowflake, the manifest's target", 'Snowflake, .env.prod']);
check('a live request adds the source and the environment, the catalog nothing', [yamlSyncQuery('catalog', ''), yamlSyncQuery('snowflake', ''), yamlSyncQuery('snowflake', '.env.prod')],
  ['', 'source=snowflake&env=', 'source=snowflake&env=.env.prod']);
check('and so does a body', [yamlSyncSourceBody({ nodes: ['a'] }, 'catalog', 'x'), yamlSyncSourceBody({ nodes: ['a'] }, 'snowflake', '.env.qa')],
  [{ nodes: ['a'] }, { nodes: ['a'], source: 'snowflake', env: '.env.qa' }]);
check('a live read says Snowflake, not the catalog, when the table is behind its code',
  yamlSyncBehind({ table_behind_code: true }, 0, when, 'snowflake'),
  "This model's table is older than its code: run it first, or the columns Snowflake has are not the ones its code builds.");
check('and when it is a deployed environment', /^Snowflake has the table of \.env\.prod, an environment deployed/.test(yamlSyncBehind({ deployed: true, catalog_env: '.env.prod' }, 0, when, 'snowflake')), true);

// The left half of the Update YAML control: a button, a word, or nothing.
var s = (status, extra) => Object.assign({ status: status }, extra || {});
check('Snowflake shows its button at once, naming the table and the environment',
  [yamlSyncState('snowflake', undefined, true, 0, 'orders', '.env.prod').kind, yamlSyncState('snowflake', undefined, true, 0, 'orders', '.env.prod').label,
    /orders.*Snowflake, \.env\.prod.*Nothing is read before you click/.test(yamlSyncState('snowflake', undefined, true, 0, 'orders', '.env.prod').title)],
  ['button', 'Update YAML', true]);
check('a catalog that disagrees is a button, its title the tip', [yamlSyncState('catalog', s('disagrees', { missing: [1, 2] }), false).kind, yamlSyncState('catalog', s('disagrees', { missing: [1, 2] }), true).title.split('\n')[0]], ['button', '+2']);
check('types alone are a button too, said so', [yamlSyncState('catalog', s('untyped', { untyped: [1] }), false).label, yamlSyncState('catalog', s('untyped', { untyped: [1] }), false).title.split('\n')[0]], ['Update YAML', '1 type, types only']);
check('a YAML that agrees says so, with or without Snowflake', [yamlSyncState('catalog', s('agrees'), false), yamlSyncState('catalog', s('agrees'), true).kind],
  [{ kind: 'ok', label: '\u2713 YAML matches', title: 'The schema YAML already says what catalog.json says.' }, 'ok']);
check('a model not checked says so only where Snowflake can be picked', [yamlSyncState('catalog', s('not_checked', { reason: 'its table is not the one catalog.json lists' }), true, 5), yamlSyncState('catalog', s('not_checked', { reason: 'x' }), false, 5)],
  [{ kind: 'quiet', label: 'YAML not checked', title: 'its table is not the one catalog.json lists' }, null]);
check('no catalog.json (not_checked, no date) says so only where Snowflake can be picked',
  [yamlSyncState('catalog', s('not_checked', { reason: 'it is not in catalog.json' }), true, 0), yamlSyncState('catalog', s('not_checked', { reason: 'it is not in catalog.json' }), false, 0)],
  [{ kind: 'quiet', label: 'no catalog.json', title: 'There is no catalog.json: run dbt docs generate, or read the columns from Snowflake.' }, null]);
check('a request that failed says the check did not answer, only where Snowflake can be picked',
  [yamlSyncState('catalog', null, true, 0), yamlSyncState('catalog', null, false, 0)],
  [{ kind: 'quiet', label: 'YAML not checked', title: 'The check did not answer.' }, null]);
