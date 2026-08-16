// Thin static UI over the pons wasm bidder: the engine holds the deal and the
// auction; JS rebuilds the DOM from each JSON snapshot (gin-rummy pattern).
import init, { WebTable, Binky, book, set_option, set_choice, describe_options } from './pkg/pons_web.js';

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
let bookNodes = null; // [{el, haystack}] for the selected partnership
let bookPair = 'ns';
let demoTimer = 0;
let boardGen = 0; // bumped per deal; stale async DD/oracle loops check it
let analysisGen = -1; // last boardGen whose DD + oracle were kicked off

const id = (x) => document.getElementById(x);

async function main() {
  await init();
  game = new WebTable(String(Math.floor(Math.random() * 2 ** 53)));
  OPTIONS = JSON.parse(describe_options()); // the Settings registry, from wasm
  // Replay saved overrides: booleans are toggles, strings are radio-choice values.
  for (const pair of PAIRS) {
    for (const [key, value] of Object.entries(stored[pair])) applyOption(pair, key, value);
  }
  localStorage.setItem(STORAGE_KEY, JSON.stringify(stored)); // persist legacy migration
  buildBiddingBox();
  for (const b of document.querySelectorAll('nav button')) {
    b.onclick = () => { location.hash = b.dataset.tab; };
  }
  window.addEventListener('hashchange', () => showTab(location.hash.slice(1)));
  id('p-deal').onclick = dealPractice;
  id('p-hint-on').onchange = renderHint;
  id('d-deal').onclick = dealDemo;
  id('d-edit').onclick = editDemo;
  id('b-filter').oninput = filterBook;
  id('b-pair').onchange = (ev) => { bookPair = ev.target.value; loadBook(); };
  initEdit();
  initBinky();
  showTab(location.hash.slice(1));
}

function showTab(tab) {
  if (!['practice', 'demo', 'book', 'edit', 'binky', 'settings'].includes(tab)) tab = 'practice';
  for (const sec of document.querySelectorAll('main > section')) {
    sec.classList.toggle('hidden', sec.id !== tab);
  }
  for (const b of document.querySelectorAll('nav button')) {
    b.classList.toggle('active', b.dataset.tab === tab);
  }
  if (tab === 'book' && !bookNodes) loadBook();
  if (tab === 'settings' && !settingsBuilt) renderSettings();
  if (tab === 'binky' && !kTable) loadBinky();
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
  renderHint();
  renderFeedback(s);
  renderReveal(s);
}

