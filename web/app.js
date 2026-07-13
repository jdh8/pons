// Thin static UI over the pons wasm bidder: the engine holds the deal and the
// auction; JS rebuilds the DOM from each JSON snapshot (gin-rummy pattern).
import init, { WebTable, book, set_option, set_choice, describe_options } from './pkg/pons_web.js';

const SEATS = ['N', 'E', 'S', 'W'];
const SEAT_NAMES = { N: 'North', E: 'East', S: 'South', W: 'West' };
const SUIT_CLASS = { '♠': 's-s', '♥': 's-h', '♦': 's-d', '♣': 's-c' };
const SUIT_KEYS = { '♠': 'spades', '♥': 'hearts', '♦': 'diamonds', '♣': 'clubs' };
const HAND_ORDER = ['♠', '♥', '♦', '♣']; // spades first in hand panels
const BOX_ORDER = ['♣', '♦', '♥', '♠', 'NT']; // bidding-box columns, low to high
const DEMO_PACE_MS = 300; // pause between demo auction reveals

const ORACLE_TOTAL = 100; // reshuffles per board
const ORACLE_CHUNK = 2; // per JS task, so the page keeps painting between them

let game;
let current = null; // the snapshot on screen
let boardCount = 0; // practice deals so far — drives the "Rotate" dealer
let bookNodes = null; // [{el, haystack}] built once from book()
let demoTimer = 0;
let boardGen = 0; // bumped per deal; stale async DD/oracle loops check it
let analysisGen = -1; // last boardGen whose DD + oracle were kicked off

const id = (x) => document.getElementById(x);

async function main() {
  await init();
  game = new WebTable(String(Math.floor(Math.random() * 2 ** 53)));
  OPTIONS = JSON.parse(describe_options()); // the Settings registry, from wasm
  // Replay saved overrides: booleans are toggles, strings are radio-choice values.
  for (const [key, value] of Object.entries(stored)) applyOption(key, value);
  buildBiddingBox();
  for (const b of document.querySelectorAll('nav button')) {
    b.onclick = () => { location.hash = b.dataset.tab; };
  }
  window.addEventListener('hashchange', () => showTab(location.hash.slice(1)));
  id('p-deal').onclick = dealPractice;
  id('d-deal').onclick = dealDemo;
  id('d-edit').onclick = editDemo;
  id('b-filter').oninput = filterBook;
  initEdit();
  showTab(location.hash.slice(1));
}

function showTab(tab) {
  if (!['practice', 'demo', 'book', 'edit', 'settings'].includes(tab)) tab = 'practice';
  for (const sec of document.querySelectorAll('main > section')) {
    sec.classList.toggle('hidden', sec.id !== tab);
  }
  for (const b of document.querySelectorAll('nav button')) {
    b.classList.toggle('active', b.dataset.tab === tab);
  }
  if (tab === 'book' && !bookNodes) loadBook();
  if (tab === 'settings' && !settingsBuilt) renderSettings();
}

// --- dealing -----------------------------------------------------------------

function dealPractice() {
  boardGen++;
  const pick = id('p-dealer').value;
  const dealer = pick === 'rotate' ? SEATS[boardCount % 4] : pick;
  boardCount++;
  const hcp = Math.min(37, Math.max(0, Number(id('p-hcp').value) || 0));
  render(JSON.parse(game.deal_practice(id('p-seat').value, dealer, id('p-vul').value, hcp)));
}

function dealDemo() {
  runDemo(game.deal_demo(id('d-dealer').value, id('d-vul').value));
}

// Hand the deal now on screen to the Edit tab so it can be tweaked and re-bid.
function editDemo() {
  if (!current || current.mode === 'practice') return;
  editAssign = assignFromHands(current.hands);
  syncFromBoard(); // repaint palette/compass/PBN from the demoed deal
  location.hash = 'edit';
}

// Animate a demo snapshot: hands at once, then the auction one call at a time.
// Shared by the random Deal button and the editor's "Bid it out" hand-off.
function runDemo(snapshotJSON) {
  boardGen++;
  clearInterval(demoTimer);
  id('d-dd').classList.add('hidden');
  const s = JSON.parse(snapshotJSON);
  if (!s) return; // deal_pbn rejected a non-full deal — nothing to animate
  let shown = 0;
  const tick = () => {
    const done = shown >= s.auction.length;
    render({ ...s, auction: s.auction.slice(0, shown), contract: done ? s.contract : null });
    if (done) {
      clearInterval(demoTimer);
      scheduleDD('d-dd');
    }
    shown++;
  };
  tick();
  demoTimer = setInterval(tick, DEMO_PACE_MS);
}

