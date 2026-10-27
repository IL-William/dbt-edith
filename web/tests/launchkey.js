// The launch key: taken from the printed link, kept per origin only once the
// server has accepted it, taken out of the address bar, sent as a header, and
// swapped for a newer one another tab stored when this one's is refused.
// JavaScriptCore has no URLSearchParams or fetch, so small ones stand in, as
// do location, history and localStorage.
// Run from the repository root: jsc web/tests/launchkey.js
var URLSearchParams = function (search) {
  this.pairs = search.replace(/^\?/, '').split('&').filter(Boolean).map(function (p) { return p.split('='); });
};
URLSearchParams.prototype.get = function (k) {
  for (var i = 0; i < this.pairs.length; i++) if (this.pairs[i][0] === k) return this.pairs[i][1];
  return null;
};
URLSearchParams.prototype.delete = function (k) { this.pairs = this.pairs.filter(function (p) { return p[0] !== k; }); };
URLSearchParams.prototype.toString = function () { return this.pairs.map(function (p) { return p.join('='); }).join('&'); };

var location = { search: '?key=abc123&tab=git', pathname: '/', hash: '#top' };
var shown = [];
var history = { replaceState: function (a, b, url) { shown.push(url); } };
var store = {};
var localStorage = {
  setItem: function (k, v) { store[k] = v; },
  getItem: function (k) { return k in store ? store[k] : null; },
};
// The server of the moment: the one key it holds, and what it was asked with.
var serverKey = 'abc123';
var asked = [];
var fetch = function (path, init) {
  var sent = (init.headers || {}).authorization || '';
  asked.push(sent);
  var ok = sent === 'Bearer ' + serverKey;
  return Promise.resolve({ ok: ok, status: ok ? 200 : 401 });
};

var KEY_STORE = 'dbt-edith-key';
var launchKey = '';
var keyPending = '';
var src = read('web/app.js');
eval(src.slice(src.indexOf('function storedKey'), src.indexOf('async function refusal')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function again(search) { location.search = search; shown = []; asked = []; return takeLaunchKey(); }

print('--- the printed link ---');
check('the key was taken from the address', launchKey, 'abc123');
check('the address loses it and keeps the rest', shown, ['/?tab=git#top']);
check('not kept until the server has said yes', store['dbt-edith-key'], undefined);
check('every request carries it as a bearer header', keyHeaders(), { authorization: 'Bearer abc123' });

request('/api/meta').then(function (r) {
  check('the server accepted it', r.ok, true);
  check('and now it is kept for this origin', store['dbt-edith-key'], 'abc123');

  print('\n--- a new tab or a reload, with no key in the address ---');
  check('the kept key is used', again(''), 'abc123');
  check('and the address is left alone', shown, []);

  print('\n--- a stale link, from an earlier launch ---');
  again('?key=old000');
  check('read from the address', launchKey, 'old000');
  return request('/api/meta');
}).then(function (r) {
  check('the server refuses it, and the page retries with the one kept', r.ok, true);
  check('asked once with each', asked, ['Bearer old000', 'Bearer abc123']);
  check('the stale key replaced nothing', store['dbt-edith-key'], 'abc123');
  check('and is not the one in use any more', launchKey, 'abc123');

  print('\n--- a tab left open across a restart in another tab ---');
  serverKey = 'new999';
  store['dbt-edith-key'] = 'new999';
  asked = [];
  return request('/api/meta');
}).then(function (r) {
  check('the refused key gives way to the newer one stored', r.ok, true);
  check('asked once with each', asked, ['Bearer abc123', 'Bearer new999']);

  print('\n--- nothing newer to take ---');
  store = {};
  serverKey = 'unknown';
  asked = [];
  return request('/api/meta');
}).then(function (r) {
  check('a refusal stands, with one try only', [r.status, asked.length], [401, 1]);

  print('\n--- a good link over an older key kept ---');
  serverKey = 'fresh01';
  store['dbt-edith-key'] = 'old000';
  again('?key=fresh01');
  return request('/api/meta');
}).then(function () {
  check('replaces it once accepted', store['dbt-edith-key'], 'fresh01');

  print('\n--- storage refused ---');
  localStorage = {
    setItem: function () { throw new Error('blocked'); },
    getItem: function () { throw new Error('blocked'); },
  };
  serverKey = 'blocked1';
  again('?key=blocked1');
  return request('/api/meta');
}).then(function (r) {
  check('the key still works from memory', r.ok, true);
  check('and none is found on a reload, so no header would be sent', again(''), '');
}).catch(function (e) { print('FAIL  ' + e); });