// The net read off the auction, not off the hidden hands: the same inferences
// the bidder itself consumes. Watch sd narrow as partner describes their hand —
// a call that fails to narrow it is a reading bug you can see while playing.
function renderHint() {
  const box = id('p-hint');
  const on = id('p-hint-on').checked;
  const rows = on ? JSON.parse(game.hint()) : null;
  box.classList.toggle('hidden', !rows);
  if (!rows) return;

  box.innerHTML =
    '<div class="seat-head">Our tricks, as the auction reads so far</div>' +
    `<div class="hintrow">${rows.map((r) => `
       <div><span class="statlabel">${colorizeCalls(r.strain)}</span>
       <span class="statbig">${r.mean.toFixed(1)}</span>
       <span class="hintsd">± ${r.sd.toFixed(1)}</span></div>`).join('')}</div>`;
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
  const nodes = JSON.parse(book(bookPair));
  const frag = document.createDocumentFragment();
  id('b-results').replaceChildren();
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

// Weights arrive as integral centinats (the engine's unit); nats read better.
function fmtWeight(w) {
  return (w / 100).toFixed(2);
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
  id('e-eval').onclick = () => { binkyFromEdit(); location.hash = 'binky'; };
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
// Only *deviations* from a row's default are persisted per partnership and
// replayed onto the wasm state at startup (applyOption routes by kind).

const STORAGE_KEY = 'pons-settings';
const PAIRS = ['ns', 'ew'];
const PAIR_NAMES = { ns: 'North–South', ew: 'East–West' };
const rawStored = JSON.parse(localStorage.getItem(STORAGE_KEY)) || {};
const asOverrides = (value) => value && typeof value === 'object' && !Array.isArray(value) ? value : {};
const oldOverrides = asOverrides(rawStored);
let stored = ('ns' in oldOverrides || 'ew' in oldOverrides)
  ? { ns: asOverrides(oldOverrides.ns), ew: asOverrides(oldOverrides.ew) }
  : { ns: { ...oldOverrides }, ew: { ...oldOverrides } };
for (const pair of PAIRS) {
  delete stored[pair].their_2c_landy;
  delete stored[pair].their_2d_multi;
}
let OPTIONS = []; // [{key, section, kind, label, default, variants?}] — filled after init()
let settingsPair = 'ns';

const ACRONYMS = { nt: 'NT', xyz: 'XYZ', rkcb: 'RKCB', dont: 'DONT', uvu: 'UvU', hcp: 'HCP', gf: 'GF', '1nt': '1NT', '3nt': '3NT', '4m': '4M', '2d': '2♦' };
const humanize = (key) => key.split('_')
  .map((w, i) => ACRONYMS[w] || (i === 0 ? w[0].toUpperCase() + w.slice(1) : w)).join(' ');
const labelOf = (opt) => opt.label || humanize(opt.key);

// The effective current value of an option (stored override, else its default).
const valueOf = (opt, pair = settingsPair) =>
  (opt.key in stored[pair] ? stored[pair][opt.key] : opt.default);

// Whether a row's master is armed. `requires` is "key" or "key=value" on this
// partnership; an `opponent:` prefix reads the other profile instead.
function isLive(opt, pair = settingsPair) {
  if (!opt.requires) return true;
  let requires = opt.requires;
  if (requires.startsWith('opponent:')) {
    pair = pair === 'ns' ? 'ew' : 'ns';
    requires = requires.slice('opponent:'.length);
  }
  const [key, want] = requires.split('=');
  const master = OPTIONS.find((o) => o.key === key);
  if (!master) return true;
  const cur = valueOf(master, pair);
  return want === undefined ? cur === true : cur === want;
}

// Push one saved value to the wasm bidder — booleans are toggles, strings choices.
function applyOption(pair, key, value) {
  if (typeof value === 'boolean') set_option(pair, key, value);
  else set_choice(pair, key, value);
}

// One option's HTML: a checkbox, or a radio set for a mutually-exclusive family.
function optHTML(opt) {
  const live = isLive(opt);
  const dis = live ? '' : ' disabled';
  const dim = live ? '' : ' dimmed';
  const needs = live ? '' : ` title="needs ${escapeHTML(opt.requires.replace('opponent:', 'opponent ').replace('=', ': '))}"`;
  if (opt.kind === 'choice') {
    const cur = valueOf(opt);
    const radios = opt.variants.map((v) =>
      `<label class="opt${dim}"${needs}><input type="radio" name="${opt.key}" data-key="${opt.key}"` +
      ` value="${v.value}"${v.value === cur ? ' checked' : ''}${dis}> ${escapeHTML(v.label)}</label>`,
    ).join('');
    return `<div class="choice"><div class="choice-label">${escapeHTML(labelOf(opt))}</div>${radios}</div>`;
  }
  return `<label class="opt${dim}"${needs}><input type="checkbox" data-key="${opt.key}"` +
    `${valueOf(opt) ? ' checked' : ''}${dis}> ${escapeHTML(labelOf(opt))}</label>`;
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
    setOption(settingsPair, el.dataset.key, el.type === 'radio' ? el.value : el.checked);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
    renderInputs(); // this row may be some other row's master
  });

  id('s-pair').onchange = (ev) => {
    settingsPair = ev.target.value;
    renderInputs();
  };

  id('s-reset').onclick = () => {
    if (!confirm(`Reset ${PAIR_NAMES[settingsPair]} convention settings to defaults?`)) return;
    stored[settingsPair] = {};
    for (const opt of OPTIONS) applyOption(settingsPair, opt.key, opt.default);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
    bookNodes = null;
    renderInputs(); // repaint checked/selected from the (now empty) overrides
  };
}