// --- rendering ---------------------------------------------------------------

function render(s) {
  current = s;
  if (s.mode === 'practice') renderPractice(s);
  else renderDemo(s);
}

function renderPractice(s) {
  id('p-info').textContent = `Dealer ${SEAT_NAMES[s.dealer]} · Vul ${s.vul}`;
  const hand = s.hands[s.seat];
  id('p-hand').innerHTML = hand
    ? `<div class="seat-head">${SEAT_NAMES[s.seat]} · ${hand.hcp} HCP</div>${handHTML(hand)}`
    : '';
  id('p-auction').innerHTML = auctionHTML(s, s.seat);
  updateBiddingBox(s);
  renderFeedback(s);
  renderReveal(s);
}

function renderDemo(s) {
  id('d-edit').disabled = false;
  id('d-info').textContent = `Dealer ${SEAT_NAMES[s.dealer]} · Vul ${s.vul}`;
  id('d-hands').innerHTML = compassHTML(s.hands);
  const auc = id('d-auction');
  auc.classList.remove('hidden');
  auc.innerHTML = auctionHTML(s, null);
}

function renderFeedback(s) {
  const box = id('p-feedback');
  const fb = s.feedback || [];
  box.classList.toggle('hidden', fb.length === 0);
  box.innerHTML = fb.map((f) => {
    const mark = f.agreed ? '<span class="ok">✓</span>' : '<span class="no">✗</span>';
    const bot = f.top.length
      ? 'bot: ' + f.top.map(([c, p]) => `${colorizeCalls(c)} ${Math.round(p)}%`).join(' · ')
      : 'book has no opinion (bot would pass)';
    return `<div class="fb-row">${mark} you: ${colorizeCalls(f.human)} · ${bot}</div>`;
  }).join('');
}

function renderReveal(s) {
  const box = id('p-reveal');
  box.classList.toggle('hidden', !s.ended);
  if (!s.ended) {
    id('p-dd').classList.add('hidden');
    id('p-oracle').classList.add('hidden');
    return;
  }
  if (analysisGen !== boardGen) {
    analysisGen = boardGen;
    runOracle(s);
    scheduleDD('p-dd');
  }
  box.innerHTML =
    `<div class="contract-line"><span class="contract">${colorizeCalls(s.contract || '')}</span></div>` +
    compassHTML(s.hands);
  const next = document.createElement('button');
  next.className = 'primary next';
  next.textContent = 'Next board';
  next.onclick = dealPractice; // same settings; Rotate advances the dealer
  box.appendChild(next);
}

// --- double dummy + oracle -----------------------------------------------------

// Solve after a paint so the "solving…" placeholder actually shows; the wasm
// solve blocks the main thread for a few hundred ms.
function scheduleDD(targetId) {
  const gen = boardGen;
  const box = id(targetId);
  box.classList.remove('hidden');
  box.innerHTML = '<div class="panel-title">Double dummy</div><div class="solving">solving…</div>';
  setTimeout(() => {
    if (gen !== boardGen) return;
    const dd = JSON.parse(game.dd_table());
    if (dd && gen === boardGen) box.innerHTML = ddHTML(dd);
  }, 50);
}

function ddHTML(dd) {
  const head = '<tr><th></th>' +
    dd.seats.map((x) => `<th>${SEAT_NAMES[x]}</th>`).join('') + '</tr>';
  const rows = dd.rows.map((r) =>
    `<tr><th>${colorizeCalls(r.strain)}</th>` +
    r.tricks.map((t) => `<td>${t}</td>`).join('') + '</tr>',
  ).join('');
  return '<div class="panel-title">Double dummy</div>' +
    `<table class="dd">${head}${rows}</table>` +
    (dd.verdict ? `<div class="verdict">${dd.verdict.map(colorizeCalls).join('<br>')}</div>` : '');
}

