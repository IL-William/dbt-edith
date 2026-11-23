// The static site's answers: web/site.js held to what Rust answers, on an invented graph.
// tests/fixtures/site_case.js is written by `DBT_EDITH_BLESS=1 cargo test site_case` and holds
// /api/lineage (every node one level each way, a few at every depth, with the caps of the page and a
// small one), /api/search, /api/node and /api/site/code as the server gives them, and /api/collineage (every
// column of two caches, an invented one and one from a seed, and the deep, capped and cut walks of the first) and
// /api/colreach for every node, and the jobs: /api/jobs and /api/jobs/node for every node, over a source that was read, one that was not and a file that does not parse, and over a jobs.yml that does not read; this makes the same requests of EdithSite and compares what comes back, key for key.
// Run from the repository root: jsc web/tests/site.js
eval(read('tests/fixtures/site_case.js'));
var EdithSite = new Function(read('web/site.js') + '\nreturn EdithSite;')();

function check(label, ok, detail) { print((ok ? 'PASS  ' : 'FAIL  ') + label + (ok || !detail ? '' : '\n        ' + detail)); }

/* Sorted keys, so two answers are the same when they say the same. Arrays keep their order, which
   is part of the answer. */
function canon(v) {
  if (Array.isArray(v)) return '[' + v.map(canon).join(',') + ']';
  if (v && typeof v === 'object') {
    return '{' + Object.keys(v).sort().map(function (k) { return JSON.stringify(k) + ':' + canon(v[k]); }).join(',') + '}';
  }
  return JSON.stringify(v);
}

var loads = [];
var sqlLoads = [];
var cllLoads = [];
var jobsLoads = [];
var site;
function EDITH_SITE_CARDS(start, list) { site.received(start, list); }
function EDITH_SITE_SQL(start, list) { site.receivedSql(start, list); }
function EDITH_SITE_CLL(raw) { site.receivedCll(raw); }
function EDITH_SITE_JOBS(raw) { site.receivedJobs(raw); }
/* A bucket's script, by the url the page would load it from: the cards', the SQL's or the column lineage's. */
function loadFrom(url, k, cards) {
  if (url === 'data/cll.js') { cllLoads.push(url); eval(SITE_CASE.cll); return; }
  if (url === 'data/jobs.js') { jobsLoads.push(url); eval(SITE_CASE.jobs); return; }
  if (url.indexOf('data/sql/') === 0) { sqlLoads.push(url); eval(SITE_CASE.sql[k]); return; }
  cards.push(url);
  eval(SITE_CASE.buckets[k]);
}
site = EdithSite.make(SITE_CASE.site, function (k, url) {
  loadFrom(url, k, loads);
  return Promise.resolve();
});
var main = site;

/* One answer after another, each compared with Rust's. A group prints one PASS, or every mismatch. */
function group(label, cases, run) {
  var bad = [];
  return cases.reduce(function (chain, c) {
    return chain.then(function () { return run(c); }).then(function (got) {
      if (canon(got) !== canon(c.want)) bad.push(c.path + '\n        expected ' + canon(c.want).slice(0, 300) + '\n        got      ' + canon(got).slice(0, 300));
    }, function (e) { bad.push(c.path + ' threw ' + e); });
  }, Promise.resolve()).then(function () {
    check(label + ' (' + cases.length + ')', bad.length === 0, bad.slice(0, 3).join('\n        '));
  });
}

function rejected(path, why) {
  return site.get(path).then(function () { check(path + ' is refused', false, 'it answered'); },
    function (e) { check(path + ' is refused: ' + why, String(e.message).indexOf(why) !== -1, String(e.message)); });
}

var ids = SITE_CASE.site.base.map(function (b) { return b.id; });