// Reflect the current values onto the existing inputs without rebuilding listeners.
function renderInputs() {
  id('s-reset').textContent = `Reset ${PAIR_NAMES[settingsPair]} to defaults`;
  for (const opt of OPTIONS) {
    const cur = valueOf(opt);
    const live = isLive(opt);
    const inputs = opt.kind === 'choice'
      ? id('settings').querySelectorAll(`input[name="${opt.key}"]`)
      : id('settings').querySelectorAll(`input[type=checkbox][data-key="${opt.key}"]`);
    for (const el of inputs) {
      el.checked = opt.kind === 'choice' ? el.value === cur : cur;
      el.disabled = !live;
      el.closest('label')?.classList.toggle('dimmed', !live);
    }
  }
}

// Apply one option to the wasm bidder and update the delta store (default-valued
// entries are dropped so localStorage only holds overrides).
function setOption(pair, key, value) {
  applyOption(pair, key, value);
  const opt = OPTIONS.find((o) => o.key === key);
  if (opt && value === opt.default) delete stored[pair][key];
  else stored[pair][key] = value;
  bookNodes = null;
}

main();

// --- evaluate (Binky Points with error bars) ---------------------------------
//
// The published table is data, not code: `binky.json` maps a suit holding to its
// contribution to the mean and to the variance, both additive across the eight
// N-S holdings.  See docs/binky-points.md.

const K_HONOURS = 'AKQJT';
// Observed = filled bars, predicted = a stepped line.  The pair validates on the
// six colour checks against --paper (protan ΔE 10.1); the form difference is a
// second encoding on top, so the two never rely on hue alone.
const K_OBSERVED = 'var(--club)';
const K_PREDICTED = 'var(--diamond)';

let kTable = null; // the parsed binky.json
let kVerdict = null; // {n, mean, sd, histogram} from the DD shuffles
let kRunning = false;

function initBinky() {
  for (const x of ['k-north', 'k-south']) id(x).oninput = () => { kVerdict = null; renderBinky(); };
  id('k-which').onchange = loadBinky;
  id('k-filter').oninput = renderBinkyTable;
  id('k-verify').onclick = runVerdict;
  id('k-from-edit').onclick = binkyFromEdit;
  id('k-to-edit').onclick = binkyToEdit;
}

function binkyNotrump() { return id('k-which').value === 'binky.json'; }

async function loadBinky() {
  kVerdict = null;
  try {
    const res = await fetch(id('k-which').value);
    if (!res.ok) throw new Error(res.status);
    kTable = await res.json();
  } catch {
    kTable = null;
    id('k-parse').textContent = 'That table has not been generated yet — run examples/binky.';
    return;
  }
  id('k-gauge').textContent =
    `${kTable.label}. Weights are excess versus an average holding; the fit is rank-deficient ` +
    'by two directions (Σn = 8 and Σn·size = 26), so the table is only defined up to ' +
    'w → w + α + β·size with 8α + 26β = 0.' +
    // Measured: best-suit sigma is flat across true-sigma quintiles (corr 0.059).
    // The mean column is fine; say so rather than quietly serving a constant.
    (binkyNotrump() ? '' :
      ' Read the mean only — benchmarked against reshuffled truth, the best-suit σ is' +
      ' effectively constant (corr 0.059), because additivity cannot see fit.');
  renderBinky();
  renderBinkyTable();
}

// "AK32.QJ4.T98.762" → four holdings, or null if it is not 13 cards in 4 suits.
function parseHolding(text) {
  const suits = text.trim().toUpperCase().split('.');
  if (suits.length !== 4) return null;
  const cards = suits.join('');
  if (cards.length !== 13 || /[^AKQJT2-9]/.test(cards)) return null;
  return suits;
}

// Look a holding up, folding spots down the way the fit merged its rare cells.
function binkyLookup(cards) {
  const honours = [...K_HONOURS].filter((h) => cards.includes(h)).join('');
  let spots = [...cards].filter((c) => !K_HONOURS.includes(c)).length;
  for (; spots >= 0; spots--) {
    const row = kTable.holdings[(honours + 'x'.repeat(spots)) || 'void'];
    if (row) return row;
  }
  return [0, 0, 0, 0];
}