// The fairness judge: the reached contract priced over reshuffles of the two
// hands the bidding side never saw.  Chunked so the page paints progress.
function runOracle() {
  const gen = boardGen;
  const box = id('p-oracle');
  box.classList.remove('hidden');
  box.innerHTML = '<div class="panel-title">Oracle (opponents reshuffled)</div>' +
    '<div class="o-body">shuffling…</div>';
  const step = () => {
    if (gen !== boardGen) return;
    const o = JSON.parse(game.oracle(ORACLE_CHUNK));
    if (!o) { box.classList.add('hidden'); return; } // passed out — nothing to judge
    const sign = o.mean_score >= 0 ? '+' : '';
    box.querySelector('.o-body').textContent =
      `${o.n}/${ORACLE_TOTAL} shuffles: makes ${Math.round(o.makes_pct)}% · ` +
      `tricks ${o.tricks_min}/${o.mean_tricks.toFixed(1)}/${o.tricks_max} · ` +
      `mean score ${sign}${Math.round(o.mean_score)}`;
    if (o.n < ORACLE_TOTAL) setTimeout(step, 0);
  };
  setTimeout(step, 50);
}

// --- HTML builders -----------------------------------------------------------

// Four suit lines, spades first; a void renders as an em dash.
function handHTML(hand) {
  return HAND_ORDER.map((g) =>
    `<div class="suitline"><span class="${SUIT_CLASS[g]}">${g}</span>` +
    `<span class="ranks">${escapeHTML(hand[SUIT_KEYS[g]]) || '—'}</span></div>`,
  ).join('');
}

// All visible hands in compass layout: N top, W left, E right, S bottom.
function compassHTML(hands) {
  const cell = (seat) => {
    const h = hands[seat];
    return `<div class="compass-seat pos-${seat.toLowerCase()}">` +
      (h ? `<div class="seat-head">${SEAT_NAMES[seat]} · ${h.hcp} HCP</div>${handHTML(h)}` : '') +
      '</div>';
  };
  return `<div class="compass">${SEATS.map(cell).join('')}</div>`;
}

// The classic auction table: fixed W/N/E/S columns (W first reads easier),
// leading blanks up to the dealer, one cell per call, wrapping every four.
const AUCTION_COLS = ['W', 'N', 'E', 'S'];