group('lineage, as /api/lineage answers it', SITE_CASE.lineage, function (c) { return site.get(c.path); })
  .then(function () { return group('search, as /api/search answers it', SITE_CASE.search, function (c) { return site.get(c.path); }); })
  .then(function () {
    // Cards come from their bucket, found by the node's number.
    return group('cards, as /api/node answers them', SITE_CASE.node, function (c) { return site.get(c.path); });
  })
  .then(function () {
    return group('the tree, as /api/dir lists it', SITE_CASE.dir, function (c) { return site.get(c.path); });
  })
  .then(function () {
    var dirs = SITE_CASE.dir;
    check('a folder is every ancestor of a file, from the root', dirs.length > 8 && dirs[0].path === '/api/dir?path=');
    return site.get('/api/dir?path=%2Fmodels%2FMixed%2F').then(function (list) {
      check('a path is read with its slashes cut, and the entries have no size', list.length === 2 && list[0].path === 'models/Mixed/alpha.sql' && list[0].size === 0);
      check('a name is ordered by its lower-cased form, as Rust wrote it', list[0].name === 'alpha.sql' && list[1].name === 'Zeta.sql');
      return rejected('/api/dir?path=models/ops', 'not a folder');
    }).then(function () {
      var snaps = dirs.filter(function (d) { return d.path === '/api/dir?path=snapshots'; })[0].want;
      check('a file of two snapshots is listed once', snaps.length === 1 && snaps[0].id === 'snapshot.shop.s_a' && snaps[0].kind === 'snapshot' && !('materialized' in snaps[0]));
      var pkg = dirs.filter(function (d) { return d.path.indexOf('dbt_packages') > 0 && d.want.some(function (e) { return !e.dir; }); })[0].want;
      check('a package model is listed where its package was found, a model by its materialization', pkg.every(function (e) { return e.path.indexOf('dbt_packages/crm/') === 0 && e.materialized === 'table'; }), JSON.stringify(pkg));
      var id = 'model.crm.accounts';
      check('a node\'s row is where its file was put, else where its file says', site.treePath(id) === 'dbt_packages/crm/models/crm/accounts.sql' && site.treePath('model.shop.dia_a') === 'models/marts/dia_a.sql');
    });
  })
  .then(function () {
    var again = loads.length;
    return site.get(SITE_CASE.node[0].path).then(function () {
      check('a card whose bucket arrived is not asked for again', loads.length === again);
    });
  })
  .then(function () {
    // Every node's card is reachable, from whichever bucket holds it, and is its own.
    return Promise.all(ids.map(function (id) { return site.get('/api/node?id=' + encodeURIComponent(id)); })).then(function (all) {
      check('every card is its node\'s', all.every(function (c, i) { return c.id === ids[i]; }));
      check('each bucket is loaded once, however many cards ask', loads.length === SITE_CASE.buckets.length &&
        loads.filter(function (u, i) { return loads.indexOf(u) === i; }).length === loads.length, loads.join());
      var names = loads.slice().sort();
      check('a bucket has the file name the page writes', names.every(function (u, i) { return u === 'data/cards/' + ('000' + i).slice(-4) + '.js'; }), names.join());
    });
  })
  .then(function () {
    // A bucket that does not load is forgotten: the next ask tries again.
    var tries = 0;
    var flaky = EdithSite.make(SITE_CASE.site, function (k) {
      tries++;
      if (tries === 1) return Promise.reject(new Error('cannot load'));
      site = flaky;
      eval(SITE_CASE.buckets[k]);
      site = main;
      return Promise.resolve();
    });
    var path = '/api/node?id=' + encodeURIComponent(ids[ids.length - 1]);
    return flaky.get(path).then(function () { check('a bucket that fails rejects its cards', false); }, function (e) {
      check('a bucket that fails rejects its cards', e.message === 'cannot load');
      return flaky.get(path).then(function (card) {
        check('and is asked for again', tries === 2 && card.id === ids[ids.length - 1]);
      });
    });
  })
  .then(function () {
    // A script that ran and gave no cards is a file that is not ours.
    var empty = EdithSite.make(SITE_CASE.site, function () { return Promise.resolve(); });
    return empty.get('/api/node?id=' + encodeURIComponent(ids[3])).then(function () { check('an empty bucket is refused', false); },
      function (e) { check('an empty bucket is refused', /did not arrive/.test(e.message), e.message); });
  })
  .then(function () { return rejected('/api/lineage?id=model.shop.nothing', 'unknown node'); })
  .then(function () { return rejected('/api/lineage', 'unknown node'); })
  .then(function () { return rejected('/api/lineage?id=' + ids[0] + '&up=abc', 'invalid up'); })
  .then(function () { return rejected('/api/lineage?id=' + ids[0] + '&down=-1', 'invalid down'); })
  .then(function () { return rejected('/api/search?q=a&limit=many', 'invalid limit'); })
  .then(function () { return rejected('/api/node?file=models/a.sql', 'not in this site'); })
  .then(function () { return rejected('/api/node?id=nothing', 'unknown node'); })
  .then(function () { return rejected('/api/reports', 'not in this site'); })
  .then(function () { return rejected('/api/git', 'not in this site'); })
  .then(function () {
    // The jobs are the orchestrator's, read when the site was written. Every other route of theirs is the project's or the warehouse's.
    return Promise.all(['/api/jobs/sources', '/api/jobs/browse', '/api/jobs/coverage', '/api/jobs/order', '/api/jobs/models', '/api/jobs/step?n=0'].map(function (p) {
      return rejected(p, 'not in this site');
    }));
  })
  .then(function () {
    // A site that wrote no jobs has neither route.
    var meta = Object.assign({}, SITE_CASE.site.meta);
    delete meta.jobs;
    var none = EdithSite.make(Object.assign({}, SITE_CASE.site, { meta: meta }), function () { return Promise.resolve(); });
    return Promise.all(['/api/jobs', '/api/jobs?look=true', '/api/jobs/node?id=' + encodeURIComponent(ids[0])].map(function (p) {
      return none.get(p).then(function () { check(p + ' is refused without jobs', false); }, function (e) { check(p + ' is refused without jobs', e.message === 'not in this site', e.message); });
    }));
  })
  .then(function () { return rejected('/ws/pty', 'not in this site'); })
  .then(function () {
    return Promise.all([site.get('/api/macros/search?q=a&limit=10'), site.get('/api/files?q=a&limit=10')]).then(function (r) {
      check('macros and files have no answer, and no error', canon(r) === '[[],[]]');
    });
  })
  .then(function () {
    // The look is kept in the browser. Without storage, or with a refusing one, it is the default.
    return site.get('/api/look').then(function (look) {
      check('the look is the default with nothing kept', canon(look) === canon({ theme: 'night', follow: false, apparitions: true, boxes: 'tombstone' }), canon(look));
      return site.send('/api/look', 'POST', { theme: 'light', follow: true, apparitions: false, boxes: 'coffin' });
    }).then(function (back) {
      check('a look saved with no storage is handed back', back.theme === 'light');
      var kept = {};
      globalThis.localStorage = { getItem: function (k) { return k in kept ? kept[k] : null; }, setItem: function (k, v) { kept[k] = String(v); } };
      return site.send('/api/look', 'POST', { theme: 'graphite', follow: false, apparitions: true, boxes: 'grimoire' }).then(function () {
        return site.get('/api/look');
      }).then(function (look) {
        check('a look saved is the look read', look.theme === 'graphite' && look.boxes === 'grimoire', canon(look));
        globalThis.localStorage = { getItem: function () { throw new Error('refused'); }, setItem: function () { throw new Error('refused'); } };
        return site.send('/api/look', 'POST', { theme: 'light' }).then(function (back2) {
          check('storage that refuses is not an error', back2.theme === 'light');
          return site.get('/api/look');
        }).then(function (look2) { check('and reads as the default', look2.theme === 'night'); });
      });
    }).then(function () { return site.send('/api/envs', 'POST', {}).then(function () { check('no other write', false); }, function (e) { check('no other write', e.message === 'not in this site'); }); });
  })
  .then(function () {
    // The code of a node: its source and compiled text, the file and the link to it, from a bucket cut apart from the cards'.
    return group('code, as /api/site/code answers it', SITE_CASE.code, function (c) { return site.get(c.path); });
  })
  .then(function () {
    var cuts = SITE_CASE.site.sql_cuts;
    check('the SQL has cuts of its own, not the cards\'', cuts.length > 1 && cuts.join() !== SITE_CASE.site.cuts.join() && SITE_CASE.sql.length === cuts.length);
    var by = {};
    SITE_CASE.code.forEach(function (c) { by[decodeURIComponent(c.path.split('id=')[1])] = c.want; });
    var crm = by['model.crm.customers'];
    check('a package\'s file is no link, and the file is still named', crm.url === null && /^dbt_packages\/crm\//.test(crm.file), JSON.stringify(crm));
    var odd = by['model.shop.spaced'];
    check('a space and a # stay in their segment of the link', odd.url === SITE_CASE.site.meta.file_base + 'models/odd%20dir/a%23b.sql', odd.url);
    check('a name that closes a script or ends a line reads back as written', by['model.shop.dia_b'].raw === 'select \'</script><!--\u2028\u2029\' as a' && by['model.shop.dia_b'].compiled === null);
    check('a CRLF is an LF', by['model.shop.dia_a'].raw === 'select 1\nfrom {{ ref(\'countries\') }}');
    check('a Python model says so', by['model.shop.orders.v1'].lang === 'python' && by['model.shop.dia_a'].lang === 'sql');
    check('a node with no code answers an empty one, its file named', by['seed.shop.countries'].raw === '' && by['seed.shop.countries'].compiled === null && by['seed.shop.countries'].file !== '');
    check('a disabled node has its source and is not compiled', by['model.shop.old_hub'].raw === 'select \'old\'' && by['model.shop.old_hub'].compiled === null);
    var asked = sqlLoads.length;
    return site.get(SITE_CASE.code[0].path).then(function () {
      check('a bucket of SQL that arrived is not asked for again', sqlLoads.length === asked);
      check('each bucket of SQL is loaded at most once, under the name the page writes', sqlLoads.length === sqlLoads.filter(function (u, i) { return sqlLoads.indexOf(u) === i; }).length &&
        sqlLoads.every(function (u) { return /^data\/sql\/\d{4}\.js$/.test(u); }), sqlLoads.join());
    });
  })
  .then(function () {
    // A bucket of SQL that does not load is forgotten, and does not take the cards with it.
    var tries = 0;
    var flaky = EdithSite.make(SITE_CASE.site, function (k, url) {
      if (url.indexOf('data/sql/') === 0) {
        tries++;
        if (tries === 1) return Promise.reject(new Error('cannot load the SQL'));
        site = flaky;
        eval(SITE_CASE.sql[k]);
        site = main;
        return Promise.resolve();
      }
      site = flaky;
      eval(SITE_CASE.buckets[k]);
      site = main;
      return Promise.resolve();
    });
    var path = SITE_CASE.code[0].path;
    return flaky.get(path).then(function () { check('SQL that fails to load rejects the code', false); }, function (e) {
      check('SQL that fails to load rejects the code', e.message === 'cannot load the SQL', e.message);
      return flaky.get(path).then(function (c) { check('and is asked for again', tries === 2 && canon(c) === canon(SITE_CASE.code[0].want)); });
    });
  })
  .then(function () {
    var plain = EdithSite.make(Object.assign({}, SITE_CASE.site, { sql_cuts: undefined }), function () { return Promise.resolve(); });
    return plain.get(SITE_CASE.code[0].path).then(function () { check('a site with no SQL has no code to give', false); }, function (e) {
      check('a site with no SQL has no code to give', /did not arrive/.test(e.message), e.message);
    });
  })
  .then(function () { return rejected('/api/site/code', 'unknown node'); })
  .then(function () { return rejected('/api/site/code?id=model.shop.nothing', 'unknown node'); })
  .then(function () {
    // The link to a file, as the page builds it.
    var url = EdithSite.fileUrl;
    var meta = { root: 'shop', file_base: 'https://github.com/acme/shop/blob/abc/analytics/' };
    check('a file of the project is linked under the base, each segment escaped', url(meta, 'shop', 'models/a b/c#d.sql') === 'https://github.com/acme/shop/blob/abc/analytics/models/a%20b/c%23d.sql');
    check('a non-ASCII name is percent-encoded as UTF-8', url(meta, 'shop', 'models/caf\u00e9.sql') === 'https://github.com/acme/shop/blob/abc/analytics/models/caf%C3%A9.sql');
    check('a package\'s file is not in the repository', url(meta, 'crm', 'models/a.sql') === '');
    check('no base, no link', url({ root: 'shop', file_base: null }, 'shop', 'models/a.sql') === '' && url({ root: 'shop' }, 'shop', 'a.sql') === '');
    check('a base that is not https is no link', url({ root: 'shop', file_base: 'javascript:alert(1)//' }, 'shop', 'a.sql') === '' && url({ root: 'shop', file_base: 'http://h/x/' }, 'shop', 'a.sql') === '');
    check('no root name, no link: no node is known to be the root\'s', url({ file_base: 'https://h/x/' }, '', 'a.sql') === '' && url({ root: '', file_base: 'https://h/x/' }, '', 'a.sql') === '');
    check('a path that goes up or is empty is no link', url(meta, 'shop', '../a.sql') === '' && url(meta, 'shop', 'a//b.sql') === '' && url(meta, 'shop', './a.sql') === '' && url(meta, 'shop', '') === '');
  })
  .then(function () {
    // Column lineage, as /api/collineage and /api/colreach answer it. The file is not loaded until a column is asked about.
    check('the column lineage is not loaded before a column is asked about', cllLoads.length === 0 && !site.cllHeld());
    return group('column lineage, as /api/collineage answers it', SITE_CASE.collineage, function (c) { return site.get(c.path); });
  })
  .then(function () { return group('the whole chain of each column, as /api/colreach answers it', SITE_CASE.colreach, function (c) { return site.get(c.path); }); })
  .then(function () {
    check('the file is loaded once, however many columns asked', cllLoads.length === 1 && site.cllHeld(), cllLoads.join());
    var randomSite = EdithSite.make(SITE_CASE.site, function () {
      site = randomSite;
      eval(SITE_CASE.random.cll);
      site = main;
      return Promise.resolve();
    });
    return group('column lineage of a cache from a seed, a tangle of cycles and roles', SITE_CASE.random.collineage, function (c) { return randomSite.get(c.path); })
      .then(function () { return group('and its chains', SITE_CASE.random.colreach, function (c) { return randomSite.get(c.path); }); })
      .then(function () {
        // A cache of edges alone: no root, no rule, no indirect role, and a column from a table no node owns.
        var plainSite = EdithSite.make(SITE_CASE.site, function () {
          site = plainSite;
          eval(SITE_CASE.plain.cll);
          site = main;
          return Promise.resolve();
        });
        return group('column lineage of a cache of edges alone', SITE_CASE.plain.collineage, function (c) { return plainSite.get(c.path); })
          .then(function () { return group('and its chains', SITE_CASE.plain.colreach, function (c) { return plainSite.get(c.path); }); });
      });
  })
  .then(function () {
    // A file that fails to load is forgotten, and a script that gave nothing is not ours.
    var tries = 0;
    var flaky = EdithSite.make(SITE_CASE.site, function () {
      tries++;
      if (tries === 1) return Promise.reject(new Error('cannot load the column lineage'));
      site = flaky;
      eval(SITE_CASE.cll);
      site = main;
      return Promise.resolve();
    });
    var path = SITE_CASE.collineage[0].path;
    return flaky.get(path).then(function () { check('a column lineage that fails to load rejects the answer', false); }, function (e) {
      check('a column lineage that fails to load rejects the answer', e.message === 'cannot load the column lineage', e.message);
      return flaky.get(path).then(function (a) { check('and is asked for again', tries === 2 && canon(a) === canon(SITE_CASE.collineage[0].want) && flaky.cllHeld()); });
    });
  })
  .then(function () {
    var empty = EdithSite.make(SITE_CASE.site, function () { return Promise.resolve(); });
    return empty.get(SITE_CASE.collineage[0].path).then(function () { check('an empty column lineage file is refused', false); },
      function (e) { check('an empty column lineage file is refused', /did not arrive/.test(e.message), e.message); });
  })
  .then(function () {
    // Two asks while the file is on its way are one load.
    var asked = 0;
    var slow = EdithSite.make(SITE_CASE.site, function () {
      asked++;
      return new Promise(function (resolve) { setTimeout(function () { site = slow; eval(SITE_CASE.cll); site = main; resolve(); }, 1); });
    });
    var a = SITE_CASE.collineage[0];
    var b = SITE_CASE.colreach[0];
    return Promise.all([slow.get(a.path), slow.get(b.path)]).then(function (r) {
      check('an ask while the file is on its way waits for the same load', asked === 1 && canon(r[0]) === canon(a.want) && canon(r[1]) === canon(b.want));
    });
  })
  .then(function () {
    var path = SITE_CASE.collineage[0].path;
    var id = ids[0];
    var parts = function (more) { return '/api/collineage?id=' + encodeURIComponent('model.shop.dia_a') + '&column=id' + more; };
    // The errors, in the server's order: the query first, then "no column lineage", the node, the column.
    return rejected('/api/collineage?column=id', 'missing id')
      .then(function () { return rejected('/api/collineage?id=' + encodeURIComponent('model.shop.dia_a'), 'missing column'); })
      .then(function () { return rejected(parts('&up=abc'), 'invalid up'); })
      .then(function () { return rejected(parts('&down=-1'), 'invalid down'); })
      .then(function () { return rejected(parts('&max=many'), 'invalid max'); })
      .then(function () { return rejected(parts('&indirect=300'), 'invalid indirect'); })
      .then(function () { return rejected('/api/collineage?id=model.shop.nothing&column=id', 'unknown node'); })
      .then(function () { return rejected('/api/collineage?id=&column=id', 'unknown node'); })
      .then(function () { return rejected(parts('').replace('column=id', 'column=nothing'), 'unknown column'); })
      .then(function () { return rejected('/api/colreach?id=model.shop.nothing', 'unknown node'); })
      .then(function () { return rejected('/api/colreach', 'missing id'); })
      .then(function () {
        // Without a column lineage: said before anything is loaded, and nothing is asked for.
        var meta = Object.assign({}, SITE_CASE.site.meta, { cll: false });
        var loaded = 0;
        var none = EdithSite.make(Object.assign({}, SITE_CASE.site, { meta: meta }), function () { loaded++; return Promise.resolve(); });
        return none.get(path).then(function () { check('a site with no column lineage refuses a column', false); }, function (e) {
          check('a site with no column lineage refuses a column, and loads nothing', e.message === 'no column lineage' && loaded === 0, e.message);
          return none.get('/api/collineage?id=model.shop.nothing&column=id').then(function () { check('and says so before the node', false); }, function (e2) {
            check('and says so before the node', e2.message === 'no column lineage', e2.message);
            return none.get('/api/colreach?id=' + encodeURIComponent(id)).then(function (rows) {
              check('and its chains are none, as the server answers', canon(rows) === '[]' && loaded === 0);
            });
          });
        });
      });
  })
  .then(function () {
    var cll = SITE_CASE.site.limits;
    check('the column canvas\'s limits are Rust\'s own', cll.col_depth === 2 && cll.col_depth_max === 20 && cll.col_drawn === 1000 && cll.col_most === 3000 && cll.rows_by === 8);
    var meta = SITE_CASE.site.meta;
    check('the site names the cache bare, never by a path, and says whether it is stale', meta.cll === true && meta.cll_tool === 'collin' && meta.cll_file === 'column_lineage.collin.json' && meta.cll_stale === false &&
      !/[\\/]/.test(meta.cll_file) && meta.cll_edges > 0 && !('cll_mtime' in meta) && !('cll_producer' in meta), JSON.stringify(meta));
    // The cases cover what the walk and the card can say: boxes, stops, caps, cuts and a tangle.
    var all = SITE_CASE.collineage.concat(SITE_CASE.random.collineage).map(function (c) { return c.want; });
    var any = function (f) { return all.some(f); };
    check('the cases hold a truncated walk, a stop, a box, a role cut, a partial column and a root', any(function (a) { return a.truncated; }) && any(function (a) { return a.stopped_at; }) &&
      any(function (a) { return a.nodes.some(function (n) { return n.id.indexOf('::') < 0; }); }) && any(function (a) { return a.levels.down.some(function (l) { return l.cut; }); }) &&
      any(function (a) { return a.nodes.some(function (n) { return n.partial; }); }) && any(function (a) { return a.nodes.some(function (n) { return n.generated; }); }) &&
      any(function (a) { return a.nodes.some(function (n) { return n.rows_by_more; }); }) && any(function (a) { return a.nodes.some(function (n) { return n.rows_of_more; }); }) &&
      any(function (a) { return a.nodes.some(function (n) { return n.rules_more; }); }) && any(function (a) { return a.nodes.some(function (n) { return n.outside; }); }));
  })
  .then(function () {
    // The jobs, as /api/jobs and /api/jobs/node answer them. The file is not loaded until the tab or a card asks.
    check('the jobs are not loaded before they are asked for', jobsLoads.length === 0 && !site.jobsHeld());
    return group('the jobs, as /api/jobs answers them', [SITE_CASE.jobs_summary], function (c) { return site.get(c.path); })
      .then(function () { return site.get('/api/jobs?look=true'); })
      .then(function (looked) {
        check('a look is asked for in vain: the answer is the one it was', canon(looked) === canon(SITE_CASE.jobs_summary.want));
        check('the file is loaded once, however many asks', jobsLoads.length === 1 && site.jobsHeld(), jobsLoads.join());
        return group('the jobs of every node, as /api/jobs/node answers them', SITE_CASE.jobs_node, function (c) { return site.get(c.path); });
      })
      .then(function () {
        check('still one load for them all', jobsLoads.length === 1 && jobsLoads[0] === 'data/jobs.js');
        var by = {};
        SITE_CASE.jobs_node.forEach(function (c) { by[c.path] = c.want; });
        var all = SITE_CASE.jobs_node.map(function (c) { return c.want; });
        var any = function (f) { return all.some(f); };
        check('the cases hold a node a term reaches, one a step might run and a source that was not read', any(function (a) { return a.groups.some(function (g) { return g.steps.some(function (s) { return s.via && s.via.length; }); }); }) &&
          any(function (a) { return a.maybe && a.maybe.length; }) && any(function (a) { return a.not_read && a.refused; }) && any(function (a) { return a.groups.length === 0; }) && any(function (a) { return a.groups.length > 1; }), 'cases');
        // What the server leaves out when empty is left out, so a node with no job says only what the sources say.
        var bare = all.filter(function (a) { return a.groups.length === 0 && !a.maybe; })[0];
        check('a node no job runs has no maybe', bare === undefined || !('maybe' in bare));
        var first = site.get(SITE_CASE.jobs_node[0].path);
        var again = site.get(SITE_CASE.jobs_node[0].path);
        return Promise.all([first, again]).then(function (r) {
          check('a job is shipped once: two nodes the same job runs share it', canon(r[0]) === canon(r[1]));
          var shared = SITE_CASE.jobs_node.filter(function (c) { return c.want.groups.length; }).slice(0, 2);
          return Promise.all(shared.map(function (c) { return site.get(c.path); })).then(function (answers) {
            check('the same job under two nodes is the one object', answers[0].groups[0].job === answers[1].groups[0].job);
          });
        });
      });
  })
  .then(function () {
    // A file that fails to load is forgotten, a script that gave nothing is not ours, and two asks on its way are one load.
    var tries = 0;
    var flaky = EdithSite.make(SITE_CASE.site, function () {
      tries++;
      if (tries === 1) return Promise.reject(new Error('cannot load the jobs'));
      site = flaky;
      eval(SITE_CASE.jobs);
      site = main;
      return Promise.resolve();
    });
    var c = SITE_CASE.jobs_node[0];
    return flaky.get(c.path).then(function () { check('jobs that fail to load reject the answer', false); }, function (e) {
      check('jobs that fail to load reject the answer', e.message === 'cannot load the jobs' && !flaky.jobsHeld(), e.message);
      return flaky.get(c.path).then(function (a) { check('and are asked for again', tries === 2 && canon(a) === canon(c.want) && flaky.jobsHeld()); });
    });
  })
  .then(function () {
    var empty = EdithSite.make(SITE_CASE.site, function () { return Promise.resolve(); });
    return empty.get('/api/jobs').then(function () { check('an empty jobs file is refused', false); },
      function (e) { check('an empty jobs file is refused', /did not arrive/.test(e.message), e.message); });
  })
  .then(function () {
    var asked = 0;
    var slow = EdithSite.make(SITE_CASE.site, function () {
      asked++;
      return new Promise(function (resolve) { setTimeout(function () { site = slow; eval(SITE_CASE.jobs); site = main; resolve(); }, 1); });
    });
    return Promise.all([slow.get('/api/jobs'), slow.get(SITE_CASE.jobs_node[0].path)]).then(function (r) {
      check('an ask while the jobs are on their way waits for the same load', asked === 1 && canon(r[0]) === canon(SITE_CASE.jobs_summary.want) && canon(r[1]) === canon(SITE_CASE.jobs_node[0].want));
    });
  })
  .then(function () {
    return rejected('/api/jobs/node?id=model.shop.nothing', 'unknown node')
      .then(function () { return rejected('/api/jobs/node', 'unknown node'); })
      .then(function () { return rejected('/api/jobs/node?id=', 'unknown node'); });
  })
  .then(function () {
    // A jobs.yml that does not read: no source, the reason said by every answer, never a node with no job.
    var refused = EdithSite.make(SITE_CASE.site, function () { site = refused; eval(SITE_CASE.jobs_refused.script); site = main; return Promise.resolve(); });
    return group('a refused jobs.yml, as /api/jobs and /api/jobs/node answer it', [SITE_CASE.jobs_refused.summary, SITE_CASE.jobs_refused.node], function (c) { return refused.get(c.path); })
      .then(function () {
        var want = SITE_CASE.jobs_refused.node.want;
        check('it says why, and that nothing is configured', want.configured === false && typeof want.sources_refused === 'string' && want.groups.length === 0);
      });
  })
  .then(function () {
    // The link to a line of a job file, as the page builds it: its source by name, each segment escaped, the host's anchor.
    var link = EdithSite.jobFileUrl;
    var meta = { jobs: { sources: [
      { name: 'orchestration', base: 'https://github.com/acme/airflow/blob/abc1234/dags/', anchor: 'L' },
      { name: 'bb', base: 'https://bitbucket.org/acme/airflow/src/abc1234/', anchor: 'lines-' },
      { name: 'azure', base: null, anchor: 'L' },
      { name: 'twin', base: 'https://h/a/', anchor: 'L' }, { name: 'twin', base: 'https://h/b/', anchor: 'L' },
      { name: 'odd', base: 'http://h/x/', anchor: 'L' }] } };
    check('a line is under the source\'s base, with the host\'s anchor', link(meta, 'orchestration', 'a b/c#d.yml', 12) === 'https://github.com/acme/airflow/blob/abc1234/dags/a%20b/c%23d.yml#L12');
    check('Bitbucket says lines-', link(meta, 'bb', 'x.yml', 3) === 'https://bitbucket.org/acme/airflow/src/abc1234/x.yml#lines-3');
    check('no line, no anchor', link(meta, 'orchestration', 'x.yml', 0) === 'https://github.com/acme/airflow/blob/abc1234/dags/x.yml');
    check('a source with no file address is no link, nor is an unknown one or two of one name', link(meta, 'azure', 'x.yml', 1) === '' && link(meta, 'nothing', 'x.yml', 1) === '' && link(meta, 'twin', 'x.yml', 1) === '');
    check('a base that is not https is no link', link(meta, 'odd', 'x.yml', 1) === '');
    check('a path that goes up or is empty is no link', link(meta, 'orchestration', '../x.yml', 1) === '' && link(meta, 'orchestration', 'a//b.yml', 1) === '' && link(meta, 'orchestration', '', 1) === '');
    check('a site with no jobs has no link', link({}, 'orchestration', 'x.yml', 1) === '' && link(null, 'orchestration', 'x.yml', 1) === '');
  })
  .then(function () {
    // The pieces the walks lean on.
    var r = EdithSite;
    check('Rust trims U+0085 and not U+FEFF', r.rustTrim('\u0085 a  ') === 'a' && r.rustTrim('﻿a') === '﻿a');
    check('a string\'s length is its UTF-8 bytes', r.utf8Len('a') === 1 && r.utf8Len('é') === 2 && r.utf8Len('€') === 3 && r.utf8Len('😀') === 4 && r.utf8Len('café') === 5);
    check('strings are compared by code point, not by UTF-16 unit', r.cmpCodePoints('Ａ', '😀') < 0 && 'Ａ' > '😀');
    check('a prefix sorts before what extends it', r.cmpCodePoints('ab', 'abc') < 0 && r.cmpCodePoints('abc', 'ab') > 0 && r.cmpCodePoints('ab', 'ab') === 0);
    var q = r.parseQuery('id=model.a.b&q=a+b%20c&flag&empty=&kind=model%2Csource');
    check('a query is read as the server reads it', q.id === 'model.a.b' && q.q === 'a b c' && q.flag === '' && q.empty === '' && q.kind === 'model,source');
    check('a query with a bad escape is kept as written', r.parseQuery('q=%E0%A4%A').q === '%E0%A4%A');
    check('an unknown kind is dropped, as the server drops it', r.parseKinds('model,,nonsense,test,unit_test').join() === 'model,test');
    check('no kind at all is every kind', r.parseKinds('').length === 0 && r.parseKinds('bogus').length === 0);
    check('the limits are Rust\'s own', SITE_CASE.site.limits.max_levels === 20 && SITE_CASE.site.limits.depth === 2 && SITE_CASE.site.limits.max === 400 &&
      SITE_CASE.site.limits.max_max === 3000 && SITE_CASE.site.limits.search === 200 && SITE_CASE.site.limits.search_max === 200000);
    check('with no page there is the factory and the pure helpers, and nothing that needs data', typeof EdithSite.make === 'function' && typeof EdithSite.fileUrl === 'function' && typeof EdithSite.jobFileUrl === 'function' && typeof EdithSite.get === 'undefined' && typeof window === 'undefined');
  })
  .catch(function (e) { print('FAIL  the harness threw ' + e + (e && e.stack ? '\n' + e.stack : '')); });
