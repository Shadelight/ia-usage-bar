import { spawn } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const edge = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const output = resolve("docs/superpowers/evidence/2026-09-11-responsive");
const profile = await mkdtemp(join(tmpdir(), "iausagebar-cdp-"));
await mkdir(output, { recursive: true });

const port = 9337;
const browser = spawn(edge, [
  "--headless=new",
  "--no-sandbox",
  "--disable-gpu",
  `--remote-debugging-port=${port}`,
  `--user-data-dir=${profile}`,
  "about:blank",
], { stdio: "ignore", windowsHide: true });

const wait = (ms) => new Promise((resolvePromise) => setTimeout(resolvePromise, ms));
let socket;
let id = 0;
const pending = new Map();

async function json(url, options) {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    try { return await (await fetch(url, options)).json(); } catch { await wait(100); }
  }
  throw new Error(`CDP unavailable: ${url}`);
}

function send(method, params = {}) {
  const messageId = ++id;
  socket.send(JSON.stringify({ id: messageId, method, params }));
  return new Promise((resolvePromise, reject) => pending.set(messageId, { resolve: resolvePromise, reject }));
}

async function evaluate(expression) {
  const response = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  return response.result.result.value;
}

async function viewport(width, height = 720) {
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await wait(80);
}

async function screenshot(name) {
  const response = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
  await writeFile(join(output, name), Buffer.from(response.result.data, "base64"));
}

try {
  const target = await json(`http://127.0.0.1:${port}/json/new?http://127.0.0.1:1420/`, { method: "PUT" });
  socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolvePromise, reject) => {
    socket.addEventListener("open", resolvePromise, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (!message.id || !pending.has(message.id)) return;
    const task = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) task.reject(new Error(message.error.message)); else task.resolve(message);
  });
  await send("Page.enable");
  await send("Runtime.enable");
  await wait(500);

  const evidence = { viewports: {}, interactions: {} };
  for (const width of [360, 390, 420, 600]) {
    await viewport(width);
    await evaluate("document.documentElement.dataset.theme='dark'; document.querySelector('#detail').scrollTop=0");
    await screenshot(`dashboard-${width}.png`);
    evidence.viewports[width] = await evaluate(`({innerWidth, horizontalOverflow: document.documentElement.scrollWidth > innerWidth, detailScrollHeight: document.querySelector('#detail').scrollHeight, detailClientHeight: document.querySelector('#detail').clientHeight})`);
  }
  await viewport(390);
  await evaluate("document.documentElement.dataset.theme='light'");
  await screenshot("dashboard-light-390.png");
  await evaluate("document.documentElement.dataset.theme='dark'");

  await viewport(360);
  evidence.interactions.vertical = await evaluate(`(() => { const el=document.querySelector('#detail'); el.scrollTop=el.scrollHeight; return {scrollTop:el.scrollTop,max:el.scrollHeight-el.clientHeight,reachedEnd:Math.abs(el.scrollTop-(el.scrollHeight-el.clientHeight))<2}; })()`);
  evidence.interactions.providers = await evaluate(`(() => { const el=document.querySelector('#tabs'); el.scrollLeft=el.scrollWidth; return {scrollLeft:el.scrollLeft,max:el.scrollWidth-el.clientWidth,reachedEnd:Math.abs(el.scrollLeft-(el.scrollWidth-el.clientWidth))<2}; })()`);

  await evaluate("document.querySelector('[data-act=settings]').click()");
  await wait(100);
  await screenshot("settings-360.png");
  evidence.interactions.settingsTabs = await evaluate(`(() => { const el=document.querySelector('#settings-cats'); el.scrollLeft=el.scrollWidth; return {scrollLeft:el.scrollLeft,max:el.scrollWidth-el.clientWidth,reachedEnd:Math.abs(el.scrollLeft-(el.scrollWidth-el.clientWidth))<2}; })()`);
  await evaluate("document.querySelector('[data-setcat=appearance]').click()");
  await wait(80);
  evidence.interactions.appearance = await evaluate(`(() => { document.querySelector('[data-theme=light]').click(); return {theme:document.documentElement.dataset.theme,storedTheme:localStorage.getItem('theme'),compactToggle:!!document.querySelector('#cfg-compact'),languageChoices:document.querySelectorAll('.choice-seg [data-act^=lang]').length}; })()`);
  await evaluate("document.documentElement.dataset.theme='dark'; localStorage.setItem('theme','dark')");
  await screenshot("appearance-360.png");
  await evaluate("document.querySelector('[data-setcat=about]').click()");
  await wait(80);
  await viewport(390);
  await screenshot("about-390.png");
  evidence.interactions.about = await evaluate(`({urlActions:[...document.querySelectorAll('.about-action[data-open-url]')].map(el=>el.dataset.openUrl),hasUpdateAction:!!document.querySelector('[data-act=check-updates]')})`);
  await evaluate("document.querySelector('[data-act=back]').click(); document.querySelector('[data-act=compact]').click()");
  await wait(100);
  await viewport(350, 220);
  await screenshot("compact-350.png");
  evidence.interactions.compact = await evaluate(`({quotaRows:document.querySelectorAll('.compact-quota').length,hasRecommendation:!!document.querySelector('.compact-recommendation'),footerHidden:getComputedStyle(document.querySelector('.foot')).display==='none',horizontalOverflow:document.documentElement.scrollWidth>innerWidth})`);

  await writeFile(join(output, "responsive-proof.json"), JSON.stringify(evidence, null, 2));
  console.log(JSON.stringify(evidence, null, 2));
} finally {
  socket?.close();
  browser.kill();
  await new Promise((resolvePromise) => browser.once("exit", resolvePromise));
  for (let attempt = 0; attempt < 5; attempt += 1) {
    try { await rm(profile, { recursive: true, force: true }); break; }
    catch { await wait(100); }
  }
}