function auctionHTML(s, humanSeat) {
  const cells = Array(AUCTION_COLS.indexOf(s.dealer)).fill(null);
  cells.push(...s.auction);
  while (cells.length % 4) cells.push(null);
  const head = AUCTION_COLS.map((x) =>
    `<th${x === humanSeat ? ' class="you"' : ''}>${SEAT_NAMES[x]}</th>`,
  ).join('');
  let body = '';
  for (let i = 0; i < cells.length; i += 4) {
    body += '<tr>' + cells.slice(i, i + 4).map(callCellHTML).join('') + '</tr>';
  }
  return `<table class="auction"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
}

function callCellHTML(call) {
  if (call == null) return '<td></td>';
  const cls = call === 'P' ? ' class="pass"' : call === 'X' || call === 'XX' ? ' class="dbl"' : '';
  return `<td${cls}>${colorizeCalls(call)}</td>`;
}

// Wrap suit glyphs in per-suit colour spans; safe on already plain text.
function colorizeCalls(text) {
  return escapeHTML(text).replace(/[♠♥♦♣]/g, (g) => `<span class="${SUIT_CLASS[g]}">${g}</span>`);
}

function escapeHTML(str) {
  const d = document.createElement('div');
  d.textContent = str;
  return d.innerHTML;
}

// --- bidding box ---------------------------------------------------------------

// Built once: 7×5 grid of contract bids (levels down, ♣ ♦ ♥ ♠ NT across),
// then a wide P / X / XX row.  Snapshots only flip the disabled flags.
function buildBiddingBox() {
  const box = id('p-bidbox');
  const grid = document.createElement('div');
  grid.className = 'bid-grid';
  for (let level = 1; level <= 7; level++) {
    for (const d of BOX_ORDER) grid.appendChild(bidButton(`${level}${d}`));
  }
  const extra = document.createElement('div');
  extra.className = 'bid-extra';
  for (const code of ['P', 'X', 'XX']) extra.appendChild(bidButton(code));
  box.append(grid, extra);
}

function bidButton(code) {
  const b = document.createElement('button');
  b.dataset.code = code;
  b.disabled = true;
  b.innerHTML = colorizeCalls(code);
  b.onclick = () => {
    if (current && current.your_turn && !current.ended) render(JSON.parse(game.bid(code)));
  };
  return b;
}

function updateBiddingBox(s) {
  const active = s.your_turn && !s.ended;
  const legal = new Set(s.legal);
  for (const b of id('p-bidbox').querySelectorAll('button')) {
    b.disabled = !active || !legal.has(b.dataset.code);
  }
  id('p-bidbox').classList.toggle('inactive', !active);
}

// --- book browser --------------------------------------------------------------

function loadBook() {
  const nodes = JSON.parse(book());
  const frag = document.createDocumentFragment();
  bookNodes = nodes.map((node) => {
    const el = document.createElement('div');
    el.className = 'node panel';
    const rules = node.rules.map((r) =>
      `<div class="rule"><span class="call">${colorizeCalls(r.call)}</span>` +
      `<span class="weight">w${fmtWeight(r.weight)}</span>` +
      `<span class="ruletext">${escapeHTML(r.text)}</span>` +
      (r.label ? `<span class="tag">${escapeHTML(r.label)}</span>` : '') +
      '</div>',
    ).join('') +
      (node.note ? `<div class="rule"><span class="ruletext">${escapeHTML(node.note)}</span></div>` : '');
    el.innerHTML =
      `<div class="node-head"><span class="badge ${node.book}">${node.book}</span>` +
      `<span class="node-auction">${colorizeCalls(node.auction)}</span></div>${rules}`;
    frag.appendChild(el);
    const haystack =
      (node.auction + ' ' + node.rules.map((r) => `${r.call} ${r.text}`).join(' ') +
        (node.note ? ' ' + node.note : '')).toLowerCase();
    const seqHay = normSeq(node.auction + ' ' + node.rules.map((r) => r.call).join(' '));
    return { el, haystack, seqHay };
  });
  id('b-results').appendChild(frag);
  filterBook();
}

// Fuzzy sequence normalizer for the book filter: ASCII shorthand ↔ book glyphs.
//   P/- → pass, C D H S → ♣♦♥♠, N or NT → notrump. Spaces dropped so the query
//   need not match the book's spacing. Deterministic (fixed map, no edit-distance).
//   ponytail: X/XX already match the haystack verbatim — deliberately untouched.
const SEQ_MAP = { '♣': 'c', '♦': 'd', '♥': 'h', '♠': 's', '-': 'p' };
function normSeq(s) {
  return s.toLowerCase()
    .replace(/nt/g, 'n')                     // notrump: nt or lone n → n
    .replace(/[♣♦♥♠-]/g, (g) => SEQ_MAP[g])  // suit glyphs + pass mark → letters
    .replace(/\s+/g, '');                    // ignore spacing (contiguous match)
}

function filterBook() {
  if (!bookNodes) return;
  const q = id('b-filter').value.trim().toLowerCase();
  const seq = normSeq(q);
  let n = 0;
  for (const { el, haystack, seqHay } of bookNodes) {
    const show = !q || haystack.includes(q) || seqHay.includes(seq);
    el.classList.toggle('hidden', !show);
    if (show) n++;
  }
  id('b-count').textContent = `${n} node${n === 1 ? '' : 's'}`;
}

function fmtWeight(w) {
  return Number.isInteger(w) ? w.toFixed(1) : String(w);
}

// --- deal editor ---------------------------------------------------------------
//
// A PBN text field that two-way-syncs with a 4×13 card palette (the lichess
// analysis-board idiom).  The whole tab is client-side: PBN is a trivial
// string, so no wasm round-trip.  State is one card→seat map; both the palette
// and the compass render from it.

const RANKS = ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2'];
const HCP = { A: 4, K: 3, Q: 2, J: 1 };
const SEAT_CYCLE = [null, 'N', 'E', 'S', 'W']; // click order; null = unassigned

let editAssign = {}; // "♠A" → "N" | "E" | "S" | "W"

function initEdit() {
  id('e-pbn').oninput = () => { editAssign = fromPBN(id('e-pbn').value); paintEdit(); };
  id('e-random').onclick = () => { editAssign = randomDeal(); syncFromBoard(); };
  id('e-clear').onclick = () => { editAssign = {}; syncFromBoard(); };
  id('e-copy').onclick = () => navigator.clipboard?.writeText(id('e-pbn').value);
  id('e-bid').onclick = () => {
    location.hash = 'demo'; // hand the edited deal to the Demo tab and bid it out
    runDemo(game.deal_pbn(toPBN(editAssign), id('d-dealer').value, id('d-vul').value));
  };
  id('e-grid').onclick = (ev) => {
    const card = ev.target.closest('button')?.dataset.card;
    if (!card) return;
    const next = SEAT_CYCLE[(SEAT_CYCLE.indexOf(editAssign[card] ?? null) + 1) % SEAT_CYCLE.length];
    if (next) editAssign[card] = next; else delete editAssign[card];
    syncFromBoard();
  };
  editAssign = randomDeal();
  syncFromBoard();
}

// Board edit → repaint everything and push the canonical PBN into the field.
function syncFromBoard() {
  paintEdit();
  id('e-pbn').value = toPBN(editAssign);
}

// Repaint from state only — never touches the text field, so typing is stable.
function paintEdit() {
  id('e-grid').innerHTML = editGridHTML();
  id('e-board').innerHTML = compassHTML(editHands());
  const n = { N: 0, E: 0, S: 0, W: 0 };
  for (const seat of Object.values(editAssign)) n[seat]++;
  const total = n.N + n.E + n.S + n.W;
  const full = total === 52 && SEATS.every((s) => n[s] === 13);
  id('e-status').textContent = full
    ? 'Full deal ✓ — click a card to cycle N→E→S→W→out, or bid it out'
    : `N ${n.N} · E ${n.E} · S ${n.S} · W ${n.W} — ${total}/52 placed`;
  id('e-bid').disabled = !full; // bots can only bid a complete deal
}

// PBN deal: "N:<N> <E> <S> <W>", each hand "spades.hearts.diamonds.clubs",
// ranks high→low.  We always emit from North (canonical); parsing honours a
// leading seat.
function toPBN(assign) {
  const holding = (seat) => HAND_ORDER.map((g) =>
    RANKS.filter((r) => assign[g + r] === seat).join('')).join('.');
  return 'N:' + SEATS.map(holding).join(' ');
}

// Tolerant parse: optional "<seat>:" prefix, whitespace-split hands clockwise,
// unknown chars (voids '-', 'x' spots) ignored; a repeated card just re-homes.
function fromPBN(text) {
  let s = text.trim();
  let start = 0;
  const m = s.match(/^([NESW])\s*:\s*/i);
  if (m) { start = SEATS.indexOf(m[1].toUpperCase()); s = s.slice(m[0].length); }
  const assign = {};
  s.split(/\s+/).filter(Boolean).forEach((hand, i) => {
    const seat = SEATS[(start + i) % 4];
    hand.split('.').forEach((holding, si) => {
      const g = HAND_ORDER[si];
      if (!g) return;
      for (const ch of holding.toUpperCase()) if (RANKS.includes(ch)) assign[g + ch] = seat;
    });
  });
  return assign;
}

function randomDeal() {
  const deck = HAND_ORDER.flatMap((g) => RANKS.map((r) => g + r));
  for (let i = deck.length - 1; i > 0; i--) { // Fisher–Yates; Math.random is fine (UI only)
    const j = Math.floor(Math.random() * (i + 1));
    [deck[i], deck[j]] = [deck[j], deck[i]];
  }
  return Object.fromEntries(deck.map((c, i) => [c, SEATS[Math.floor(i / 13)]]));
}

// Inverse of editHands(): a rendered hands object → the editAssign card→seat map.
function assignFromHands(hands) {
  const assign = {};
  for (const seat of SEATS) {
    const h = hands[seat];
    if (!h) continue;
    for (const g of HAND_ORDER) for (const r of (h[SUIT_KEYS[g]] || '')) assign[g + r] = seat;
  }
  return assign;
}

// One HandJson-shaped object per seat, so compassHTML/handHTML render as-is.
function editHands() {
  const hands = {};
  for (const seat of SEATS) {
    const h = { hcp: 0 };
    for (const g of HAND_ORDER) {
      const ranks = RANKS.filter((r) => editAssign[g + r] === seat);
      h[SUIT_KEYS[g]] = ranks.join('');
      for (const r of ranks) h.hcp += HCP[r] || 0;
    }
    hands[seat] = h;
  }
  return hands;
}

// 4 suit rows × 13 rank cells; each cell tinted by its owner seat (legend in CSS).
function editGridHTML() {
  return HAND_ORDER.map((g) =>
    `<div class="editrow"><span class="${SUIT_CLASS[g]} editsuit">${g}</span>` +
    RANKS.map((r) => {
      const seat = editAssign[g + r];
      return `<button class="editcell${seat ? ' seat-' + seat.toLowerCase() : ''}" ` +
        `data-card="${g}${r}">${r}<small>${seat || ''}</small></button>`;
    }).join('') + '</div>',
  ).join('');
}

// --- settings -------------------------------------------------------------------
//
// The Settings tab is built entirely from the wasm registry (describe_options()):
// one row per bidding knob, grouped by section, so a convention added in Rust shows
// up here automatically.  A "toggle" is a checkbox; a "choice" is a mutually-
// exclusive radio family (e.g. defense to their 1NT), backed by one engine enum.
// Only *deviations* from a row's default are persisted to localStorage and replayed
// onto the wasm state at startup (applyOption routes by kind).

const STORAGE_KEY = 'pons-settings';
let stored = JSON.parse(localStorage.getItem(STORAGE_KEY)) || {}; // {key: value}, defaults omitted
let OPTIONS = []; // [{key, section, kind, label, default, variants?}] — filled after init()

const ACRONYMS = { nt: 'NT', xyz: 'XYZ', rkcb: 'RKCB', dont: 'DONT', uvu: 'UvU', hcp: 'HCP', gf: 'GF', '1nt': '1NT', '3nt': '3NT', '4m': '4M', '2d': '2♦' };
const humanize = (key) => key.split('_')
  .map((w, i) => ACRONYMS[w] || (i === 0 ? w[0].toUpperCase() + w.slice(1) : w)).join(' ');
const labelOf = (opt) => opt.label || humanize(opt.key);

// The effective current value of an option (stored override, else its default).
const valueOf = (opt) => (opt.key in stored ? stored[opt.key] : opt.default);

// Push one saved value to the wasm bidder — booleans are toggles, strings choices.
function applyOption(key, value) {
  if (typeof value === 'boolean') set_option(key, value);
  else set_choice(key, value);
}

// One option's HTML: a checkbox, or a radio set for a mutually-exclusive family.
function optHTML(opt) {
  if (opt.kind === 'choice') {
    const cur = valueOf(opt);
    const radios = opt.variants.map((v) =>
      `<label class="opt"><input type="radio" name="${opt.key}" data-key="${opt.key}"` +
      ` value="${v.value}"${v.value === cur ? ' checked' : ''}> ${escapeHTML(v.label)}</label>`,
    ).join('');
    return `<div class="choice"><div class="choice-label">${escapeHTML(labelOf(opt))}</div>${radios}</div>`;
  }
  return `<label class="opt"><input type="checkbox" data-key="${opt.key}"` +
    `${valueOf(opt) ? ' checked' : ''}> ${escapeHTML(labelOf(opt))}</label>`;
}

let settingsBuilt = false;

function renderSettings() {
  settingsBuilt = true;
  // Group by section in first-appearance order.
  const order = [];
  const bySection = new Map();
  for (const opt of OPTIONS) {
    if (!bySection.has(opt.section)) { bySection.set(opt.section, []); order.push(opt.section); }
    bySection.get(opt.section).push(opt);
  }
  id('s-options').innerHTML = order.map((name) =>
    `<div class="panel"><div class="panel-title">${escapeHTML(name)}</div><div class="optlist">` +
    bySection.get(name).map(optHTML).join('') + '</div></div>',
  ).join('');

  id('settings').addEventListener('change', (ev) => {
    const el = ev.target.closest('input[type=checkbox], input[type=radio]');
    if (!el) return;
    setOption(el.dataset.key, el.type === 'radio' ? el.value : el.checked);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
  });

  id('s-reset').onclick = () => {
    if (!confirm('Reset all convention settings to defaults?')) return;
    stored = {};
    for (const opt of OPTIONS) applyOption(opt.key, opt.default);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
    renderInputs(); // repaint checked/selected from the (now empty) overrides
  };
}

// Reflect the current values onto the existing inputs without rebuilding listeners.
function renderInputs() {
  for (const opt of OPTIONS) {
    const cur = valueOf(opt);
    if (opt.kind === 'choice') {
      for (const r of id('settings').querySelectorAll(`input[name="${opt.key}"]`)) r.checked = (r.value === cur);
    } else {
      const cb = id('settings').querySelector(`input[type=checkbox][data-key="${opt.key}"]`);
      if (cb) cb.checked = cur;
    }
  }
}

// Apply one option to the wasm bidder and update the delta store (default-valued
// entries are dropped so localStorage only holds overrides).
function setOption(key, value) {
  applyOption(key, value);
  const opt = OPTIONS.find((o) => o.key === key);
  if (opt && value === opt.default) delete stored[key];
  else stored[key] = value;
}

main();
