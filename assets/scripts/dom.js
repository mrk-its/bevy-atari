/* dom utilies */

function domReady(fn) {
	document.addEventListener("DOMContentLoaded", fn);
	if (document.readyState === "interactive" || document.readyState === "complete") {
		fn();
	}
}

function s(sel) {
	return document.querySelector(sel);
}

function insert(el, html) {
	if (typeof el === "string" || el instanceof String) {
		el = s(el);
	} 
	el.insertAdjacentHTML("beforeend", html);
}

function show(sel, disp) {
	if (disp == undefined) disp = "block";
	s(sel).style.display = disp;
}

function hide(sel) {
	s(sel).style.display = "none";
}

function toggle(el) {
	if (window.getComputedStyle(el).display === "block") {
		el.style.display = "none";
		return;
	}
	el.style.display = "block";
}

