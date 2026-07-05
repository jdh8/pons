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

let game;
let current = null; // the snapshot on screen
let boardCount = 0; // practice deals so far — drives the "Rotate" dealer
let bookNodes = null; // [{el, haystack}] built once from book()
let demoTimer = 0;

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
  const pick = id('p-dealer').value;
  const dealer = pick === 'rotate' ? SEATS[boardCount % 4] : pick;
  boardCount++;
  const hcp = Math.min(37, Math.max(0, Number(id('p-hcp').value) || 0));
  render(JSON.parse(game.deal_practice(id('p-seat').value, dealer, id('p-vul').value, hcp)));
}

function dealDemo() {
  clearInterval(demoTimer);
  const s = JSON.parse(game.deal_demo(id('d-dealer').value, id('d-vul').value));
  // Deal-out feel: show the hands at once, then the auction one call at a time.
  let shown = 0;
  const tick = () => {
    const done = shown >= s.auction.length;
    render({ ...s, auction: s.auction.slice(0, shown), contract: done ? s.contract : null });
    if (done) clearInterval(demoTimer);
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
  if (!s.ended) return;
  box.innerHTML =
    `<div class="contract-line"><span class="contract">${colorizeCalls(s.contract || '')}</span></div>` +
    compassHTML(s.hands);
  const next = document.createElement('button');
  next.className = 'primary next';
  next.textContent = 'Next board';
  next.onclick = dealPractice; // same settings; Rotate advances the dealer
  box.appendChild(next);
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
    ).join('');
    el.innerHTML =
      `<div class="node-head"><span class="badge ${node.book}">${node.book}</span>` +
      `<span class="node-auction">${colorizeCalls(node.auction)}</span></div>${rules}`;
    frag.appendChild(el);
    const haystack =
      (node.auction + ' ' + node.rules.map((r) => `${r.call} ${r.text}`).join(' ')).toLowerCase();
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