function normalCdf(z) {
  // Abramowitz & Stegun 26.2.17 — the approximation the Rust crate also serves.
  const b = [0.31938153, -0.356563782, 1.781477937, -1.821255978, 1.330274429];
  const x = Math.abs(z);
  const t = 1 / (1 + 0.2316419 * x);
  const poly = b.reduceRight((acc, c) => (acc + c) * t, 0);
  const upper = 0.3989422804014327 * Math.exp(-0.5 * x * x) * poly;
  return z < 0 ? upper : 1 - upper;
}

// The two hands' eight holdings summed: predictive sigma prices the contracts,
// physical sigma is what the DD verdict should actually match.
function binkyEvaluate() {
  const hands = ['k-north', 'k-south'].map((x) => {
    const parsed = parseHolding(id(x).value);
    id(x).classList.toggle('bad', parsed === null);
    return parsed;
  });
  if (!kTable || hands.some((h) => h === null)) return null;

  const holdings = hands.flat();
  let mu = kTable.mean_const;
  let variance = kTable.var_const;
  let physical = kTable.physical_var_const ?? null;
  for (const h of holdings) {
    const [m, v, pv] = binkyLookup(h);
    mu += m;
    variance += v;
    if (physical !== null) physical += pv;
  }
  return {
    holdings,
    mu,
    sd: Math.sqrt(Math.max(variance, 0.01)),
    physical: physical === null ? null : Math.sqrt(Math.max(physical, 0.01)),
  };
}

function renderBinky() {
  const e = binkyEvaluate();
  if (!e) {
    // No table means loadBinky already wrote why; leave that message standing.
    if (kTable) {
      id('k-parse').textContent =
        'Each hand needs thirteen cards as spades.hearts.diamonds.clubs — e.g. AK32.QJ4.T98.762';
    }
    id('k-readout').innerHTML = '';
    return;
  }
  id('k-parse').textContent = '';

  const game = binkyNotrump() ? 9 : 10;
  const p = (k) => 1 - normalCdf((k - 0.5 - e.mu) / e.sd);
  const rows = binkyNotrump()
    ? [['1NT', 7], ['2NT', 8], ['3NT', 9], ['4NT', 10], ['5NT', 11], ['6NT', 12], ['7NT', 13]]
    : [['2-level', 8], ['3-level', 9], ['4 of a major', 10], ['5 of a minor', 11],
       ['small slam', 12], ['grand slam', 13]];
  // IMP break-evens against a cold alternative (docs/binky-points.md).
  const BREAK_EVEN = { 9: ['45.5%', '37.5%'], 10: ['45.5%', '37.5%'],
                       12: ['50.0%', '50.0%'], 13: ['58.3%', '56.7%'] };

  id('k-readout').innerHTML =
    `<div class="statrow">
       <div><span class="statlabel">expected tricks</span><span class="statbig">${e.mu.toFixed(2)}</span></div>
       <div><span class="statlabel">σ predictive</span><span class="statbig">${e.sd.toFixed(2)}</span></div>
       <div><span class="statlabel">σ physical</span><span class="statbig">${
         e.physical === null ? '—' : e.physical.toFixed(2)}</span></div>
       <div><span class="statlabel">P(game)</span><span class="statbig">${(100 * p(game)).toFixed(0)}%</span></div>
     </div>
     <p class="hint"><strong>Predictive</strong> σ prices the contracts below — it includes the
     table's own error, which is what makes those probabilities calibrated.
     <strong>Physical</strong> σ is the hands' genuine volatility over the opponents' possible
     splits. The gap between them is the table's ignorance, and the DD verdict measures the
     physical one.</p>
     <p class="hint">Holdings: ${e.holdings.map((h) => holdingKey(h)).join(' · ')}</p>
     <table class="ddtable"><thead><tr><th>contract</th><th>tricks</th><th>P(make)</th>
       <th>NV break-even</th><th>vul</th></tr></thead><tbody>` +
    rows.map(([name, k]) => {
      const [nv, vul] = BREAK_EVEN[k] ?? ['', ''];
      const pct = 100 * p(k);
      const made = nv && pct >= parseFloat(nv);
      return `<tr><td>${name}</td><td>${k}</td>` +
             `<td${made ? ' class="win"' : ''}>${pct.toFixed(1)}%</td><td>${nv}</td><td>${vul}</td></tr>`;
    }).join('') + '</tbody></table>';

  renderVerdict(e);
}

