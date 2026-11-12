// The static site's answers: web/site.js held to what Rust answers, on an invented graph.
// tests/fixtures/site_case.js is written by `DBT_EDITH_BLESS=1 cargo test site_case` and holds
// /api/lineage (every node one level each way, a few at every depth, with the caps of the page and a
// small one), /api/search and /api/node as the server gives them; this makes the same requests of
// EdithSite and compares what comes back, key for key.
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
var site;
function EDITH_SITE_CARDS(start, list) { site.received(start, list); }
site = EdithSite.make(SITE_CASE.site, function (k, url) {
  loads.push(url);
  eval(SITE_CASE.buckets[k]);
  return Promise.resolve();
});

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
  .then(function () { return rejected('/api/jobs?look=true', 'not in this site'); })
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
    check('with no page there is the factory and nothing else', typeof EdithSite.make === 'function' && typeof EdithSite.get === 'undefined' && typeof window === 'undefined');
  })
  .catch(function (e) { print('FAIL  the harness threw ' + e + (e && e.stack ? '\n' + e.stack : '')); });
