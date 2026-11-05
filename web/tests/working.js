// Edith's mark moves while she waits on something slow: which requests count,
// and when the mark starts, fades and stops, on a clock of its own.
// Run from the repository root: jsc web/tests/working.js
var src = read('web/app.js');
var css = read('web/app.css');
var html = read('web/index.html');

var now = 0, timers = [], nextId = 1;
var setTimeout = function (fn, ms) { var id = nextId++; timers.push({ id: id, at: now + ms, fn: fn }); return id; };
var clearTimeout = function (id) { timers = timers.filter(function (t) { return t.id !== id; }); };
function advance(ms) {
  var end = now + ms;
  for (;;) {
    timers.sort(function (a, b) { return a.at - b.at; });
    var t = timers[0];
    if (!t || t.at > end) break;
    timers.shift();
    now = t.at;
    t.fn();
  }
  now = end;
}
var classes = {};
var brand = { classList: {
  add: function () { for (var i = 0; i < arguments.length; i++) classes[arguments[i]] = true; },
  remove: function () { for (var i = 0; i < arguments.length; i++) delete classes[arguments[i]]; },
  contains: function (c) { return !!classes[c]; },
} };
var document = { getElementById: function (id) { return id === 'brand' ? brand : null; } };
function state() { return (classes.working ? 'working' : 'still') + (classes.flowing ? ', flowing' : ''); }

var S = { collin: null };
eval(src.slice(src.indexOf('const WORK_ROUTES'), src.indexOf('async function request')));
eval(src.slice(src.indexOf('async function request'), src.indexOf('const api')));
// What request() leans on, stubbed: the answer is set by each check below.
var fetchCalls = 0, fetchAnswers = [], refusals = 0, accepted = 0;
function keyHeaders() { return {}; }
function keyRefused() { return refusals-- > 0; }
function keyAccepted() { accepted++; }
var fetch = function () {
  fetchCalls++;
  var a = fetchAnswers.shift();
  return a instanceof Error ? Promise.reject(a) : Promise.resolve(a);
};
// A const declared in eval stays inside it; the functions it declares do not.
var WORK_DELAY = +/const WORK_DELAY = (\d+)/.exec(src)[1];
var WORK_FADE = +/const WORK_FADE = (\d+)/.exec(src)[1];

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }

print('--- which requests are work ---');
check('reading the manifest again', isWork('/api/reload'), true);
check('a column\'s lineage, which may run collin first', isWork('/api/collineage?id=model.x&column=amount&up=2&down=2'), true);
check('collin run from the settings', isWork('/api/collineage/generate'), true);
check('a column fetched from Snowflake', isWork('/api/collineage/fetch'), true);
check('the query history', isWork('/api/snowflake/history'), true);
check('the last builds', isWork('/api/snowflake/builds'), true);
check('two collin versions compared', isWork('/api/collin/compare?a=0.11.0&b=0.12.0'), true);
check('a push', isWork('/api/git/push'), true);
check('a pull', isWork('/api/git/pull'), true);
check('a fetch', isWork('/api/git/fetch'), true);
check('the release check', isWork('/api/update/check'), true);
check('not a hover\'s column reach', isWork('/api/colreach?id=model.x'), false);
check('not a search, however slow', isWork('/api/search?q=ord'), false);
check('not the git status a poll reads', isWork('/api/git'), false);
check('not a commit, which stays on the machine', isWork('/api/git/commit'), false);
check('not the update status the page reads on its own', isWork('/api/update'), false);
check('not a route that only starts like one', isWork('/api/reloaded'), false);

print('\n--- when the mark moves ---');
workBegan();
advance(WORK_DELAY - 1);
check('still until the delay has passed', state(), 'still');
advance(1);
check('moving once it has', state(), 'working, flowing');
workEnded();
check('the packets fade when the work ends', state(), 'working');
advance(WORK_FADE);
check('and the loop stops once they have', state(), 'still');
check('no timer left behind', timers.length, 0);

workBegan();
advance(WORK_DELAY - 100);
workEnded();
check('an answer within the delay never sets it moving', state(), 'still');
advance(WORK_DELAY + WORK_FADE);
check('nor does it later', state(), 'still');

workBegan();
advance(WORK_DELAY + 500);
workBegan();
workEnded();
check('two at once: it keeps moving while one is out', state(), 'working, flowing');
workEnded();
advance(WORK_FADE);
check('and stops with the last', state(), 'still');

workBegan();
advance(WORK_DELAY);
workEnded();
advance(WORK_FADE / 2);
workBegan();
check('work again while it fades: the loop is not taken away', state(), 'working');
advance(WORK_DELAY);
check('and the packets come back past the delay', state(), 'working, flowing');
workEnded();
advance(WORK_FADE);
check('then it stops as usual', state(), 'still');

workBegan();
advance(WORK_DELAY);
workEnded();
advance(WORK_FADE / 2);
workBegan();
advance(WORK_DELAY - 100);
workEnded();
check('a short request during the fade: the packets are not cut short', state(), 'working');
advance(WORK_FADE - 1);
check('the fade still has its time', state(), 'working');
advance(1);
check('and runs out on its own timer', state(), 'still');
check('no timer left behind after it', timers.length, 0);

