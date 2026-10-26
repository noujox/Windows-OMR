// Self-contained browser extension tests. Run with: node satellite/tests/extension.test.cjs
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const base = path.resolve(__dirname, '../extension');

function detector(route, visible = true, label = 'Salir de la llamada', retry = false, micInitially = null, micAfter = undefined) {
  const sent = [];
  let probe;
  let receive;
  let currentMic = micInitially;
  const button = {
    disabled: false,
    textContent: '',
    getClientRects: () => visible ? [1] : [],
    getAttribute: name => name === 'aria-label' ? label : null,
  };
  const micButton = {
    disabled: false,
    textContent: '',
    getClientRects: () => visible ? [1] : [],
    getAttribute: name => name === 'aria-label' ? currentMic : null,
  };
  const context = {
    location: { hostname: 'meet.google.com', pathname: route },
    crypto: { randomUUID: () => '11223344-5566-7788-99aa-bbccddeeff00' },
    document: {
      documentElement: {},
      querySelectorAll: () => [button, ...(currentMic ? [micButton] : [])],
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
  if (micAfter !== undefined) {
    currentMic = micAfter;
    probe();
    probe();
  }
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

// Smart Mic Sync uses accessible labels, never captures audio or page globals.
const meet = '/meet/wpe-ofqg-hij';
assert.equal(detector(meet, true, 'Salir de la llamada', false, 'Activar micrófono')[0].mic, 'muted');
assert.equal(detector(meet, true, 'Salir de la llamada', false, 'Desactivar micrófono')[0].mic, 'live');
assert.equal(detector(meet, true, 'Salir de la llamada', false, 'Turn on microphone')[0].mic, 'muted');
assert.equal(detector(meet, true, 'Salir de la llamada', false, 'Turn off microphone')[0].mic, 'live');
assert.equal(detector(meet)[0].mic, 'unknown', 'ambiguous browser UI fails closed');
const down = detector(meet, true, 'Salir de la llamada', false, 'Mute microphone', 'Unmute microphone');
assert.deepEqual(down.map(e => [e.event,e.mic]), [['join','live'],['beat','muted']],
  'muting immediately changes the local mic status');
const up = detector(meet, true, 'Salir de la llamada', false, 'Unmute microphone', 'Mute microphone');
assert.deepEqual(up.map(e => [e.event,e.mic]), [['join','muted'],['beat','unknown'],['beat','live']],
  'unmuting requires two successive positive observations');
const lost = detector(meet, true, 'Salir de la llamada', false, 'Mute microphone', null);
assert.deepEqual(lost.map(e => [e.event,e.mic]), [['join','live'],['beat','unknown']],
  'lost button must silence the mic when Smart Mic Sync is enabled');

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
  const send = (event, session, mic) => new Promise((resolve,reject) => {
    const result = listener({type:'satellite',event,session,mic},
      {tab:{id:7},url:'https://meet.google.com/meet/wpe-ofqg-hij'},resolve);
    assert.equal(result,true,'onMessage must return literal true (not Promise)');
    setTimeout(() => reject(new Error('no async response')), 1000).unref();
  });
  const session='meet.11223344556677889900';
  assert.equal((await send('join',session,'muted')).ok,true);
  assert.equal((await send('beat',session,'live')).ok,true);
  assert.deepEqual(native.map(x=>x.data.event),['join','beat']);
  assert.deepEqual(native.map(x=>x.data.mic),['muted','live']);
  assert.equal((await send('beat',session,'malicious\nstop')).ok,true);
  assert.equal(native.at(-1).data.mic,'unknown', 'untrusted state cannot reach host');
}

background().then(() => console.log('Satellite extension tests passed (Meet routes, safety, Edge async response).'))
  .catch(error => { console.error(error);process.exitCode=1; });
