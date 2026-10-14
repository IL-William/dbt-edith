// Which surface draws over which: a toast is often the answer to a button in
// a dialog, the tour or the shortcut list, and under their backdrop it reads as
// nothing having happened. It takes no clicks, since it now covers them.
// Run from the repository root: jsc web/tests/stacking.js
var css = read('web/app.css');

function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }
function rule(selector) {
  var at = css.indexOf('\n' + selector + ' {');
  return at < 0 ? '' : css.slice(at, css.indexOf('}', at));
}
function z(selector) {
  var m = /z-index:\s*(\d+)/.exec(rule(selector));
  return m ? +m[1] : NaN;
}

['#modal', '#tour', '#shortcuts', '#palette'].forEach(function (s) {
  ok('a toast draws over ' + s, z('#toast') > z(s));
});
ok('and takes no click', /pointer-events:\s*none/.test(rule('#toast')));
