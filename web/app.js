// Thin static UI over the pons wasm bidder: the engine holds the deal and the
// auction; JS rebuilds the DOM from each JSON snapshot (gin-rummy pattern).
import init, { WebTable, book } from './pkg/pons_web.js';

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
  buildBiddingBox();
  for (const b of document.querySelectorAll('nav button')) {
    b.onclick = () => { location.hash = b.dataset.tab; };
  }
  window.addEventListener('hashchange', () => showTab(location.hash.slice(1)));
  id('p-deal').onclick = dealPractice;
  id('d-deal').onclick = dealDemo;
  id('b-filter').oninput = filterBook;
  showTab(location.hash.slice(1));
}

function showTab(tab) {
  if (!['practice', 'demo', 'book'].includes(tab)) tab = 'practice';
  for (const sec of document.querySelectorAll('main > section')) {
    sec.classList.toggle('hidden', sec.id !== tab);
  }
  for (const b of document.querySelectorAll('nav button')) {
    b.classList.toggle('active', b.dataset.tab === tab);
  }
  if (tab === 'book' && !bookNodes) loadBook();
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
  boardGen++;
  clearInterval(demoTimer);
  id('d-dd').classList.add('hidden');
  const s = JSON.parse(game.deal_demo(id('d-dealer').value, id('d-vul').value));
  // Deal-out feel: show the hands at once, then the auction one call at a time.
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
  id('d-info').textContent = `Dealer ${SEAT_NAMES[s.dealer]} · Vul ${s.vul}`;
  id('d-hands').innerHTML = compassHTML(s.hands);
  const auc = id('d-auction');
  auc.classList.remove('hidden');
  auc.innerHTML = auctionHTML(s, null);
  id('d-contract').innerHTML = s.contract
    ? `<span class="contract">${colorizeCalls(s.contract)}</span>` : '';
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
    (dd.verdict ? `<div class="verdict">${colorizeCalls(dd.verdict)}</div>` : '');
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
    return { el, haystack };
  });
  id('b-results').appendChild(frag);
  filterBook();
}

function filterBook() {
  if (!bookNodes) return;
  const q = id('b-filter').value.trim().toLowerCase();
  let n = 0;
  for (const { el, haystack } of bookNodes) {
    const show = !q || haystack.includes(q);
    el.classList.toggle('hidden', !show);
    if (show) n++;
  }
  id('b-count').textContent = `${n} node${n === 1 ? '' : 's'}`;
}

function fmtWeight(w) {
  return Number.isInteger(w) ? w.toFixed(1) : String(w);
}

main();
