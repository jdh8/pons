// Thin static UI over the pons wasm bidder: the engine holds the deal and the
// auction; JS rebuilds the DOM from each JSON snapshot (gin-rummy pattern).
import init, { WebTable, Binky, book, point_census, set_option, set_choice, describe_options } from './pkg/pons_web.js';

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
  initCalc();
  initCompanion();
  showTab(location.hash.slice(1));
}

function showTab(tab) {
  if (!['practice', 'demo', 'book', 'edit', 'binky', 'odds', 'partner', 'settings'].includes(tab)) tab = 'practice';
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
    // Ladder in nats when an authored node answered this hand (a precedence),
    // odds when the floor did — `authored` is per hand, not per node.
    const show = (v) => (f.authored ? v.toFixed(2) : Math.round(v) + '%');
    const bot = f.top.length
      ? 'bot: ' + f.top.map(([c, v]) => `${colorizeCalls(c)} ${show(v)}`).join(' · ')
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

// --- Boxes: the grammar the Odds and Partner tabs share -----------------------
// A hand is a union of boxes, one table row each: four suit cells (a length
// range or a holding), a points range on the gauge the column header names,
// and a "where" predicate for what the cells cannot say.
const GAUGES = [ // [column header, plain name in a where, what it counts]
  ['HCP', 'hcp', 'high-card points'],
  ['UP', 'up', "upgraded points: HCP + upgrade, the bidder's scale"],
  ...[...'♠♥♦♣'].map((suit, i) => [`SP${suit}`, `sp${'shdc'[i]}`, `support points, ${suit} trumps`]),
];
// One hand's values as a where reads them — its slots, from the hand's base,
// in the vector the predicate is evaluated on: four lengths, the six gauges,
// each suit's hcp, freakness.
const SLOT = { len: 0, gauge: 4, suitHcp: 10, freak: 14 };
const HAND_SLOTS = 15;
// The names of the hand at `base`: ♠ ♥ ♦ ♣ (or s h d c) the lengths, ♠.hcp a
// suit's own hcp, the gauges hcp, up, sp♠ … (or sps …), freak — and p, the
// gauge the box's Points column is on.
function handVars(base, gauge, prefix = '') {
  const vars = {};
  const name = (text, slot) => { vars[prefix + text] = base + slot; };
  [...'shdc'].forEach((letter, i) => {
    for (const suit of [letter, '♠♥♦♣'[i]]) {
      name(suit, SLOT.len + i);
      name(`${suit}.hcp`, SLOT.suitHcp + i);
      name(`sp${suit}`, SLOT.gauge + 2 + i);
    }
  });
  name('hcp', SLOT.gauge); name('up', SLOT.gauge + 1); name('p', SLOT.gauge + gauge); name('freak', SLOT.freak);
  return vars;
}

// "5+", "4-6", "3", "" → [min, max]; null when unparseable.  A dash needs a
// number on both sides: "3-" and "-3" read as "3 minus" and "negative 3".
function parseRange(text, cap) {
  const t = text.replace(/\s/g, '');
  if (t === '') return [0, cap];
  let m;
  if ((m = t.match(/^(\d+)\+$/))) return [+m[1], cap];
  if ((m = t.match(/^(\d+)-(\d+)$/))) return [+m[1], +m[2]];
  if ((m = t.match(/^(\d+)$/))) return [+m[1], +m[1]];
  return null;
}

// A box's "where" predicate.  Numbers are tropical rational functions —
// integers, + − max() min() — of the names in `vars` (`handVars`; a dot
// joins a name's parts, as in ♠.hcp and my.♠).  Comparisons chain
// (s >= h >= d means both); conditions join with ! (not), & (and), | (or),
// and a comma — an "and" that binds loosest, so `a | b, c` is (a | b) & c.
// Typed like Rust: a number is never a condition, nor the reverse.
// A distribution is a condition, lexed whole: WBF's 5431 (♠♥♦♣ in order),
// (5431) (any order) and (54)(31) (within each pair), or the dashed forms that
// alone carry a 10+ suit, 7=6=0=0 (in order) and 7-6-0-0 (any order), never
// mixed; x is any number of cards — 54xx, (55)xx, 5-5-x-x.
// Returns v → bool over the vector `vars` indexes, tagged with `used`, the
// slots it reads (a caller can skip an unread dimension), and `alike`, the
// runs of length slots a distribution reads in any order (those suits still
// permute).  "" → always true; null when unparseable or ill-typed.
// Over a vector with NaN for what is not known yet the predicate is
// three-valued: undefined while it hangs on a NaN, so false already rules the
// known part out.
const WHERE_NAME = '(?:sp[♠♥♦♣]|[a-z]+|[♠♥♦♣])';
const WHERE_DIST = '(?:\\d+|x)(?:[-=](?:\\d+|x)){3}(?![\\w=-])|(?=(?:\\(?[\\dx]\\)?){4}(?![\\w(]))(?:[\\dx]|\\([\\dx]{2,4}\\))+';
const WHERE_TOKEN = new RegExp(`${WHERE_DIST}|\\d+|[<>!=]=|&&|\\|\\||${WHERE_NAME}(?:\\.${WHERE_NAME})*|\\S`, 'g');
const WHERE_CMP = {
  '>': (a, b) => a > b, '>=': (a, b) => a >= b, '≥': (a, b) => a >= b,
  '<': (a, b) => a < b, '<=': (a, b) => a <= b, '≤': (a, b) => a <= b,
  '=': (a, b) => a === b, '==': (a, b) => a === b, '!=': (a, b) => a !== b, '≠': (a, b) => a !== b,
};
const WHERE_AND = (a, b) => (a === false || b === false ? false : a === true ? b : a);
const WHERE_OR = (a, b) => (a === true || b === true ? true : a === false ? b : a);
function compileWhere(text, vars) {
  const toks = text.toLowerCase().match(WHERE_TOKEN) || [];
  const used = new Set(), alike = [];
  if (!toks.length) return Object.assign(() => true, { used, alike });
  let i = 0;
  const eat = (...ts) => (ts.includes(toks[i]) ? toks[i++] : null);
  const need = (t) => { if (!eat(t)) throw new SyntaxError(`expected ${t}`); };
  // a parsed node is {f, bool}: its evaluator and whether it is a condition
  const typed = (node, bool) => {
    if (node.bool !== bool) throw new TypeError(bool ? 'expected a condition' : 'expected a number');
    return node.f;
  };
  const num = (f) => ({ f, bool: false });
  // one left-associative binary level over operands of one type
  const fold = (next, ops, bool) => () => {
    let l = next();
    for (let op; (op = eat(...Object.keys(ops)));) {
      const a = typed(l, bool), b = typed(next(), bool), f = ops[op];
      l = { f: (v) => f(a(v), b(v)), bool };
    }
    return l;
  };
  // a distribution: runs of suits, each holding its lengths in any order
  const dist = (t) => {
    const sep = /[-=]/.exec(t)?.[0];
    const runs = !sep ? [...t.matchAll(/\(([\dx]+)\)|[\dx]/g)].map((m) => [...(m[1] || m[0])])
      : sep === '=' ? t.split('=').map((len) => [len]) : [t.split('-')];
    const lens = runs.flat(), sum = lens.reduce((n, len) => n + (len === 'x' ? 0 : +len), 0);
    if (lens.length !== 4 || !lens.every((len) => /^(?:\d+|x)$/.test(len)) || sum > 13 || (sum < 13 && !lens.includes('x'))) {
      throw new SyntaxError(`no such distribution: ${t}`);
    }
    let suit = 0;
    const groups = runs.map((run) => ({ slots: run.map(() => vars['shdc'[suit++]]), want: run.filter((len) => len !== 'x').map(Number) }))
      .filter(({ want }) => want.length);
    for (const { slots } of groups) if (slots.length > 1) alike.push(slots); else used.add(slots[0]);
    return { f: (v) => groups.every(({ slots, want }) => {
      const have = slots.map((slot) => v[slot]);
      return want.every((n) => { const k = have.indexOf(n); return k >= 0 && have.splice(k, 1); });
    }), bool: true };
  };
  const atom = () => {
    const t = toks[i++];
    if (/^(?:\d+|x)[-=]/.test(t) || (/^[\dx()]+$/.test(t) && t.replace(/[()]/g, '').length === 4)) return dist(t);
    if (/^\d+$/.test(t)) return num(() => +t);
    if (Object.hasOwn(vars, t)) { const slot = vars[t]; used.add(slot); return num((v) => v[slot]); }
    if (t === '-') { const a = typed(atom(), false); return num((v) => -a(v)); }
    if (t === '(') { const e = all(); need(')'); return e; }
    if (t === 'max' || t === 'min') {
      need('(');
      const args = [typed(sum(), false)];
      while (eat(',')) args.push(typed(sum(), false));
      need(')');
      return num((v) => Math[t](...args.map((a) => a(v))));
    }
    throw new SyntaxError(`unexpected ${t}`);
  };
  const sum = fold(atom, { '+': (a, b) => a + b, '-': (a, b) => a - b }, false);
  const cmp = () => {
    const first = sum(), rest = [], ops = [];
    for (let op; (op = eat(...Object.keys(WHERE_CMP)));) { ops.push(WHERE_CMP[op]); rest.push(typed(sum(), false)); }
    if (!ops.length) return first;
    const terms = [typed(first, false), ...rest];
    return { f: (v) => { const x = terms.map((t) => t(v)); return x.some(Number.isNaN) ? undefined : ops.every((f, k) => f(x[k], x[k + 1])); }, bool: true };
  };
  const not = () => {
    if (!eat('not', '!')) return cmp();
    const a = typed(not(), true);
    return { f: (v) => { const x = a(v); return x === undefined ? x : !x; }, bool: true };
  };
  const and = fold(not, { and: WHERE_AND, '&': WHERE_AND, '&&': WHERE_AND }, true);
  const or = fold(and, { or: WHERE_OR, '|': WHERE_OR, '||': WHERE_OR }, true);
  const all = fold(or, { ',': WHERE_AND }, true);
  try {
    const f = typed(all(), true);
    return i < toks.length ? null : Object.assign(f, { used, alike });
  } catch { return null; }
}

const HONORS = 'AKQJT'; // the named cards, bit 16 >> index; the ten is an honor that scores 0 hcp
const popcount = (mask) => { let n = 0; for (; mask; mask >>= 1) n += mask & 1; return n; };
function choose(n, k) {
  if (k < 0 || k > n) return 0;
  let c = 1;
  for (let i = 1; i <= k; i++) c = (c * (n - k + i)) / i;
  return Math.round(c);
}

// A suit cell as a holding: a regex over the suit written high to low —
// A K Q J T by name, x a card 2–9, . any card — matched whole, so "AKxx" is
// exactly that, "AKx*" any number of small cards under the AK, "AK...+" the AK
// in a 5+ suit, ".*K.*" the king anywhere, "[^A]*" no ace.  A digit is only
// ever a count ("x{3}"): "Q4" is malformed, not Q-4 doubleton or Q-fourth.
// → {range: [min, max] length, admit: [mask * 9 + spots] → 0/1}, or null when
// malformed or matching no holding.
const HOLDINGS = []; // [mask, spots, text], all 32 × 9
for (let mask = 0; mask < 32; mask++) for (let spots = 0; spots <= 8; spots++) {
  HOLDINGS.push([mask, spots, [...HONORS].filter((_, i) => mask & (16 >> i)).join('') + 'x'.repeat(spots)]);
}
function compileHolding(text) {
  const t = text.replace(/\s/g, '').toUpperCase().replaceAll('X', 'x');
  if (!/^[AKQJTx.*+?()[\]^|{},\d]+$/.test(t) || /\d/.test(t.replace(/\{\d*,?\d*\}/g, ''))) return null;
  let re;
  try { re = new RegExp(`^(?:${t})$`); } catch { return null; }
  const admit = new Uint8Array(32 * 9);
  let lo = 14, hi = -1;
  for (const [mask, spots, holding] of HOLDINGS) {
    if (!re.test(holding)) continue;
    admit[mask * 9 + spots] = 1;
    lo = Math.min(lo, holding.length); hi = Math.max(hi, holding.length);
  }
  return hi < 0 ? null : { range: [lo, hi], admit };
}
// "AKT52.K83.94.762" (dots, or spaces; - or nothing = void) → four cell texts
// with the spots as x, or null when it is not thirteen distinct cards.
function splitHand(text) {
  const parts = text.trim().toUpperCase().split(text.includes('.') ? '.' : /\s+/);
  if (parts.length !== 4 || !parts.every((part) => /^(-|[AKQJT2-9X]*)$/.test(part))) return null;
  const suits = parts.map((part) => [...HONORS].filter((ch) => part.includes(ch)).join('') + 'x'.repeat(part.replace(/[-AKQJT]/g, '').length));
  return suits.join('').length === 13 ? suits.map((suit) => suit || '0') : null;
}

// One box's six cell texts → [♠, ♥, ♦, ♣, points, where], null where a cell is
// malformed.  A suit is {range} for a length, {range, admit} for a holding.
function parseCells(texts, vars) {
  const suits = texts.slice(0, 4).map((t) => {
    const range = parseRange(t, 13);
    return range ? { range } : compileHolding(t);
  });
  return [...suits, parseRange(texts[4], 60), compileWhere(texts[5], vars)];
}
// `p` is the slot of the gauge the points range is on
const makeBox = ([s, h, d, c, pts, where], p) =>
  ({ lens: [s, h, d, c].map((x) => x.range), cells: [s, h, d, c].map((x) => x.admit), pts, p, where });
const ANY_BOX = makeBox(parseCells(['', '', '', '', '', ''], {}), SLOT.gauge);
const MAX_BOXES = 26; // to a union: the boxes still admitted are a bit set
const inLens = (box, lens) => lens.every((l, i) => l >= box.lens[i][0] && l <= box.lens[i][1]);
// does the box's suit cell take this holding: `mask` honors in a `len`-card suit
const admits = (box, suit, mask, len) => len >= box.lens[suit][0] && len <= box.lens[suit][1]
  && (!box.cells[suit] || box.cells[suit][mask * 9 + len - popcount(mask)] === 1);
const meets = (box, v) => v[box.p] >= box.pts[0] && v[box.p] <= box.pts[1] && box.where(v);

// A table of boxes: its header with the gauge picker, one empty box, the
// "+ box" button beside it, and a whole hand pasted into a suit cell filling
// the row.
function initBoxes(table, onChange) {
  const help = (target) => `<button class="help" popovertarget="help-${target}" aria-label="Help">?</button>`;
  const options = GAUGES.map(([, name, what], g) => `<option value="${g}">${name.toUpperCase()}&emsp;${what}</option>`).join('');
  const gauge = `<label class="gauge"><span>HCP</span><select aria-label="Point gauge" title="The gauge this column's ranges are on">${options}</select></label>`;
  id(table).innerHTML = `<thead><tr><th class="s-s">♠ ${help('suit')}</th><th class="s-h">♥</th><th class="s-d">♦</th><th class="s-c">♣</th>
    <th>${gauge}</th><th>Where ${help('where')}</th><th></th></tr></thead><tbody></tbody>`;
  const select = id(table).querySelector('select');
  select.onchange = () => { select.previousElementSibling.textContent = GAUGES[select.value][0]; onChange(); };
  id(`${table}-add`).onclick = () => { addBox(table, onChange); onChange(); };
  id(table).addEventListener('paste', (ev) => {
    const cells = [...ev.target.closest('tr').querySelectorAll('input')];
    const at = cells.indexOf(ev.target);
    const hand = at >= 0 && at < 4 && splitHand(ev.clipboardData.getData('text'));
    if (!hand) return;
    ev.preventDefault();
    hand.forEach((text, i) => { cells[i].value = text; });
    onChange();
  });
  addBox(table, onChange);
}
// the gauge the table's Points column is on, as a GAUGES index
const gaugeOf = (table) => +id(table).querySelector('select').value;

function addBox(table, onChange) {
  const tr = document.createElement('tr');
  for (let i = 0; i < 6; i++) {
    const td = document.createElement('td');
    const input = document.createElement('input');
    input.type = 'text'; input.spellcheck = false; input.placeholder = 'any';
    if (i === 5) input.className = 'where';
    input.oninput = onChange;
    td.append(input); tr.append(td);
  }
  const td = document.createElement('td');
  const rm = document.createElement('button');
  rm.className = 'secondary'; rm.textContent = '×'; rm.title = 'Remove this box';
  rm.onclick = () => { tr.remove(); onChange(); };
  td.append(rm); tr.append(td);
  id(table).tBodies[0].append(tr);
}

// The table's well-formed boxes; a malformed cell is marked and drops its box.
function readBoxes(table, vars) {
  const boxes = [];
  for (const tr of id(table).tBodies[0].rows) {
    const cells = [...tr.querySelectorAll('input')];
    const parts = parseCells(cells.map((c) => c.value), vars);
    cells.forEach((c, i) => c.classList.toggle('bad', parts[i] === null));
    if (parts.every(Boolean)) boxes.push(makeBox(parts, vars.p));
  }
  return boxes;
}

// --- Calc tab: exact shape × points probabilities -----------------------------
// A union of boxes (and any Pavlicek-style constraint) is a predicate over the
// 560 hand shapes times the point gauges, so the count is an exact
// convolution of per-suit holding censuses.  The census comes from the wasm
// (`point_census`, the crate's own evaluators), the shape terms are mirrored
// here: `upgrade` = [unbalanced] + [two longest ≥ 10] − wasted-honor suits,
// floored at 0; support points swap the side suits to hcp_plus and count the
// trump suit's plain HCP.  No sampling.
// Each box's "where" is evaluated by brute force on every (shape, gauge
// values) state the convolution reaches.
const TOTAL_HANDS = 635013559600;
let CENSUS = null; // [len] → rows of [honors, hcp, wasted, hcp_plus, count], from wasm
const SHAPES = []; // [♠,♥,♦,♣] lengths, all 560
let HELD = null; // [len][honors] → [hcp, wasted, hcp_plus], the census by holding
let JOINT = null; // gaugeJoint by SHAPES index
let GAUGE_SLACK = null; // by gauge, the most it exceeds the hcp; never less
for (let s = 0; s <= 13; s++) for (let h = 0; s + h <= 13; h++) for (let d = 0; s + h + d <= 13; d++) {
  SHAPES.push([s, h, d, 13 - s - h - d]);
}
const twoLongest = (lens) => { const [a, b] = [...lens].sort((x, y) => y - x); return a + b; };
const isBalanced = (lens) => lens.every((l) => l >= 2) && lens.filter((l) => l === 2).length <= 1;
// the shape terms of the gauges: every sp's bonus, and what `up` adds before wasted suits
const longBonus = (lens) => (twoLongest(lens) >= 10 ? 1 : 0);
const upgradeBase = (lens) => (isBalanced(lens) ? 0 : 1) + longBonus(lens);
// Pavlicek's freakness: a point per card over four or under three in each
// suit, plus 1 for a singleton or 2 for a void — 4333 = 0, 4432 = 1, 5332 = 2.
const freakness = (lens) => lens.reduce((f, l) => f + Math.max(0, l - 4, 3 - l), 0) + (lens.includes(0) ? 2 : lens.includes(1) ? 1 : 0);

// Hands of this shape counted jointly by every gauge: a list of [values in
// GAUGES order, each suit's hcp, boxes admitting, count].  The convolution
// state packs six sums in base 64 — HCP, wasted suits, and per trump the
// support sum (hcp_plus on the side suits, plain HCP in trumps).  hcp_plus is
// nearly HCP plus a function of length, so the joint stays small: 45k states
// over all 560 shapes.
// Beside the sums the state carries what the boxes read off single suits:
// which of `boxes` every suit's cell admits (a bit per box), and the hcp of
// each suit in `suits` (base 11 above the bits; the others report 0).
function gaugeJoint(lens, boxes = [ANY_BOX], suits = []) {
  const SIG = 2 ** boxes.length;
  let acc = new Map([[SIG - 1, new Map([[0, 1]])]]); // bits and suit hcps → packed sums → count
  lens.forEach((len, suit) => {
    const step = new Map();
    for (const [honors, hcp, wasted, plus, n] of CENSUS[len]) {
      let bits = 0;
      boxes.forEach((box, i) => { if (admits(box, suit, honors, len)) bits |= 1 << i; });
      if (!bits) continue;
      let code = hcp + 64 * wasted;
      for (let t = 0; t < 4; t++) code += 64 ** (2 + t) * (t === suit ? hcp : plus);
      const side = bits + (suits.includes(suit) ? SIG * 11 ** suit * hcp : 0);
      if (!step.has(side)) step.set(side, new Map());
      step.get(side).set(code, (step.get(side).get(code) || 0) + n);
    }
    const next = new Map();
    for (const [a, sums] of acc) for (const [b, more] of step) {
      const bits = (a % SIG) & (b % SIG);
      if (!bits) continue;
      const side = a - (a % SIG) + b - (b % SIG) + bits;
      if (!next.has(side)) next.set(side, new Map());
      const dst = next.get(side);
      for (const [x, m] of sums) for (const [y, n] of more) dst.set(x + y, (dst.get(x + y) || 0) + m * n);
    }
    acc = next;
  });
  const bonus = longBonus(lens), base = upgradeBase(lens);
  return [...acc].flatMap(([side, sums]) => {
    const each = [0, 1, 2, 3].map((suit) => Math.floor(side / SIG / 11 ** suit) % 11);
    return [...sums].map(([code, n]) => {
      const sum = (k) => Math.floor(code / 64 ** k) % 64;
      const support = [2, 3, 4, 5].map((k) => sum(k) + bonus);
      return [[sum(0), sum(0) + Math.max(0, base - sum(1)), ...support], each, side % SIG, n];
    });
  });
}

// Hands in the union of the boxes: {total, byShape: [[lens, count], …]}.
// ponytail: a box that names honors or reads a suit's hcp re-convolves each
// shape it fits, and every suit hcp read multiplies the states by up to 11 —
// instant for one, seconds for all four over every shape.  Upgrade path: a
// Compute button, as Partner has.
function oddsCount(boxes) {
  const suits = [0, 1, 2, 3].filter((suit) => boxes.some((box) => box.where.used.has(SLOT.suitHcp + suit)));
  const byShape = [];
  let total = 0;
  SHAPES.forEach((lens, k) => {
    const mine = boxes.filter((box) => inLens(box, lens));
    if (!mine.length) return;
    const named = suits.length > 0 || mine.some((box) => box.cells.some(Boolean));
    const v = [...lens];
    v[SLOT.freak] = freakness(lens);
    let count = 0;
    for (const [values, each, bits, n] of named ? gaugeJoint(lens, mine, suits) : JOINT[k]) {
      for (let g = 0; g < 6; g++) v[SLOT.gauge + g] = values[g];
      for (let suit = 0; suit < 4; suit++) v[SLOT.suitHcp + suit] = each[suit];
      if (mine.some((box, i) => (!named || (bits >> i) & 1) && meets(box, v))) count += n;
    }
    if (count) { total += count; byShape.push([lens, count]); }
  });
  return { total, byShape };
}

function initCalc() {
  const vars = handVars(0, 1); // p = up
  const chk = (text, ...v) => compileWhere(text, vars)(v);
  console.assert(chk('s > h', 5, 4, 2, 2) && !chk('s > h >= d', 5, 2, 4, 2)
    && chk('p + max(s, h) >= 20, !(c > 0 | hcp < 12)', 5, 4, 4, 0, 13, 15)
    && chk('♠ + ♠.hcp = 9 & s.hcp > sp♥', 5, 4, 4, 0, 13, 15, 0, 3, 0, 0, 4)
    && [...compileWhere('sps > my.♦.hcp', { ...vars, ...handVars(HAND_SLOTS, 0, 'my.') }).used].join() === '6,27'
    && chk('5431', 5, 4, 3, 1) && chk('(54)(31)', 4, 5, 1, 3) && !chk('(54)(31)', 5, 3, 4, 1) && chk('5-5-x-x', 0, 5, 3, 5)
    && chk('7=6=x=x', 7, 6, 0, 0) && chk('!(4432) & 44xx', 4, 4, 4, 1) && [...compileWhere('54xx', vars).used].join() === '0,1'
    && compileWhere('(54)xx', vars).alike.join() === '0,1' && compileWhere('s > 4 | hcp > 11', vars)([4, , , , NaN]) === undefined
    && compileWhere('s > 4, hcp > 11', vars)([4, , , , NaN]) === false
    && ['s >', 's + h', '!s', 'pts > 0', 'hcp_s > 0', 'my.s > 0', 's .hcp > 0', '5432', '5=4-3-1', 'hcp > 5431'].every((text) => compileWhere(text, vars) === null), 'compileWhere');
  CENSUS = JSON.parse(point_census());
  HELD = CENSUS.map((rows) => {
    const byHonors = [];
    for (const [honors, hcp, wasted, plus] of rows) byHonors[honors] = [hcp, wasted, plus];
    return byHonors;
  });
  JOINT = SHAPES.map((lens) => gaugeJoint(lens));
  GAUGE_SLACK = GAUGES.map(() => 0);
  let under = false;
  for (const joint of JOINT) for (const [values] of joint) values.forEach((value, g) => {
    under ||= value < values[0];
    GAUGE_SLACK[g] = Math.max(GAUGE_SLACK[g], value - values[0]);
  });
  console.assert(!under, 'a gauge under the hcp');
  // the ♠K is in C(51,12) hands, the ♠A alone in a suit (4 hcp there) in C(39,12)
  const box = (...texts) => makeBox(parseCells(texts, vars), vars.p);
  console.assert(oddsCount([box('.*K.*', '', '', '', '', '')]).total === choose(51, 12)
    && oddsCount([box('1', '', '', '', '', 's.hcp = 4')]).total === choose(39, 12)
    && oddsCount([box('', '', '', '', '', '(5431)')]).total === 24 * oddsCount([box('5', '4', '3', '1', '', '')]).total, 'oddsCount');
  initBoxes('c-boxes', renderCalc);
  renderCalc();
}

function renderCalc() {
  const boxes = readBoxes('c-boxes', handVars(0, gaugeOf('c-boxes')));
  const out = id('c-out');
  if (!boxes.length) { out.innerHTML = '<p class="hint">Enter at least one well-formed box.</p>'; return; }
  if (boxes.length > MAX_BOXES) { out.innerHTML = `<p class="hint">At most ${MAX_BOXES} boxes.</p>`; return; }
  const { total, byShape } = oddsCount(boxes);
  const prob = total / TOTAL_HANDS;
  byShape.sort((a, b) => b[1] - a[1]);
  const top = byShape.slice(0, 8).map(([lens, n]) =>
    `<tr><td>${lens.join('=')}</td><td>${(100 * n / TOTAL_HANDS).toFixed(3)}%</td><td>${(100 * n / total).toFixed(1)}%</td></tr>`).join('');
  out.innerHTML = `
    <div class="statrow">
      <div><span class="statlabel">Probability</span><span class="statbig">${(100 * prob).toFixed(4)}%</span></div>
      <div><span class="statlabel">Odds</span><span class="statbig">${total ? '1 in ' + (1 / prob).toLocaleString(undefined, { maximumFractionDigits: 1 }) : '—'}</span></div>
      <div><span class="statlabel">Hands</span><span class="statbig">${total.toLocaleString()}</span></div>
    </div>
    ${byShape.length ? `<table class="ddtable"><thead><tr><th>Shape ♠=♥=♦=♣</th><th>Of all hands</th><th>Of the union</th></tr></thead><tbody>${top}</tbody></table>` : ''}
    ${byShape.length > 8 ? `<p class="hint">…and ${byShape.length - 8} more shapes.</p>` : ''}`;
}

// --- Companion tab: partner's hand given yours ---------------------------------
// After Pavlicek's Companion Hand Calculator, but each hand is a union of
// boxes.  Exact: per suit the two hands draw disjoint holdings — the honors
// A K Q J T by identity, the eight spots by count — so each suit gives a joint
// census of (your hcp, partner's hcp) per (your length, partner's length);
// four suits convolve into a 2-D table per shape pair, and the boxes read
// it.  A holding cell reads the honors, which the table does not keep: the
// convolution state also carries which boxes every suit so far admits (a bit
// per box), so a fully named hand costs one shape and one holding a suit.
// The other gauges ride beside a hand's hcp as a few small sums (`gaugeSums`),
// widening its axis of the table.
// Suits no box tells apart permute freely, so one pair of shapes is counted
// for its whole orbit: 24 to one when the boxes are points only.
// ponytail: O(your shapes × partner shapes × 2-D convolution) — a named
// hand or a shaped box is instant, "16+ any shape" opposite any hand (560 ×
// 560 pairs, 24 to an orbit) takes half a second, and seconds once the boxes
// name every suit.  Upgrade paths: when no box reads your lengths, carry your
// running length in the state instead of enumerating your shape; count a
// symmetry of the union ("either major"), not only of each box.
// ponytail: the tables are dense, and every gauge a hand's boxes read widens
// its axis up to fourfold — past MAX_CELLS the count is refused (about six
// gauges between the two hands).  Upgrade path: sparse tables.
const COMPANION_HANDS = 8122425444; // 39 choose 13
const MAX_CELLS = 2 ** 23;

// One value vector serves both hands' boxes: partner's slots, then yours,
// which a partner box reads under `my.`.
const MINE = HAND_SLOTS;
// the slots the boxes read: every where's, and the gauge of a bounded points range
const slotsRead = (boxes) => new Set(boxes.flatMap((box) =>
  [...box.where.used, ...(box.pts[0] > 0 || box.pts[1] < 60 ? [box.p] : [])]));

// What the boxes make the hand at `base` carry beside its hcp: one sum over
// the suits per gauge read — the wasted suits behind `up`, and behind each sp
// what the side suits add to their hcp (hcp_plus − hcp).  A suit's term moves
// only when the suit is short, so each is counted from its least over the
// holdings of that length (`lo`), by at most `span`; the sums pack into one
// index, mixed radix by `place`.
function gaugeSums(base, read) {
  const terms = [];
  const up = base + SLOT.gauge + 1;
  if (read.has(up)) terms.push({ slot: up, of: (suit, [, wasted]) => wasted });
  for (let trump = 0; trump < 4; trump++) {
    const slot = up + 1 + trump;
    if (read.has(slot)) terms.push({ slot, support: true, of: (suit, [hcp, , plus]) => (suit === trump ? 0 : plus - hcp) });
  }
  let size = 1;
  for (const term of terms) {
    const ranges = [0, 1, 2, 3].map((suit) => HELD.map((byHonors) => {
      const values = byHonors.filter(Boolean).map((held) => term.of(suit, held));
      return [Math.min(...values), Math.max(...values)];
    }));
    term.lo = ranges.map((byLen) => byLen.map(([lo]) => lo));
    term.span = ranges.map((byLen) => byLen.map(([lo, hi]) => hi - lo));
    term.place = size;
    term.radix = 1 + Math.max(...SHAPES.map((lens) => lens.reduce((sum, len, suit) => sum + term.span[suit][len], 0)));
    size *= term.radix;
  }
  // how far one suit can move the index
  const reach = !terms.length ? () => 0
    : (suit, len) => terms.reduce((sum, term) => sum + term.place * term.span[suit][len], 0);
  return {
    size, reach,
    // a holding's step along the index
    offset: !terms.length ? () => 0 : (suit, len, honors) => terms.reduce((sum, term) =>
      sum + term.place * (term.of(suit, HELD[len][honors]) - term.lo[suit][len]), 0),
    // a shape's constants: the index's `top`, and what decoding it needs
    shape: (lens) => ({
      lens, freak: freakness(lens), bonus: longBonus(lens), upgrade: upgradeBase(lens),
      top: lens.reduce((sum, len, suit) => sum + reach(suit, len), 0),
      least: terms.map((term) => lens.reduce((sum, len, suit) => sum + term.lo[suit][len], 0)),
    }),
    // write the hand of this shape, hcp and index into the value vector
    set(v, shape, hcp, index) {
      for (let suit = 0; suit < 4; suit++) v[base + suit] = shape.lens[suit];
      v[base + SLOT.gauge] = hcp;
      v[base + SLOT.freak] = shape.freak;
      for (let k = 0; k < terms.length; k++) {
        const term = terms[k], sum = Math.floor(index / term.place) % term.radix + shape.least[k];
        v[term.slot] = hcp + (term.support ? sum + shape.bonus : Math.max(0, shape.upgrade - sum));
      }
    },
  };
}

// Count (your hand, partner's hand) pairs: `mine` boxes your hand, `known`
// conditions partner, `query` asks.
// Returns {mine, known, query, byShape}: hands of yours in the union, pairs
// meeting known, pairs meeting both, and per partner shape [known, query] —
// or null when the tables would pass MAX_CELLS.
function companionCount(mine, known, query) {
  const partner = [...known, ...query];
  // can the box of the hand at `base` take this shape: its cells, and what
  // its where says of that hand's lengths and freakness alone
  const shaped = (box, lens, base) => {
    if (!inLens(box, lens)) return false;
    const v = new Array(2 * HAND_SLOTS).fill(NaN);
    lens.forEach((len, suit) => { v[base + suit] = len; });
    v[base + SLOT.freak] = freakness(lens);
    return box.where(v) !== false;
  };
  const myLens = SHAPES.filter((lens) => mine.some((b) => shaped(b, lens, MINE)));
  const partnerLens = SHAPES.filter((lens) => known.some((b) => shaped(b, lens, 0)));
  const read = slotsRead([...mine, ...partner]);
  const mySums = gaugeSums(MINE, read), partnerSums = gaugeSums(0, read);
  const myShapes = myLens.map(mySums.shape), partnerShapes = partnerLens.map(partnerSums.shape);
  const W = 38;
  const top = (shapes) => Math.max(0, ...shapes.map((shape) => shape.top));
  if ((top(myShapes) + 1) * (top(partnerShapes) + 1) * W * W > MAX_CELLS) return null;
  // an hcp axis no gauge reads collapses to 0; a read one is pruned to its
  // range, every gauge being the hcp and at most GAUGE_SLACK more
  const gauged = (base) => GAUGES.some((_, g) => read.has(base + SLOT.gauge + g));
  const least = (box) => Math.max(0, box.pts[0] - GAUGE_SLACK[box.p % HAND_SLOTS - SLOT.gauge]);
  const myRead = gauged(MINE);
  const myMax = myRead ? Math.min(37, Math.max(...mine.map((b) => b.pts[1]))) : 0;
  const myMin = Math.min(...mine.map(least));
  const partnerRead = gauged(0);
  const knownMax = partnerRead ? Math.min(37, Math.max(...known.map((b) => b.pts[1]))) : 0;
  const knownMin = Math.min(...known.map(least));
  // a per-suit hcp some box reads is enumerated outside the convolution:
  // that suit's step keeps only the holdings of that value
  const enumerated = [0, MINE].flatMap((base) => [0, 1, 2, 3]
    .filter((suit) => read.has(base + SLOT.suitHcp + suit)).map((suit) => ({ mine: base === MINE, suit })));
  const values = enumerated.map(() => 0);
  // the boxes still admitted are a bit per box, packed as (yours) * SIG + (partner's)
  const SIG = 2 ** partner.length, knownBits = 2 ** known.length - 1;
  const pool = new Map(); // spare tables, by size
  const spare = (size) => { if (!pool.has(size)) pool.set(size, []); return pool.get(size); };
  const take = (free, size) => (free.pop() || new Float64Array(size)).fill(0);
  const out = { mine: 0, known: 0, query: 0, byShape: new Map() };
  // The suits no box tells apart: every cell blank, and no where reading the
  // suit's length, hcp or sp.  Permuting them in both hands at once changes no
  // count, so a pair of shapes stands for its orbit: the pair whose
  // (partner's length, yours) never rises along these suits.  A distribution
  // reads a run of suits in any order — (54)xx, (5431) — so it only splits
  // them into those inside the run and those outside.
  let alike = [[0, 1, 2, 3].filter((suit) =>
    [...mine, ...partner].every((box) => box.lens[suit][0] <= 0 && box.lens[suit][1] >= 13 && !box.cells[suit])
    && [0, MINE].every((base) => [SLOT.len, SLOT.suitHcp, SLOT.gauge + 2].every((slot) => !read.has(base + slot + suit))))];
  for (const box of [...mine, ...partner]) for (const slots of box.where.alike) {
    const run = slots.map((slot) => slot % HAND_SLOTS);
    alike = alike.flatMap((suits) => [suits.filter((suit) => run.includes(suit)), suits.filter((suit) => !run.includes(suit))]);
  }
  // the size of the pair's orbit, 0 off its representative
  const orbit = (p1, p2) => {
    let n = 1;
    for (const suits of alike) {
      for (let i = 1, run = 1; i < suits.length; i++) {
        const a = suits[i - 1], b = suits[i], drop = (p2[a] - p2[b]) * 14 + p1[a] - p1[b];
        if (drop < 0) return 0;
        run = drop ? 1 : run + 1;
        n = n * (i + 1) / run;
      }
    }
    return n;
  };
  const perms = (xs) => (xs.length < 2 ? [xs] : xs.flatMap((x, i) => perms(xs.filter((_, j) => j !== i)).map((rest) => [x, ...rest])));
  // the shapes a representative's partner shape stands for
  const images = (lens) => [...new Set(alike.reduce((all, suits) => all.flatMap((from) => perms(suits).map((to) => {
    const image = [...from];
    suits.forEach((suit, i) => { image[to[i]] = from[suit]; });
    return image;
  })), [lens]).map((image) => image.join('=')))];
  // The suit step for (your length, partner's) under the enumerated values:
  // [your boxes admitting, partner's, flat [h1, i1, h2, i2, n, …]] per distinct
  // pair of box sets, over every disjoint pair of holdings — each hand's hcp
  // and step along its gauge-sum index.  `solo` is your hand alone: partner
  // holds nothing of the suit and his boxes are not asked.
  const stepCache = new Map(); // per enumeration
  const stepFor = (suit, l1, l2, solo) => {
    const cacheKey = ((suit * 14 + l1) * 14 + l2) * 2 + (solo ? 1 : 0);
    const hit = stepCache.get(cacheKey);
    if (hit) return hit;
    const groups = new Map();
    for (let a = 0; a < 32; a++) {
      const spots = l1 - popcount(a);
      if (spots < 0 || spots > 8) continue;
      let sm = 0;
      mine.forEach((box, i) => { if (admits(box, suit, a, l1)) sm |= 1 << i; });
      if (!sm) continue;
      const h1 = HELD[l1][a][0], i1 = mySums.offset(suit, l1, a);
      for (let b = 0; b < 32; b++) {
        const n = a & b ? 0 : choose(8, spots) * choose(8 - spots, l2 - popcount(b));
        if (!n) continue;
        let sp = SIG - 1;
        if (!solo) {
          sp = 0;
          partner.forEach((box, i) => { if (admits(box, suit, b, l2)) sp |= 1 << i; });
          if (!(sp & knownBits)) continue;
        }
        const h2 = HELD[l2][b][0], i2 = solo ? 0 : partnerSums.offset(suit, l2, b);
        if (enumerated.some((e, i) => e.suit === suit && (e.mine ? h1 : h2) !== values[i])) continue;
        const sig = sm * SIG + sp;
        const key = (((myRead ? h1 : 0) * mySums.size + i1) * 16 + (partnerRead ? h2 : 0)) * partnerSums.size + i2;
        if (!groups.has(sig)) groups.set(sig, new Map());
        groups.get(sig).set(key, (groups.get(sig).get(key) || 0) + n);
      }
    }
    const step = [...groups].map(([sig, rows]) => [Math.floor(sig / SIG), sig % SIG, [...rows].flatMap(([key, n]) => {
      const mineKey = Math.floor(key / partnerSums.size / 16);
      return [Math.floor(mineKey / mySums.size), mineKey % mySums.size, Math.floor(key / partnerSums.size) % 16, key % partnerSums.size, n];
    })]);
    stepCache.set(cacheKey, step);
    return step;
  };
  // Four suits convolved, for your shape and partner's (none: your hand
  // alone): box sets → table over (your index, your hcp) × (partner's index,
  // partner's hcp), a cell at ((i1 * W + h1) * N2 + i2 * W + h2).  The caller
  // hands the tables back to `free`.
  const convolve = (s1, s2, free) => {
    const solo = !s2, p1 = s1.lens, p2 = s2?.lens;
    const kMin = solo ? 0 : knownMin;
    const N2 = solo ? W : (s2.top + 1) * W;
    const size = (s1.top + 1) * W * N2;
    let acc = new Map([[(2 ** mine.length - 1) * SIG + SIG - 1, take(free, size)]]);
    acc.values().next().value[0] = 1;
    let hi1 = 0, hi2 = 0, top1 = 0, top2 = 0;
    for (let suit = 0; suit < 4; suit++) {
      const step = stepFor(suit, p1[suit], solo ? 0 : p2[suit], solo), rem = 10 * (3 - suit);
      const next = new Map();
      for (const [sig, src] of acc) {
        for (const [sm, sp, rows] of step) {
          const m = Math.floor(sig / SIG) & sm, p = (sig % SIG) & sp;
          if (!m || !(solo || p & knownBits)) continue;
          let dst = next.get(m * SIG + p);
          if (!dst) next.set(m * SIG + p, dst = take(free, size));
          for (let k = 0; k < rows.length; k += 5) {
            const h1 = rows[k], h2 = rows[k + 2], n = rows[k + 4];
            const shift = (rows[k + 1] * W + h1) * N2 + rows[k + 3] * W + h2;
            const aLo = Math.max(0, myMin - rem - h1), aHi = Math.min(hi1, myMax - h1);
            const bLo = Math.max(0, kMin - rem - h2), bHi = Math.min(hi2, knownMax - h2);
            for (let i1 = 0; i1 <= top1; i1++) {
              for (let i2 = 0; i2 <= top2; i2++) {
                const corner = i1 * W * N2 + i2 * W;
                for (let a = aLo; a <= aHi; a++) {
                  const from = corner + a * N2, to = from + shift;
                  for (let b = bLo; b <= bHi; b++) dst[to + b] += n * src[from + b];
                }
              }
            }
          }
        }
        free.push(src);
      }
      acc = next;
      hi1 = Math.min(myMax, hi1 + 10); hi2 = Math.min(knownMax, hi2 + 10);
      top1 += mySums.reach(suit, p1[suit]);
      if (!solo) top2 += partnerSums.reach(suit, p2[suit]);
    }
    return acc;
  };
  do {
    stepCache.clear();
    const v = new Array(2 * HAND_SLOTS).fill(0);
    enumerated.forEach((e, i) => { v[(e.mine ? MINE : 0) + SLOT.suitHcp + e.suit] = values[i]; });
    // is this hand of yours in one of your boxes that every suit admitted
    const myFit = (m, shape, a, i1) => { mySums.set(v, shape, a, i1); return mine.some((box, i) => (m >> i) & 1 && meets(box, v)); };
    for (const s1 of myShapes) {
      const p1 = s1.lens, alone = spare((s1.top + 1) * W * W);
      for (const [sig, table] of convolve(s1, null, alone)) {
        for (let i1 = 0; i1 <= s1.top; i1++) {
          for (let a = 0; a <= myMax; a++) {
            const x = table[(i1 * W + a) * W];
            if (x && myFit(Math.floor(sig / SIG), s1, a, i1)) out.mine += x;
          }
        }
        alone.push(table);
      }
      for (const s2 of partnerShapes) {
        const p2 = s2.lens;
        const weight = orbit(p1, p2);
        if (!weight || p1.some((l, i) => l + p2[i] > 13)) continue;
        const N2 = (s2.top + 1) * W, free = spare((s1.top + 1) * W * N2);
        let kn = 0, qu = 0;
        for (const [sig, table] of convolve(s1, s2, free)) {
          const m = Math.floor(sig / SIG), p = sig % SIG;
          for (let i1 = 0; i1 <= s1.top; i1++) {
            for (let a = 0; a <= myMax; a++) {
              if (!myFit(m, s1, a, i1)) continue;
              for (let i2 = 0; i2 <= s2.top; i2++) {
                const row = (i1 * W + a) * N2 + i2 * W;
                for (let b = 0; b <= knownMax; b++) {
                  const x = table[row + b];
                  if (!x) continue;
                  partnerSums.set(v, s2, b, i2);
                  if (!known.some((box, i) => (p >> i) & 1 && meets(box, v))) continue;
                  kn += x;
                  if (query.some((box, i) => (p >> (known.length + i)) & 1 && meets(box, v))) qu += x;
                }
              }
            }
          }
          free.push(table);
        }
        if (!kn) continue;
        out.known += weight * kn; out.query += weight * qu;
        const keys = s2.images ||= images(p2), share = weight / keys.length;
        for (const key of keys) {
          const row = out.byShape.get(key) || [0, 0];
          out.byShape.set(key, [row[0] + share * kn, row[1] + share * qu]);
        }
      }
    }
    // next combination of enumerated values, base 11
    let i = 0;
    while (i < values.length && ++values[i] > 10) values[i++] = 0;
    if (i === values.length) break;
  } while (true);
  return out;
}

function initCompanion() {
  // a named hand: 37 hcp leaves partner three jacks, all of them C(36,10) ways;
  // a shape: partner's spades hypergeometric; a holding: the ♠K is in C(25,12)
  // hands of the black suits; a gauge: your hands as Odds counts them; an
  // orbit: the minors alike, partner's thirteen clubs counted off his diamonds
  const vars = handVars(MINE, 1); // p = up
  const box = (...texts) => makeBox(parseCells([...texts, '', ''].slice(0, 6), vars), MINE + SLOT.gauge);
  const all = companionCount([box('AKQ', 'AKQ', 'AKQ', 'AKQJ')], [ANY_BOX], [{ ...ANY_BOX, pts: [3, 3] }]);
  const shaped = companionCount([box('5', '5', '3', '0')], [ANY_BOX], [box('4+', '', '', '')]);
  const king = companionCount([box('.*K.*', '0', '0', '')], [box('13', '', '', '')], []);
  const alike = companionCount([box('7', '6', '', '')], [ANY_BOX], []);
  const gauged = ['5', '4', '', '0-1', '', 'p + s >= 18 & sph > up'];
  const mine = companionCount([makeBox(parseCells(gauged, vars), vars.p)], [box('', '', '', '13')], []).mine;
  const odds = oddsCount([makeBox(parseCells(gauged, handVars(0, 1)), SLOT.gauge + 1)]).total;
  let fit = 0;
  for (let k = 4; k <= 8; k++) fit += choose(8, k) * choose(31, 13 - k);
  const near = (x, y) => Math.abs(x - y) <= 1e-9 * Math.abs(y);
  console.assert(near(all.known, all.mine * COMPANION_HANDS) && near(all.query, all.mine * choose(36, 10))
    && near(shaped.mine, choose(13, 5) ** 2 * choose(13, 3)) && near(shaped.known, shaped.mine * COMPANION_HANDS)
    && near(shaped.query / shaped.known, fit / COMPANION_HANDS)
    && near(king.mine, choose(25, 12)) && mine > 0 && mine === odds
    && near(alike.known, alike.mine * COMPANION_HANDS) && near(alike.byShape.get('0=0=0=13')[0], alike.mine)
    && [[4, 3, 3, 3], [4, 4, 3, 2], [5, 3, 3, 2], [4, 4, 4, 1], [5, 4, 2, 2], [5, 4, 3, 1], [7, 2, 2, 2], [6, 4, 3, 0]].map(freakness).join() === '0,1,2,3,3,4,6,7'
    && splitHand('AKT52.K83.-.76432').join() === 'AKTxx,Kxx,0,xxxxx' && splitHand('AK5+') === null
    && compileHolding('AKx*').range.join() === '2,10' && compileHolding('Q4') === null && compileHolding('KA') === null, 'companion');
  for (const table of ['x-mine', 'x-known', 'x-query']) initBoxes(table, validateCompanion);
  id('partner').onkeydown = (ev) => { if (ev.key === 'Enter' && ev.target.tagName === 'INPUT') renderCompanion(); };
  id('x-run').onclick = renderCompanion;
}

// A partner box reads its own hand, and yours under `my.`; each table's `p`
// is the gauge its own Points column is on.
function readCompanion() {
  const mine = readBoxes('x-mine', handVars(MINE, gaugeOf('x-mine')));
  const [known, query] = ['x-known', 'x-query'].map((table) =>
    readBoxes(table, { ...handVars(0, gaugeOf(table)), ...handVars(MINE, gaugeOf('x-mine'), 'my.') }));
  return { mine, known: known.length ? known : [ANY_BOX], query };
}
function validateCompanion() { readCompanion(); }

function renderCompanion() {
  const { mine, known, query } = readCompanion();
  const out = id('x-out');
  if (!mine.length) { out.innerHTML = '<p class="hint">Enter at least one well-formed box for your hand.</p>'; return; }
  if (mine.length > MAX_BOXES || known.length + query.length > MAX_BOXES) { out.innerHTML = `<p class="hint">At most ${MAX_BOXES} boxes for your hand, and ${MAX_BOXES} for partner's.</p>`; return; }
  const t0 = performance.now();
  const n = companionCount(mine, known, query);
  const ms = performance.now() - t0;
  if (!n) { out.innerHTML = '<p class="hint">These boxes read too many point gauges at once to count exactly — ask for fewer, or narrow the shapes.</p>'; return; }
  const pct = (x, y) => (y ? `${(100 * x / y).toFixed(2)}%` : '—');
  const rows = [...n.byShape].sort((a, b) => b[1][0] - a[1][0]);
  const top = rows.slice(0, 8).map(([shape, [kn, qu]]) =>
    `<tr><td>${shape}</td><td>${pct(kn, n.known)}</td><td>${pct(qu, kn)}</td></tr>`).join('');
  out.innerHTML = `
    <div class="statrow">
      ${query.length ? `<div><span class="statlabel">Query, given known</span><span class="statbig">${pct(n.query, n.known)}</span></div>` : ''}
      <div><span class="statlabel">Known, given your hand</span><span class="statbig">${pct(n.known, n.mine * COMPANION_HANDS)}</span></div>
      <div><span class="statlabel">Partner hands counted</span><span class="statbig">${(n.known / n.mine).toLocaleString(undefined, { maximumFractionDigits: 0 })}</span></div>
    </div>
    ${rows.length ? `<table class="ddtable"><thead><tr><th>Partner ♠=♥=♦=♣</th><th>Of known</th><th>Query within</th></tr></thead><tbody>${top}</tbody></table>` : ''}
    ${rows.length > 8 ? `<p class="hint">…and ${rows.length - 8} more shapes.</p>` : ''}
    <p class="hint">${ms.toFixed(0)} ms</p>`;
}