function holdingKey(cards) {
  const honours = [...K_HONOURS].filter((h) => cards.includes(h)).join('');
  const spots = [...cards].filter((c) => !K_HONOURS.includes(c)).length;
  return (honours + 'x'.repeat(spots)) || 'void';
}

// --- the double-dummy verdict ------------------------------------------------
//
// Fix both N-S hands, reshuffle East-West, solve each layout.  Conditioned on the
// two N-S hands the posterior over the hidden 26 cards IS uniform over E-W splits,
// so this is ground truth with no sampler to be biased — the same check
// `examples/binky --benchmark` runs natively.

const K_CHUNK = 10; // solves per JS task, so the page keeps painting

async function runVerdict() {
  const e = binkyEvaluate();
  if (!e || kRunning) return;
  const total = Number(id('k-shuffles').value);
  const engine = Binky.create(id('k-north').value.trim(), id('k-south').value.trim(),
                              binkyNotrump(), String(Math.floor(Math.random() * 2 ** 53)));
  if (!engine) { id('k-progress').textContent = 'Those two hands overlap.'; return; }

  kRunning = true;
  id('k-verify').disabled = true;
  for (let done = 0; done < total; done += K_CHUNK) {
    kVerdict = JSON.parse(engine.run(Math.min(K_CHUNK, total - done)));
    id('k-progress').textContent = `${kVerdict.n} / ${total} layouts solved`;
    renderVerdict(e);
    await new Promise((r) => setTimeout(r, 0)); // yield so the browser repaints
  }
  id('k-progress').textContent = `${kVerdict.n} layouts solved`;
  id('k-verify').disabled = false;
  kRunning = false;
}

function renderVerdict(e) {
  const box = id('k-verdict');
  if (!kVerdict || !kVerdict.n) { box.classList.add('hidden'); return; }
  box.classList.remove('hidden');

  // Overlay the column the histogram actually tests. The shuffles measure the
  // spread given the two hands, which is the PHYSICAL column; drawing the
  // predictive one here would look like a miscalibrated fit when it is simply
  // answering a different question (it carries the table's own error too).
  const overlaySd = e.physical ?? e.sd;
  const overlayName = e.physical === null ? 'predictive' : 'physical';
  // P(T = k) from the fitted Gaussian, by differencing the CDF on half-trick
  // boundaries — the same continuity correction the crate's `p_at_least` uses.
  const predicted = kVerdict.histogram.map((_, k) =>
    normalCdf((k + 0.5 - e.mu) / overlaySd) - normalCdf((k - 0.5 - e.mu) / overlaySd));
  const observed = kVerdict.histogram.map((c) => c / kVerdict.n);
  const peak = Math.max(...observed, ...predicted, 1e-6);

  // Trim the empty tails so the plot spends its width where the mass is.
  let lo = 0, hi = 13;
  while (lo < hi && observed[lo] < 0.005 && predicted[lo] < 0.005) lo++;
  while (hi > lo && observed[hi] < 0.005 && predicted[hi] < 0.005) hi--;

  const bars = [];
  for (let k = lo; k <= hi; k++) {
    const o = (100 * observed[k]) / peak;
    const pr = (100 * predicted[k]) / peak;
    bars.push(
      `<div class="kbar" title="${k} tricks — observed ${(100 * observed[k]).toFixed(1)}%, ` +
      `predicted ${(100 * predicted[k]).toFixed(1)}%">` +
      `<div class="kbarstack"><div class="kobserved" style="height:${o}%"></div>` +
      `<div class="kpredicted" style="bottom:${pr}%"></div></div>` +
      `<div class="kbarlabel">${k}</div></div>`);
  }

  const sdGap = e.physical === null ? null : e.physical - kVerdict.sd;
  box.innerHTML =
    `<h3>Double-dummy verdict — ${kVerdict.n} East-West shuffles</h3>
     <p class="hint">Both N-S hands fixed; only the opponents' 26 cards are redealt. Conditioned
     on your two hands, that posterior is exactly uniform — there is no sampler here to be
     biased, so these are the true conditional moments up to sampling noise
     (σ's own standard error is about ${(kVerdict.sd / Math.sqrt(2 * kVerdict.n)).toFixed(3)} tricks).</p>
     <div class="klegend">
       <span><i class="kswatch" style="background:${K_OBSERVED}"></i>observed (double dummy)</span>
       <span><i class="kswatch kline" style="background:${K_PREDICTED}"></i>the table's ${overlayName} Gaussian</span>
     </div>
     <div class="kchart">${bars.join('')}</div>
     <div class="kaxis">tricks</div>
     <table class="ddtable"><thead><tr><th></th><th>mean</th><th>σ</th></tr></thead><tbody>
       <tr><td>observed</td><td>${kVerdict.mean.toFixed(3)}</td><td>${kVerdict.sd.toFixed(3)}</td></tr>
       <tr><td>table, predictive</td><td>${e.mu.toFixed(3)}</td><td>${e.sd.toFixed(3)}</td></tr>
       ${e.physical === null ? '' :
         `<tr><td>table, physical</td><td>${e.mu.toFixed(3)}</td><td>${e.physical.toFixed(3)}</td></tr>`}
     </tbody></table>
     <p class="hint">${
       sdGap === null
         ? 'This table has no physical column — regenerate it with --variance-truth.'
         : `Physical σ is ${sdGap >= 0 ? 'over' : 'under'} the observed spread by ` +
           `${Math.abs(sdGap).toFixed(3)} tricks. Predictive σ sits ` +
           `${(e.sd - kVerdict.sd).toFixed(3)} above it by design: it carries the table's own error.`}</p>`;
}

