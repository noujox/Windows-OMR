// Self-contained browser extension tests. Run with: node satellite/tests/extension.test.cjs
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const base = path.resolve(__dirname, '../extension');

function detector(route, visible = true, label = 'Salir de la llamada', retry = false) {
  const sent = [];
  let probe;
  let receive;
  const button = {
    disabled: false,
    textContent: '',
    getClientRects: () => visible ? [1] : [],
    getAttribute: name => name === 'aria-label' ? label : null,
  };
  const context = {
    location: { hostname: 'meet.google.com', pathname: route },
    crypto: { randomUUID: () => '11223344-5566-7788-99aa-bbccddeeff00' },
    document: {
      documentElement: {},
      querySelectorAll: () => [button],
      querySelector: () => null,
    },
    MutationObserver: class { observe() {} },
    setInterval: fn => { probe = fn; },
    setTimeout: () => {},
    addEventListener: () => {},
    chrome: { runtime: {
      sendMessage: data => { sent.push(data); return Promise.resolve(); },
      onMessage: {addListener: fn => {receive = fn;}},
    } },
    console,
    Date,
  };
  vm.runInNewContext(fs.readFileSync(path.join(base,'detector.js'), 'utf8'), context);
  probe();
  probe();
  if (retry) {
    let response;
    receive({type:'satellite-retry'}, {}, reply => {response=reply;});
    assert.equal(response?.active, visible && label === 'Salir de la llamada');
  }
  return sent;
}

assert.equal(detector('/wpe-ofqg-hij').length, 1, 'legacy Meet path');
assert.equal(detector('/meet/wpe-ofqg-hij').length, 1, 'current Meet path');
assert.equal(detector('/meet/wpe-ofqg-hij/', true).length, 1, 'nested Meet path with slash');
assert.equal(detector('/meet/').length, 0, 'Meet lobby must not trigger recording');
assert.equal(detector('/meet/wpe-ofqg-hij', false).length, 0, 'URL alone is insufficient');
assert.equal(detector('/meet/wpe-ofqg-hij', true, 'Join now').length, 0, 'only active leave control qualifies');
assert.equal(detector('/meet/wpe-ofqg-hij', true, 'Salir de la llamada', true).length, 2,
  'explicit retry on a live call sends a fresh join');
assert.equal(detector('/meet/wpe-ofqg-hij', false, 'Salir de la llamada', true).length, 0,
  'retry must not claim a page without an active call');

async function background() {
  let listener;
  const native = [];
  const context = {
    URL,
    console,
    chrome: {
      runtime: {
        onMessage: { addListener: fn => { listener = fn; } },
        sendNativeMessage: async (host, data) => { native.push({host, data}); return {connected:true}; },
      },
      storage: {
        local: { get: async () => ({enabled:true}) },
        session: { set: async () => {} },
        onChanged: { addListener: () => {} },
      },
      tabs: { onRemoved: { addListener: () => {} } },
    },
  };
  vm.runInNewContext(fs.readFileSync(path.join(base,'background.js'), 'utf8'), context);
  const send = (event, session) => new Promise((resolve,reject) => {
    const result = listener({type:'satellite',event,session},
      {tab:{id:7},url:'https://meet.google.com/meet/wpe-ofqg-hij'},resolve);
    assert.equal(result,true,'onMessage must return literal true (not Promise)');
    setTimeout(() => reject(new Error('no async response')), 1000).unref();
  });
  const session='meet.11223344556677889900';
  assert.equal((await send('join',session)).ok,true);
  assert.equal((await send('beat',session)).ok,true);
  assert.deepEqual(native.map(x=>x.data.event),['join','beat']);
}

background().then(() => console.log('Satellite extension tests passed (Meet routes, safety, Edge async response).'))
  .catch(error => { console.error(error);process.exitCode=1; });
