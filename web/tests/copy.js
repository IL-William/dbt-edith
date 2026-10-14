// The clipboard: what copyText answers, and the glyph a copy button shows.
// Run from the repository root: jsc web/tests/copy.js
var src = read('web/app.js');
var fn = src.slice(src.indexOf('async function copyText'), src.indexOf('function selectKindCounts'));

// Stubs: a clipboard that answers when told to, the toasts, the timers, and a
// button that keeps its click handler so a click can be awaited.
var toasts = [];
function toast(m) { toasts.push(m); }
var pending = [];
var navigator = { clipboard: { writeText: function (t) {
  return new Promise(function (ok, no) { pending.push({ text: t, ok: ok, no: no }); });
} } };
var timers = [];
function setTimeout(f) { timers.push(f); return timers.length; }
function clearTimeout(id) { if (id) timers[id - 1] = null; }
var document = { createElement: function () {
  return { attrs: {}, setAttribute: function (k, v) { this.attrs[k] = v; },
           addEventListener: function (e, f) { this.click = f; } };
} };
eval(fn);

function check(label, got, want) {
  print((got === want ? 'PASS  ' : 'FAIL  ') + label);
  if (got !== want) print('        expected: ' + want + '\n        got     : ' + got);
}
function glyph(b) { return b.innerHTML.indexOf('<rect') >= 0 ? 'copy' : b.innerHTML.indexOf('M3.5 8.5') >= 0 ? 'check' : '?'; }
function last() { return pending[pending.length - 1]; }
function runTimers() { var t = timers; timers = []; t.forEach(function (f) { if (f) f(); }); }

var q = Promise.resolve();
function step(f) { q = q.then(f); }

step(function () {
  print('--- copyText ---');
  var p = copyText('customers');
  check('hands the text to the clipboard', last().text, 'customers');
  last().ok();
  return p.then(function (r) {
    check('says true once the clipboard took it', r, true);
    check('  and the toast names what was copied', toasts.pop(), 'copied customers');
  });
});
step(function () {
  var p = copyText('select 1', 'the SQL');
  last().ok();
  return p.then(function () { check('a label names it in the toast instead', toasts.pop(), 'copied the SQL'); });
});
step(function () {
  var p = copyText('customers');
  last().no(new Error('denied'));
  return p.then(function (r) {
    check('says false when the browser refuses', r, false);
    check('  and the toast says so', toasts.pop(), 'clipboard unavailable');
  });
});
step(function () {
  var kept = navigator.clipboard;
  navigator.clipboard = undefined;
  return copyText('customers').then(function (r) {
    navigator.clipboard = kept;
    check('says false with no clipboard at all', r, false);
    check('  and the toast says so', toasts.pop(), 'clipboard unavailable');
  });
});

var b;
step(function () {
  print('\n--- copyGlyph ---');
  b = copyGlyph('jaffle_shop.raw_orders', 'Copy the name');
  check('a button that submits nothing', b.type, 'button');
  check('styled as the other icon buttons', b.className, 'icon');
  check('its tooltip', b.title, 'Copy the name');
  check('read out with what it copies', b.attrs['aria-label'], 'Copy jaffle_shop.raw_orders');
  check('shows the copy glyph', glyph(b), 'copy');
  var p = b.click();
  check('a click copies the text it was made with', last().text, 'jaffle_shop.raw_orders');
  last().ok();
  return p.then(function () { check('a copy that went through shows the check', glyph(b), 'check'); });
});
step(function () {
  runTimers();
  check('  then the copy glyph again', glyph(b), 'copy');
});
step(function () {
  var p = b.click();
  last().no(new Error('denied'));
  return p.then(function () { check('a refused copy shows no check', glyph(b), 'copy'); });
});
step(function () {
  // The first click's check is up, with its timer, when a second click is refused.
  var p1 = b.click();
  last().ok();
  return p1.then(function () {
    check('a first click shows the check', glyph(b), 'check');
    var p2 = b.click();
    check('a second click takes it down at once', glyph(b), 'copy');
    last().no(new Error('denied'));
    return p2;
  }).then(function () {
    check('  and its refusal leaves no check', glyph(b), 'copy');
    check('  nor the first click\'s timer', timers.filter(Boolean).length, 0);
  });
});
step(function () {
  // Two clicks in flight: the second is refused, then the first goes through.
  var p1 = b.click(), first = last();
  var p2 = b.click(), second = last();
  second.no(new Error('denied'));
  return p2.then(function () {
    first.ok();
    return p1;
  }).then(function () {
    check('an earlier click that answers late shows no check over a later refusal', glyph(b), 'copy');
    check('  and starts no timer', timers.filter(Boolean).length, 0);
  });
});

// jsc says nothing about a rejected promise and exits 0, so a step that throws
// has to say so itself, or the checks before it would vouch for the file.
q.then(function () {}, function (e) { print('FAIL  a step threw ' + e); });