// --- handoff with the Edit tab ----------------------------------------------

function binkyFromEdit() {
  const hands = editHands();
  for (const [box, seat] of [['k-north', 'N'], ['k-south', 'S']]) {
    id(box).value = HAND_ORDER.map((g) => hands[seat][SUIT_KEYS[g]] || '').join('.');
  }
  kVerdict = null;
  renderBinky();
}

function binkyToEdit() {
  const hands = ['k-north', 'k-south'].map((x) => parseHolding(id(x).value));
  if (hands.some((h) => h === null)) return;
  // Only N-S move; E-W becomes whatever is left, so the editor shows a full deal.
  const assign = {};
  for (const [suits, seat] of [[hands[0], 'N'], [hands[1], 'S']]) {
    suits.forEach((holding, i) => { for (const r of holding) assign[HAND_ORDER[i] + r] = seat; });
  }
  const rest = [];
  for (const g of HAND_ORDER) for (const r of RANKS) if (!assign[g + r]) rest.push(g + r);
  rest.forEach((card, i) => { assign[card] = i < rest.length / 2 ? 'E' : 'W'; });
  editAssign = assign;
  syncFromBoard();
  location.hash = 'edit';
}

function renderBinkyTable() {
  if (!kTable) return;
  const needle = id('k-filter').value.trim().toLowerCase();
  const hasPhysical = kTable.physical_var_const !== undefined;
  const cell = (x) => `<td class="${x < 0 ? 'lose' : 'win'}">${x >= 0 ? '+' : ''}${x.toFixed(3)}</td>`;
  const rows = Object.entries(kTable.holdings)
    .filter(([name]) => name.toLowerCase().includes(needle))
    .map(([name, row]) => {
      const n = row[row.length - 1];
      return `<tr><td>${name}</td>${cell(row[0])}${cell(row[1])}` +
             (hasPhysical ? cell(row[2]) : '') + `<td>${n.toLocaleString()}</td></tr>`;
    });
  id('k-table').innerHTML =
    '<table class="ddtable"><thead><tr><th>holding</th><th>μ (tricks)</th><th>predictive var</th>' +
    (hasPhysical ? '<th>physical var</th>' : '') + '<th>deals</th></tr></thead><tbody>' +
    rows.join('') + '</tbody></table>';
}
