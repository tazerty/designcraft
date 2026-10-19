#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Fumée Evora (patch EVORA-CIC-00) : le build web démarre dans une iframe servie par une AUTRE origine.

Bibliothèque standard Python seule, aucune action tierce. Chrome sans interface (préinstallé sur le
runner) est piloté par le protocole DevTools sur tuyaux (--remote-debugging-pipe : messages JSON
terminés par un octet nul sur les descripteurs 3 et 4), même technique que la fumée de tazerty/pdfcraft.

  origine moteur  : http://127.0.0.1:<PORT_ENGINE>/  sert le site empaqueté ;
  origine parente : http://localhost:<PORT_PARENT>/  sert une page qui l'encadre (sandbox
                    « allow-scripts allow-same-origin », comme la coquille d'Evora).

Deux hôtes et deux ports : deux origines et deux sites, la situation du mode B d'Evora.
Scénarios, chacun sur un profil Chrome vierge (démarrage à froid) :
  base    BLOQUANT : le moteur démarre ;
  csp     RAPPORT  : même chose sous la CSP cible des documents moteur (empreintes de csp-hashes.txt,
                     frame-ancestors limité à l'origine parente) ; le verdict est relevé, il ne bloque pas
                     avant le pont EHP (CIC-07) ;
  temoin  BLOQUANT À L'ENVERS : le module .wasm répond 404 ; le scénario DOIT être rouge, et rouge par
                     le détecteur de démarrage : la preuve que la fumée sait rougir.

Démarrage établi quand, dans le document moteur : le panneau d'erreur du moteur existe (preuve que le
main wasm a tourné, il le crée en premier), le voile « #designcraft_loading » est retiré (le moteur ne le
retire qu'après un démarrage réussi), le canvas « #designcraft_canvas » est dimensionné par eframe
(largeur différente des 300 px par défaut) et le panneau d'erreur n'est pas affiché.
Succès du scénario : démarrage établi, aucune exception JavaScript (page parente ou moteur), aucune
ressource du moteur en erreur HTTP, aucun téléchargement. Les erreurs de console sont relevées en rapport.

