/* The hero's opening, as on the coming-soon page, about six seconds: the shop's name is written in aether fire at
   the centre of the screen, glides up into its place while the words around it fade in, then the crystal appears
   and the gold border draws itself. Any click, tap, key or scroll, or turning the phone, skips to the finished hero.
   It plays only with JavaScript and when the visitor has not asked for reduced motion, and only when the site opens
   at the homepage, at the top (not at another page, or at a link with a #); otherwise the hero shows finished,
   as it does when someone comes back to the homepage from another page.
   The site draws its hero once its WebAssembly has loaded, so this waits for it. The letter outlines are the name's
   own: Cinzel (SemiBold) and Cormorant Garamond Italic, SIL Open Font License 1.1. */
(function () {
  var root = document.documentElement;
  try {
    if ((window.matchMedia && matchMedia('(prefers-reduced-motion: reduce)').matches) || !('animate' in root) ||
        !window.requestAnimationFrame || !window.MutationObserver) return;
  } catch (e) { return; }
  if (document.getElementById('title') || location.hash || !/^\/(index\.html)?$/.test(location.pathname)) return;
  root.classList.add('intro');

  var watch = new MutationObserver(function () { if (document.getElementById('title')) { watch.disconnect(); start(); } });
  watch.observe(document.body, { childList: true, subtree: true });
  setTimeout(function () {   // no hero after 30 seconds: stop waiting, so a late one shows as it is
    watch.disconnect();
    if (!document.getElementById('title')) root.classList.remove('intro');
  }, 30000);

  function start() {
  var hero = document.querySelector('.hero');
  if (!hero || window.scrollY > 2 || location.hash) { root.classList.remove('intro'); return; }
  var G = {"cinzel":{"upm":1000,"g":{"A":["M356 718 644 68Q660 38 682 25Q707 10 730 10H738V0H451V10H460Q479 10 494 26Q509 43 498 73L444 212H197L139 69Q134 51 138 39Q143 25 155 18Q167 10 179 10H188V0H-22V10H-13Q10 10 34 25Q55 37 69 62L302 596Q304 600 310 616Q317 631 326 651Q334 671 340 690Q347 708 347 718Z","M213 250H429L323 520Z"],"e":["M534 170 490 0H41V10H54Q77 10 93 26Q109 41 110 63V537Q109 560 93 575Q77 590 54 590H41V600H372Q401 600 434 606Q467 611 480 615V472H469V483Q469 519 449 540Q429 562 392 563H227V316H360Q390 317 407 336Q424 355 424 383V392H434V204H424V214Q424 241 407 260Q390 280 360 280H227V37H356Q402 37 434 54Q467 70 489 100Q511 130 525 170Z"],"t":["M581 616V473H570V485Q571 520 550 542Q531 562 497 564H355V63Q356 41 372 26Q388 10 412 10H423V0H170V10H181Q204 10 220 26Q235 41 237 61V564H96Q62 562 43 542Q23 520 23 485V473H12V616Q21 614 38 610Q56 606 78 604Q100 601 119 601H473Q493 601 515 604Q537 606 556 610Q574 613 581 616Z"],"h":["M108 63V537Q108 560 92 575Q76 590 53 590H41V600H293V590H282Q259 590 242 575Q226 560 226 537V316H525V537H524Q524 560 508 575Q491 590 469 590H457V600H709V590H697Q674 590 658 575Q642 560 642 537H641V63Q642 41 658 26Q674 10 697 10H709V0H457V10H469Q491 10 508 26Q524 41 524 63H525V280H226V63Q226 41 242 26Q259 10 282 10H293V0H41V10H53Q76 10 92 26Q108 41 108 63Z"],"r":["M112 0H46V10H58Q81 10 97 26Q109 38 112 55V547Q109 564 97 575Q81 590 59 590H46V600H334Q378 600 416 590Q455 581 484 562Q514 543 531 512Q548 481 548 437Q548 399 532 364Q515 329 483 305Q451 281 403 274Q428 265 453 240Q478 215 493 192Q496 186 507 168Q518 151 536 128Q553 104 571 81Q591 55 607 40Q623 24 640 17Q658 10 681 10V0H614Q560 0 523 10Q486 20 461 41Q436 62 414 93Q407 104 396 123Q384 142 372 162Q359 183 350 202Q340 220 337 230Q322 260 302 276Q282 293 263 293V302H273Q283 302 299 302Q322 302 345 308Q368 315 388 335Q407 355 417 397Q420 405 422 418Q423 430 422 444Q421 478 410 500Q399 522 382 536Q365 549 345 555Q325 561 307 561Q278 563 256 562Q237 561 230 562V63Q231 39 248 24Q265 10 285 10H297V0Z"],"l":["M535 170 490 0H41V10H54Q77 10 93 26Q109 41 110 63V537Q109 560 92 575Q76 590 53 590H41V600H295V590H283Q261 590 244 575Q228 560 227 537V37H356Q402 37 434 54Q467 70 489 100Q511 130 525 170Z"],"u":["M270 600V591H256Q237 591 224 578Q210 566 210 548V223Q210 129 254 76Q299 22 379 22Q463 22 510 72Q557 123 558 212V548Q557 566 544 578Q531 590 512 590H498V600H678V591H664Q645 591 632 578Q618 566 617 548V216Q617 110 552 48Q486 -13 367 -13Q233 -13 162 48Q92 109 92 215V548Q91 566 78 578Q65 591 46 591H31V600Z"]}},"amp":{"upm":1000,"g":{"&":["M311 -12Q249 -12 201 12Q153 36 121 78Q89 119 78 171Q66 223 77 278Q89 333 123 375Q157 417 204 441Q251 465 300 465Q325 465 349 458Q368 452 383 440Q402 462 416 484Q439 519 439 551Q439 571 432 582Q426 592 417 598Q408 604 401 612Q394 619 394 635Q394 652 404 662Q415 673 432 673Q457 673 471 652Q485 631 485 597Q485 553 461 510Q437 466 398 424Q359 383 311 344Q263 304 213 267Q176 239 144 213Q145 205 147 197Q160 146 190 110Q219 73 260 53Q300 33 345 33Q406 33 455 64Q504 96 535 158Q559 206 566 272Q555 277 543 284Q513 300 484 312Q456 325 431 325Q415 325 407 318Q399 312 399 304Q399 294 404 286L414 269Q419 260 419 250Q419 241 413 232Q407 224 391 224Q380 224 371 231Q362 238 362 258Q362 285 380 310Q397 335 426 351Q454 367 483 367Q510 367 538 356Q567 344 596 328Q624 313 650 302Q677 290 699 290Q712 290 720 296Q728 301 728 309Q728 318 722 326Q716 333 710 341Q703 349 703 361Q703 373 710 380Q718 386 732 386Q744 386 752 380Q761 373 761 352Q761 327 746 302Q731 276 707 260Q683 243 654 243Q629 243 601 256Q592 259 584 263Q574 184 543 128Q505 58 445 23Q385 -12 311 -12Z","M325 382Q310 414 290 425Q267 437 248 437Q222 437 201 420Q180 402 166 374Q152 345 144 310Q137 269 141 232Q162 249 186 268Q229 301 274 338Q300 359 325 382Z"]}}};
  var NS = 'http://www.w3.org/2000/svg';
  var h1 = document.getElementById('title'), panel = hero.querySelector('.panel'), emblem = hero.querySelector('.emblem');
  var texts = ['.overline', '.divider', '.soon', '.lore', '.hero-cta'].map(function (q) { return hero.querySelector(q); })
    .filter(function (e) { return e; });   // a line the shop doesn't show is skipped
  var corners = [].slice.call(panel.querySelectorAll('.corner'));   // tl, tr, bl, br
  // seconds from the start: writing, the glide up, text, swap to the real title, crystal, border, end
  var T = { write: 0.3, step: 0.17, dur: 0.5, move0: 3.1, move1: 3.95, text: 3.6, swap: 3.95, crystal: 4.35, border: 4.95, bdur: 0.9, cool: 0.35 };
  T.end = T.border + Math.max(T.bdur, 0.12 + T.bdur * 0.9) + T.cool + 0.05;
  var anims = [], overlays = [], raf = 0, done = false, started = false, t0 = 0, last = 0, fired = {};
  var vw = 0, vh = 0, k = 1, TB, at, from, flameG, wrap, flameSvg, mv, glyphs = [], fr = null, shim = h1.getAnimations ? h1.getAnimations()[0] : null;

  function el(tag, attrs, parent) { var e = document.createElementNS(NS, tag); for (var a in attrs) e.setAttribute(a, attrs[a]); if (parent) parent.appendChild(e); return e; }
  function clamp(v) { return v < 0 ? 0 : v > 1 ? 1 : v; }
  function ease(p) { return p < 0.5 ? 4 * p * p * p : 1 - Math.pow(-2 * p + 2, 3) / 2; }
  function play(e, frames, opts) { var a = e.animate(frames, opts); anims.push(a); return a; }

  // Sparks and the glow of the pen tip, on a canvas over everything while the intro runs
  var cv = document.createElement('canvas'), cx = cv.getContext('2d'), sparks = [], dpr = Math.min(window.devicePixelRatio || 1, 2), dots = {}, inked = false;
  cv.className = 'intro-sparks'; cv.setAttribute('aria-hidden', 'true');
  function dot(c) {   // one soft round spark per colour, drawn once and reused (no new gradient per spark per frame)
    if (!dots[c]) {
      var d = document.createElement('canvas'); d.width = d.height = 64;
      var x = d.getContext('2d'), g = x.createRadialGradient(32, 32, 0, 32, 32, 32);
      g.addColorStop(0, 'rgba(' + c + ',1)'); g.addColorStop(1, 'rgba(' + c + ',0)'); x.fillStyle = g; x.fillRect(0, 0, 64, 64); dots[c] = d;
    }
    return dots[c];
  }
  function spark(x, y, n, spread, speed, gold) {
    for (var i = 0; i < n && sparks.length < 500; i++) {
      var a = Math.random() * 6.283, v = speed * (0.3 + Math.random() * 0.7);
      sparks.push({ x: x + (Math.random() - 0.5) * spread, y: y + (Math.random() - 0.5) * spread, vx: Math.cos(a) * v, vy: Math.sin(a) * v - speed * 0.35,
        life: 0, max: 0.35 + Math.random() * 0.6, r: 0.6 + Math.random() * 1.5, c: Math.random() < gold ? '255,214,140' : '205,238,255' });
    }
  }
  function drawSparks(dt) {
    if (inked) { cx.clearRect(0, 0, vw, vh); inked = false; }   // an empty canvas is left alone
    if (!sparks.length) return;
    inked = true; cx.globalCompositeOperation = 'lighter';
    for (var i = sparks.length - 1; i >= 0; i--) {
      var s = sparks[i]; s.life += dt;
      if (s.life >= s.max) { sparks.splice(i, 1); continue; }
      s.vx *= 0.96; s.vy = s.vy * 0.96 - 22 * dt; s.x += s.vx * dt; s.y += s.vy * dt;   // embers drift upwards as they fade
      cx.globalAlpha = 1 - s.life / s.max; cx.drawImage(dot(s.c), s.x - s.r * 4, s.y - s.r * 4, s.r * 8, s.r * 8);
    }
    cx.globalAlpha = 1;
  }
  function glow(x, y, r, rgb) {
    var g = cx.createRadialGradient(x, y, 0, x, y, r); inked = true;
    g.addColorStop(0, 'rgba(255,255,255,.95)'); g.addColorStop(0.25, 'rgba(' + rgb + ',.75)'); g.addColorStop(1, 'rgba(' + rgb + ',0)');
    cx.fillStyle = g; cx.beginPath(); cx.arc(x, y, r, 0, 6.283); cx.fill();
  }

  // Where each letter of the real title sits: its pen position, baseline and size, read from the laid-out page
  function measure() {
    var out = [], rg = document.createRange();
    [].forEach.call(h1.querySelectorAll('.w, .amp'), function (sp) {
      var font = sp.classList.contains('amp') ? G.amp : G.cinzel, s = parseFloat(getComputedStyle(sp).fontSize) / font.upm;
      var mk = document.createElement('span');
      mk.style.cssText = 'display:inline-block;width:0;height:0;margin:0;padding:0;border:0;vertical-align:baseline';
      sp.appendChild(mk); var base = mk.getBoundingClientRect().bottom; sp.removeChild(mk);
      var tn = sp.firstChild;
      for (var i = 0; i < tn.length; i++) {
        var d = font.g[tn.data.charAt(i)]; if (!d) continue;
        rg.setStart(tn, i); rg.setEnd(tn, i + 1);
        out.push({ x: rg.getBoundingClientRect().left, y: base, s: s, d: d, r: rg.getBoundingClientRect() });
      }
    });
    return out;
  }

  function build() {
    vw = window.innerWidth; vh = window.innerHeight;
    glyphs = measure();
    if (glyphs.length !== 13) throw new Error('title not measured');
    var x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9;
    glyphs.forEach(function (g) { x0 = Math.min(x0, g.r.left); y0 = Math.min(y0, g.r.top); x1 = Math.max(x1, g.r.right); y1 = Math.max(y1, g.r.bottom); });
    TB = { x: x0, y: y0, w: x1 - x0, h: y1 - y0, cx: (x0 + x1) / 2, cy: (y0 + y1) / 2 };
    k = Math.max(1, Math.min(1.3, vw * 0.9 / TB.w, vh * 0.5 / TB.h));
    // The title is drawn in a box over the real one: the gold letters, and the flame on a layer of its own above them. Until the
    // glide the box waits centred on the screen, a little below its place so that it visibly rises, and enlarged. The glide is a
    // transform animation, which the browser runs apart from the page's scripts, so it stays smooth. Where the title stands
    // still, before and after, the box's corner is on a whole device pixel, so that the letters are drawn sharp.
    var pad = 40, dp = window.devicePixelRatio || 1, bx = Math.floor((TB.x - pad) * dp) / dp, by = Math.floor((TB.y - pad) * dp) / dp;
    var bw = Math.ceil((TB.x + TB.w + pad - bx) * dp) / dp, bh = Math.ceil((TB.y + TB.h + pad - by) * dp) / dp, vb = bx + ' ' + by + ' ' + bw + ' ' + bh, box = 'position:absolute;left:0;top:0;overflow:visible';
    var y0 = Math.min(vh * 0.7, Math.max(vh * 0.56, TB.cy + vh * 0.08)), ox = TB.cx - bx, oy = TB.cy - by;   // the box turns about the title's centre
    at = [Math.round((vw / 2 - k * ox) * dp) / dp - bx - ox * (1 - k), Math.round((y0 - k * oy) * dp) / dp - by - oy * (1 - k)];   // its shift before the glide
    from = 'translate(' + at[0] + 'px,' + at[1] + 'px) scale(' + k + ')';
    wrap = document.createElement('div'); wrap.className = 'intro-title'; wrap.setAttribute('aria-hidden', 'true');
    wrap.style.cssText = 'left:' + bx + 'px;top:' + by + 'px;width:' + bw + 'px;height:' + bh + 'px;transform-origin:' + ox + 'px ' + oy + 'px;transform:' + from;
    var gold = el('svg', { width: bw, height: bh, viewBox: vb, style: box }), defs = el('defs', {}, gold);
    flameSvg = el('svg', { width: bw, height: bh, viewBox: vb, style: box }); wrap.appendChild(gold); wrap.appendChild(flameSvg);
    // the gold is the real title's own gradient, as the page draws it with its shimmer at the start, so the hand-over is seamless
    var hr = h1.getBoundingClientRect(), gw = hr.width * 2.4, ux = Math.sin(100 * Math.PI / 180), uy = -Math.cos(100 * Math.PI / 180), gl = (gw * ux + hr.height * Math.abs(uy)) / 2, gx = hr.left + gw / 2, gy = hr.top + hr.height / 2;
    var gr = el('linearGradient', { id: 'ia-gold', gradientUnits: 'userSpaceOnUse', x1: gx - ux * gl, y1: gy - uy * gl, x2: gx + ux * gl, y2: gy + uy * gl }, defs);
    [['0', '#9c7228'], ['.2', '#f8e9bb'], ['.38', '#d9b45a'], ['.5', '#fff8de'], ['.62', '#d9b45a'], ['.8', '#f8e9bb'], ['1', '#9c7228']]
      .forEach(function (st) { el('stop', { offset: st[0], 'stop-color': st[1] }, gr); });
    var f = el('filter', { id: 'ia-glow', x: '-20%', y: '-40%', width: '140%', height: '180%', 'color-interpolation-filters': 'sRGB' }, el('defs', {}, flameSvg));
    el('feGaussianBlur', { 'in': 'SourceAlpha', stdDeviation: 3, result: 'a' }, f);
    el('feFlood', { 'flood-color': '#8fdcff', 'flood-opacity': '.95', result: 'c1' }, f);
    el('feComposite', { 'in': 'c1', in2: 'a', operator: 'in', result: 'g1' }, f);
    el('feGaussianBlur', { 'in': 'SourceAlpha', stdDeviation: 9, result: 'b' }, f);
    el('feFlood', { 'flood-color': '#2a7fd4', 'flood-opacity': '.9', result: 'c2' }, f);
    el('feComposite', { 'in': 'c2', in2: 'b', operator: 'in', result: 'g2' }, f);
    var fm = el('feMerge', {}, f); ['g2', 'g1', 'SourceGraphic'].forEach(function (n) { el('feMergeNode', { 'in': n }, fm); });
    var mask = el('mask', { id: 'ia-mask', maskUnits: 'userSpaceOnUse', x: TB.x - 20, y: TB.y - 20, width: TB.w + 40, height: TB.h + 40 }, defs);
    el('rect', { x: TB.x - 10, y: TB.y - 10, width: TB.w + 20, height: TB.h + 20, fill: 'url(#ia-gold)', mask: 'url(#ia-mask)' }, gold);
    flameG = el('g', { filter: 'url(#ia-glow)' }, flameSvg);
    document.body.appendChild(wrap); overlays.push(wrap);
    glyphs.forEach(function (g, i) {
      var tr = 'translate(' + g.x + ' ' + g.y + ') scale(' + g.s + ' ' + (-g.s) + ')';
      g.fill = el('path', { d: g.d.join(''), transform: tr, fill: '#fff', 'fill-rule': 'evenodd', 'fill-opacity': 0 }, mask);
      g.fg = el('g', { transform: tr }, flameG);
      g.cs = g.d.map(function (d, j) {
        var p = el('path', { d: d, fill: 'none', stroke: '#f2fbff', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' }, g.fg);
        var L = p.getTotalLength();
        p.setAttribute('stroke-dasharray', L + ' ' + (L + 1)); p.setAttribute('stroke-dashoffset', L);
        return { p: p, L: L, j: j };
      });
      g.tip = g.cs.reduce(function (a, b) { return b.L > a.L ? b : a; });   // the pen follows the longest stroke
      g.t = T.write + i * T.step; g.dur = T.dur * (g.cs.length > 1 ? 1.1 : 1);
    });
    cv.width = Math.round(vw * dpr); cv.height = Math.round(vh * dpr); cv.style.width = vw + 'px'; cv.style.height = vh + 'px';
    cx.setTransform(dpr, 0, 0, dpr, 0, 0); document.body.appendChild(cv); overlays.push(cv);
  }

  // The glide carries the title exactly onto the real one, and the flame dies down as it rises.
  function glide() {
    mv = play(wrap, [{ transform: from }, { transform: 'none' }], { duration: (T.move1 - T.move0) * 1000, easing: 'cubic-bezier(.65,0,.35,1)', fill: 'forwards' });
    play(flameSvg, [{ opacity: 1 }, { opacity: 0 }], { duration: 500, fill: 'forwards' });
  }
  function toScreen(x, y) { return [TB.cx + at[0] + (x - TB.cx) * k, TB.cy + at[1] + (y - TB.cy) * k]; }   // page point -> screen, before the glide

  // Letter g at time t: its outline drawn by the pen, its gold filling in, its flame cooling. end: the glide begins, the gold is complete.
  function write(g, t, end) {
    var p = clamp((t - g.t) / g.dur), cool = clamp((t - g.t - g.dur) / 0.6);
    g.cs.forEach(function (c) { c.p.setAttribute('stroke-dashoffset', c.L * (1 - ease(clamp(p * 1.15 - c.j * 0.15)))); });
    g.fill.setAttribute('fill-opacity', end ? 1 : clamp((t - g.t - g.dur * 0.5) / 0.45));
    g.fg.setAttribute('opacity', p < 1 ? 1 : 0.35 + 0.65 * (1 - cool));   // the letter being written burns hotter and wider, then cools
    g.fg.setAttribute('stroke-width', (1.4 + 1.1 * (1 - cool)) / g.s);
    if (cool >= 1 && t - g.t >= g.dur * 0.5 + 0.45) g.done = true;   // nothing left to change
    if (p > 0 && p < 1) {
      var q = ease(clamp(p * 1.15 - g.tip.j * 0.15)), pt = g.tip.p.getPointAtLength(g.tip.L * q), xy = toScreen(g.x + pt.x * g.s, g.y - pt.y * g.s);
      glow(xy[0], xy[1], 12 * k, '160,225,255'); spark(xy[0], xy[1], 2, 2, 60, 0.45);
    }
  }

  function fadeIn(e, delay) {
    e.classList.add('in');
    var o = getComputedStyle(e).opacity;
    play(e, [{ opacity: 0, translate: '0 12px' }, { opacity: o, translate: '0 0' }], { duration: 750, delay: delay, easing: 'cubic-bezier(.2,.7,.3,1)', fill: 'backwards' });
  }

  function crystalIn() {
    var ds = ' drop-shadow(0 0 10px rgba(127,216,255,.75)) drop-shadow(0 0 32px rgba(42,127,212,.55))', rings = emblem.querySelectorAll('.ring');
    emblem.classList.add('in');
    play(emblem, [{ opacity: 0 }, { opacity: 1 }], { duration: 350, easing: 'ease-out', fill: 'backwards' });
    play(emblem.querySelector('.disc'), [{ opacity: 0, scale: '.5' }, { opacity: 1, scale: '1.04', offset: 0.7 }, { opacity: 1, scale: '1' }],
      { duration: 700, easing: 'cubic-bezier(.2,.7,.3,1)', fill: 'backwards' });
    play(rings[0], [{ opacity: 0, scale: '.7', rotate: '-50deg' }, { opacity: 1, scale: '1', rotate: '0deg' }], { duration: 900, easing: 'cubic-bezier(.2,.8,.2,1)', fill: 'backwards' });
    play(rings[1], [{ opacity: 0, scale: '1.25', rotate: '40deg' }, { opacity: 1, scale: '1', rotate: '0deg' }], { duration: 900, delay: 80, easing: 'cubic-bezier(.2,.8,.2,1)', fill: 'backwards' });
    play(emblem.querySelector('.crystal'), [{ opacity: 0, scale: '.15', filter: 'brightness(3.2)' + ds }, { opacity: 1, scale: '1.12', filter: 'brightness(1.8)' + ds, offset: 0.55 },
      { opacity: 1, scale: '1', filter: 'brightness(1)' + ds }], { duration: 800, delay: 120, easing: 'cubic-bezier(.2,.7,.3,1)', fill: 'backwards' });
    var fl = document.createElement('div'); fl.className = 'intro-flash'; emblem.appendChild(fl); overlays.push(fl);
    play(fl, [{ opacity: 0, scale: '.2' }, { opacity: 0.95, scale: '.9', offset: 0.3 }, { opacity: 0, scale: '1.7' }], { duration: 900, easing: 'ease-out', fill: 'forwards' });
    var r = emblem.getBoundingClientRect(); spark(r.left + r.width / 2, r.top + r.height / 2, 70, r.width * 0.2, 260, 0.35);
  }

  // The border: both gold lines drawn in two halves from under the crystal round to the bottom centre; the outer
  // halo ring and the panel's shadows then glow in (CSS transition on .framed)
  function halves(W, H, o, r) {
    var x0 = o, y0 = o, x1 = W - o, y1 = H - o, c = W / 2, a = ' A' + r + ' ' + r + ' 0 0 ';
    return ['M' + c + ' ' + y0 + ' H' + (x0 + r) + a + '0 ' + x0 + ' ' + (y0 + r) + ' V' + (y1 - r) + a + '0 ' + (x0 + r) + ' ' + y1 + ' H' + c,
            'M' + c + ' ' + y0 + ' H' + (x1 - r) + a + '1 ' + x1 + ' ' + (y0 + r) + ' V' + (y1 - r) + a + '1 ' + (x1 - r) + ' ' + y1 + ' H' + c];
  }
  function frameStart() {
    var r = panel.getBoundingClientRect(), W = r.width, H = r.height, ins = parseFloat(getComputedStyle(panel, '::before').top) || 10;
    fr = { W: W, H: H, svg: el('svg', { 'class': 'intro-frame', 'aria-hidden': 'true', width: W, height: H, viewBox: '0 0 ' + W + ' ' + H }), lines: [] };
    panel.insertBefore(fr.svg, panel.firstChild); overlays.push(fr.svg); fr.pr = fr.svg.getBoundingClientRect();
    // [offset of the line's centre from the panel's edge, corner radius, final colour (as the CSS draws it), delay, share of the time]
    [[0.5, 3.5, [217, 180, 90, 0.42], 0, 1], [1.5 + ins, 1.5, [217, 180, 90, 0.13], 0.12, 0.9]].forEach(function (L, n) {
      halves(W, H, L[0], L[1]).forEach(function (d) {
        var p = el('path', { d: d, fill: 'none', stroke: 'rgba(251,238,203,.95)', 'stroke-width': 1 }, fr.svg), len = p.getTotalLength();
        p.setAttribute('stroke-dasharray', len + ' ' + (len + 1)); p.setAttribute('stroke-dashoffset', len);
        fr.lines.push({ p: p, L: len, col: L[2], delay: L[3], dur: T.bdur * L[4], main: n === 0 });
      });
    });
    fr.corners = [{ els: [corners[0], corners[1]], at: (W / 2) / (W + H) }, { els: [corners[2], corners[3]], at: (W / 2 + H) / (W + H) }];
  }
  function pop(c) {
    c.classList.add('in');
    play(c, [{ opacity: 0, scale: '.3' }, { opacity: 1, scale: '1.3', offset: 0.6 }, { opacity: 1, scale: '1' }], { duration: 450, easing: 'ease-out', fill: 'backwards' });
    var r = c.getBoundingClientRect(); spark(r.left + r.width / 2, r.top + r.height / 2, 14, 6, 120, 0.8);
  }
  function frameStep(s) {
    var pr = fr.pr, settled = true, mainP = 0;
    fr.lines.forEach(function (l) {
      var p = clamp((s - l.delay) / l.dur), cool = clamp((s - l.delay - l.dur) / T.cool), c = l.col, h = [251, 238, 203, 0.95];
      if (cool < 1) settled = false;
      l.p.setAttribute('stroke-dashoffset', l.L * (1 - p));
      l.p.setAttribute('stroke', 'rgba(' + [0, 1, 2].map(function (i) { return Math.round(h[i] + (c[i] - h[i]) * cool); }).join(',') + ',' + (h[3] + (c[3] - h[3]) * cool) + ')');
      if (l.main) {
        mainP = p;
        if (p > 0 && p < 1) { var pt = l.p.getPointAtLength(l.L * p); glow(pr.left + pt.x, pr.top + pt.y, 10, '255,220,150'); spark(pr.left + pt.x, pr.top + pt.y, 2, 2, 50, 0.85); }
      }
    });
    fr.corners.forEach(function (c) { if (!c.done && mainP >= c.at) { c.done = true; c.els.forEach(pop); } });
    if (mainP >= 0.5) panel.classList.add('glassed');
    if (settled && !fr.done) { fr.done = true; panel.classList.add('framed'); fr.svg.remove(); }   // the CSS border takes over, identical
  }

  function frame(now) {
    if (done) return;
    if (!t0) t0 = now;
    var t = (now - t0) / 1000, dt = last ? Math.min((now - last) / 1000, 0.05) : 0; last = now;
    drawSparks(dt); cx.globalCompositeOperation = 'lighter';
    if (!fired.ink) {   // the writing; once the glide begins nothing in the title changes, so only the browser moves it
      if (t >= T.move0) { fired.ink = 1; glide(); }
      else flameG.setAttribute('opacity', 0.88 + 0.12 * Math.sin(t * 31) * Math.sin(t * 17.3));
      glyphs.forEach(function (g) { if (t >= g.t && !g.done) write(g, t, fired.ink); });
    } else if (flameSvg.parentNode && t >= T.move0 + 0.55) flameSvg.remove();   // the flame has died down: nothing left to draw
    if (fr && !fr.done) frameStep(t - T.border);
    cx.globalCompositeOperation = 'source-over';
    if (t >= T.text && !fired.text) { fired.text = 1; texts.forEach(function (e, i) { fadeIn(e, i * 110); }); }
    if (mv && !fired.swap && mv.currentTime >= (T.move1 - T.move0) * 1000 - 1) {   // the glide is over: the real title, identical, takes over in one frame
      fired.swap = 1; h1.classList.add('in'); if (shim) shim.play(); wrap.remove();
    }
    if (t >= T.crystal && !fired.crystal) { fired.crystal = 1; crystalIn(); }
    if (t >= T.border && !fired.border) { fired.border = 1; frameStart(); }
    if (t >= T.end && (!fr || fr.done)) return finish(false);
    raf = requestAnimationFrame(frame);
  }

  // The finished page: everything in its final state, the intro's own pieces removed.
  var EV = ['pointerdown', 'keydown', 'wheel', 'touchstart', 'scroll'];   // scroll: the shop's page scrolls, e.g. by its scrollbar
  function skip() { finish(true); }
  function onHide() { if (document.hidden) finish(true); }
  function onResize() { if (Math.abs(window.innerWidth - vw) > 1) finish(true); }   // a turned phone, not the address bar settling
  function finish(skipped) {
    if (done) return; done = true;
    cancelAnimationFrame(raf);
    anims.forEach(function (a) { try { a.cancel(); } catch (e) {} });
    overlays.forEach(function (o) { if (o.parentNode) o.parentNode.removeChild(o); });
    [h1, emblem].concat(texts, corners).forEach(function (e) { e.classList.add('in'); });
    panel.classList.add('glassed', 'framed'); if (shim) shim.play();
    EV.forEach(function (ev) { window.removeEventListener(ev, skip, true); });
    window.removeEventListener('resize', onResize); document.removeEventListener('visibilitychange', onHide);
    if (skipped) root.classList.remove('intro');
    else setTimeout(function () { root.classList.remove('intro'); }, 1000);   // after the shadows' last fade
  }
  window.aaIntroFinish = finish;

  function begin() {
    if (started || done) return; started = true;   // not after the hero has already been shown finished
    try { build(); } catch (e) { return finish(true); }
    EV.forEach(function (ev) { window.addEventListener(ev, skip, { capture: true, passive: true }); });
    window.addEventListener('resize', onResize); document.addEventListener('visibilitychange', onHide);
    if (shim) { shim.pause(); shim.currentTime = 0; }   // the title's shimmer waits at its start until the hand-over
    raf = requestAnimationFrame(frame);
    setTimeout(function () { finish(true); }, 20000);   // never leave the page unfinished
  }
  var fonts = document.fonts && document.fonts.load ? Promise.all([document.fonts.load('600 48px Cinzel'), document.fonts.load('italic 500 48px "Cormorant Garamond"')]) : Promise.resolve();
  var fontsIn = false;
  setTimeout(function () { if (!fontsIn) finish(true); }, 4000);   // fonts slow to arrive: show the hero as it is
  fonts.then(function () {
    fontsIn = true;
    if (!document.hidden) return begin();
    document.addEventListener('visibilitychange', function wait() { if (!document.hidden) { document.removeEventListener('visibilitychange', wait); begin(); } });
  }, function () { finish(true); });
  }
})();
