//#region ../phemios/node_modules/solid-js/dist/solid.js
var e = {
	context: void 0,
	registry: void 0,
	effects: void 0,
	done: !1,
	getContextId() {
		return t(this.context.count);
	},
	getNextContextId() {
		return t(this.context.count++);
	}
};
function t(t) {
	let n = String(t), r = n.length - 1;
	return e.context.id + (r ? String.fromCharCode(96 + r) : "") + n;
}
function n(t) {
	e.context = t;
}
var r = (e, t) => e === t, i = Symbol("solid-proxy"), a = typeof Proxy == "function", o = Symbol("solid-track"), s = { equals: r }, c = null, l = pe, u = 1, d = 2, f = {
	owned: null,
	cleanups: null,
	context: null,
	owner: null
}, p = null, m = null, h = null, g = null, _ = null, v = 0;
function y(e, t) {
	let n = h, r = p, i = e.length === 0, a = t === void 0 ? r : t, o = i ? f : {
		owned: null,
		cleanups: null,
		context: a ? a.context : null,
		owner: a
	}, s = i ? e : () => e(() => C(() => O(o)));
	p = o, h = null;
	try {
		return E(s, !0);
	} finally {
		h = n, p = r;
	}
}
function b(e, t) {
	t = t ? Object.assign({}, s, t) : s;
	let n = {
		value: e,
		observers: null,
		observerSlots: null,
		comparator: t.equals || void 0
	};
	return [ce.bind(n), (e) => (typeof e == "function" && (e = m && m.running && m.sources.has(n) ? e(n.tValue) : e(n.value)), le(n, e))];
}
function x(e, t, n) {
	w(de(e, t, !1, u));
}
function ee(e, t, n) {
	l = me;
	let r = de(e, t, !1, u), i = se && ae(se);
	i && (r.suspense = i), (!n || !n.render) && (r.user = !0), _ ? _.push(r) : w(r);
}
function S(e, t, n) {
	n = n ? Object.assign({}, s, n) : s;
	let r = de(e, t, !0, 0);
	return r.observers = null, r.observerSlots = null, r.comparator = n.equals || void 0, w(r), ce.bind(r);
}
function C(e) {
	if (h === null) return e();
	let t = h;
	h = null;
	try {
		return e();
	} finally {
		h = t;
	}
}
function te(e) {
	return p === null || (p.cleanups === null ? p.cleanups = [e] : p.cleanups.push(e)), e;
}
var [ne, re] = /*@__PURE__*/ b(!1);
function ie(e, t) {
	let n = Symbol("context");
	return {
		id: n,
		Provider: xe(n),
		defaultValue: e
	};
}
function ae(e) {
	let t;
	return p && p.context && (t = p.context[e.id]) !== void 0 ? t : e.defaultValue;
}
function oe(e) {
	let t = S(e), n = S(() => be(t()));
	return n.toArray = () => {
		let e = n();
		return Array.isArray(e) ? e : e == null ? [] : [e];
	}, n;
}
var se;
function ce() {
	let e = m && m.running;
	if (this.sources && (e ? this.tState : this.state)) {
		if ((e ? this.tState : this.state) === u) w(this);
		else {
			let e = g;
			g = null, E(() => D(this), !1), g = e;
		}
	}
	if (h) {
		let e = this.observers;
		if (!e || e[e.length - 1] !== h) {
			let t = e ? e.length : 0;
			h.sources ? (h.sources.push(this), h.sourceSlots.push(t)) : (h.sources = [this], h.sourceSlots = [t]), e ? (e.push(h), this.observerSlots.push(h.sources.length - 1)) : (this.observers = [h], this.observerSlots = [h.sources.length - 1]);
		}
	}
	return e && m.sources.has(this) ? this.tValue : this.value;
}
function le(e, t, n) {
	let r = m && m.running && m.sources.has(e) ? e.tValue : e.value;
	if (!e.comparator || !e.comparator(r, t)) {
		if (m) {
			let r = m.running;
			(r || !n && m.sources.has(e)) && (m.sources.add(e), e.tValue = t), r || (e.value = t);
		} else e.value = t;
		e.observers && e.observers.length && E(() => {
			for (let t = 0; t < e.observers.length; t += 1) {
				let n = e.observers[t], r = m && m.running;
				r && m.disposed.has(n) || ((r ? !n.tState : !n.state) && (n.pure ? g.push(n) : _.push(n), n.observers && he(n)), r ? n.tState = u : n.state = u);
			}
			if (g.length > 1e6) throw g = [], Error();
		}, !1);
	}
	return t;
}
function w(e) {
	if (!e.fn) return;
	O(e);
	let t = v;
	ue(e, m && m.running && m.sources.has(e) ? e.tValue : e.value, t), m && !m.running && m.sources.has(e) && queueMicrotask(() => {
		E(() => {
			m && (m.running = !0), h = p = e, ue(e, e.tValue, t), h = p = null;
		}, !1);
	});
}
function ue(e, t, n) {
	let r, i = p, a = h;
	h = p = e;
	try {
		r = e.fn(t);
	} catch (t) {
		return e.pure && (m && m.running ? (e.tState = u, e.tOwned && e.tOwned.forEach(O), e.tOwned = void 0) : (e.state = u, e.owned && e.owned.forEach(O), e.owned = null)), e.updatedAt = n + 1, ye(t);
	} finally {
		h = a, p = i;
	}
	(!e.updatedAt || e.updatedAt <= n) && (e.updatedAt != null && "observers" in e ? le(e, r, !0) : m && m.running && e.pure ? (m.sources.has(e) || (e.value = r), m.sources.add(e), e.tValue = r) : e.value = r, e.updatedAt = n);
}
function de(e, t, n, r = u, i) {
	let a = {
		fn: e,
		state: r,
		updatedAt: null,
		owned: null,
		sources: null,
		sourceSlots: null,
		cleanups: null,
		value: t,
		owner: p,
		context: p ? p.context : null,
		pure: n
	};
	return m && m.running && (a.state = 0, a.tState = r), p === null || p !== f && (m && m.running && p.pure ? p.tOwned ? p.tOwned.push(a) : p.tOwned = [a] : p.owned ? p.owned.push(a) : p.owned = [a]), a;
}
function T(e) {
	let t = m && m.running;
	if ((t ? e.tState : e.state) === 0) return;
	if ((t ? e.tState : e.state) === d) return D(e);
	if (e.suspense && C(e.suspense.inFallback)) return e.suspense.effects.push(e);
	let n = [e];
	for (; (e = e.owner) && (!e.updatedAt || e.updatedAt < v);) {
		if (t && m.disposed.has(e)) return;
		(t ? e.tState : e.state) && n.push(e);
	}
	for (let r = n.length - 1; r >= 0; r--) {
		if (e = n[r], t) {
			let t = e, i = n[r + 1];
			for (; (t = t.owner) && t !== i;) if (m.disposed.has(t)) return;
		}
		if ((t ? e.tState : e.state) === u) w(e);
		else if ((t ? e.tState : e.state) === d) {
			let t = g;
			g = null, E(() => D(e, n[0]), !1), g = t;
		}
	}
}
function E(e, t) {
	if (g) return e();
	let n = !1;
	t || (g = []), _ ? n = !0 : _ = [], v++;
	try {
		let t = e();
		return fe(n), t;
	} catch (e) {
		n || (_ = null), g = null, ye(e);
	}
}
function fe(e) {
	if (g &&= (pe(g), null), e) return;
	let t;
	if (m) {
		if (!m.promises.size && !m.queue.size) {
			let e = m.sources, n = m.disposed;
			_.push.apply(_, m.effects), t = m.resolve;
			for (let e of _) "tState" in e && (e.state = e.tState), delete e.tState;
			m = null, E(() => {
				for (let e of n) O(e);
				for (let t of e) {
					if (t.value = t.tValue, t.owned) for (let e = 0, n = t.owned.length; e < n; e++) O(t.owned[e]);
					t.tOwned && (t.owned = t.tOwned), delete t.tValue, delete t.tOwned, t.tState = 0;
				}
				re(!1);
			}, !1);
		} else if (m.running) {
			m.running = !1, m.effects.push.apply(m.effects, _), _ = null, re(!0);
			return;
		}
	}
	let n = _;
	_ = null, n.length && E(() => l(n), !1), t && t();
}
function pe(e) {
	for (let t = 0; t < e.length; t++) T(e[t]);
}
function me(t) {
	let r, i = 0;
	for (r = 0; r < t.length; r++) {
		let e = t[r];
		e.user ? t[i++] = e : T(e);
	}
	if (e.context) {
		if (e.count) {
			e.effects ||= [], e.effects.push(...t.slice(0, i));
			return;
		}
		n();
	}
	for (e.effects && (e.done || !e.count) && (t = [...e.effects, ...t], i += e.effects.length, delete e.effects), r = 0; r < i; r++) T(t[r]);
}
function D(e, t) {
	let n = m && m.running;
	n ? e.tState = 0 : e.state = 0;
	for (let r = 0; r < e.sources.length; r += 1) {
		let i = e.sources[r];
		if (i.sources) {
			let e = n ? i.tState : i.state;
			e === u ? i !== t && (!i.updatedAt || i.updatedAt < v) && T(i) : e === d && D(i, t);
		}
	}
}
function he(e) {
	let t = m && m.running;
	for (let n = 0; n < e.observers.length; n += 1) {
		let r = e.observers[n];
		(t ? !r.tState : !r.state) && (t ? r.tState = d : r.state = d, r.pure ? g.push(r) : _.push(r), r.observers && he(r));
	}
}
function O(e) {
	let t;
	if (e.sources) for (; e.sources.length;) {
		let t = e.sources.pop(), n = e.sourceSlots.pop(), r = t.observers;
		if (r && r.length) {
			let e = r.pop(), i = t.observerSlots.pop();
			n < r.length && (e.sourceSlots[i] = n, r[n] = e, t.observerSlots[n] = i);
		}
	}
	if (e.tOwned) {
		for (t = e.tOwned.length - 1; t >= 0; t--) O(e.tOwned[t]);
		delete e.tOwned;
	}
	if (m && m.running && e.pure) ge(e, !0);
	else if (e.owned) {
		for (t = e.owned.length - 1; t >= 0; t--) O(e.owned[t]);
		e.owned = null;
	}
	if (e.cleanups) {
		for (t = e.cleanups.length - 1; t >= 0; t--) e.cleanups[t]();
		e.cleanups = null;
	}
	m && m.running ? e.tState = 0 : e.state = 0;
}
function ge(e, t) {
	if (t || (e.tState = 0, m.disposed.add(e)), e.owned) for (let t = 0; t < e.owned.length; t++) ge(e.owned[t]);
}
function _e(e) {
	return e instanceof Error ? e : Error(typeof e == "string" ? e : "Unknown error", { cause: e });
}
function ve(e, t, n) {
	try {
		for (let n of t) n(e);
	} catch (e) {
		ye(e, n && n.owner || null);
	}
}
function ye(e, t = p) {
	let n = c && t && t.context && t.context[c], r = _e(e);
	if (!n) throw r;
	_ ? _.push({
		fn() {
			ve(r, n, t);
		},
		state: u
	}) : ve(r, n, t);
}
function be(e) {
	if (typeof e == "function" && !e.length) return be(e());
	if (Array.isArray(e)) {
		let t = [];
		for (let n = 0; n < e.length; n++) {
			let r = be(e[n]);
			if (Array.isArray(r)) {
				if (r.length < 32768) t.push.apply(t, r);
				else for (let e = 0; e < r.length; e++) t.push(r[e]);
			} else t.push(r);
		}
		return t;
	}
	return e;
}
function xe(e, t) {
	return function(t) {
		let n;
		return x(() => n = C(() => (p.context = {
			...p.context,
			[e]: t.value
		}, oe(() => t.children))), void 0), n;
	};
}
var Se = Symbol("fallback");
function Ce(e) {
	for (let t = 0; t < e.length; t++) e[t]();
}
function we(e, t, n = {}) {
	let r = [], i = [], a = [], s = 0, c = t.length > 1 ? [] : null;
	return te(() => Ce(a)), () => {
		let l = e() || [], u = l.length, d, f;
		return l[o], C(() => {
			let e, t, o, m, h, g, _, v, b;
			if (u === 0) s !== 0 && (Ce(a), a = [], r = [], i = [], s = 0, c &&= []), n.fallback && (r = [Se], i[0] = y((e) => (a[0] = e, n.fallback())), s = 1);
			else if (s === 0) {
				for (i = Array(u), f = 0; f < u; f++) r[f] = l[f], i[f] = y(p);
				s = u;
			} else {
				for (o = Array(u), m = Array(u), c && (h = Array(u)), g = 0, _ = Math.min(s, u); g < _ && r[g] === l[g]; g++);
				for (_ = s - 1, v = u - 1; _ >= g && v >= g && r[_] === l[v]; _--, v--) o[v] = i[_], m[v] = a[_], c && (h[v] = c[_]);
				for (e = /* @__PURE__ */ new Map(), t = Array(v + 1), f = v; f >= g; f--) b = l[f], d = e.get(b), t[f] = d === void 0 ? -1 : d, e.set(b, f);
				for (d = g; d <= _; d++) b = r[d], f = e.get(b), f !== void 0 && f !== -1 ? (o[f] = i[d], m[f] = a[d], c && (h[f] = c[d]), f = t[f], e.set(b, f)) : a[d]();
				for (f = g; f < u; f++) f in o ? (i[f] = o[f], a[f] = m[f], c && (c[f] = h[f], c[f](f))) : i[f] = y(p);
				i = i.slice(0, s = u), r = l.slice(0);
			}
			return i;
		});
		function p(e) {
			if (a[f] = e, c) {
				let [e, n] = b(f);
				return c[f] = n, t(l[f], e);
			}
			return t(l[f]);
		}
	};
}
function k(e, t) {
	return C(() => e(t || {}));
}
function Te() {
	return !0;
}
var Ee = {
	get(e, t, n) {
		return t === i ? n : e.get(t);
	},
	has(e, t) {
		return t === i || e.has(t);
	},
	set: Te,
	deleteProperty: Te,
	getOwnPropertyDescriptor(e, t) {
		return {
			configurable: !0,
			enumerable: !0,
			get() {
				return e.get(t);
			},
			set: Te,
			deleteProperty: Te
		};
	},
	ownKeys(e) {
		return e.keys();
	}
};
function De(e) {
	return (e = typeof e == "function" ? e() : e) ? e : {};
}
function Oe() {
	for (let e = 0, t = this.length; e < t; ++e) {
		let t = this[e]();
		if (t !== void 0) return t;
	}
}
function A(...e) {
	let t = !1;
	for (let n = 0; n < e.length; n++) {
		let r = e[n];
		t ||= !!r && i in r, e[n] = typeof r == "function" ? (t = !0, S(r)) : r;
	}
	if (a && t) return new Proxy({
		get(t) {
			for (let n = e.length - 1; n >= 0; n--) {
				let r = De(e[n])[t];
				if (r !== void 0) return r;
			}
		},
		has(t) {
			for (let n = e.length - 1; n >= 0; n--) if (t in De(e[n])) return !0;
			return !1;
		},
		keys() {
			let t = [];
			for (let n = 0; n < e.length; n++) t.push(...Object.keys(De(e[n])));
			return [...new Set(t)];
		}
	}, Ee);
	let n = {}, r = Object.create(null);
	for (let t = e.length - 1; t >= 0; t--) {
		let i = e[t];
		if (!i) continue;
		let a = Object.getOwnPropertyNames(i);
		for (let e = a.length - 1; e >= 0; e--) {
			let t = a[e];
			if (t === "__proto__" || t === "constructor") continue;
			let o = Object.getOwnPropertyDescriptor(i, t);
			if (!r[t]) r[t] = o.get ? {
				enumerable: !0,
				configurable: !0,
				get: Oe.bind(n[t] = [o.get.bind(i)])
			} : o.value === void 0 ? void 0 : o;
			else {
				let e = n[t];
				e && (o.get ? e.push(o.get.bind(i)) : o.value !== void 0 && e.push(() => o.value));
			}
		}
	}
	let o = {}, s = Object.keys(r);
	for (let e = s.length - 1; e >= 0; e--) {
		let t = s[e], n = r[t];
		n && n.get ? Object.defineProperty(o, t, n) : o[t] = n ? n.value : void 0;
	}
	return o;
}
function ke(e, ...t) {
	let n = t.length;
	if (a && i in e) {
		let r = n > 1 ? t.flat() : t[0], i = /* @__PURE__ */ new Set(), a = t.map((t) => {
			let n = t.filter((e) => !i.has(e) && (i.add(e), !0));
			return new Proxy({
				get(t) {
					return n.includes(t) ? e[t] : void 0;
				},
				has(t) {
					return n.includes(t) && t in e;
				},
				keys() {
					return n.filter((t) => t in e);
				}
			}, Ee);
		});
		return a.push(new Proxy({
			get(t) {
				return r.includes(t) ? void 0 : e[t];
			},
			has(t) {
				return !r.includes(t) && t in e;
			},
			keys() {
				return Object.keys(e).filter((e) => !r.includes(e));
			}
		}, Ee)), a;
	}
	let r = [];
	for (let e = 0; e <= n; e++) r[e] = {};
	for (let i of Object.getOwnPropertyNames(e)) {
		let a = n;
		for (let e = 0; e < t.length; e++) if (t[e].includes(i)) {
			a = e;
			break;
		}
		let o = Object.getOwnPropertyDescriptor(e, i);
		!o.get && !o.set && o.enumerable && o.writable && o.configurable ? r[a][i] = o.value : Object.defineProperty(r[a], i, o);
	}
	return r;
}
var Ae = 0;
function je() {
	return e.context ? e.getNextContextId() : `cl-${Ae++}`;
}
var Me = (e) => `Stale read from <${e}>.`;
function j(e) {
	let t = "fallback" in e && { fallback: () => e.fallback };
	return S(we(() => e.each, e.children, t || void 0));
}
function M(e) {
	let t = e.keyed, n = S(() => e.when, void 0, void 0), r = t ? n : S(n, void 0, { equals: (e, t) => !e == !t });
	return S(() => {
		let i = r();
		if (i) {
			let a = e.children;
			return typeof a == "function" && a.length > 0 ? C(() => a(t ? i : () => {
				if (!C(r)) throw Me("Show");
				return n();
			})) : a;
		}
		return e.fallback;
	}, void 0, void 0);
}
//#endregion
//#region ../phemios/node_modules/solid-js/web/dist/web.js
var Ne = /*#__PURE__*/ new Set([
	"className",
	"value",
	"readOnly",
	"noValidate",
	"formNoValidate",
	"isMap",
	"noModule",
	"playsInline",
	"adAuctionHeaders",
	"allowFullscreen",
	"browsingTopics",
	"defaultChecked",
	"defaultMuted",
	"defaultSelected",
	"disablePictureInPicture",
	"disableRemotePlayback",
	"preservesPitch",
	"shadowRootClonable",
	"shadowRootCustomElementRegistry",
	"shadowRootDelegatesFocus",
	"shadowRootSerializable",
	"sharedStorageWritable",
	.../* @__PURE__ */ "allowfullscreen.async.alpha.autofocus.autoplay.checked.controls.default.disabled.formnovalidate.hidden.indeterminate.inert.ismap.loop.multiple.muted.nomodule.novalidate.open.playsinline.readonly.required.reversed.seamless.selected.adauctionheaders.browsingtopics.credentialless.defaultchecked.defaultmuted.defaultselected.defer.disablepictureinpicture.disableremoteplayback.preservespitch.shadowrootclonable.shadowrootcustomelementregistry.shadowrootdelegatesfocus.shadowrootserializable.sharedstoragewritable".split(".")
]), Pe = /*#__PURE__*/ new Set([
	"innerHTML",
	"textContent",
	"innerText",
	"children"
]), Fe = /*#__PURE__*/ Object.assign(Object.create(null), {
	className: "class",
	htmlFor: "for"
}), Ie = /*#__PURE__*/ Object.assign(Object.create(null), {
	class: "className",
	novalidate: {
		$: "noValidate",
		FORM: 1
	},
	formnovalidate: {
		$: "formNoValidate",
		BUTTON: 1,
		INPUT: 1
	},
	ismap: {
		$: "isMap",
		IMG: 1
	},
	nomodule: {
		$: "noModule",
		SCRIPT: 1
	},
	playsinline: {
		$: "playsInline",
		VIDEO: 1
	},
	readonly: {
		$: "readOnly",
		INPUT: 1,
		TEXTAREA: 1
	},
	adauctionheaders: {
		$: "adAuctionHeaders",
		IFRAME: 1
	},
	allowfullscreen: {
		$: "allowFullscreen",
		IFRAME: 1
	},
	browsingtopics: {
		$: "browsingTopics",
		IMG: 1
	},
	defaultchecked: {
		$: "defaultChecked",
		INPUT: 1
	},
	defaultmuted: {
		$: "defaultMuted",
		AUDIO: 1,
		VIDEO: 1
	},
	defaultselected: {
		$: "defaultSelected",
		OPTION: 1
	},
	disablepictureinpicture: {
		$: "disablePictureInPicture",
		VIDEO: 1
	},
	disableremoteplayback: {
		$: "disableRemotePlayback",
		AUDIO: 1,
		VIDEO: 1
	},
	preservespitch: {
		$: "preservesPitch",
		AUDIO: 1,
		VIDEO: 1
	},
	shadowrootclonable: {
		$: "shadowRootClonable",
		TEMPLATE: 1
	},
	shadowrootdelegatesfocus: {
		$: "shadowRootDelegatesFocus",
		TEMPLATE: 1
	},
	shadowrootserializable: {
		$: "shadowRootSerializable",
		TEMPLATE: 1
	},
	sharedstoragewritable: {
		$: "sharedStorageWritable",
		IFRAME: 1,
		IMG: 1
	}
});
function Le(e, t) {
	let n = Ie[e];
	return typeof n == "object" ? n[t] ? n.$ : void 0 : n;
}
var Re = /*#__PURE__*/ new Set([
	"beforeinput",
	"click",
	"dblclick",
	"contextmenu",
	"focusin",
	"focusout",
	"input",
	"keydown",
	"keyup",
	"mousedown",
	"mousemove",
	"mouseout",
	"mouseover",
	"mouseup",
	"pointerdown",
	"pointermove",
	"pointerout",
	"pointerover",
	"pointerup",
	"touchend",
	"touchmove",
	"touchstart"
]), ze = /*#__PURE__*/ new Set(/* @__PURE__ */ "altGlyph.altGlyphDef.altGlyphItem.animate.animateColor.animateMotion.animateTransform.circle.clipPath.color-profile.cursor.defs.desc.ellipse.feBlend.feColorMatrix.feComponentTransfer.feComposite.feConvolveMatrix.feDiffuseLighting.feDisplacementMap.feDistantLight.feDropShadow.feFlood.feFuncA.feFuncB.feFuncG.feFuncR.feGaussianBlur.feImage.feMerge.feMergeNode.feMorphology.feOffset.fePointLight.feSpecularLighting.feSpotLight.feTile.feTurbulence.filter.font.font-face.font-face-format.font-face-name.font-face-src.font-face-uri.foreignObject.g.glyph.glyphRef.hkern.image.line.linearGradient.marker.mask.metadata.missing-glyph.mpath.path.pattern.polygon.polyline.radialGradient.rect.set.stop.svg.switch.symbol.text.textPath.tref.tspan.use.view.vkern".split(".")), Be = {
	xlink: "http://www.w3.org/1999/xlink",
	xml: "http://www.w3.org/XML/1998/namespace"
}, N = (e) => S(() => e());
function Ve(e, t, n) {
	let r = n.length, i = t.length, a = r, o = 0, s = 0, c = t[i - 1].nextSibling, l = null;
	for (; o < i || s < a;) if (t[o] === n[s]) o++, s++;
	else {
		for (; t[i - 1] === n[a - 1];) i--, a--;
		if (i === o) {
			let t = a < r ? s ? n[s - 1].nextSibling : n[a - s] : c;
			for (; s < a;) e.insertBefore(n[s++], t);
		} else if (a === s) for (; o < i;) (!l || !l.has(t[o])) && t[o].remove(), o++;
		else if (t[o] === n[a - 1] && n[s] === t[i - 1]) {
			let r = t[--i].nextSibling;
			e.insertBefore(n[s++], t[o++].nextSibling), e.insertBefore(n[--a], r), t[i] = n[a];
		} else {
			if (!l) {
				l = /* @__PURE__ */ new Map();
				let e = s;
				for (; e < a;) l.set(n[e], e++);
			}
			let r = l.get(t[o]);
			if (r != null) {
				if (s < r && r < a) {
					let c = o, u = 1, d;
					for (; ++c < i && c < a && (d = l.get(t[c])) != null && d === r + u;) u++;
					if (u > r - s) {
						let i = t[o];
						for (; s < r;) e.insertBefore(n[s++], i);
					} else e.replaceChild(n[s++], t[o++]);
				} else o++;
			} else t[o++].remove();
		}
	}
}
var He = "_$DX_DELEGATE";
function Ue(e, t, n, r = {}) {
	let i;
	return y((r) => {
		i = r, t === document ? e() : R(t, e(), !n && t.firstChild ? null : void 0, n);
	}, r.owner), () => {
		i(), t.textContent = "";
	};
}
function P(e, t, n, r) {
	let i, a = () => {
		let t = r ? document.createElementNS("http://www.w3.org/1998/Math/MathML", "template") : document.createElement("template");
		return t.innerHTML = e, n ? t.content.firstChild.firstChild : r ? t.firstChild : t.content.firstChild;
	}, o = t ? () => C(() => document.importNode(i ||= a(), !0)) : () => (i ||= a()).cloneNode(!0);
	return o.cloneNode = o, o;
}
function F(e, t = window.document) {
	let n = t[He] || (t[He] = /* @__PURE__ */ new Set());
	for (let r = 0, i = e.length; r < i; r++) {
		let i = e[r];
		n.has(i) || (n.add(i), t.addEventListener(i, rt));
	}
}
function I(e, t, n) {
	z(e) || (n == null ? e.removeAttribute(t) : e.setAttribute(t, n));
}
function We(e, t, n, r) {
	z(e) || (r == null ? e.removeAttributeNS(t, n) : e.setAttributeNS(t, n, r));
}
function Ge(e, t, n) {
	z(e) || (n ? e.setAttribute(t, "") : e.removeAttribute(t));
}
function L(e, t) {
	z(e) || (t == null ? e.removeAttribute("class") : e.className = t);
}
function Ke(e, t, n, r) {
	if (r) Array.isArray(n) ? (e[`$$${t}`] = n[0], e[`$$${t}Data`] = n[1]) : e[`$$${t}`] = n;
	else if (Array.isArray(n)) {
		let r = n[0];
		e.addEventListener(t, n[0] = (t) => r.call(e, n[1], t));
	} else e.addEventListener(t, n, typeof n != "function" && n);
}
function qe(e, t, n = {}) {
	let r = Object.keys(t || {}), i = Object.keys(n), a, o;
	for (a = 0, o = i.length; a < o; a++) {
		let r = i[a];
		r && r !== "undefined" && !t[r] && (tt(e, r, !1), delete n[r]);
	}
	for (a = 0, o = r.length; a < o; a++) {
		let i = r[a], o = !!t[i];
		i && i !== "undefined" && n[i] !== o && o && (tt(e, i, !0), n[i] = o);
	}
	return n;
}
function Je(e, t, n) {
	if (!t) return n ? I(e, "style") : t;
	let r = e.style;
	if (typeof t == "string") return r.cssText = t;
	typeof n == "string" && (r.cssText = n = void 0), n ||= {}, t ||= {};
	let i, a;
	for (a in n) t[a] ?? r.removeProperty(a), delete n[a];
	for (a in t) i = t[a], i !== n[a] && (r.setProperty(a, i), n[a] = i);
	return n;
}
function Ye(e, t, n) {
	n == null ? e.style.removeProperty(t) : e.style.setProperty(t, n);
}
function Xe(e, t = {}, n, r) {
	let i = {};
	return r || x(() => i.children = B(e, t.children, i.children)), x(() => typeof t.ref == "function" && Ze(t.ref, e)), x(() => Qe(e, t, n, !0, i, !0)), i;
}
function Ze(e, t, n) {
	return C(() => e(t, n));
}
function R(e, t, n, r) {
	if (n !== void 0 && !r && (r = []), typeof t != "function") return B(e, t, r, n);
	x((r) => B(e, t(), r, n), r);
}
function Qe(e, t, n, r, i = {}, a = !1) {
	t ||= {};
	for (let r in i) if (!(r in t)) {
		if (r === "children") continue;
		i[r] = nt(e, r, null, i[r], n, a, t);
	}
	for (let o in t) {
		if (o === "children") {
			r || B(e, t.children);
			continue;
		}
		let s = t[o];
		i[o] = nt(e, o, s, i[o], n, a, t);
	}
}
function $e(t) {
	let n, r;
	return !z() || !(n = e.registry.get(r = ot())) ? t() : (e.completed && e.completed.add(n), e.registry.delete(r), n);
}
function z(t) {
	return !!e.context && !e.done && (!t || t.isConnected);
}
function et(e) {
	return e.toLowerCase().replace(/-([a-z])/g, (e, t) => t.toUpperCase());
}
function tt(e, t, n) {
	let r = t.trim().split(/\s+/);
	for (let t = 0, i = r.length; t < i; t++) e.classList.toggle(r[t], n);
}
function nt(e, t, n, r, i, a, o) {
	let s, c, l, u, d;
	if (t === "style") return Je(e, n, r);
	if (t === "classList") return qe(e, n, r);
	if (n === r) return r;
	if (t === "ref") a || n(e);
	else if (t.slice(0, 3) === "on:") {
		let i = t.slice(3);
		r && e.removeEventListener(i, r, typeof r != "function" && r), n && e.addEventListener(i, n, typeof n != "function" && n);
	} else if (t.slice(0, 10) === "oncapture:") {
		let i = t.slice(10);
		r && e.removeEventListener(i, r, !0), n && e.addEventListener(i, n, !0);
	} else if (t.slice(0, 2) === "on") {
		let i = t.slice(2).toLowerCase(), a = Re.has(i);
		if (!a && r) {
			let t = Array.isArray(r) ? r[0] : r;
			e.removeEventListener(i, t);
		}
		(a || n) && (Ke(e, i, n, a), a && F([i]));
	} else if (t.slice(0, 5) === "attr:") I(e, t.slice(5), n);
	else if (t.slice(0, 5) === "bool:") Ge(e, t.slice(5), n);
	else if ((d = t.slice(0, 5) === "prop:") || (l = Pe.has(t)) || !i && ((u = Le(t, e.tagName)) || (c = Ne.has(t))) || (s = e.nodeName.includes("-") || "is" in o)) {
		if (d) t = t.slice(5), c = !0;
		else if (z(e)) return n;
		t === "class" || t === "className" ? L(e, n) : s && !c && !l ? e[et(t)] = n : (t === "value" || t === "defaultValue") && (e.nodeName === "INPUT" || e.nodeName === "TEXTAREA") ? e[u || t] = n ?? "" : e[u || t] = n;
	} else {
		let r = i && t.indexOf(":") > -1 && Be[t.split(":")[0]];
		r ? We(e, r, t, n) : I(e, Fe[t] || t, n);
	}
	return n;
}
function rt(t) {
	if (e.registry && e.events && e.events.find(([e, n]) => n === t)) return;
	let n = t.target, r = `$$${t.type}`, i = t.target, a = t.currentTarget, o = (e) => Object.defineProperty(t, "target", {
		configurable: !0,
		value: e
	}), s = () => {
		let e = n[r];
		if (e && !n.disabled) {
			let i = n[`${r}Data`];
			if (i === void 0 ? e.call(n, t) : e.call(n, i, t), t.cancelBubble) return;
		}
		return n.host && typeof n.host != "string" && !n.host._$host && n.contains(t.target) && o(n.host), !0;
	}, c = () => {
		for (; s() && (n = n._$host || n.parentNode || n.host););
	};
	Object.defineProperty(t, "currentTarget", {
		configurable: !0,
		get() {
			return n || document;
		}
	}), e.registry && !e.done && (e.done = _$HY.done = !0);
	try {
		if (t.composedPath) {
			let e = t.composedPath();
			o(e[0]);
			for (let t = 0; t < e.length - 2 && (n = e[t], s()); t++) {
				if (n._$host) {
					n = n._$host, c();
					break;
				}
				if (n.parentNode === a) break;
			}
		} else c();
		o(i);
	} finally {
		t = n = null;
	}
}
function B(e, t, n, r, i) {
	let a = z(e);
	if (a) {
		!n && (n = [...e.childNodes]);
		let t = [];
		for (let e = 0; e < n.length; e++) {
			let r = n[e];
			r.nodeType === 8 && r.data.slice(0, 2) === "!$" ? r.remove() : t.push(r);
		}
		n = t;
	}
	for (; typeof n == "function";) n = n();
	if (t === n) return n;
	let o = typeof t, s = r !== void 0;
	if (e = s && n[0] && n[0].parentNode || e, o === "string" || o === "number") {
		if (a || o === "number" && (t = t.toString(), t === n)) return n;
		if (s) {
			let i = n[0];
			i && i.nodeType === 3 ? i.data !== t && (i.data = t) : i = document.createTextNode(t), n = V(e, n, r, i);
		} else n = n !== "" && typeof n == "string" ? e.firstChild.data = t : e.textContent = t;
	} else if (t == null || o === "boolean") {
		if (a) return n;
		n = V(e, n, r);
	} else if (o === "function") return x(() => {
		let i = t();
		for (; typeof i == "function";) i = i();
		n = B(e, i, n, r);
	}), () => n;
	else if (Array.isArray(t)) {
		let o = [], c = n && Array.isArray(n);
		if (it(o, t, n, i)) return x(() => n = B(e, o, n, r, !0)), () => n;
		if (a) {
			if (!o.length) return n;
			if (r === void 0) return n = [...e.childNodes];
			let t = o[0];
			if (t.parentNode !== e) return n;
			let i = [t];
			for (; (t = t.nextSibling) !== r;) i.push(t);
			return n = i;
		}
		if (o.length === 0) {
			if (n = V(e, n, r), s) return n;
		} else c ? n.length === 0 ? at(e, o, r) : Ve(e, n, o) : (n && V(e), at(e, o));
		n = o;
	} else if (t.nodeType) {
		if (a && t.parentNode) return n = s ? [t] : t;
		if (Array.isArray(n)) {
			if (s) return n = V(e, n, r, t);
			V(e, n, null, t);
		} else n == null || n === "" || !e.firstChild ? e.appendChild(t) : e.replaceChild(t, e.firstChild);
		n = t;
	}
	return n;
}
function it(e, t, n, r) {
	let i = !1;
	for (let a = 0, o = t.length; a < o; a++) {
		let o = t[a], s = n && n[e.length], c;
		if (o != null && o !== !0 && o !== !1) {
			if ((c = typeof o) == "object" && o.nodeType) e.push(o);
			else if (Array.isArray(o)) i = it(e, o, s) || i;
			else if (c === "function") {
				if (r) {
					for (; typeof o == "function";) o = o();
					i = it(e, Array.isArray(o) ? o : [o], Array.isArray(s) ? s : [s]) || i;
				} else e.push(o), i = !0;
			} else {
				let t = String(o);
				s && s.nodeType === 3 && s.data === t ? e.push(s) : e.push(document.createTextNode(t));
			}
		}
	}
	return i;
}
function at(e, t, n = null) {
	for (let r = 0, i = t.length; r < i; r++) e.insertBefore(t[r], n);
}
function V(e, t, n, r) {
	if (n === void 0) return e.textContent = "";
	let i = r || document.createTextNode("");
	if (t.length) {
		let r = !1;
		for (let a = t.length - 1; a >= 0; a--) {
			let o = t[a];
			if (i !== o) {
				let t = o.parentNode === e;
				!r && !a ? t ? e.replaceChild(i, o) : e.insertBefore(i, n) : t && o.remove();
			} else r = !0;
		}
	} else e.insertBefore(i, n);
	return [i];
}
function ot() {
	return e.getNextContextId();
}
var st = "http://www.w3.org/2000/svg";
function ct(e, t = !1, n = void 0) {
	return t ? document.createElementNS(st, e) : document.createElement(e, { is: n });
}
function lt(t, n) {
	let r = S(t);
	return S(() => {
		let t = r();
		switch (typeof t) {
			case "function": return C(() => t(n));
			case "string":
				let r = ze.has(t), i = e.context ? $e() : ct(t, r, C(() => n.is));
				return Xe(i, n, r), i;
		}
	});
}
function ut(e) {
	let [, t] = ke(e, ["component"]);
	return lt(() => e.component, t);
}
//#endregion
//#region ../phemios/src/analysis-report/disclosure.ts
var dt = [
	"integrity",
	"detection",
	"identity",
	"measurements",
	"technical",
	"coverage",
	"fingerprints"
];
function ft(e, t, n) {
	return dt.filter((r) => r === t ? n : e.includes(r));
}
//#endregion
//#region ../phemios/node_modules/lucide-solid/dist/source/context.jsx
var pt = ie({
	size: 24,
	color: "currentColor",
	strokeWidth: 2,
	absoluteStrokeWidth: !1,
	nonScalingStroke: !1,
	class: ""
}), mt = /*#__PURE__*/ P("<svg>"), H = {
	xmlns: "http://www.w3.org/2000/svg",
	width: 24,
	height: 24,
	viewBox: "0 0 24 24",
	fill: "none",
	stroke: "currentColor",
	"stroke-width": 2,
	"stroke-linecap": "round",
	"stroke-linejoin": "round"
}, ht = (...e) => e.filter((e, t, n) => !!e && e.trim() !== "" && n.indexOf(e) === t).join(" ").trim();
function gt(e) {
	return e != null;
}
function _t(e, t = {}) {
	let n = t.attributeNames ?? {}, r = (e) => n[e] ?? e, i = e.size ?? e.width ?? H.width, a = e.size ?? e.height ?? H.height, o = e.aliases?.filter((e) => typeof e == "string" && e.trim() !== "").map((e) => `lucide-${e}`) ?? [], s = [...e.name ? [`lucide-${e.name}`] : [], ...o], c = t.className?.split(" ").filter(Boolean) ?? [], l = t.includeDefaultClasses === !1 ? ht(...c) : ht("lucide", ...s, ...c), u = t.absoluteStrokeWidth ? Number(t.strokeWidth ?? H["stroke-width"]) * Number(e.size ?? e.width ?? H.width) / Number(t.size ?? t.width ?? H.width) : t.strokeWidth ?? H["stroke-width"];
	return [
		"svg",
		{
			...Object.entries(H).reduce((e, [t, n]) => (e[r(t)] = n, e), {}),
			..."color" in t && t.color && { [r("stroke")]: t.color },
			..."size" in t && gt(t.size) && {
				[r("width")]: t.size,
				[r("height")]: t.size
			},
			..."width" in t && gt(t.width) && { [r("width")]: t.width },
			..."height" in t && gt(t.height) && { [r("height")]: t.height },
			[r("stroke-width")]: u,
			...l && { [r("class")]: l },
			[r("viewBox")]: `0 0 ${i} ${a}`,
			...t.hasA11yProp === !1 ? { [r("aria-hidden")]: "true" } : {},
			..."attributes" in t && t.attributes
		},
		e.node.map((e) => {
			let [n, i, a] = e, o = t.nonScalingStroke ? {
				[r("vector-effect")]: "non-scaling-stroke",
				...i
			} : i;
			return a ? [
				n,
				o,
				a
			] : [n, o];
		})
	];
}
var vt = _t, yt = (e) => {
	for (let t in e) if (t.startsWith("aria-") || t === "role" || t === "title") return !0;
	return !1;
}, U = (e) => {
	let [t, n] = ke(e, [
		"color",
		"size",
		"width",
		"height",
		"strokeWidth",
		"children",
		"class",
		"icon",
		"iconNode",
		"absoluteStrokeWidth",
		"nonScalingStroke"
	]), r = ae(pt), i = S(() => t.icon ?? {
		node: t.iconNode ?? [],
		size: 24,
		aliases: []
	}), a = S(() => vt(i(), {
		color: t.color ?? r.color,
		width: t.width ?? t.size ?? r.size,
		height: t.height ?? t.size ?? r.size,
		strokeWidth: t.strokeWidth ?? r.strokeWidth,
		absoluteStrokeWidth: t.absoluteStrokeWidth ?? r.absoluteStrokeWidth,
		nonScalingStroke: t.nonScalingStroke ?? r.nonScalingStroke,
		className: ht("lucide-icon", r.class, t.class),
		hasA11yProp: !!t.children || yt(n),
		attributes: n
	}));
	return (() => {
		var e = mt();
		return Xe(e, A(() => a()[1]), !0, !0), R(e, k(j, {
			get each() {
				return a()[2] ?? [];
			},
			children: ([e, t]) => k(ut, A({ component: e }, t))
		})), e;
	})();
}, bt = {
	name: "arrow-down-a-z",
	size: 24,
	node: [
		["path", {
			d: "m3 16 4 4 4-4",
			key: "1co6wj"
		}],
		["path", {
			d: "M7 20V4",
			key: "1yoxec"
		}],
		["path", {
			d: "M20 8h-5",
			key: "1vsyxs"
		}],
		["path", {
			d: "M15 10V6.5a2.5 2.5 0 0 1 5 0V10",
			key: "ag13bf"
		}],
		["path", {
			d: "M15 14h5l-5 6h5",
			key: "ur5jdg"
		}]
	],
	aliases: ["arrow-down-az"]
};
bt.node;
var xt = (e) => k(U, A(e, { icon: bt })), St = {
	name: "arrow-down-up",
	size: 24,
	node: [
		["path", {
			d: "m3 16 4 4 4-4",
			key: "1co6wj"
		}],
		["path", {
			d: "M7 20V4",
			key: "1yoxec"
		}],
		["path", {
			d: "m21 8-4-4-4 4",
			key: "1c9v7m"
		}],
		["path", {
			d: "M17 4v16",
			key: "7dpous"
		}]
	]
};
St.node;
var Ct = (e) => k(U, A(e, { icon: St })), wt = {
	name: "arrow-down-wide-narrow",
	size: 24,
	node: [
		["path", {
			d: "m3 16 4 4 4-4",
			key: "1co6wj"
		}],
		["path", {
			d: "M7 20V4",
			key: "1yoxec"
		}],
		["path", {
			d: "M11 4h10",
			key: "1w87gc"
		}],
		["path", {
			d: "M11 8h7",
			key: "djye34"
		}],
		["path", {
			d: "M11 12h4",
			key: "q8tih4"
		}]
	],
	aliases: ["sort-desc"]
};
wt.node;
var Tt = (e) => k(U, A(e, { icon: wt })), Et = {
	name: "arrow-left",
	size: 24,
	node: [["path", {
		d: "m12 19-7-7 7-7",
		key: "1l729n"
	}], ["path", {
		d: "M19 12H5",
		key: "x3x0zl"
	}]]
};
Et.node;
var Dt = (e) => k(U, A(e, { icon: Et })), Ot = {
	name: "arrow-right",
	size: 24,
	node: [["path", {
		d: "M5 12h14",
		key: "1ays0h"
	}], ["path", {
		d: "m12 5 7 7-7 7",
		key: "xquz4c"
	}]]
};
Ot.node;
var kt = (e) => k(U, A(e, { icon: Ot })), At = {
	name: "arrow-up",
	size: 24,
	node: [["path", {
		d: "m5 12 7-7 7 7",
		key: "hav0vg"
	}], ["path", {
		d: "M12 19V5",
		key: "x0mq9r"
	}]]
};
At.node;
var jt = (e) => k(U, A(e, { icon: At })), Mt = {
	name: "audio-waveform",
	size: 24,
	node: [["path", {
		d: "M2 13a2 2 0 0 0 2-2V7a2 2 0 0 1 4 0v13a2 2 0 0 0 4 0V4a2 2 0 0 1 4 0v13a2 2 0 0 0 4 0v-4a2 2 0 0 1 2-2",
		key: "57tc96"
	}]]
};
Mt.node;
var Nt = (e) => k(U, A(e, { icon: Mt })), Pt = {
	name: "bold",
	size: 24,
	node: [["path", {
		d: "M6 12h9a4 4 0 0 1 0 8H7a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h7a4 4 0 0 1 0 8",
		key: "mg9rjx"
	}]]
};
Pt.node;
var Ft = (e) => k(U, A(e, { icon: Pt })), It = {
	name: "book-open",
	size: 24,
	node: [["path", {
		d: "M12 5v16",
		key: "1f6ucr"
	}], ["path", {
		d: "M20.001 19A2 2 0 0022 17V5a2 2 0 00-1.999-2L16 3.002A5 5 0 0012 5a5 5 0 00-4-2H4a2 2 0 00-2 2v12a2 2 0 001.999 2H8a5 5 0 014 2 5 5 0 014-2z",
		key: "1fyvmf"
	}]]
};
It.node;
var Lt = (e) => k(U, A(e, { icon: It })), Rt = {
	name: "bookmark",
	size: 24,
	node: [["path", {
		d: "M17 3a2 2 0 0 1 2 2v15a1 1 0 0 1-1.496.868l-4.512-2.578a2 2 0 0 0-1.984 0l-4.512 2.578A1 1 0 0 1 5 20V5a2 2 0 0 1 2-2z",
		key: "oz39mx"
	}]]
};
Rt.node;
var zt = (e) => k(U, A(e, { icon: Rt })), Bt = {
	name: "calendar",
	size: 24,
	node: [
		["path", {
			d: "M8 2v3",
			key: "1ioesn"
		}],
		["path", {
			d: "M16 2v3",
			key: "otl347"
		}],
		["rect", {
			x: "3",
			y: "3",
			width: "18",
			height: "18",
			rx: "2",
			key: "h1oib"
		}],
		["path", {
			d: "M3 9h18",
			key: "1pudct"
		}]
	]
};
Bt.node;
var Vt = (e) => k(U, A(e, { icon: Bt })), Ht = {
	name: "chart-no-axes-column",
	size: 24,
	node: [
		["path", {
			d: "M5 21v-6",
			key: "1hz6c0"
		}],
		["path", {
			d: "M12 21V3",
			key: "1lcnhd"
		}],
		["path", {
			d: "M19 21V9",
			key: "unv183"
		}]
	],
	aliases: ["bar-chart-2"]
};
Ht.node;
var Ut = (e) => k(U, A(e, { icon: Ht })), Wt = {
	name: "chart-no-axes-column-increasing",
	size: 24,
	node: [
		["path", {
			d: "M5 21v-6",
			key: "1hz6c0"
		}],
		["path", {
			d: "M12 21V9",
			key: "uvy0l4"
		}],
		["path", {
			d: "M19 21V3",
			key: "11j9sm"
		}]
	],
	aliases: ["bar-chart"]
};
Wt.node;
var Gt = (e) => k(U, A(e, { icon: Wt })), Kt = {
	name: "check",
	size: 24,
	node: [["path", {
		d: "M20 6 9 17l-5-5",
		key: "1gmf2c"
	}]]
};
Kt.node;
var qt = (e) => k(U, A(e, { icon: Kt })), Jt = {
	name: "chevron-down",
	size: 24,
	node: [["path", {
		d: "m6 9 6 6 6-6",
		key: "qrunsl"
	}]]
};
Jt.node;
var Yt = (e) => k(U, A(e, { icon: Jt })), Xt = {
	name: "chevron-right",
	size: 24,
	node: [["path", {
		d: "m9 18 6-6-6-6",
		key: "mthhwq"
	}]]
};
Xt.node;
var Zt = (e) => k(U, A(e, { icon: Xt })), Qt = {
	name: "chevron-up",
	size: 24,
	node: [["path", {
		d: "m18 15-6-6-6 6",
		key: "153udz"
	}]]
};
Qt.node;
var $t = (e) => k(U, A(e, { icon: Qt })), en = {
	name: "circle-question-mark",
	size: 24,
	node: [
		["circle", {
			cx: "12",
			cy: "12",
			r: "10",
			key: "1mglay"
		}],
		["path", {
			d: "M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3",
			key: "1u773s"
		}],
		["path", {
			d: "M12 17h.01",
			key: "p32p05"
		}]
	],
	aliases: ["help-circle", "circle-help"]
};
en.node;
var tn = (e) => k(U, A(e, { icon: en })), nn = {
	name: "combine",
	size: 24,
	node: [
		["path", {
			d: "M14 3a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1",
			key: "1l7d7l"
		}],
		["path", {
			d: "M19 3a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1",
			key: "9955pe"
		}],
		["path", {
			d: "m7 15 3 3",
			key: "4hkfgk"
		}],
		["path", {
			d: "m7 21 3-3H5a2 2 0 0 1-2-2v-2",
			key: "1xljwe"
		}],
		["rect", {
			x: "14",
			y: "14",
			width: "7",
			height: "7",
			rx: "1",
			key: "1cdgtw"
		}],
		["rect", {
			x: "3",
			y: "3",
			width: "7",
			height: "7",
			rx: "1",
			key: "zi3rio"
		}]
	]
};
nn.node;
var rn = (e) => k(U, A(e, { icon: nn })), an = {
	name: "disc-3",
	size: 24,
	node: [
		["circle", {
			cx: "12",
			cy: "12",
			r: "10",
			key: "1mglay"
		}],
		["path", {
			d: "M6 12c0-1.7.7-3.2 1.8-4.2",
			key: "oqkarx"
		}],
		["circle", {
			cx: "12",
			cy: "12",
			r: "2",
			key: "1c9p78"
		}],
		["path", {
			d: "M18 12c0 1.7-.7 3.2-1.8 4.2",
			key: "1eah9h"
		}]
	]
};
an.node;
var on = (e) => k(U, A(e, { icon: an })), sn = {
	name: "disc-album",
	size: 24,
	node: [
		["rect", {
			width: "18",
			height: "18",
			x: "3",
			y: "3",
			rx: "2",
			key: "afitv7"
		}],
		["circle", {
			cx: "12",
			cy: "12",
			r: "5",
			key: "nd82uf"
		}],
		["path", {
			d: "M12 12h.01",
			key: "1mp3jc"
		}]
	]
};
sn.node;
var cn = (e) => k(U, A(e, { icon: sn })), ln = {
	name: "download",
	size: 24,
	node: [
		["path", {
			d: "M12 15V3",
			key: "m9g1x1"
		}],
		["path", {
			d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4",
			key: "ih7n3h"
		}],
		["path", {
			d: "m7 10 5 5 5-5",
			key: "brsn70"
		}]
	]
};
ln.node;
var un = (e) => k(U, A(e, { icon: ln })), dn = {
	name: "eject",
	size: 24,
	node: [["path", {
		d: "M4 13a1 1 0 0 1-.72-1.695l7.257-7.668a2 2 0 0 1 2.926 0l7.256 7.668A1 1 0 0 1 20 13z",
		key: "ua5u6w"
	}], ["rect", {
		x: "3",
		y: "17",
		width: "18",
		height: "4",
		rx: "1",
		key: "kj6cfs"
	}]]
};
dn.node;
var fn = (e) => k(U, A(e, { icon: dn })), pn = {
	name: "external-link",
	size: 24,
	node: [
		["path", {
			d: "M15 3h6v6",
			key: "1q9fwt"
		}],
		["path", {
			d: "M10 14 21 3",
			key: "gplh6r"
		}],
		["path", {
			d: "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6",
			key: "a6xqqp"
		}]
	]
};
pn.node;
var mn = (e) => k(U, A(e, { icon: pn })), hn = {
	name: "eye",
	size: 24,
	node: [["path", {
		d: "M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0",
		key: "1nclc0"
	}], ["circle", {
		cx: "12",
		cy: "12",
		r: "3",
		key: "1v7zrd"
	}]]
};
hn.node;
var gn = (e) => k(U, A(e, { icon: hn })), _n = {
	name: "eye-off",
	size: 24,
	node: [
		["path", {
			d: "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49",
			key: "ct8e1f"
		}],
		["path", {
			d: "M14.084 14.158a3 3 0 0 1-4.242-4.242",
			key: "151rxh"
		}],
		["path", {
			d: "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143",
			key: "13bj9a"
		}],
		["path", {
			d: "m2 2 20 20",
			key: "1ooewy"
		}]
	]
};
_n.node;
var vn = (e) => k(U, A(e, { icon: _n })), yn = {
	name: "file-clock",
	size: 24,
	node: [
		["path", {
			d: "M16 22h2a2 2 0 0 0 2-2V8a2.4 2.4 0 0 0-.706-1.706l-3.588-3.588A2.4 2.4 0 0 0 14 2H6a2 2 0 0 0-2 2v2.85",
			key: "ryk6xj"
		}],
		["path", {
			d: "M14 2v5a1 1 0 0 0 1 1h5",
			key: "wfsgrz"
		}],
		["path", {
			d: "M8 14v2.2l1.6 1",
			key: "6m4bie"
		}],
		["circle", {
			cx: "8",
			cy: "16",
			r: "6",
			key: "10v15b"
		}]
	]
};
yn.node;
var bn = (e) => k(U, A(e, { icon: yn })), xn = {
	name: "file-down",
	size: 24,
	node: [
		["path", {
			d: "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z",
			key: "1oefj6"
		}],
		["path", {
			d: "M14 2v5a1 1 0 0 0 1 1h5",
			key: "wfsgrz"
		}],
		["path", {
			d: "M12 18v-6",
			key: "17g6i2"
		}],
		["path", {
			d: "m9 15 3 3 3-3",
			key: "1npd3o"
		}]
	]
};
xn.node;
var Sn = (e) => k(U, A(e, { icon: xn })), Cn = {
	name: "file-music",
	size: 24,
	node: [
		["path", {
			d: "M11.65 22H18a2 2 0 0 0 2-2V8a2.4 2.4 0 0 0-.706-1.706l-3.588-3.588A2.4 2.4 0 0 0 14 2H6a2 2 0 0 0-2 2v10.35",
			key: "5ad7z2"
		}],
		["path", {
			d: "M14 2v5a1 1 0 0 0 1 1h5",
			key: "wfsgrz"
		}],
		["path", {
			d: "M8 20v-7l3 1.474",
			key: "1ggyb9"
		}],
		["circle", {
			cx: "6",
			cy: "20",
			r: "2",
			key: "j7wjp0"
		}]
	]
};
Cn.node;
var wn = (e) => k(U, A(e, { icon: Cn })), Tn = {
	name: "file-text",
	size: 24,
	node: [
		["path", {
			d: "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z",
			key: "1oefj6"
		}],
		["path", {
			d: "M14 2v5a1 1 0 0 0 1 1h5",
			key: "wfsgrz"
		}],
		["path", {
			d: "M10 9H8",
			key: "b1mrlr"
		}],
		["path", {
			d: "M16 13H8",
			key: "t4e002"
		}],
		["path", {
			d: "M16 17H8",
			key: "z1uh3a"
		}]
	]
};
Tn.node;
var En = (e) => k(U, A(e, { icon: Tn })), Dn = {
	name: "folder-open",
	size: 24,
	node: [["path", {
		d: "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2",
		key: "usdka0"
	}]]
};
Dn.node;
var On = (e) => k(U, A(e, { icon: Dn })), kn = {
	name: "gallery-horizontal-end",
	size: 24,
	node: [
		["path", {
			d: "M2 7v10",
			key: "a2pl2d"
		}],
		["path", {
			d: "M6 5v14",
			key: "1kq3d7"
		}],
		["rect", {
			width: "12",
			height: "18",
			x: "10",
			y: "3",
			rx: "2",
			key: "13i7bc"
		}]
	]
};
kn.node;
var An = (e) => k(U, A(e, { icon: kn })), jn = {
	name: "globe",
	size: 24,
	node: [
		["circle", {
			cx: "12",
			cy: "12",
			r: "10",
			key: "1mglay"
		}],
		["path", {
			d: "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20",
			key: "13o1zl"
		}],
		["path", {
			d: "M2 12h20",
			key: "9i4pu4"
		}]
	]
};
jn.node;
var Mn = (e) => k(U, A(e, { icon: jn })), Nn = {
	name: "grip-vertical",
	size: 24,
	node: [
		["circle", {
			cx: "9",
			cy: "12",
			r: "1",
			key: "1vctgf"
		}],
		["circle", {
			cx: "9",
			cy: "5",
			r: "1",
			key: "hp0tcf"
		}],
		["circle", {
			cx: "9",
			cy: "19",
			r: "1",
			key: "fkjjf6"
		}],
		["circle", {
			cx: "15",
			cy: "12",
			r: "1",
			key: "1tmaij"
		}],
		["circle", {
			cx: "15",
			cy: "5",
			r: "1",
			key: "19l28e"
		}],
		["circle", {
			cx: "15",
			cy: "19",
			r: "1",
			key: "f4zoj3"
		}]
	]
};
Nn.node;
var Pn = (e) => k(U, A(e, { icon: Nn })), Fn = {
	name: "hard-drive",
	size: 24,
	node: [
		["path", {
			d: "M10 16h.01",
			key: "1bzywj"
		}],
		["path", {
			d: "M2.212 11.577a2 2 0 0 0-.212.896V18a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-5.527a2 2 0 0 0-.212-.896L18.55 5.11A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z",
			key: "18tbho"
		}],
		["path", {
			d: "M21.946 12.013H2.054",
			key: "zqlbp7"
		}],
		["path", {
			d: "M6 16h.01",
			key: "1pmjb7"
		}]
	]
};
Fn.node;
var In = (e) => k(U, A(e, { icon: Fn })), Ln = {
	name: "heading-2",
	size: 24,
	node: [
		["path", {
			d: "M4 12h8",
			key: "17cfdx"
		}],
		["path", {
			d: "M4 18V6",
			key: "1rz3zl"
		}],
		["path", {
			d: "M12 18V6",
			key: "zqpxq5"
		}],
		["path", {
			d: "M21 18h-4c0-4 4-3 4-6 0-1.5-2-2.5-4-1",
			key: "9jr5yi"
		}]
	]
};
Ln.node;
var Rn = (e) => k(U, A(e, { icon: Ln })), zn = {
	name: "heart",
	size: 24,
	node: [["path", {
		d: "M2 9.5a5.5 5.5 0 0 1 9.591-3.676.56.56 0 0 0 .818 0A5.49 5.49 0 0 1 22 9.5c0 2.29-1.5 4-3 5.5l-5.492 5.313a2 2 0 0 1-3 .019L5 15c-1.5-1.5-3-3.2-3-5.5",
		key: "mvr1a0"
	}]]
};
zn.node;
var Bn = (e) => k(U, A(e, { icon: zn })), Vn = {
	name: "rotate-ccw-clock",
	size: 24,
	node: [
		["path", {
			d: "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8",
			key: "1357e3"
		}],
		["path", {
			d: "M3 3v5h5",
			key: "1xhq8a"
		}],
		["path", {
			d: "M12 7v5l4 2",
			key: "1fdv2h"
		}]
	],
	aliases: ["history"]
};
Vn.node;
var Hn = (e) => k(U, A(e, { icon: Vn })), Un = {
	name: "italic",
	size: 24,
	node: [
		["line", {
			x1: "19",
			x2: "10",
			y1: "4",
			y2: "4",
			key: "15jd3p"
		}],
		["line", {
			x1: "14",
			x2: "5",
			y1: "20",
			y2: "20",
			key: "bu0au3"
		}],
		["line", {
			x1: "15",
			x2: "9",
			y1: "4",
			y2: "20",
			key: "uljnxc"
		}]
	]
};
Un.node;
var Wn = (e) => k(U, A(e, { icon: Un })), Gn = {
	name: "keyboard",
	size: 24,
	node: [
		["path", {
			d: "M10 8h.01",
			key: "1r9ogq"
		}],
		["path", {
			d: "M12 12h.01",
			key: "1mp3jc"
		}],
		["path", {
			d: "M14 8h.01",
			key: "1primd"
		}],
		["path", {
			d: "M16 12h.01",
			key: "1l6xoz"
		}],
		["path", {
			d: "M18 8h.01",
			key: "emo2bl"
		}],
		["path", {
			d: "M6 8h.01",
			key: "x9i8wu"
		}],
		["path", {
			d: "M7 16h10",
			key: "wp8him"
		}],
		["path", {
			d: "M8 12h.01",
			key: "czm47f"
		}],
		["rect", {
			width: "20",
			height: "16",
			x: "2",
			y: "4",
			rx: "2",
			key: "18n3k1"
		}]
	]
};
Gn.node;
var Kn = (e) => k(U, A(e, { icon: Gn })), qn = {
	name: "layers",
	size: 24,
	node: [
		["path", {
			d: "M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z",
			key: "zw3jo"
		}],
		["path", {
			d: "M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12",
			key: "1wduqc"
		}],
		["path", {
			d: "M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17",
			key: "kqbvx6"
		}]
	],
	aliases: ["layers-3"]
};
qn.node;
var Jn = (e) => k(U, A(e, { icon: qn })), Yn = {
	name: "layout-grid",
	size: 24,
	node: [
		["rect", {
			width: "7",
			height: "7",
			x: "3",
			y: "3",
			rx: "1",
			key: "1g98yp"
		}],
		["rect", {
			width: "7",
			height: "7",
			x: "14",
			y: "3",
			rx: "1",
			key: "6d4xhi"
		}],
		["rect", {
			width: "7",
			height: "7",
			x: "14",
			y: "14",
			rx: "1",
			key: "nxv5o0"
		}],
		["rect", {
			width: "7",
			height: "7",
			x: "3",
			y: "14",
			rx: "1",
			key: "1bb6yr"
		}]
	]
};
Yn.node;
var Xn = (e) => k(U, A(e, { icon: Yn })), Zn = {
	name: "library",
	size: 24,
	node: [
		["path", {
			d: "m16 6 4 14",
			key: "ji33uf"
		}],
		["path", {
			d: "M12 6v14",
			key: "1n7gus"
		}],
		["path", {
			d: "M8 8v12",
			key: "1gg7y9"
		}],
		["path", {
			d: "M4 4v16",
			key: "6qkkli"
		}]
	]
};
Zn.node;
var Qn = (e) => k(U, A(e, { icon: Zn })), $n = {
	name: "list",
	size: 24,
	node: [
		["path", {
			d: "M3 5h.01",
			key: "18ugdj"
		}],
		["path", {
			d: "M3 12h.01",
			key: "nlz23k"
		}],
		["path", {
			d: "M3 19h.01",
			key: "noohij"
		}],
		["path", {
			d: "M8 5h13",
			key: "1pao27"
		}],
		["path", {
			d: "M8 12h13",
			key: "1za7za"
		}],
		["path", {
			d: "M8 19h13",
			key: "m83p4d"
		}]
	]
};
$n.node;
var er = (e) => k(U, A(e, { icon: $n })), tr = {
	name: "list-ordered",
	size: 24,
	node: [
		["path", {
			d: "M11 5h10",
			key: "1cz7ny"
		}],
		["path", {
			d: "M11 12h10",
			key: "1438ji"
		}],
		["path", {
			d: "M11 19h10",
			key: "11t30w"
		}],
		["path", {
			d: "M4 4h1v5",
			key: "10yrso"
		}],
		["path", {
			d: "M4 9h2",
			key: "r1h2o0"
		}],
		["path", {
			d: "M6.5 20H3.4c0-1 2.6-1.925 2.6-3.5a1.5 1.5 0 0 0-2.6-1.02",
			key: "xtkcd5"
		}]
	]
};
tr.node;
var nr = (e) => k(U, A(e, { icon: tr })), rr = {
	name: "lock-keyhole",
	size: 24,
	node: [
		["circle", {
			cx: "12",
			cy: "16",
			r: "1",
			key: "1au0dj"
		}],
		["rect", {
			x: "3",
			y: "10",
			width: "18",
			height: "12",
			rx: "2",
			key: "6s8ecr"
		}],
		["path", {
			d: "M7 10V7a5 5 0 0 1 10 0v3",
			key: "1pqi11"
		}]
	]
};
rr.node;
var ir = (e) => k(U, A(e, { icon: rr })), ar = {
	name: "menu",
	size: 24,
	node: [
		["path", {
			d: "M4 5h16",
			key: "1tepv9"
		}],
		["path", {
			d: "M4 12h16",
			key: "1lakjw"
		}],
		["path", {
			d: "M4 19h16",
			key: "1djgab"
		}]
	]
};
ar.node;
var or = (e) => k(U, A(e, { icon: ar })), sr = {
	name: "mic-vocal",
	size: 24,
	node: [
		["path", {
			d: "m11 7.601-5.994 8.19a1 1 0 0 0 .1 1.298l.817.818a1 1 0 0 0 1.314.087L15.09 12",
			key: "80a601"
		}],
		["path", {
			d: "M16.5 21.174C15.5 20.5 14.372 20 13 20c-2.058 0-3.928 2.356-6 2-2.072-.356-2.775-3.369-1.5-4.5",
			key: "j0ngtp"
		}],
		["circle", {
			cx: "16",
			cy: "7",
			r: "5",
			key: "d08jfb"
		}]
	],
	aliases: ["mic-2"]
};
sr.node;
var cr = (e) => k(U, A(e, { icon: sr })), lr = {
	name: "minus",
	size: 24,
	node: [["path", {
		d: "M5 12h14",
		key: "1ays0h"
	}]]
};
lr.node;
var ur = (e) => k(U, A(e, { icon: lr })), dr = {
	name: "monitor-off",
	size: 24,
	node: [
		["path", {
			d: "M12 17v4",
			key: "1riwvh"
		}],
		["path", {
			d: "M17 17H4a2 2 0 0 1-2-2V5a2 2 0 0 1 1.184-1.826",
			key: "cv7jms"
		}],
		["path", {
			d: "m2 2 20 20",
			key: "1ooewy"
		}],
		["path", {
			d: "M8 21h8",
			key: "1ev6f3"
		}],
		["path", {
			d: "M8.656 3H20a2 2 0 0 1 2 2v10a2 2 0 0 1-.293 1.042",
			key: "z8ni2w"
		}]
	]
};
dr.node;
var fr = (e) => k(U, A(e, { icon: dr })), pr = {
	name: "monitor-play",
	size: 24,
	node: [
		["path", {
			d: "M15.033 9.44a.647.647 0 0 1 0 1.12l-4.065 2.352a.645.645 0 0 1-.968-.56V7.648a.645.645 0 0 1 .967-.56z",
			key: "vbtd3f"
		}],
		["path", {
			d: "M12 17v4",
			key: "1riwvh"
		}],
		["path", {
			d: "M8 21h8",
			key: "1ev6f3"
		}],
		["rect", {
			x: "2",
			y: "3",
			width: "20",
			height: "14",
			rx: "2",
			key: "x3v2xh"
		}]
	]
};
pr.node;
var mr = (e) => k(U, A(e, { icon: pr })), hr = {
	name: "mountain",
	size: 24,
	node: [["path", {
		d: "m8 3 4 8 5-5 5 15H2L8 3z",
		key: "otkl63"
	}]]
};
hr.node;
var gr = (e) => k(U, A(e, { icon: hr })), _r = {
	name: "eighth-note",
	size: 24,
	node: [["path", {
		d: "m19 6-5.2-3.9a.5.5 0 0 0-.8.4V18",
		key: "nsdg4r"
	}], ["circle", {
		cx: "9",
		cy: "18",
		r: "4",
		key: "1972g7"
	}]],
	aliases: ["music-2"]
};
_r.node;
var vr = (e) => k(U, A(e, { icon: _r })), yr = {
	name: "network",
	size: 24,
	node: [
		["rect", {
			x: "16",
			y: "16",
			width: "6",
			height: "6",
			rx: "1",
			key: "4q2zg0"
		}],
		["rect", {
			x: "2",
			y: "16",
			width: "6",
			height: "6",
			rx: "1",
			key: "8cvhb9"
		}],
		["rect", {
			x: "9",
			y: "2",
			width: "6",
			height: "6",
			rx: "1",
			key: "1egb70"
		}],
		["path", {
			d: "M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3",
			key: "1jsf9p"
		}],
		["path", {
			d: "M12 12V8",
			key: "2874zd"
		}]
	]
};
yr.node;
var br = (e) => k(U, A(e, { icon: yr })), xr = {
	name: "palette",
	size: 24,
	node: [
		["path", {
			d: "M12 22a1 1 0 0 1 0-20 10 9 0 0 1 10 9 5 5 0 0 1-5 5h-2.25a1.75 1.75 0 0 0-1.4 2.8l.3.4a1.75 1.75 0 0 1-1.4 2.8z",
			key: "e79jfc"
		}],
		["circle", {
			cx: "13.5",
			cy: "6.5",
			r: ".5",
			fill: "currentColor",
			key: "1okk4w"
		}],
		["circle", {
			cx: "17.5",
			cy: "10.5",
			r: ".5",
			fill: "currentColor",
			key: "f64h9f"
		}],
		["circle", {
			cx: "6.5",
			cy: "12.5",
			r: ".5",
			fill: "currentColor",
			key: "qy21gx"
		}],
		["circle", {
			cx: "8.5",
			cy: "7.5",
			r: ".5",
			fill: "currentColor",
			key: "fotxhn"
		}]
	]
};
xr.node;
var Sr = (e) => k(U, A(e, { icon: xr })), Cr = {
	name: "panel-left",
	size: 24,
	node: [["rect", {
		width: "18",
		height: "18",
		x: "3",
		y: "3",
		rx: "2",
		key: "afitv7"
	}], ["path", {
		d: "M9 3v18",
		key: "fh3hqa"
	}]],
	aliases: ["sidebar"]
};
Cr.node;
var wr = (e) => k(U, A(e, { icon: Cr })), Tr = {
	name: "pause",
	size: 24,
	node: [["rect", {
		x: "14",
		y: "3",
		width: "5",
		height: "18",
		rx: "1",
		key: "kaeet6"
	}], ["rect", {
		x: "5",
		y: "3",
		width: "5",
		height: "18",
		rx: "1",
		key: "1wsw3u"
	}]]
};
Tr.node;
var Er = (e) => k(U, A(e, { icon: Tr })), Dr = {
	name: "pencil",
	size: 24,
	node: [["path", {
		d: "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
		key: "1a8usu"
	}], ["path", {
		d: "m15 5 4 4",
		key: "1mk7zo"
	}]]
};
Dr.node;
var Or = (e) => k(U, A(e, { icon: Dr })), kr = {
	name: "piano",
	size: 24,
	node: [
		["path", {
			d: "M10 13v4",
			key: "5krxfa"
		}],
		["path", {
			d: "M14 13v4",
			key: "72xrsi"
		}],
		["path", {
			d: "M18 13v4",
			key: "1i7sbu"
		}],
		["path", {
			d: "M2 13h20",
			key: "5evz65"
		}],
		["path", {
			d: "M22 11.5A3.5 3.5 0 0018.5 8a3.52 3.52 0 01-3.173-2A7 7 0 002 9v10a2 2 0 002 2h16a2 2 0 002-2z",
			key: "1txdgy"
		}],
		["path", {
			d: "M6 13v4",
			key: "9v0cap"
		}]
	]
};
kr.node;
var Ar = (e) => k(U, A(e, { icon: kr })), jr = {
	name: "pilcrow",
	size: 24,
	node: [
		["path", {
			d: "M13 4v16",
			key: "8vvj80"
		}],
		["path", {
			d: "M17 4v16",
			key: "7dpous"
		}],
		["path", {
			d: "M19 4H9.5a4.5 4.5 0 0 0 0 9H13",
			key: "sh4n9v"
		}]
	]
};
jr.node;
var Mr = (e) => k(U, A(e, { icon: jr })), Nr = {
	name: "play",
	size: 24,
	node: [["path", {
		d: "M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z",
		key: "10ikf1"
	}]]
};
Nr.node;
var Pr = (e) => k(U, A(e, { icon: Nr })), Fr = {
	name: "plus",
	size: 24,
	node: [["path", {
		d: "M5 12h14",
		key: "1ays0h"
	}], ["path", {
		d: "M12 5v14",
		key: "s699le"
	}]]
};
Fr.node;
var Ir = (e) => k(U, A(e, { icon: Fr })), Lr = {
	name: "mic-signal",
	size: 24,
	node: [
		["path", {
			d: "M12 17v4",
			key: "1riwvh"
		}],
		["path", {
			d: "M18 11a6 6 0 00-3-5.197",
			key: "1lvu40"
		}],
		["path", {
			d: "M2 11a10 10 0 015-8.662",
			key: "bida4p"
		}],
		["path", {
			d: "M22 11a10 10 0 00-5-8.662",
			key: "idvinr"
		}],
		["path", {
			d: "M6 11a6 6 0 013-5.197",
			key: "17n2ii"
		}],
		["path", {
			d: "M9 21h6",
			key: "1udhl7"
		}],
		["rect", {
			x: "10",
			y: "9",
			width: "4",
			height: "8",
			rx: "2",
			key: "1l8p2f"
		}]
	],
	aliases: ["podcast"]
};
Lr.node;
var Rr = (e) => k(U, A(e, { icon: Lr })), zr = {
	name: "quote",
	size: 24,
	node: [["path", {
		d: "M16 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z",
		key: "rib7q0"
	}], ["path", {
		d: "M5 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z",
		key: "1ymkrd"
	}]]
};
zr.node;
var Br = (e) => k(U, A(e, { icon: zr })), Vr = {
	name: "radio",
	size: 24,
	node: [
		["path", {
			d: "M16.247 7.761a6 6 0 0 1 0 8.478",
			key: "1fwjs5"
		}],
		["path", {
			d: "M19.075 4.933a10 10 0 0 1 0 14.134",
			key: "ehdyv1"
		}],
		["path", {
			d: "M4.925 19.067a10 10 0 0 1 0-14.134",
			key: "1q22gi"
		}],
		["path", {
			d: "M7.753 16.239a6 6 0 0 1 0-8.478",
			key: "r2q7qm"
		}],
		["circle", {
			cx: "12",
			cy: "12",
			r: "2",
			key: "1c9p78"
		}]
	]
};
Vr.node;
var Hr = (e) => k(U, A(e, { icon: Vr })), Ur = {
	name: "rectangle-vertical",
	size: 24,
	node: [["rect", {
		width: "12",
		height: "20",
		x: "6",
		y: "2",
		rx: "2",
		key: "1oxtiu"
	}]]
};
Ur.node;
var Wr = (e) => k(U, A(e, { icon: Ur })), Gr = {
	name: "redo-2",
	size: 24,
	node: [["path", {
		d: "m15 14 5-5-5-5",
		key: "12vg1m"
	}], ["path", {
		d: "M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13",
		key: "6uklza"
	}]]
};
Gr.node;
var Kr = (e) => k(U, A(e, { icon: Gr })), qr = {
	name: "refresh-cw",
	size: 24,
	node: [
		["path", {
			d: "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8",
			key: "v9h5vc"
		}],
		["path", {
			d: "M21 3v5h-5",
			key: "1q7to0"
		}],
		["path", {
			d: "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16",
			key: "3uifl3"
		}],
		["path", {
			d: "M8 16H3v5",
			key: "1cv678"
		}]
	]
};
qr.node;
var Jr = (e) => k(U, A(e, { icon: qr })), Yr = {
	name: "repeat",
	size: 24,
	node: [
		["path", {
			d: "m17 2 4 4-4 4",
			key: "nntrym"
		}],
		["path", {
			d: "M3 11v-1a4 4 0 0 1 4-4h14",
			key: "84bu3i"
		}],
		["path", {
			d: "m7 22-4-4 4-4",
			key: "1wqhfi"
		}],
		["path", {
			d: "M21 13v1a4 4 0 0 1-4 4H3",
			key: "1rx37r"
		}]
	]
};
Yr.node;
var Xr = (e) => k(U, A(e, { icon: Yr })), Zr = {
	name: "repeat-1",
	size: 24,
	node: [
		["path", {
			d: "m17 2 4 4-4 4",
			key: "nntrym"
		}],
		["path", {
			d: "M3 11v-1a4 4 0 0 1 4-4h14",
			key: "84bu3i"
		}],
		["path", {
			d: "m7 22-4-4 4-4",
			key: "1wqhfi"
		}],
		["path", {
			d: "M21 13v1a4 4 0 0 1-4 4H3",
			key: "1rx37r"
		}],
		["path", {
			d: "M11 10h1v4",
			key: "70cz1p"
		}]
	]
};
Zr.node;
var Qr = (e) => k(U, A(e, { icon: Zr })), $r = {
	name: "scan",
	size: 24,
	node: [
		["path", {
			d: "M3 7V5a2 2 0 0 1 2-2h2",
			key: "aa7l1z"
		}],
		["path", {
			d: "M17 3h2a2 2 0 0 1 2 2v2",
			key: "4qcy5o"
		}],
		["path", {
			d: "M21 17v2a2 2 0 0 1-2 2h-2",
			key: "6vwrx8"
		}],
		["path", {
			d: "M7 21H5a2 2 0 0 1-2-2v-2",
			key: "ioqczr"
		}]
	]
};
$r.node;
var ei = (e) => k(U, A(e, { icon: $r })), ti = {
	name: "search",
	size: 24,
	node: [["path", {
		d: "m21 21-4.34-4.34",
		key: "14j7rj"
	}], ["circle", {
		cx: "11",
		cy: "11",
		r: "8",
		key: "4ej97u"
	}]]
};
ti.node;
var ni = (e) => k(U, A(e, { icon: ti })), ri = {
	name: "settings",
	size: 24,
	node: [["path", {
		d: "M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915",
		key: "1i5ecw"
	}], ["circle", {
		cx: "12",
		cy: "12",
		r: "3",
		key: "1v7zrd"
	}]]
};
ri.node;
var ii = (e) => k(U, A(e, { icon: ri })), ai = {
	name: "shield-check",
	size: 24,
	node: [["path", {
		d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z",
		key: "oel41y"
	}], ["path", {
		d: "m9 12 2 2 4-4",
		key: "dzmm74"
	}]]
};
ai.node;
var oi = (e) => k(U, A(e, { icon: ai })), si = {
	name: "shuffle",
	size: 24,
	node: [
		["path", {
			d: "m18 14 4 4-4 4",
			key: "10pe0f"
		}],
		["path", {
			d: "m18 2 4 4-4 4",
			key: "pucp1d"
		}],
		["path", {
			d: "M2 18h1.973a4 4 0 0 0 3.3-1.7l5.454-8.6a4 4 0 0 1 3.3-1.7H22",
			key: "1ailkh"
		}],
		["path", {
			d: "M2 6h1.972a4 4 0 0 1 3.6 2.2",
			key: "km57vx"
		}],
		["path", {
			d: "M22 18h-6.041a4 4 0 0 1-3.3-1.8l-.359-.45",
			key: "os18l9"
		}]
	]
};
si.node;
var ci = (e) => k(U, A(e, { icon: si })), li = {
	name: "skip-back",
	size: 24,
	node: [["path", {
		d: "M17.971 4.285A2 2 0 0 1 21 6v12a2 2 0 0 1-3.029 1.715l-9.997-5.998a2 2 0 0 1-.003-3.432z",
		key: "15892j"
	}], ["path", {
		d: "M3 20V4",
		key: "1ptbpl"
	}]]
};
li.node;
var ui = (e) => k(U, A(e, { icon: li })), di = {
	name: "skip-forward",
	size: 24,
	node: [["path", {
		d: "M21 4v16",
		key: "7j8fe9"
	}], ["path", {
		d: "M6.029 4.285A2 2 0 0 0 3 6v12a2 2 0 0 0 3.029 1.715l9.997-5.998a2 2 0 0 0 .003-3.432z",
		key: "zs4d6"
	}]]
};
di.node;
var fi = (e) => k(U, A(e, { icon: di })), pi = {
	name: "sliders-horizontal",
	size: 24,
	node: [
		["path", {
			d: "M10 5H3",
			key: "1qgfaw"
		}],
		["path", {
			d: "M12 19H3",
			key: "yhmn1j"
		}],
		["path", {
			d: "M14 3v4",
			key: "1sua03"
		}],
		["path", {
			d: "M16 17v4",
			key: "1q0r14"
		}],
		["path", {
			d: "M21 12h-9",
			key: "1o4lsq"
		}],
		["path", {
			d: "M21 19h-5",
			key: "1rlt1p"
		}],
		["path", {
			d: "M21 5h-7",
			key: "1oszz2"
		}],
		["path", {
			d: "M8 10v4",
			key: "tgpxqk"
		}],
		["path", {
			d: "M8 12H3",
			key: "a7s4jb"
		}]
	]
};
pi.node;
var mi = (e) => k(U, A(e, { icon: pi })), hi = {
	name: "smartphone",
	size: 24,
	node: [["rect", {
		width: "14",
		height: "20",
		x: "5",
		y: "2",
		rx: "2",
		ry: "2",
		key: "1yt0o3"
	}], ["path", {
		d: "M12 18h.01",
		key: "mhygvu"
	}]]
};
hi.node;
var gi = (e) => k(U, A(e, { icon: hi })), _i = {
	name: "sparkles",
	size: 24,
	node: [
		["path", {
			d: "M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z",
			key: "1s2grr"
		}],
		["path", {
			d: "M20 2v4",
			key: "1rf3ol"
		}],
		["path", {
			d: "M22 4h-4",
			key: "gwowj6"
		}],
		["circle", {
			cx: "4",
			cy: "20",
			r: "2",
			key: "6kqj1y"
		}]
	],
	aliases: ["stars"]
};
_i.node;
var vi = (e) => k(U, A(e, { icon: _i })), yi = {
	name: "square",
	size: 24,
	node: [["rect", {
		width: "18",
		height: "18",
		x: "3",
		y: "3",
		rx: "2",
		key: "afitv7"
	}]]
};
yi.node;
var bi = (e) => k(U, A(e, { icon: yi })), xi = {
	name: "square-play",
	size: 24,
	node: [["rect", {
		x: "3",
		y: "3",
		width: "18",
		height: "18",
		rx: "2",
		key: "h1oib"
	}], ["path", {
		d: "M9 9.003a1 1 0 0 1 1.517-.859l4.997 2.997a1 1 0 0 1 0 1.718l-4.997 2.997A1 1 0 0 1 9 14.996z",
		key: "kmsa83"
	}]],
	aliases: ["play-square"]
};
xi.node;
var Si = (e) => k(U, A(e, { icon: xi })), Ci = {
	name: "square-terminal",
	size: 24,
	node: [
		["path", {
			d: "m7 11 2-2-2-2",
			key: "1lz0vl"
		}],
		["path", {
			d: "M11 13h4",
			key: "1p7l4v"
		}],
		["rect", {
			width: "18",
			height: "18",
			x: "3",
			y: "3",
			rx: "2",
			ry: "2",
			key: "1m3agn"
		}]
	],
	aliases: ["terminal-square"]
};
Ci.node;
var wi = (e) => k(U, A(e, { icon: Ci })), Ti = {
	name: "star",
	size: 24,
	node: [["path", {
		d: "M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z",
		key: "r04s7s"
	}]]
};
Ti.node;
var Ei = (e) => k(U, A(e, { icon: Ti })), Di = {
	name: "stethoscope",
	size: 24,
	node: [
		["path", {
			d: "M11 2v2",
			key: "1539x4"
		}],
		["path", {
			d: "M5 2v2",
			key: "1yf1q8"
		}],
		["path", {
			d: "M5 3H4a2 2 0 0 0-2 2v4a6 6 0 0 0 12 0V5a2 2 0 0 0-2-2h-1",
			key: "rb5t3r"
		}],
		["path", {
			d: "M8 15a6 6 0 0 0 12 0v-3",
			key: "x18d4x"
		}],
		["circle", {
			cx: "20",
			cy: "10",
			r: "2",
			key: "ts1r5v"
		}]
	]
};
Di.node;
var Oi = (e) => k(U, A(e, { icon: Di })), ki = {
	name: "tags",
	size: 24,
	node: [
		["path", {
			d: "M13.172 2a2 2 0 0 1 1.414.586l6.71 6.71a2.4 2.4 0 0 1 0 3.408l-4.592 4.592a2.4 2.4 0 0 1-3.408 0l-6.71-6.71A2 2 0 0 1 6 9.172V3a1 1 0 0 1 1-1z",
			key: "16rjxf"
		}],
		["path", {
			d: "M2 7v6.172a2 2 0 0 0 .586 1.414l6.71 6.71a2.4 2.4 0 0 0 3.191.193",
			key: "178nd4"
		}],
		["circle", {
			cx: "10.5",
			cy: "6.5",
			r: ".5",
			fill: "currentColor",
			key: "12ikhr"
		}]
	]
};
ki.node;
var Ai = (e) => k(U, A(e, { icon: ki })), ji = {
	name: "trash",
	size: 24,
	node: [
		["path", {
			d: "M10 11v6",
			key: "nco0om"
		}],
		["path", {
			d: "M14 11v6",
			key: "outv1u"
		}],
		["path", {
			d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6",
			key: "miytrc"
		}],
		["path", {
			d: "M3 6h18",
			key: "d0wm0j"
		}],
		["path", {
			d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",
			key: "e791ji"
		}]
	],
	aliases: ["trash-2"]
};
ji.node;
var Mi = (e) => k(U, A(e, { icon: ji })), Ni = {
	name: "triangle-alert",
	size: 24,
	node: [
		["path", {
			d: "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3",
			key: "wmoenq"
		}],
		["path", {
			d: "M12 9v4",
			key: "juzpu7"
		}],
		["path", {
			d: "M12 17h.01",
			key: "p32p05"
		}]
	],
	aliases: ["alert-triangle"]
};
Ni.node;
var Pi = (e) => k(U, A(e, { icon: Ni })), Fi = {
	name: "trophy",
	size: 24,
	node: [
		["path", {
			d: "M10 14.66V17a1 1 0 0 1-1 1 2 2 0 0 0-2 2v2",
			key: "pwuv1l"
		}],
		["path", {
			d: "M14 14.66V17a1 1 0 0 0 1 1 2 2 0 0 1 2 2v2",
			key: "1y54w1"
		}],
		["path", {
			d: "M17.916 10H19.5A2.5 2.5 0 0 0 22 7.5V5a1 1 0 0 0-1-1h-3",
			key: "e30mpu"
		}],
		["path", {
			d: "M4 22h16",
			key: "57wxv0"
		}],
		["path", {
			d: "M6 9a6 6 0 0 0 12 0V3a1 1 0 0 0-1-1H7a1 1 0 0 0-1 1z",
			key: "1mhfuq"
		}],
		["path", {
			d: "M6.084 10H4.5A2.5 2.5 0 0 1 2 7.5V5a1 1 0 0 1 1-1h3",
			key: "i0yafy"
		}]
	]
};
Fi.node;
var Ii = (e) => k(U, A(e, { icon: Fi })), Li = {
	name: "undo-2",
	size: 24,
	node: [["path", {
		d: "M9 14 4 9l5-5",
		key: "102s5s"
	}], ["path", {
		d: "M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11",
		key: "f3b9sd"
	}]]
};
Li.node;
var Ri = (e) => k(U, A(e, { icon: Li })), zi = {
	name: "unplug",
	size: 24,
	node: [
		["path", {
			d: "m19 5 3-3",
			key: "yk6iyv"
		}],
		["path", {
			d: "m2 22 3-3",
			key: "19mgm9"
		}],
		["path", {
			d: "M6.3 20.3a2.4 2.4 0 0 0 3.4 0L12 18l-6-6-2.3 2.3a2.4 2.4 0 0 0 0 3.4Z",
			key: "goz73y"
		}],
		["path", {
			d: "M7.5 13.5 10 11",
			key: "7xgeeb"
		}],
		["path", {
			d: "M10.5 16.5 13 14",
			key: "10btkg"
		}],
		["path", {
			d: "m12 6 6 6 2.3-2.3a2.4 2.4 0 0 0 0-3.4l-2.6-2.6a2.4 2.4 0 0 0-3.4 0Z",
			key: "1snsnr"
		}]
	]
};
zi.node;
var Bi = (e) => k(U, A(e, { icon: zi })), Vi = {
	name: "upload",
	size: 24,
	node: [
		["path", {
			d: "M12 3v12",
			key: "1x0j5s"
		}],
		["path", {
			d: "m17 8-5-5-5 5",
			key: "7q97r8"
		}],
		["path", {
			d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4",
			key: "ih7n3h"
		}]
	]
};
Vi.node;
var Hi = (e) => k(U, A(e, { icon: Vi })), Ui = {
	name: "users",
	size: 24,
	node: [
		["path", {
			d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2",
			key: "1yyitq"
		}],
		["path", {
			d: "M16 3.128a4 4 0 0 1 0 7.744",
			key: "16gr8j"
		}],
		["path", {
			d: "M22 21v-2a4 4 0 0 0-3-3.87",
			key: "kshegd"
		}],
		["circle", {
			cx: "9",
			cy: "7",
			r: "4",
			key: "nufk8"
		}]
	]
};
Ui.node;
var Wi = (e) => k(U, A(e, { icon: Ui })), Gi = {
	name: "volume-2",
	size: 24,
	node: [
		["path", {
			d: "M11 4.702a.705.705 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298z",
			key: "uqj9uw"
		}],
		["path", {
			d: "M16 9a5 5 0 0 1 0 6",
			key: "1q6k2b"
		}],
		["path", {
			d: "M19.364 18.364a9 9 0 0 0 0-12.728",
			key: "ijwkga"
		}]
	]
};
Gi.node;
var Ki = (e) => k(U, A(e, { icon: Gi })), qi = {
	name: "volume-x",
	size: 24,
	node: [
		["path", {
			d: "M11 4.702a.7.7 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.7.7 0 0 0 11 19.298z",
			key: "1p7khw"
		}],
		["path", {
			d: "m16.5 14.5 5-5",
			key: "cul3yw"
		}],
		["path", {
			d: "m16.5 9.5 5 5",
			key: "1akey5"
		}]
	]
};
qi.node;
var Ji = (e) => k(U, A(e, { icon: qi })), Yi = {
	name: "waves-horizontal",
	size: 24,
	node: [
		["path", {
			d: "M2 12q2.5 2 5 0t5 0 5 0 5 0",
			key: "8ddzzs"
		}],
		["path", {
			d: "M2 19q2.5 2 5 0t5 0 5 0 5 0",
			key: "1wj4st"
		}],
		["path", {
			d: "M2 5q2.5 2 5 0t5 0 5 0 5 0",
			key: "69x50u"
		}]
	],
	aliases: ["waves"]
};
Yi.node;
var Xi = (e) => k(U, A(e, { icon: Yi })), Zi = {
	name: "x",
	size: 24,
	node: [["path", {
		d: "M18 6 6 18",
		key: "1bl5f8"
	}], ["path", {
		d: "m6 6 12 12",
		key: "d8bk6v"
	}]]
};
Zi.node;
//#endregion
//#region ../phemios/src/components/icon-symbols.ts
var Qi = new Map(Object.entries({
	ipod: gi,
	"ipod-nano": Wr,
	"ipod-touch": gi,
	"ipod-square": Si,
	"ipod-shuffle": ci,
	folder: On,
	integrity: oi,
	"cover-size": ei,
	external: mn,
	concerts: cr,
	"visualizer-off": fr,
	terminal: wi,
	crossfade: rn,
	"chevron-right": Zt,
	book: Lt,
	trophy: Ii,
	lock: ir,
	"cover-flow": An,
	keyboard: Kn,
	grip: Pn,
	reverse: Ct,
	playlist: wn,
	help: tn,
	moods: Xi,
	visualizer: mr,
	"lyrics-download": Sn,
	disconnect: Bi,
	landscape: gr,
	"lyrics-timed": bn,
	lyrics: En,
	plus: Ir,
	minus: ur,
	"chevron-up": $t,
	"chevron-down": Yt,
	check: qt,
	podcast: Rr,
	gear: ii,
	sparkles: vi,
	palette: Sr,
	trash: Mi,
	edit: Or,
	doctor: Oi,
	radio: Hr,
	muted: Ji,
	refresh: Jr,
	settings: mi,
	stop: bi,
	close: (e) => k(U, A(e, { icon: Zi })),
	upload: Hi,
	download: un,
	spectrum: Ut,
	star: Ei,
	bold: Ft,
	italic: Wn,
	heading: Rn,
	paragraph: Mr,
	ordered: nr,
	quote: Br,
	undo: Ri,
	redo: Kr,
	stats: Gt,
	calendar: Vt,
	alphabet: xt,
	previous: ui,
	next: fi,
	pause: Er,
	repeat: Xr,
	"repeat-one": Qr,
	shuffle: ci,
	volume: Ki,
	menu: or,
	sort: Tt,
	users: Wi,
	cd: cn,
	eject: fn,
	disc: on,
	layers: Jn,
	grid: Xn,
	globe: Mn,
	missing: Pi,
	eye: gn,
	"eye-off": vn,
	label: zt,
	note: vr,
	genres: Ar,
	tags: Ai,
	heart: Bn,
	library: Qn,
	history: Hn,
	list: er,
	wave: Nt,
	graph: br,
	drive: In,
	search: ni,
	sidebar: wr,
	play: Pr,
	up: jt,
	forward: kt,
	back: Dt
}));
Object.freeze([...Qi.keys()]);
function $i(e) {
	return Qi.get(e) ?? er;
}
//#endregion
//#region ../phemios/src/components/Icon.tsx
function W(e) {
	return k(ut, {
		get component() {
			return $i(e.name);
		},
		size: 18,
		get viewBox() {
			return e.name === "stop" ? "-2 -2 28 28" : "0 0 24 24";
		},
		strokeWidth: 1.6,
		color: "currentColor",
		"aria-hidden": "true"
	});
}
//#endregion
//#region ../phemios/src/analysis-report/ReportDisclosure.tsx
var ea = /*#__PURE__*/ P("<details><summary>");
function G(e) {
	return (() => {
		var t = ea(), n = t.firstChild;
		return t.addEventListener("toggle", (t) => {
			let n = t.currentTarget;
			n.open !== e.open && (e.onChange(n.open), n.open = e.open);
		}), n.$$click = (t) => {
			t.preventDefault(), e.onChange(!e.open);
		}, R(n, () => e.label, null), R(n, k(W, { name: "chevron-down" }), null), R(t, () => e.children, null), x((n) => {
			var r = `far-disclosure ${e.section ? "far-section" : ""}`, i = e.open;
			return r !== n.e && L(t, n.e = r), i !== n.t && (t.open = n.t = i), n;
		}, {
			e: void 0,
			t: void 0
		}), t;
	})();
}
F(["click"]);
//#endregion
//#region ../phemios/src/analysis-report/AnalysisTechnical.tsx
var ta = /*#__PURE__*/ P("<dt>"), na = /*#__PURE__*/ P("<dd> % FS"), ra = /*#__PURE__*/ P("<dd> dB"), ia = /*#__PURE__*/ P("<dd>"), aa = /*#__PURE__*/ P("<dl>"), oa = /*#__PURE__*/ P("<p>"), sa = /*#__PURE__*/ P("<dd> LUFS · <!> s");
function ca(e) {
	let t = () => e.data, n = e.tr;
	return k(G, {
		get label() {
			return n("Mesures complémentaires");
		},
		get open() {
			return e.expanded;
		},
		get onChange() {
			return e.onExpandedChange;
		},
		section: !0,
		get children() {
			return [(() => {
				var r = aa();
				return R(r, k(j, {
					get each() {
						return t().loudnessPeaks;
					},
					children: (t) => [(() => {
						var e = ta();
						return R(e, () => n(t.kind === "momentary" ? "LUFS-M max · fenêtre 400 ms" : "LUFS-S max · fenêtre 3 s")), e;
					})(), (() => {
						var n = sa(), r = n.firstChild, i = r.nextSibling;
						return i.nextSibling, R(n, () => t.lufs.toLocaleString(e.locale), r), R(n, () => t.start.toLocaleString(e.locale), i), n;
					})()]
				}), null), R(r, k(M, {
					get when() {
						return t().dc !== void 0;
					},
					get children() {
						return [(() => {
							var e = ta();
							return R(e, () => n("Décalage continu max.")), e;
						})(), (() => {
							var n = na(), r = n.firstChild;
							return R(n, () => ((t().dc ?? 0) * 100).toLocaleString(e.locale), r), n;
						})()];
					}
				}), null), R(r, k(M, {
					get when() {
						return t().balance !== void 0;
					},
					get children() {
						return [(() => {
							var e = ta();
							return R(e, () => n("Balance droite − gauche")), e;
						})(), (() => {
							var n = ra(), r = n.firstChild;
							return R(n, () => t().balance?.toLocaleString(e.locale), r), n;
						})()];
					}
				}), null), R(r, k(M, {
					get when() {
						return t().lattice !== void 0;
					},
					get children() {
						return [(() => {
							var e = ta();
							return R(e, () => n("Indice de signature de quantification")), e;
						})(), (() => {
							var n = ia();
							return R(n, () => t().lattice?.toLocaleString(e.locale, { maximumFractionDigits: 5 })), n;
						})()];
					}
				}), null), r;
			})(), (() => {
				var e = oa();
				return R(e, () => n("L’indice de signature n’est pas une probabilité ni un pourcentage de confiance.")), e;
			})()];
		}
	});
}
//#endregion
//#region ../phemios/src/analysis-report/coverage.ts
function la(e) {
	let t = e.analyses_run;
	if (t == null) return e;
	if (!Array.isArray(t) || t.length > 64 || t.some((e) => typeof e != "string" || e.length > 64)) throw Error("Couverture FlacCompagnon invalide.");
	let n = { ...e };
	for (let [e, r] of Object.entries({
		authenticity: ["detections", "lattice_score"],
		"bit-depth": ["real_bit_depth", "bit_depth_evidence"],
		spectrum: ["cutoff_hz", "cutoff_ratio"],
		stereo: ["fake_stereo", "stereo_balance"],
		phase: [
			"phase_correlation",
			"phase_inverted",
			"local_phase"
		],
		"hf-stereo": ["high_frequency_stereo"],
		"dc-offset": ["dc_offset"],
		clipping: ["clipping"],
		dynamics: ["dr_db"],
		loudness: [
			"integrated_lufs",
			"loudness_peaks",
			"loudness_range_lu"
		],
		"flac-md5": ["flac_md5"],
		fingerprints: ["file_md5", "file_crc32"]
	})) if (!t.includes(e)) for (let e of r) n[e] = null;
	if (n.discontinuities && typeof n.discontinuities == "object" && !Array.isArray(n.discontinuities)) {
		let e = { ...n.discontinuities };
		for (let n of ["clicks", "dropouts"]) t.includes(n) || Object.assign(e, { [n]: null });
		n.discontinuities = e;
	}
	return n;
}
//#endregion
//#region ../phemios/src/analysis-report/measurements.ts
function K(e) {
	if (e == null) return {};
	if (typeof e != "object" || Array.isArray(e)) throw Error("Données FlacCompagnon invalides.");
	return e;
}
function q(e, t = -Infinity, n = Infinity) {
	if (e != null) {
		if (typeof e != "number" || !Number.isFinite(e) || e < t || e > n) throw Error("Mesure FlacCompagnon invalide.");
		return e;
	}
}
function ua(e) {
	return typeof e == "string" ? e : "";
}
function da(e) {
	let t = K(e), n = la(t.source_data ? K(t.source_data) : t);
	if (t.source !== "flaccompagnon") throw Error("Source d’analyse non prise en charge.");
	let r = K(n.clipping), i = K(n.detections), a = [], o = [];
	function s(e, t, n, r, i, s = -Infinity, c = Infinity) {
		let l = q(n, s, c);
		l === void 0 ? o.push(e) : a.push({
			label: e,
			unit: t,
			value: l,
			low: r,
			high: i
		});
	}
	s("Niveau intégré", "LUFS", n.integrated_lufs, -60, 0), s("Dynamique", "dB DR", n.dr_db, 0, 30, 0), s("Étendue de sonie", "LU", n.loudness_range_lu, 0, 30, 0), s("Crête échantillon", "dBFS", r.peak_dbfs ?? n.peak_dbfs, -60, 3), s("Crête vraie", "dBTP", r.true_peak_dbtp ?? n.true_peak_dbtp, -60, 3), s("Précision déclarée", "bits", n.declared_bits, 0, 32, 0, 64), s("Précision mesurée", "bits", n.real_bit_depth, 0, 32, 0, 64);
	let c = n.sample_rate === 0 ? void 0 : q(n.sample_rate, 1);
	c ? s("Coupure spectrale", "Hz", n.cutoff_hz, 0, c / 2, 0) : o.push("Coupure spectrale"), s("Corrélation stéréo", "", n.phase_correlation, -1, 1, -1, 1), s("Balance D/G", "dB", K(n.stereo_balance).right_minus_left_db, -12, 12);
	let l = K(n.local_phase);
	if (l.bands !== void 0 && !Array.isArray(l.bands)) throw Error("Bandes de phase invalides.");
	for (let e of l.bands?.slice(0, 16) ?? []) {
		let t = K(e), n = q(t.low_hz, 0), r = q(t.high_hz, 0);
		n !== void 0 && r !== void 0 && s(`Phase ${n}–${r} Hz`, "", K(t.summary).correlation, -1, 1, -1, 1);
	}
	l.bands?.length || o.push("Phase par bandes");
	let u = q(n.duration_secs, 0), d = K(n.discontinuities), f = 0, p = !0, m = 0, h = [];
	for (let [e, t] of [["clicks", "Clic"], ["dropouts", "Coupure"]]) {
		let n = K(d[e ?? ""]), r = q(n.count, 0, 2 ** 53 - 1);
		if (r !== void 0 && !Number.isInteger(r)) throw Error("Nombre d’événements invalide.");
		if (r === void 0 && (p = !1), f += r ?? 0, !Number.isSafeInteger(f)) throw Error("Nombre d’événements trop grand.");
		if (n.events !== void 0 && !Array.isArray(n.events)) throw Error("Événements invalides.");
		let i = n.events ?? [];
		m += i.length;
		for (let e of i.slice(0, Math.max(0, 1e3 - h.length))) {
			let n = K(e), r = q(n.start_secs, 0), i = q(n.duration_secs, 0), a = q(n.channel, 1);
			if (r === void 0 || i === void 0 || a === void 0 || !Number.isInteger(a)) throw Error("Position d’événement invalide.");
			if (u !== void 0 && u > 0 && (r > u || r + i > u + 1e-6)) throw Error("Position d’événement hors de la piste.");
			h.push({
				start: r,
				duration: i,
				channel: a,
				kind: t ?? "Événement"
			});
		}
	}
	return {
		gauges: a,
		missing: o,
		events: h,
		duration: u,
		eventCount: p ? f : void 0,
		retainedEvents: m,
		stale: typeof t.stale == "boolean" ? t.stale : void 0,
		summary: ua(i.summary ?? (n.analyses_run == null ? t.summary : void 0)),
		detail: ua(i.detail ?? (n.analyses_run == null ? t.detail : void 0)),
		error: ua(n.error ?? t.error),
		md5: ua(K(n.flac_md5).state ?? (n.analyses_run == null ? t.md5_state : void 0)),
		clipped: typeof r.clipped == "boolean" ? r.clipped : n.analyses_run == null && typeof t.clipped == "boolean" ? t.clipped : void 0
	};
}
function J(e) {
	if (!e || typeof e != "object" || Array.isArray(e)) throw Error("Données FlacCompagnon invalides.");
	return e;
}
var Y = (e) => typeof e == "string" ? e : "";
function X(e) {
	return e == null ? {} : J(e);
}
function Z(e, t = 0, n = 2 ** 53 - 1) {
	if (e != null) {
		if (typeof e != "number" || !Number.isFinite(e) || e < t || e > n) throw Error("Mesure FlacCompagnon invalide.");
		return e;
	}
}
function Q(e) {
	if (e != null) {
		if (typeof e != "boolean") throw Error("Données FlacCompagnon invalides.");
		return e;
	}
}
function fa(e) {
	let t = J(e);
	if (t.source !== void 0 && t.source !== "flaccompagnon") throw Error("Source d’analyse non prise en charge.");
	let n = la(t.source_data == null ? t : J(t.source_data));
	for (let [e, t] of Object.entries({
		file_name: 1024,
		path: 16384,
		format: 64,
		codec: 128,
		error: 16384
	})) {
		let r = n[e];
		if (r != null && (typeof r != "string" || r.length > t)) throw Error("Données FlacCompagnon invalides.");
	}
	let r = da({
		...t,
		source: "flaccompagnon",
		source_data: n
	}), i = X(n.detections), a = X(n.clipping), o = X(n.flac_md5), s = X(n.bit_depth_evidence), c = Y(n.format), l = Y(n.file_name) || Y(n.path).split(/[\\/]/).pop() || "", u = Z(n.channels, 0, 64), d = Z(n.declared_bits, 0, 64), f = Z(n.real_bit_depth, 0, 64), p = Z(s.stored_bits, 0, 64);
	for (let e of [
		u,
		d,
		f,
		p
	]) if (e !== void 0 && !Number.isInteger(e)) throw Error("Mesure FlacCompagnon invalide.");
	let m = Y(o.state) || (n.analyses_run == null ? Y(t.md5_state) : "");
	if (m && ![
		"Match",
		"Mismatch",
		"NoSignature",
		"Present",
		"Error"
	].includes(m)) throw Error("État MD5 FlacCompagnon invalide.");
	if (Y(i.detail).length > 32768 || Y(o.detail).length > 16384) throw Error("Données FlacCompagnon invalides.");
	for (let [e, t] of [["file_md5", /^[a-f0-9]{32}$/i], ["file_crc32", /^[a-f0-9]{8}$/i]]) if (n[e] != null && !t.test(Y(n[e]))) throw Error("Empreinte de fichier invalide.");
	let h = X(n.loudness_peaks), g = ["momentary", "short_term"].flatMap((e) => {
		let t = X(h[e]), n = Z(t.lufs, -Infinity, Infinity), r = Z(t.start_secs);
		return n === void 0 || r === void 0 ? [] : [{
			kind: e,
			lufs: n,
			start: r
		}];
	}), _ = Z(X(n.dc_offset).max_abs), v = Z(n.lattice_score, 0, 1), y = [
		{
			key: "upscaling",
			label: "Upscaled",
			name: "Profondeur artificiellement augmentée"
		},
		{
			key: "upsampling",
			label: "Upsampled",
			name: "Suréchantillonnage"
		},
		{
			key: "transcoding",
			label: "Transcoded",
			name: "Transcodage avec pertes"
		}
	].map((e) => ({
		...e,
		value: Q(i[e.key])
	})), b = y.filter((e) => e.value === !0).map((e) => e.label), x = Y(i.summary ?? (n.analyses_run == null ? t.summary : void 0));
	return b.length || b.push(x === "Clean" ? "Clean" : x === "Flagged" ? "Indices détectés" : "Non analysé"), {
		...r,
		data: n,
		loudnessPeaks: g,
		dc: _,
		lattice: v,
		filename: l,
		format: c,
		codec: Y(n.codec),
		channels: u || void 0,
		bits: d,
		effective: f,
		stored: p,
		integerDepthApplicable: !/^(MP3|AAC)$/i.test(c) && !/vorbis|opus|aac|mp3/i.test(Y(n.codec)),
		bitMethod: Y(s.method),
		checks: y,
		labels: b,
		md5State: m,
		md5Detail: Y(o.detail),
		applicableMd5: !c || c.toUpperCase() === "FLAC",
		extension: l.match(/\.([^./\\]+)$/)?.[1]?.toLowerCase(),
		extensionMismatch: c ? Q(n.ext_mismatch) : void 0,
		rate: Z(n.sample_rate) || void 0,
		size: Z(n.size_bytes) || void 0,
		bitrate: Z(n.bitrate_kbps),
		dualMono: Q(n.fake_stereo),
		inverted: Q(n.phase_inverted),
		correlation: Z(n.phase_correlation, -1, 1),
		balance: Z(X(n.stereo_balance).right_minus_left_db, -Infinity, Infinity),
		cutoff: Z(n.cutoff_hz),
		clipped: Q(a.clipped ?? (n.analyses_run == null ? t.clipped : void 0)),
		clippedSamples: Z(a.clipped_samples),
		clipEvents: Z(a.clip_events),
		fileMd5: Y(n.file_md5),
		crc32: Y(n.file_crc32),
		coverage: Array.isArray(n.analyses_run) ? n.analyses_run.filter((e) => typeof e == "string") : void 0
	};
}
function pa(e) {
	if (e.length > 16777216 || new TextEncoder().encode(e).length > 16777216) throw Error("Le rapport dépasse la limite de 16 Mio.");
	let t;
	try {
		t = JSON.parse(e);
	} catch {
		throw Error("Le fichier JSON est illisible.");
	}
	let n = J(t), r = n;
	if (n.format === "flaccompagnon-report") {
		if (n.version !== 1) throw Error("Version de rapport FlacCompagnon non prise en charge.");
		r = J(n.report);
	} else if ("version" in n || "report" in n) throw Error("Rapport FlacCompagnon non reconnu.");
	let i = Array.isArray(r.files) ? r.files : [r];
	if (!i.length || i.length > 2e3) throw Error("Le rapport doit contenir entre 1 et 2 000 fichiers.");
	for (let e of i) {
		let t = J(e);
		if (!Y(t.file_name) || typeof t.format != "string" || t.source_data !== void 0) throw Error("Rapport FlacCompagnon non reconnu.");
		fa(e);
	}
	return i.map((e) => ({
		source: "flaccompagnon",
		source_data: e
	}));
}
//#endregion
//#region ../phemios/src/analysis-report/presentation.ts
var ma = (e, ...t) => e.replace(/\{(\d+)\}/g, (e, n) => Number(n) < t.length ? String(t[Number(n)]) : e);
function ha(e) {
	let t = {
		Match: {
			tone: "good",
			label: "MD5 audio FLAC conforme",
			detail: "L’audio décodé correspond à la signature enregistrée dans le FLAC.",
			proof: "La comparaison porte sur l’audio entièrement décodé et la signature interne STREAMINFO. Les tags et la pochette n’entrent pas dans ce contrôle."
		},
		Mismatch: {
			tone: "bad",
			label: "MD5 audio FLAC différent",
			detail: "L’audio décodé ne correspond pas à la signature enregistrée.",
			proof: "La comparaison a abouti, mais les signatures diffèrent. Vérifiez le fichier ou sa source ; la cause exacte n’est pas déterminée par ce seul résultat."
		},
		NoSignature: {
			tone: "unknown",
			label: "Aucune signature MD5 audio",
			detail: "Ce FLAC ne contient pas de signature de référence exploitable.",
			proof: "Ce n’est pas un résultat KO. L’empreinte du fichier entier est une information différente."
		},
		Present: {
			tone: "unknown",
			label: "MD5 audio présent, non vérifié",
			detail: "La signature existe, mais sa comparaison n’a pas été effectuée.",
			proof: "L’existence d’une signature ne confirme pas que l’audio lui correspond."
		},
		Error: {
			tone: "bad",
			label: "Vérification MD5 interrompue",
			detail: "Le contrôle MD5 n’a pas pu aboutir.",
			proof: "Une erreur de lecture se distingue d’une comparaison terminée avec un MD5 différent."
		}
	}, n = {
		tone: "unknown",
		label: "Intégrité non vérifiée",
		detail: "Aucun verdict de vérification audio n’est fourni.",
		proof: "Une mesure absente n’est ni OK ni KO. Une empreinte du fichier seul ne vérifie pas son intégrité."
	}, r = e.applicableMd5 ? t[e.md5State] ?? n : {
		...n,
		label: e.format.toUpperCase() === "OGG" ? "Ogg : contrôle des pages non fourni" : "MD5 audio FLAC non applicable",
		detail: "Ce rapport ne fournit pas de verdict d’intégrité pour ce format."
	};
	return e.stale === !0 ? {
		...r,
		tone: "warning",
		detail: "Le fichier a changé depuis cette analyse. Ce résultat concerne l’ancienne version."
	} : e.stale === void 0 && r.tone === "good" ? {
		...r,
		detail: "Conforme dans le rapport ; le fichier actuel n’a pas été vérifié par Phémios."
	} : r;
}
var ga = {
	"Niveau intégré": "Plus près de zéro signifie plus fort. Cela ne note pas la qualité du morceau.",
	Dynamique: "Rapport entre les crêtes et les passages soutenus les plus forts ; différent de la LRA.",
	"Étendue de sonie": "Variation des niveaux à court terme au fil du morceau ; interprétation approximative sous une minute.",
	"Crête échantillon": "Maximum des échantillons stockés. Le drapeau d’écrêtage est indiqué séparément.",
	"Crête vraie": "Estimation des crêtes entre les échantillons ; ce n’est pas une mesure de la sortie physique.",
	"Précision déclarée": "Profondeur annoncée par le conteneur pour un signal PCM entier.",
	"Précision mesurée": "Précision estimée du signal ; elle ne certifie pas l’origine de l’enregistrement.",
	"Coupure spectrale": "La coupure seule ne prouve pas un suréchantillonnage ni une conversion avec pertes.",
	"Corrélation stéréo": "Une valeur faible ou négative peut être un choix artistique.",
	"Balance D/G": "Différence RMS droite moins gauche, sans pondération."
}, _a = /*#__PURE__*/ P("<div class=far-identity>"), va = /*#__PURE__*/ P("<div><span></span><strong></strong><small>");
function ya(e) {
	let t = (t, n = "") => t === void 0 ? e.tr("Non mesuré") : `${new Intl.NumberFormat(e.locale, { maximumFractionDigits: 2 }).format(t)}${n ? ` ${n}` : ""}`, n = () => [
		{
			label: "Extension → format identifié",
			value: `${e.data.extension ? `.${e.data.extension}` : e.tr("Extension inconnue")} → ${e.data.format || e.tr("Format inconnu")}`,
			note: e.data.extensionMismatch === !0 ? e.tr("Extension incohérente") : e.data.extensionMismatch === !1 ? e.tr("Extension cohérente") : e.tr("Correspondance non renseignée"),
			warn: e.data.extensionMismatch === !0
		},
		{
			label: "Codec",
			value: [e.data.format, e.data.codec].filter(Boolean).join(" · ") || e.tr("Non renseigné"),
			note: e.tr("Le format sans perte ne certifie pas l’origine de la source.")
		},
		{
			label: "Canaux",
			value: e.data.channels === 2 ? e.tr("2 canaux · stéréo") : t(e.data.channels, e.tr("canaux")),
			note: e.data.dualMono === !0 ? e.tr("Dual mono détecté") : e.data.dualMono === !1 ? e.tr("Dual mono non détecté") : e.tr("Dual mono non mesuré"),
			warn: e.data.dualMono === !0
		},
		{
			label: "Profondeur de bits",
			value: e.data.integerDepthApplicable ? e.tr("Déclarés : {0} · effectifs : {1}", t(e.data.bits), `${e.data.effective !== void 0 && e.data.bitMethod !== "Stored" ? "≈ " : ""}${t(e.data.effective)}`) : e.tr("Non applicable au codec avec pertes"),
			note: e.data.stored === void 0 ? e.tr("La précision effective est une estimation, pas une preuve d’origine.") : e.tr("Bits occupés dans les échantillons entiers : {0}.", t(e.data.stored)),
			warn: e.data.bits !== void 0 && e.data.effective !== void 0 && e.data.effective < e.data.bits
		},
		{
			label: "Fréquence d’échantillonnage",
			value: e.data.rate === void 0 ? e.tr("Non mesuré") : t(e.data.rate / 1e3, "kHz"),
			note: e.tr("Coupure mesurée : {0}", e.data.cutoff === void 0 ? e.tr("Non mesuré") : t(e.data.cutoff / 1e3, "kHz"))
		},
		{
			label: "Stéréo et polarité",
			value: e.tr("Corrélation : {0}", t(e.data.correlation)),
			note: e.data.inverted === !0 ? e.tr("Inversion de polarité suspectée") : e.data.inverted === !1 ? e.tr("Inversion de polarité non signalée") : e.tr("Polarité non mesurée"),
			warn: e.data.inverted === !0
		},
		{
			label: "Durée · taille · débit global",
			value: [
				e.data.duration === void 0 ? e.tr("Non mesuré") : `${Math.floor(e.data.duration / 60)}:${String(Math.floor(e.data.duration % 60)).padStart(2, "0")}`,
				e.data.size === void 0 ? e.tr("Taille inconnue") : t(e.data.size / 1e6, "Mo"),
				t(e.data.bitrate, "kb/s")
			].join(" · "),
			note: e.tr("Débit du fichier entier, conteneur compris.")
		}
	];
	return (() => {
		var t = _a();
		return R(t, k(j, {
			get each() {
				return n();
			},
			children: (t) => (() => {
				var n = va(), r = n.firstChild, i = r.nextSibling, a = i.nextSibling;
				return R(r, () => e.tr(t.label)), R(i, () => t.value), R(a, (() => {
					var e = N(() => !!t.warn);
					return () => e() && k(W, { name: "missing" });
				})(), null), R(a, () => t.note, null), x(() => a.classList.toggle("far-warning", !!t.warn)), n;
			})()
		})), t;
	})();
}
//#endregion
//#region ../phemios/src/analysis-report/EventTimeline.tsx
var ba = /*#__PURE__*/ P("<figure class=far-events><svg viewBox=\"0 0 600 44\"preserveAspectRatio=none role=group><line x1=0 y1=22 x2=600 y2=22></line></svg><figcaption><span>0:00</span><span></span></figcaption><output role=status class=far-event-detail>"), xa = /*#__PURE__*/ P("<svg><g role=button tabindex=0><rect class=far-event-hit width=18 height=22></rect><rect height=14></svg>", !1, !0, !1);
function Sa(e) {
	let t = je(), [n, r] = b(), [i, a] = b(), [o, s] = b(), c = () => e.events[n() ?? i() ?? o() ?? -1], l = () => {
		r(void 0), a(void 0), s(void 0);
	};
	ee(() => {
		e.events, l();
	});
	let u = (t) => t.toLocaleString(e.locale, { maximumFractionDigits: 6 }), d = (t) => {
		let n = Math.round(t * 100);
		return `${Math.floor(n / 6e3)}:${(n % 6e3 / 100).toLocaleString(e.locale, {
			minimumIntegerDigits: 2,
			maximumFractionDigits: 2
		})}`;
	}, f = (t) => `${e.tr(t.kind)} · ${u(t.start)} s · ${e.tr("Durée")} : ${u(t.duration)} s · ${e.tr("Canal")} ${t.channel}`, p = (t) => t.start / e.duration * 600, m = (e) => {
		n() === e ? l() : r(e);
	};
	return (() => {
		var r = ba(), i = r.firstChild;
		i.firstChild;
		var o = i.nextSibling, u = o.firstChild.nextSibling, h = o.nextSibling;
		return r.$$keydown = (e) => {
			e.key === "Escape" && (e.preventDefault(), l());
		}, R(i, k(j, {
			get each() {
				return e.events;
			},
			children: (r, i) => (() => {
				var o = xa(), l = o.firstChild, u = l.nextSibling;
				return o.$$keydown = (e) => {
					(e.key === "Enter" || e.key === " ") && (e.preventDefault(), m(i()));
				}, o.addEventListener("blur", () => s(void 0)), o.addEventListener("focus", () => s(i())), o.addEventListener("pointerleave", () => a(void 0)), o.addEventListener("pointerenter", (e) => {
					e.pointerType === "mouse" && a(i());
				}), o.$$click = () => m(i()), x((a) => {
					var s = f(r), d = n() === i(), m = c() === r ? t : void 0, h = Math.min(582, Math.max(0, p(r) - 9)), g = r.kind === "Clic" ? 0 : 22, _ = Math.min(598, p(r)), v = r.kind === "Clic" ? 5 : 25, y = Math.min(600 - Math.min(598, p(r)), Math.max(2, r.duration / e.duration * 600));
					return s !== a.e && I(o, "aria-label", a.e = s), d !== a.t && I(o, "aria-pressed", a.t = d), m !== a.a && I(o, "aria-describedby", a.a = m), h !== a.o && I(l, "x", a.o = h), g !== a.i && I(l, "y", a.i = g), _ !== a.n && I(u, "x", a.n = _), v !== a.s && I(u, "y", a.s = v), y !== a.h && I(u, "width", a.h = y), a;
				}, {
					e: void 0,
					t: void 0,
					a: void 0,
					o: void 0,
					i: void 0,
					n: void 0,
					s: void 0,
					h: void 0
				}), o;
			})()
		}), null), R(u, () => d(e.duration)), I(h, "id", t), R(h, k(M, {
			get when() {
				return c();
			},
			children: (e) => f(e())
		})), x(() => I(i, "aria-label", e.tr("Positions conservées des clics et coupures sur {0} secondes", e.duration.toLocaleString(e.locale, { maximumFractionDigits: 2 })))), r;
	})();
}
F(["keydown", "click"]);
//#endregion
//#region ../phemios/src/analysis-report/MeasurementMarker.tsx
var Ca = /*#__PURE__*/ P("<span class=far-measure-tooltip role=tooltip><strong> : </strong><span>"), wa = /*#__PURE__*/ P("<div class=far-marker-hint><button type=button class=far-rail-button><span class=far-scale aria-hidden=true><i>");
function Ta(e) {
	let t = je(), n, [r, i] = b(!1), [a, o] = b(!1), [s, c] = b(!1), [l, u] = b(!1), d = () => !l() && (r() || a() || s()), f = () => {
		c(!1), u(!0);
	};
	return ee(() => {
		if (!s()) return;
		let e = (e) => {
			e.target instanceof Node && !n.contains(e.target) && f();
		};
		document.addEventListener("pointerdown", e), te(() => document.removeEventListener("pointerdown", e));
	}), (() => {
		var r = wa(), a = r.firstChild, l = a.firstChild.firstChild;
		return Ze((e) => {
			n = e;
		}, r), a.$$keydown = (e) => {
			e.key === "Escape" && (e.preventDefault(), f());
		}, a.$$click = () => {
			s() ? f() : (c(!0), u(!1));
		}, a.addEventListener("blur", () => {
			o(!1), c(!1), u(!0);
		}), a.addEventListener("focus", () => {
			o(!0), u(!1);
		}), a.addEventListener("pointerleave", () => i(!1)), a.addEventListener("pointerenter", (e) => {
			e.pointerType === "mouse" && (i(!0), u(!1));
		}), R(r, k(M, {
			get when() {
				return d();
			},
			get children() {
				var n = Ca(), r = n.firstChild, i = r.firstChild, a = r.nextSibling;
				return I(n, "id", t), R(r, () => e.label, i), R(r, () => e.value, null), R(a, () => e.help), n;
			}
		}), null), x((n) => {
			var r = `${e.label} : ${e.value}`, i = d() ? t : void 0, o = s(), c = `${Math.max(0, Math.min(100, e.position))}%`;
			return r !== n.e && I(a, "aria-label", n.e = r), i !== n.t && I(a, "aria-describedby", n.t = i), o !== n.a && I(a, "aria-pressed", n.a = o), c !== n.o && Ye(l, "left", n.o = c), n;
		}, {
			e: void 0,
			t: void 0,
			a: void 0,
			o: void 0
		}), r;
	})();
}
F(["click", "keydown"]);
//#endregion
//#region ../phemios/src/analysis-report/MeasurementGauge.tsx
var Ea = /*#__PURE__*/ P("<figure><figcaption></figcaption><strong> <small></small></strong><div class=far-scale-labels><span></span><span> </span></div><p>");
function Da(e) {
	let t = () => `${new Intl.NumberFormat(e.locale, { maximumFractionDigits: 2 }).format(e.gauge.value)} ${e.gauge.unit}`.trim(), n = () => e.tr(ga[e.gauge.label] ?? "Mesure de phase locale, à interpréter selon l’enregistrement.");
	return (() => {
		var r = Ea(), i = r.firstChild, a = i.nextSibling, o = a.firstChild, s = o.nextSibling, c = a.nextSibling, l = c.firstChild, u = l.nextSibling, d = u.firstChild, f = c.nextSibling;
		return R(i, () => e.tr(e.gauge.label)), R(a, () => new Intl.NumberFormat(e.locale, { maximumFractionDigits: 2 }).format(e.gauge.value), o), R(s, () => e.gauge.unit), R(r, k(Ta, {
			get position() {
				return 100 * (e.gauge.value - e.gauge.low) / (e.gauge.high - e.gauge.low);
			},
			get label() {
				return e.tr(e.gauge.label);
			},
			get value() {
				return t();
			},
			get help() {
				return n();
			}
		}), c), R(l, () => e.gauge.low), R(u, () => e.gauge.high, d), R(u, () => e.gauge.unit, null), R(f, n), r;
	})();
}
//#endregion
//#region ../phemios/src/analysis-report/AnalysisMeasures.tsx
var Oa = /*#__PURE__*/ P("<div class=\"far-measures far-additional\">"), ka = /*#__PURE__*/ P("<p class=far-muted>"), Aa = /*#__PURE__*/ P("<section class=far-section><h3></h3><div class=far-measures></div><p class=far-hint>"), ja = /*#__PURE__*/ P("<p>"), Ma = /*#__PURE__*/ P("<p class=far-hint>"), Na = /*#__PURE__*/ P("<section class=far-section><h3></h3><div class=far-observation><div><strong></strong></div></div><div class=far-observation><div><strong></strong><p>"), Pa = /*#__PURE__*/ P("<div class=far-observation><div><strong> : <!> dBTP</strong><p>");
function Fa(e) {
	let t = (t) => new Intl.NumberFormat(e.locale, { maximumFractionDigits: 2 }).format(t), n = () => e.data.gauges.filter((e) => [
		"Niveau intégré",
		"Dynamique",
		"Étendue de sonie"
	].includes(e.label)), r = () => e.data.gauges.filter((e) => ![
		"Niveau intégré",
		"Dynamique",
		"Étendue de sonie"
	].includes(e.label));
	return [(() => {
		var t = Aa(), i = t.firstChild, a = i.nextSibling, o = a.nextSibling;
		return R(i, () => e.tr("Ce que décrivent les niveaux")), R(a, k(j, {
			get each() {
				return n();
			},
			children: (t) => k(Da, {
				gauge: t,
				get tr() {
					return e.tr;
				},
				get locale() {
					return e.locale;
				}
			})
		})), R(t, k(M, {
			get when() {
				return r().length;
			},
			get children() {
				return k(G, {
					get label() {
						return e.tr("Autres mesures");
					},
					get open() {
						return e.expanded;
					},
					get onChange() {
						return e.onExpandedChange;
					},
					get children() {
						var t = Oa();
						return R(t, k(j, {
							get each() {
								return r();
							},
							children: (t) => k(Da, {
								gauge: t,
								get tr() {
									return e.tr;
								},
								get locale() {
									return e.locale;
								}
							})
						})), t;
					}
				});
			}
		}), o), R(t, k(M, {
			get when() {
				return !e.data.gauges.length;
			},
			get children() {
				var t = ka();
				return R(t, () => e.tr("Ce rapport ne contient pas de mesures représentables.")), t;
			}
		}), o), R(o, () => e.tr("Repères descriptifs, sans note globale ni plage « bon / mauvais ».")), t;
	})(), (() => {
		var n = Na(), r = n.firstChild, i = r.nextSibling, a = i.firstChild, o = a.firstChild, s = i.nextSibling, c = s.firstChild, l = c.firstChild, u = l.nextSibling;
		return R(r, () => e.tr("À regarder ou à écouter")), R(n, k(j, {
			get each() {
				return e.data.gauges.filter((e) => e.label === "Crête vraie");
			},
			children: (n) => (() => {
				var r = Pa(), i = r.firstChild, a = i.firstChild, o = a.firstChild, s = o.nextSibling;
				s.nextSibling;
				var c = a.nextSibling;
				return R(r, k(W, { get name() {
					return n.value > 0 ? "missing" : "wave";
				} }), i), R(a, () => e.tr("Crête vraie"), o), R(a, () => t(n.value), s), R(c, () => e.tr("Estimation des crêtes entre les échantillons ; distincte de l’écrêtage des échantillons.")), x(() => r.classList.toggle("far-warning", n.value > 0)), r;
			})()
		}), i), R(i, k(W, { get name() {
			return e.data.clipped === !0 ? "missing" : "wave";
		} }), a), R(o, () => e.tr(e.data.clipped === !0 ? "Écrêtage détecté." : e.data.clipped === !1 ? "Aucun écrêtage signalé." : "Écrêtage non renseigné.")), R(a, k(M, {
			get when() {
				return e.data.clipEvents !== void 0;
			},
			get children() {
				var t = ja();
				return R(t, () => e.tr("{0} événement(s) · {1} échantillon(s)", e.data.clipEvents, e.data.clippedSamples ?? e.tr("Non renseigné"))), t;
			}
		}), null), R(s, k(W, { get name() {
			return (e.data.eventCount ?? e.data.events.length) > 0 ? "missing" : "wave";
		} }), c), R(l, (() => {
			var t = N(() => e.data.eventCount === void 0);
			return () => t() ? e.tr("Nombre de clics/coupures inconnu.") : e.tr("{0} clic(s)/coupure(s) détecté(s).", e.data.eventCount);
		})()), R(u, () => e.tr("Ces événements peuvent appartenir à l’enregistrement. Vérifiez les passages à l’oreille.")), R(n, k(M, {
			get when() {
				return N(() => !!e.data.duration)() && e.data.events.length;
			},
			get children() {
				return k(Sa, {
					get events() {
						return e.data.events;
					},
					get duration() {
						return e.data.duration ?? 0;
					},
					get locale() {
						return e.locale;
					},
					get tr() {
						return e.tr;
					}
				});
			}
		}), null), R(n, k(M, {
			get when() {
				return e.data.retainedEvents > 0;
			},
			get children() {
				var t = Ma();
				return R(t, () => e.tr("{0} positions conservées · {1} affichées", e.data.retainedEvents, e.data.events.length)), t;
			}
		}), null), x((t) => {
			var n = e.data.clipped === !0, r = (e.data.eventCount ?? e.data.events.length) > 0;
			return n !== t.e && i.classList.toggle("far-warning", t.e = n), r !== t.t && s.classList.toggle("far-warning", t.t = r), t;
		}, {
			e: void 0,
			t: void 0
		}), n;
	})()];
}
//#endregion
//#region ../phemios/src/analysis-report/AnalysisReport.tsx
var Ia = /*#__PURE__*/ P("<p class=far-error role=alert> "), La = /*#__PURE__*/ P("<h2>"), Ra = /*#__PURE__*/ P("<small class=far-muted>"), $ = /*#__PURE__*/ P("<p>"), za = /*#__PURE__*/ P("<p class=far-diagnostic>"), Ba = /*#__PURE__*/ P("<div class=\"far-observation far-error\"role=alert><div><strong></strong><p class=far-diagnostic>"), Va = /*#__PURE__*/ P("<p> "), Ha = /*#__PURE__*/ P("<dt>"), Ua = /*#__PURE__*/ P("<dd><code>"), Wa = /*#__PURE__*/ P("<dl>"), Ga = /*#__PURE__*/ P("<article class=flac-analysis-report><header class=far-header><div><p class=far-kicker>FlacCompagnon</p></div><div class=far-header-actions><button class=far-expansion type=button></button></div></header><section><div class=far-integrity-heading><strong></strong><span class=far-shield></span></div><p></p></section><section class=far-detection><div class=far-detection-heading><h3></h3><div class=far-badges></div></div><p>"), Ka = /*#__PURE__*/ P("<span>"), qa = /*#__PURE__*/ P("<div class=far-check><span></span><strong>");
function Ja(e) {
	let [t, n] = b(e.initialSections ?? (e.initialDetail ? dt : [])), r = () => e.sections ?? t(), i = (e) => r().includes(e), a = (t) => {
		e.sections === void 0 && n(t), e.onSectionsChange?.(t);
	}, o = (e, t) => a(ft(r(), e, t)), s = (...t) => (e.translate ?? ma)(...t), c = S(() => {
		try {
			return { data: fa(e.analysis) };
		} catch (e) {
			return { error: e instanceof Error ? e.message : String(e) };
		}
	}), l = (e) => ha(e);
	return k(M, {
		get when() {
			return c().data;
		},
		get fallback() {
			return (() => {
				var e = Ia(), t = e.firstChild;
				return R(e, () => s("Analyse indisponible :"), t), R(e, () => s(c().error ?? ""), null), e;
			})();
		},
		children: (t) => (() => {
			var n = Ga(), c = n.firstChild, u = c.firstChild;
			u.firstChild;
			var d = u.nextSibling, f = d.firstChild, p = c.nextSibling, m = p.firstChild, h = m.firstChild, g = h.nextSibling, _ = m.nextSibling, v = p.nextSibling, y = v.firstChild, b = y.firstChild, ee = b.nextSibling, S = y.nextSibling;
			return R(u, k(M, {
				get when() {
					return e.title || t().filename;
				},
				get children() {
					var n = La();
					return R(n, () => e.title || t().filename), n;
				}
			}), null), R(u, k(M, {
				get when() {
					return t().stale !== !1;
				},
				get children() {
					var e = Ra();
					return R(e, () => s(t().stale ? "Rapport historique · fichier modifié" : "Rapport conservé · fichier actuel non vérifié")), e;
				}
			}), null), R(d, () => e.actions, f), f.$$click = () => a(r().length ? [] : [...dt]), R(f, k(W, { name: "chevron-down" })), R(h, k(W, { get name() {
				return N(() => l(t()).tone === "bad")() ? "close" : N(() => l(t()).tone === "good")() ? "check" : l(t()).tone === "warning" ? "missing" : "minus";
			} }), null), R(h, () => s(l(t()).label), null), R(g, k(W, { name: "integrity" })), R(_, () => s(l(t()).detail)), R(p, k(G, {
				get label() {
					return s("Voir le détail du contrôle");
				},
				get open() {
					return i("integrity");
				},
				onChange: (e) => o("integrity", e),
				get children() {
					return [
						(() => {
							var e = $();
							return R(e, () => s(l(t()).proof)), e;
						})(),
						k(M, {
							get when() {
								return t().md5Detail;
							},
							get children() {
								var e = za();
								return R(e, () => t().md5Detail), e;
							}
						}),
						(() => {
							var e = $();
							return R(e, () => s("L’intégrité audio ne note ni le mastering ni l’origine de l’enregistrement.")), e;
						})()
					];
				}
			}), null), R(n, k(M, {
				get when() {
					return t().error;
				},
				get children() {
					var e = Ba(), n = e.firstChild, r = n.firstChild, i = r.nextSibling;
					return R(e, k(W, { name: "missing" }), n), R(r, () => s("Analyse incomplète")), R(i, () => t().error), e;
				}
			}), v), R(b, () => s("Détection")), R(ee, k(j, {
				get each() {
					return t().labels;
				},
				children: (e) => (() => {
					var t = Ka();
					return t.classList.toggle("far-warning", e !== "Clean" && e !== "Non analysé"), R(t, k(M, {
						when: e !== "Clean" && e !== "Non analysé",
						get children() {
							return k(W, { name: "missing" });
						}
					}), null), R(t, () => s(e), null), t;
				})()
			})), R(S, () => s(t().labels.includes("Clean") ? "Aucun indice dans les contrôles réalisés. Ce résultat ne certifie pas l’origine de l’enregistrement." : "Les détections sont des indices à examiner, indépendants du résultat MD5.")), R(v, k(G, {
				get label() {
					return s("Comprendre la détection");
				},
				get open() {
					return i("detection");
				},
				onChange: (e) => o("detection", e),
				get children() {
					return [k(j, {
						get each() {
							return t().checks;
						},
						children: (e) => (() => {
							var t = qa(), n = t.firstChild, r = n.nextSibling;
							return R(n, () => s(e.name)), R(r, (() => {
								var t = N(() => e.value === !0);
								return () => t() && k(W, { name: "missing" });
							})(), null), R(r, () => s(e.value === !0 ? "Détecté" : e.value === !1 ? "Non détecté" : "Non renseigné"), null), x(() => r.classList.toggle("far-warning", e.value === !0)), t;
						})()
					}), k(M, {
						get when() {
							return t().detail;
						},
						get children() {
							var e = za();
							return R(e, () => t().detail), e;
						}
					})];
				}
			}), null), R(n, k(G, {
				get label() {
					return s("Le fichier et le signal");
				},
				get open() {
					return i("identity");
				},
				onChange: (e) => o("identity", e),
				section: !0,
				get children() {
					return k(ya, {
						get data() {
							return t();
						},
						tr: s,
						get locale() {
							return e.locale ?? "fr";
						}
					});
				}
			}), null), R(n, k(Fa, {
				get data() {
					return t();
				},
				get expanded() {
					return i("measurements");
				},
				onExpandedChange: (e) => o("measurements", e),
				tr: s,
				get locale() {
					return e.locale ?? "fr";
				}
			}), null), R(n, k(ca, {
				get data() {
					return t();
				},
				tr: s,
				get locale() {
					return e.locale ?? "fr";
				},
				get expanded() {
					return i("technical");
				},
				onExpandedChange: (e) => o("technical", e)
			}), null), R(n, k(M, {
				get when() {
					return t().missing.length;
				},
				get children() {
					return k(G, {
						get label() {
							return s("Mesures non fournies par ce rapport");
						},
						get open() {
							return i("coverage");
						},
						onChange: (e) => o("coverage", e),
						section: !0,
						get children() {
							return [
								(() => {
									var e = $();
									return R(e, () => t().missing.map((e) => s(e)).join(" · ")), e;
								})(),
								k(M, {
									get when() {
										return t().coverage;
									},
									get children() {
										var e = Va(), n = e.firstChild;
										return R(e, () => s("Analyses demandées :"), n), R(e, () => t().coverage?.join(" · "), null), e;
									}
								}),
								(() => {
									var e = $();
									return R(e, () => s("Une valeur absente n’est pas zéro. Aucune mesure n’est déduite des autres pistes.")), e;
								})()
							];
						}
					});
				}
			}), null), R(n, k(M, {
				get when() {
					return t().fileMd5 || t().crc32;
				},
				get children() {
					return k(G, {
						get label() {
							return s("Empreintes du fichier");
						},
						get open() {
							return i("fingerprints");
						},
						onChange: (e) => o("fingerprints", e),
						section: !0,
						get children() {
							return [(() => {
								var e = $();
								return R(e, () => s("Elles identifient les octets, tags et images compris. Sans référence à comparer, elles ne constituent pas un verdict d’intégrité.")), e;
							})(), (() => {
								var e = Wa();
								return R(e, k(M, {
									get when() {
										return t().fileMd5;
									},
									get children() {
										return [(() => {
											var e = Ha();
											return R(e, () => s("MD5 du fichier")), e;
										})(), (() => {
											var e = Ua(), n = e.firstChild;
											return R(n, () => t().fileMd5), e;
										})()];
									}
								}), null), R(e, k(M, {
									get when() {
										return t().crc32;
									},
									get children() {
										return [(() => {
											var e = Ha();
											return R(e, () => s("CRC32 du fichier")), e;
										})(), (() => {
											var e = Ua(), n = e.firstChild;
											return R(n, () => t().crc32), e;
										})()];
									}
								}), null), e;
							})()];
						}
					});
				}
			}), null), x((i) => {
				var a = e.locale ?? "fr", o = s(r().length ? "Tout replier" : "Tout déplier"), c = s(r().length ? "Tout replier" : "Tout déplier"), u = r().length > 0, d = `far-integrity far-${l(t()).tone}`, m = s("Intégrité audio");
				return a !== i.e && I(n, "lang", i.e = a), o !== i.t && I(f, "title", i.t = o), c !== i.a && I(f, "aria-label", i.a = c), u !== i.o && I(f, "aria-expanded", i.o = u), d !== i.i && L(p, i.i = d), m !== i.n && I(p, "aria-label", i.n = m), i;
			}, {
				e: void 0,
				t: void 0,
				a: void 0,
				o: void 0,
				i: void 0,
				n: void 0
			}), n;
		})()
	});
}
F(["click"]);
//#endregion
//#region ../phemios/src/analysis-report/AnalysisDocument.tsx
var Ya = /*#__PURE__*/ P("<nav class=far-files><div class=far-files-heading><p></p><span role=img>"), Xa = /*#__PURE__*/ P("<div class=analysis-document>"), Za = /*#__PURE__*/ P("<button type=button><span>.</span><span></span><span role=img>");
function Qa(e) {
	let [t, n] = b(0);
	ee(() => {
		e.files, n(0);
	});
	let r = () => Math.min(t(), Math.max(0, e.files.length - 1)), i = () => e.translate ?? ma, a = S(() => e.files.map((e, t) => {
		let n = fa(e);
		return {
			name: n.filename || i()("Piste {0}", t + 1),
			...ha(n)
		};
	}));
	return (() => {
		var t = Xa();
		return R(t, k(M, {
			get when() {
				return e.files.length > 1;
			},
			get children() {
				var t = Ya(), o = t.firstChild.firstChild, s = o.nextSibling;
				return R(o, () => i()("{0} pistes dans le rapport", e.files.length)), R(s, k(W, { name: "integrity" })), R(t, k(j, {
					get each() {
						return a();
					},
					children: (e, t) => (() => {
						var a = Za(), o = a.firstChild, s = o.firstChild, c = o.nextSibling, l = c.nextSibling;
						return a.$$click = () => n(t()), R(o, () => t() + 1, s), R(c, () => e.name), R(l, k(W, { get name() {
							return N(() => e.tone === "bad")() ? "close" : N(() => e.tone === "good")() ? "check" : e.tone === "warning" ? "missing" : "minus";
						} })), x((n) => {
							var o = r() === t() ? "true" : void 0, s = `far-${e.tone}`, c = i()(e.label), u = `${i()(e.label)} · ${i()(e.detail)}`;
							return o !== n.e && I(a, "aria-current", n.e = o), s !== n.t && L(l, n.t = s), c !== n.a && I(l, "aria-label", n.a = c), u !== n.o && I(l, "title", n.o = u), n;
						}, {
							e: void 0,
							t: void 0,
							a: void 0,
							o: void 0
						}), a;
					})()
				}), null), x((e) => {
					var n = i()("Pistes du rapport"), r = i()("Intégrité audio"), o = i()("Intégrité audio"), c = !!(a().length > 0 && a().every((e) => e.tone === "good"));
					return n !== e.e && I(t, "aria-label", e.e = n), r !== e.t && I(s, "aria-label", e.t = r), o !== e.a && I(s, "title", e.a = o), c !== e.o && s.classList.toggle("far-good", e.o = c), e;
				}, {
					e: void 0,
					t: void 0,
					a: void 0,
					o: void 0
				}), t;
			}
		}), null), R(t, k(Ja, {
			get analysis() {
				return e.files[r()];
			},
			get locale() {
				return e.locale;
			},
			get translate() {
				return e.translate;
			},
			get initialDetail() {
				return e.initialDetail;
			},
			get initialSections() {
				return e.initialSections;
			},
			get sections() {
				return e.sections;
			},
			get onSectionsChange() {
				return e.onSectionsChange;
			}
		}), null), t;
	})();
}
F(["click"]);
//#endregion
//#region ../phemios/src/analysis-report/index.tsx
function $a(e, t) {
	let [n, r] = b({
		...t,
		files: pa(t.document)
	}), i = Ue(() => k(Qa, {
		get files() {
			return n().files;
		},
		get locale() {
			return n().locale;
		},
		get translate() {
			return n().translate;
		},
		get initialDetail() {
			return n().detailed;
		},
		get initialSections() {
			return n().sections;
		},
		get onSectionsChange() {
			return n().onSectionsChange;
		}
	}), e), a = !1;
	return {
		update(e) {
			if (a) throw Error("Analysis report is disposed.");
			let t = pa(e.document);
			r({
				...e,
				files: t
			});
		},
		dispose() {
			a || (a = !0, i(), r({
				document: "",
				files: []
			}));
		}
	};
}
//#endregion
export { Qa as AnalysisDocument, Ja as AnalysisReport, dt as analysisSectionIds, $a as mountAnalysisReport, pa as parseAnalysisDocument };