Usage : smoke.py SITE_DIR CSP_HASHES_FILE OUT_DIR REPORT_MD
Statut 1 si « base » échoue ou si le témoin ne rougit pas par le démarrage ; 2 sur un usage invalide.
"""

import base64
import fcntl
import functools
import http.server
import json
import os
import select
import shutil
import sys
import tempfile
import threading
import time

PORT_ENGINE = 8851
PORT_PARENT = 8852
ENGINE_ORIGIN = f"http://127.0.0.1:{PORT_ENGINE}"
PARENT_ORIGIN = f"http://localhost:{PORT_PARENT}"
IFRAME_W, IFRAME_H = 1280, 800
START_TIMEOUT = 150.0  # secondes ; borne d'instrument (SwiftShader sans GPU), pas un budget produit
SETTLE = 3.0  # secondes d'observation après le démarrage, pour les exceptions tardives
EXCEPTION_GRACE = 15.0  # secondes laissées au démarrage après une première exception
MIME = {".html": "text/html; charset=utf-8", ".js": "text/javascript", ".wasm": "application/wasm",
        ".css": "text/css", ".json": "application/json", ".svg": "image/svg+xml", ".png": "image/png",
        ".txt": "text/plain; charset=utf-8", ".ico": "image/x-icon"}

TIMING = """(() => {
  if (window.__evoraT) return;
  const T = window.__evoraT = {};
  const mark = (k) => { if (!(k in T)) T[k] = Math.round(performance.now()); };
  addEventListener('TrunkApplicationStarted', () => mark('trunkStarted'));
  new MutationObserver(() => {
    if (document.getElementById('designcraft_graphics_error')) mark('mainRan');
    if (document.readyState !== 'loading' && !document.getElementById('designcraft_loading')) mark('loadingRemoved');
    const c = document.getElementById('designcraft_canvas');
    if (c && c.width !== 300) mark('canvasSized');
  }).observe(document, {childList: true, subtree: true, attributes: true, attributeFilter: ['width']});
})()"""

PROBE = """(() => {
  const c = document.getElementById('designcraft_canvas');
  const p = document.getElementById('designcraft_graphics_error');
  const shown = !!p && getComputedStyle(p).display !== 'none';
  return JSON.stringify({
    url: location.href, readyState: document.readyState, canvas: !!c, w: c ? c.width : 0, h: c ? c.height : 0,
    loading: !!document.getElementById('designcraft_loading'), panel: !!p, panelShown: shown,
    panelText: shown ? p.innerText.slice(0, 600) : '', webgpu: !!navigator.gpu,
    t: Math.round(performance.now()), marks: window.__evoraT || null});
})()"""


class Handler(http.server.SimpleHTTPRequestHandler):
    csp = None
    sabotage = None
    page = None
    log = None

    def log_message(self, fmt, *args):
        pass

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        if self.csp and self.path.split("?")[0].endswith((".html", "/")):
            self.send_header("Content-Security-Policy", self.csp)
        super().end_headers()

    def send_response(self, code, message=None):
        if self.log is not None:
            self.log.append((self.path, code))
        super().send_response(code, message)

    def guess_type(self, path):
        return MIME.get(os.path.splitext(path)[1].lower(), "application/octet-stream")

    def do_GET(self):
        path = self.path.split("?")[0]
        if self.page is not None:
            if path in ("/", "/index.html"):
                body = self.page.encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", MIME[".html"])
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            else:
                self.send_error(404)
            return
        if self.sabotage and path.endswith(self.sabotage):
            self.send_error(404)
            return
        super().do_GET()


def start_server(host, port, **attrs):
    directory = attrs.pop("directory", None)
    handler = type("H", (Handler,), attrs)
    if directory:
        handler = functools.partial(handler, directory=directory)
    httpd = http.server.ThreadingHTTPServer((host, port), handler)
    httpd.daemon_threads = True
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


class Chrome:
    """Chrome sans interface piloté par --remote-debugging-pipe (descripteurs 3 et 4)."""

    def __init__(self, binary, profile, log_path):
        cmd_r, self.cmd_w = os.pipe()
        self.res_r, res_w = os.pipe()
        hi_r = fcntl.fcntl(cmd_r, fcntl.F_DUPFD_CLOEXEC, 20)
        hi_w = fcntl.fcntl(res_w, fcntl.F_DUPFD_CLOEXEC, 20)
        os.close(cmd_r)
        os.close(res_w)
        self.log = open(log_path, "wb")
        args = [binary, "--headless=new", "--remote-debugging-pipe", f"--user-data-dir={profile}",
                "--no-first-run", "--no-default-browser-check", "--disable-extensions", "--disable-sync",
                # macOS : sans ce drapeau, chaque lancement clone l'application (aucun effet sous Linux).
                "--disable-features=MacAppCodeSignClone",
                "--disable-background-networking", "--disable-component-update", "--disable-default-apps",
                "--mute-audio", "--no-sandbox", "--disable-dev-shm-usage", "--ignore-gpu-blocklist",
                "--enable-webgl", "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
                f"--window-size={IFRAME_W + 40},{IFRAME_H + 40}", "about:blank"]
        actions = [(os.POSIX_SPAWN_OPEN, 0, "/dev/null", os.O_RDONLY, 0),
                   (os.POSIX_SPAWN_DUP2, self.log.fileno(), 1), (os.POSIX_SPAWN_DUP2, self.log.fileno(), 2),
                   (os.POSIX_SPAWN_DUP2, hi_r, 3), (os.POSIX_SPAWN_DUP2, hi_w, 4)]
        self.pid = os.posix_spawn(binary, args, os.environ, file_actions=actions)
        os.close(hi_r)
        os.close(hi_w)
        self.buf, self.next_id, self.responses, self.handlers = b"", 0, {}, []

    def send(self, method, params=None, session=None):
        self.next_id += 1
        msg = {"id": self.next_id, "method": method, "params": params or {}}
        if session:
            msg["sessionId"] = session
        os.write(self.cmd_w, json.dumps(msg).encode() + b"\0")
        return self.next_id

    def pump(self, timeout):
        ready, _, _ = select.select([self.res_r], [], [], timeout)
        if not ready:
            return
        chunk = os.read(self.res_r, 1 << 20)
        if not chunk:
            raise RuntimeError("Chrome a fermé le tuyau DevTools")
        self.buf += chunk
        while b"\0" in self.buf:
            raw, self.buf = self.buf.split(b"\0", 1)
            msg = json.loads(raw)
            if "id" in msg:
                self.responses[msg["id"]] = msg
            else:
                for h in self.handlers:
                    h(msg)

    def call(self, method, params=None, session=None, timeout=30.0):
        i = self.send(method, params, session)
        end = time.monotonic() + timeout
        while i not in self.responses:
            left = end - time.monotonic()
            if left <= 0:
                raise TimeoutError(f"{method} sans réponse")
            self.pump(min(left, 0.25))
        msg = self.responses.pop(i)
        if "error" in msg:
            raise RuntimeError(f"{method} : {msg['error']}")
        return msg.get("result", {})

    def close(self):
        try:
            self.send("Browser.close")
            end = time.monotonic() + 10
            while time.monotonic() < end:
                if os.waitpid(self.pid, os.WNOHANG)[0]:
                    break
                time.sleep(0.1)
            else:
                os.kill(self.pid, 9)
                os.waitpid(self.pid, 0)
        except OSError:
            pass
        self.log.close()


def remote_text(args):
    out = []
    for a in args or []:
        out.append(str(a.get("value", a.get("description", a.get("unserializableValue", a.get("type", ""))))))
    return " ".join(out)


def parent_page():
    return (f'<!DOCTYPE html><html lang="fr"><head><meta charset="utf-8"><title>Evora : page parente</title>'
            f'<link rel="icon" href="data:,"></head><body style="margin:0">'
            f'<iframe id="engine" src="{ENGINE_ORIGIN}/index.html" width="{IFRAME_W}" height="{IFRAME_H}" '
            f'style="border:0;display:block" sandbox="allow-scripts allow-same-origin" '
            f'allow="clipboard-read; clipboard-write; fullscreen"></iframe></body></html>')


def target_csp(hashes):
    sources = " ".join(f"'{h}'" for h in hashes)
    return ("default-src 'self'; script-src 'self' 'wasm-unsafe-eval' " + sources + "; worker-src 'none'; "
            "connect-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; font-src 'self'; "
            f"frame-ancestors {PARENT_ORIGIN}; object-src 'none'; base-uri 'none'")


def run_scenario(name, site, out_dir, chrome_bin, csp=None, sabotage=None):
    requests_log = []
    engine = start_server("127.0.0.1", PORT_ENGINE, directory=site, csp=csp, sabotage=sabotage, log=requests_log)
    parent = start_server("localhost", PORT_PARENT, page=parent_page(), log=[])
    profile = tempfile.mkdtemp(prefix=f"evora-chrome-{name}-")
    chrome = Chrome(chrome_bin, profile, os.path.join(out_dir, f"chrome-{name}.log"))
    res = {"scenario": name, "csp": bool(csp), "sabotage": sabotage, "exceptions": [], "console": [],
           "log": [], "downloads": 0, "probe": None, "backend": None, "reasons": []}
    sessions = {}

    def on_event(msg):
        method, params, sess = msg.get("method"), msg.get("params", {}), msg.get("sessionId")
        who = sessions.get(sess, "navigateur")
        if method == "Target.attachedToTarget":
            child, info = params["sessionId"], params.get("targetInfo", {})
            if info.get("type") == "iframe":
                sessions[child] = "moteur"
                for m in ("Runtime.enable", "Log.enable", "Page.enable"):
                    chrome.send(m, {}, child)
                chrome.send("Page.addScriptToEvaluateOnNewDocument", {"source": TIMING}, child)
            chrome.send("Runtime.runIfWaitingForDebugger", {}, child)
        elif method == "Runtime.exceptionThrown":
            res.setdefault("first_exception_at", time.monotonic())
            d = params.get("exceptionDetails", {})
            res["exceptions"].append(f"{who} : {d.get('text', '')} {remote_text([d.get('exception', {})])}".strip())
        elif method == "Runtime.consoleAPICalled":
            text = remote_text(params.get("args"))
            if "wgpu backend" in text:
                res["backend"] = text[:200]
            if params.get("type") in ("error", "assert"):
                res["console"].append(f"{who} : {text[:400]}")
        elif method == "Log.entryAdded":
            e = params.get("entry", {})
            if e.get("level") == "error":
                res["log"].append(f"{who} : {e.get('text', '')[:300]} {e.get('url', '')}".strip())
        elif method in ("Browser.downloadWillBegin", "Page.downloadWillBegin"):
            res["downloads"] += 1

    chrome.handlers.append(on_event)
    start = time.monotonic()
    try:
        res["chrome"] = chrome.call("Browser.getVersion").get("product", "?")
        chrome.call("Browser.setDownloadBehavior", {"behavior": "deny", "eventsEnabled": True})
        target = chrome.call("Target.createTarget", {"url": "about:blank"})["targetId"]
        page = chrome.call("Target.attachToTarget", {"targetId": target, "flatten": True})["sessionId"]
        sessions[page] = "parente"
        for m in ("Page.enable", "Runtime.enable", "Log.enable"):
            chrome.call(m, {}, page)
        chrome.call("Target.setAutoAttach", {"autoAttach": True, "waitForDebuggerOnStart": True, "flatten": True}, page)
        chrome.call("Page.navigate", {"url": f"{PARENT_ORIGIN}/"}, page)
        deadline = start + START_TIMEOUT
        while time.monotonic() < deadline:
            chrome.pump(0.25)
            eng = [s for s, w in sessions.items() if w == "moteur"]
            if not eng:
                continue
            try:
                r = chrome.call("Runtime.evaluate", {"expression": PROBE, "returnByValue": True}, eng[0], 10)
                res["probe"] = json.loads(r.get("result", {}).get("value") or "null")
            except (RuntimeError, TimeoutError, ValueError):
                continue
            p = res["probe"] or {}
            if p.get("panelShown") or (p.get("panel") and not p.get("loading") and p.get("w", 300) != 300):
                break
            # Une exception sans démarrage : verdict après une dernière chance, sans attendre la borne.
            if "first_exception_at" in res and time.monotonic() - res["first_exception_at"] > EXCEPTION_GRACE:
                break
        settle = time.monotonic() + SETTLE
        while time.monotonic() < settle:
            chrome.pump(0.25)
        res["seconds_to_verdict"] = round(time.monotonic() - start, 1)
        shot = chrome.call("Page.captureScreenshot", {"format": "png"}, page)
        with open(os.path.join(out_dir, f"capture-{name}.png"), "wb") as fh:
            fh.write(base64.b64decode(shot["data"]))
    except (RuntimeError, TimeoutError, OSError) as e:
        res["reasons"].append(f"instrument : {e}")
    finally:
        chrome.close()
        engine.shutdown()
        parent.shutdown()
        engine.server_close()
        parent.server_close()
        shutil.rmtree(profile, ignore_errors=True)
    res["requests"] = len(requests_log)
    res["failed_requests"] = [f"{p} -> {c}" for p, c in requests_log if c >= 400 and not p.startswith("/favicon")]
    p = res["probe"] or {}
    started = bool(p.get("panel")) and not p.get("loading") and p.get("w", 300) != 300 and not p.get("panelShown")
    if not started:
        res["reasons"].append("demarrage : " + ("panneau d'erreur affiché : " + p.get("panelText", "")[:200]
                                                if p.get("panelShown") else
                                                f"moteur non démarré (main wasm vu : {bool(p.get('panel'))}, "
                                                f"voile présent : {p.get('loading')}, canvas : {p.get('w')} px)"))
    if res["exceptions"]:
        res["reasons"].append(f"exceptions : {len(res['exceptions'])}")
    if res["failed_requests"]:
        res["reasons"].append("ressources en erreur : " + ", ".join(res["failed_requests"][:5]))
    if res["downloads"]:
        res["reasons"].append(f"téléchargements : {res['downloads']}")
    if not requests_log:
        res["reasons"].append("instrument : aucune requête vue par le serveur du moteur")
    res["ok"] = not res["reasons"]
    with open(os.path.join(out_dir, f"result-{name}.json"), "w", encoding="utf-8") as fh:
        json.dump(res, fh, ensure_ascii=False, indent=1)
    return res


def find_chrome():
    for c in ("google-chrome", "google-chrome-stable", "chromium", "chromium-browser"):
        if shutil.which(c):
            return shutil.which(c)
    mac = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    return mac if os.path.exists(mac) else None


def wasm_name(site):
    names = [n for n in os.listdir(site) if n.endswith("_bg.wasm")]
    return names[0] if len(names) == 1 else None


def main(argv):
    if len(argv) != 5:
        print(__doc__)
        return 2
    site, hashes_file, out_dir, report_path = argv[1:]
    os.makedirs(out_dir, exist_ok=True)
    chrome_bin = find_chrome()
    wasm = wasm_name(site)
    with open(hashes_file, encoding="utf-8") as fh:
        hashes = [l.strip() for l in fh if l.strip()]
    if not chrome_bin or not wasm or not hashes:
        print(f"::error::fumée impossible : Chrome {chrome_bin}, module {wasm}, empreintes CSP {len(hashes)}")
        return 1
    runs = [run_scenario("base", site, out_dir, chrome_bin),
            run_scenario("csp", site, out_dir, chrome_bin, csp=target_csp(hashes)),
            run_scenario("temoin", site, out_dir, chrome_bin, sabotage=wasm)]
    base, csp, temoin = runs
    temoin_ok = (not temoin["ok"]) and any(r.startswith("demarrage") for r in temoin["reasons"])
    with open(report_path, "a", encoding="utf-8") as rep:
        rep.write("### Fumée depuis une autre origine (Chrome sans interface)\n\n")
        rep.write(f"Moteur `{ENGINE_ORIGIN}`, page parente `{PARENT_ORIGIN}` ; {base.get('chrome', '?')}.\n\n")
        rep.write("| Scénario | Rôle | Verdict | Démarrage (marques, ms) | Moteur de rendu | Exceptions | Console (erreurs) | Requêtes | Raisons |\n")
        rep.write("|---|---|---|---|---|---|---|---|---|\n")
        roles = {"base": "bloquant", "csp": "rapport", "temoin": "doit rougir par le démarrage"}
        for r in runs:
            verdict = ("rouge attendu" if temoin_ok else "TÉMOIN MUET") if r is temoin else ("vert" if r["ok"] else "ROUGE")
            marks = json.dumps((r["probe"] or {}).get("marks"), ensure_ascii=False)
            rep.write(f"| {r['scenario']} | {roles[r['scenario']]} | {verdict} | `{marks}` | {r['backend'] or 'n/d'} | "
                      f"{len(r['exceptions'])} | {len(r['console']) + len(r['log'])} | {r['requests']} | "
                      f"{'; '.join(r['reasons']) or '-'} |\n")
        rep.write("\n")
        for r in runs:
            for item in (r["exceptions"] + r["console"] + r["log"])[:12]:
                rep.write(f"- {r['scenario']} : `{item[:300]}`\n")
        rep.write("\n")
    for r in runs:
        print(f"::notice::fumée {r['scenario']} : ok={r['ok']} raisons={r['reasons']} moteur={r['backend']} "
              f"marques={(r['probe'] or {}).get('marks')}")
    status = 0
    if not base["ok"]:
        print("::error::fumée « base » rouge : " + "; ".join(base["reasons"]))
        status = 1
    if not temoin_ok:
        print("::error::témoin de fumée muet : un .wasm absent ne fait pas rougir le détecteur de démarrage")
        status = 1
    if not csp["ok"]:
        print("::warning::fumée « csp » (rapport) rouge : " + "; ".join(csp["reasons"]))
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv))
