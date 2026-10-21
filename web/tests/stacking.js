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

['#modal', '#tour', '#shortcuts', '#palette'].forEach(function (s) {
  ok('a notice card draws over ' + s, z('.notice') > z(s));
});
ok('and is a card, not a backdrop: it does not cover the page', !/inset:\s*0/.test(rule('.notice')));

var app = read('web/app.js');
var closer = app.slice(app.indexOf('const onKey = (e) => {\n    if (e.key !== \'Escape\') return;'));
closer = closer.slice(0, closer.indexOf('document.addEventListener'));
ok('Escape is left alone outside the terminal, the card and the bare page',
  /if \(!inTerm && t !== document\.body && !card\.contains\(t\)\) return;\s+closeNotice\(\)/.test(closer));
ok('an Escape the card takes is marked taken, so the find bar and the panel keep theirs',
  /closeNotice\(\);[\s\S]*?\n    e\.preventDefault\(\);/.test(closer));
ok('and in the terminal it goes no further', /if \(inTerm\) e\.stopPropagation\(\)/.test(closer));
ok('a click on the card keeps the focus where it was', /card\.addEventListener\('mousedown', \(e\) => e\.preventDefault\(\)\)/.test(app));