// Each step is what S.collin becomes, then followCollinWork() as the page calls it.
function collinIs(st) { S.collin = st === null ? null : { state: st }; followCollinWork(); }
print('\n--- a collin run counts as work ---');
collinIs('running');
advance(WORK_DELAY);
check('a run the page only hears of by polling moves the mark', state(), 'working, flowing');
collinIs('done');
check('its end lets the packets fade', state(), 'working');
advance(WORK_FADE);
check('and the loop stops', state(), 'still');

collinIs('running');
collinIs('running');
collinIs('running');
advance(WORK_DELAY);
collinIs('failed');
advance(WORK_FADE);
check('running said three times is one count: a failure ends it', state(), 'still');

collinIs('running');
advance(WORK_DELAY);
collinIs(null);
advance(WORK_FADE);
check('S.collin forgotten after a failed regenerate ends it', state(), 'still');

collinIs('done');
collinIs(null);
check('no run, no count to end', state(), 'still');
check('no timer left behind by the runs', timers.length, 0);

workBegan();
collinIs('running');
advance(WORK_DELAY);
workEnded();
check('a request ending while a run goes on leaves it moving', state(), 'working, flowing');
collinIs('done');
advance(WORK_FADE);
check('and the run ending stops it', state(), 'still');

print('\n--- the drawing ---');
ok('the mark is in the brand button, before the name, with a packet and a box per arm',
  /<button id="brand"[^>]*>[\s\S]*?<svg class="brandmk"[\s\S]*?<\/svg><b>/.test(html)
  && (html.match(/class="bm-pk b[012]"/g) || []).length === 3 && (html.match(/class="bm-to b[012]"/g) || []).length === 3);
ok('the button keeps her name as its label, the i being dotless', /id="brand"[^>]*aria-label="Edith"/.test(html));
ok('the packets run only while she works', /\.brand\.working \.bm-pk \{ animation: bm-pk/.test(css) && !/^\.bm-pk \{[^}]*animation/m.test(css));
var boxRule = /\.brand\.working \.bm-to \{([^}]*)\}/.exec(css);
ok('the boxes light in step with the packets: same delays on .b1 and .b2, the box rule setting none of its own',
  /\.brand\.working \.b1 \{ animation-delay: -\.6s; \}/.test(css) && /\.brand\.working \.b2 \{ animation-delay: -1\.2s; \}/.test(css)
  && !!boxRule && /animation-name: bm-lit/.test(boxRule[1]) && !/animation-delay|animation:/.test(boxRule[1]));
ok('no box is lit while no packet runs, whatever its animation says',
  /\.brand\.working:not\(\.flowing\) \.bm-to \{ fill: var\(--fg-dim\) !important; \}/.test(css));
ok('still under reduced motion, the arms in the accent instead',
  /@media \(prefers-reduced-motion: reduce\) \{\s*\.brand\.working \.bm-pk, \.brand\.working \.bm-to \{ animation: none; \}[\s\S]*?\.brand\.flowing \.bm-arm \{[^}]*stroke: var\(--accent\)/.test(css));
ok('the fade in the stylesheet is the one the page waits for',
  new RegExp('\\.bm-pk \\{[^}]*transition: opacity ' + String(WORK_FADE / 1000).replace(/^0\./, '\\.') + 's').test(css));

print('\n--- request() hands the mark back ---');
async function checkRequest() {
  var boom = new Error('connection refused');
  fetchAnswers = [boom];
  var got = null;
  try { await request('/api/reload'); } catch (e) { got = e; }
  check('a failed fetch: the error reaches the caller', got === boom, true);
  advance(WORK_DELAY + WORK_FADE);
  // The count is a let inside the eval, out of reach: a leaked one would show
  // as a mark that starts after the delay and never stops.
  check('the mark is still once the timers have run', state(), 'still');
  check('no timer left behind after the failure', timers.length, 0);

  fetchAnswers = [{ status: 401, ok: false }, { status: 200, ok: true }];
  fetchCalls = 0; refusals = 1; accepted = 0;
  var r = await request('/api/reload');
  check('a refused key retried: the second answer comes back', r.status, 200);
  check('the request was sent twice', fetchCalls, 2);
  check('the key was accepted once', accepted, 1);
  advance(WORK_DELAY + WORK_FADE);
  check('the mark is still after the retry', state(), 'still');

  var begun = 0, ended = 0;
  var wb = workBegan, we = workEnded;
  workBegan = function () { begun++; wb(); };
  workEnded = function () { ended++; we(); };
  fetchAnswers = [{ status: 401, ok: false }, { status: 200, ok: true }];
  refusals = 1;
  await request('/api/reload');
  check('began once over the retry', begun, 1);
  check('ended once over the retry', ended, 1);
  fetchAnswers = [{ status: 200, ok: true }];
  await request('/api/search?q=ord');
  check('a route that is not work touches neither', [begun, ended], [1, 1]);
  workBegan = wb; workEnded = we;
}
checkRequest().then(function () {}, function (e) { print('FAIL  request() checks threw: ' + e); });
