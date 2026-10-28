#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Fumée Evora « pont-ehp » (patches EVORA-CIC-07 à CIC-10, CIC-15) : le pont hôte natif de Cicero.

Bibliothèque standard Python seule. Réutilise le pilote Chrome de smoke.py (protocole DevTools sur tuyaux).
La page parente (origine B, http://localhost:8852) est une coquille minimale : elle encadre le moteur
(origine A, http://127.0.0.1:8851, ?host=evora&lang=fr) et une fenêtre témoin de sa propre origine,
distincte de la page parente. Le banc sert evora-host.json comme le provisionnement d'Evora, avec la seule
origine B admise. Tout ce que la page parente reçoit est journalisé ; le banc pilote les échanges et
juge chaque étiquette :

  READY            un seul evora:ready conforme (app layout, ehp 1.x, les 15 capacités attendues)
  INIT-V2          un evora:init de version 2 reste sans réponse
  INIT-EXTRA       un evora:init dont l'enveloppe a un champ en trop reste sans réponse
  INIT-ACK         le vrai evora:init est accusé positivement (re = son id)
  STARTED          le moteur démarre (canvas dimensionné, voile retiré)
  CATALOG          evora:commands : empreinte, schéma sans paramètre, aucune commande de la coquille
                   (file., app., help., prefs., window.) hors des exports admis un par un
  NEW-DOC          evora:open {new: true} : accusé positif, evora:dirty false, un document
  MODIFY           une commande du catalogue (Nouveau gabarit…, Entrée) : evora:dirty true
  SAVE-REQ         evora:request-save : un evora:save conforme (re, docRef, .designcraft, zip)
  S-ACK-1          aucun evora:dirty false tant que la coquille n'a pas répondu
  S-ACK-2          evora:saved {ok: true} : evora:dirty false
  S-ACK-3          une seconde demande : evora:ack {ok: true, code: clean}
  S-ACK-4          evora:saved {ok: false, conflict} : le document reste modifié, la demande suivante
                   rend un evora:save
  S-ACK-5          modification pendant l'envoi puis evora:saved {ok: true} : il reste modifié
  OPEN-BYTES       evora:open des octets enregistrés : accusé positif, dirty false, UN document
  EXPORT-PDF       une exportation PDF rend un evora:export (application/pdf, %PDF)
  EXPORT-PNG       une exportation PNG rend un evora:export (image/png, signature PNG)
  PICK             Placer : evora:pick {purpose: import, accept, multiple: false}, puis evora:file et
                   evora:ack ferment l'échange
  SHELL-REFUSED    file.revert et help.about (coquille) refusés : capability
  PARAMS-REFUSED   une commande avec un paramètre (chemin) reste sans réponse et sans effet
  DESTRUCTIVE      une commande destructive sans confirmed : forbidden (ou n/a si le catalogue n'en a pas)
  SOURCE-REFUSED   un message de la fenêtre témoin (même origine, autre fenêtre) est ignoré
  FILTER-NEW       Ctrl+N au clavier n'ouvre aucun dialogue de nouveau document (filtre de l'hôte)
  FOCUS-OUT        F6 : un seul evora:focus-out
  LANG             evora:lang (de) : accusé, catalogue renvoyé en allemand
  THEME            evora:theme : accusé
  FONT             evora:font d'une police de craft-fonts : accusé, famille listée
  FONT-INVALID     des octets qui ne sont pas une police : invalid_content
  PURGE            une clé posée dans le stockage du moteur ; evora:purge : accusé, stockage vide
                   (localStorage, sessionStorage, IndexedDB, Cache Storage, OPFS)
  NO-DOWNLOAD      aucun téléchargement (Browser.downloadWillBegin)
  NO-EXCEPTION     aucune exception JavaScript

Usage : smoke_ehp.py SITE_DIR FONTS_DIR OUT_DIR REPORT_MD
Statut 1 si une étiquette est rouge ; la liste exacte des étiquettes rouges est imprimée
(« ROUGES : … ») pour les témoins groupés.
"""

import base64
import functools
import glob
import http.server
import json
import os
import shutil
import sys
import tempfile
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import smoke  # noqa: E402  (pilote Chrome et serveurs de la fumée de base)

ENGINE_ORIGIN = smoke.ENGINE_ORIGIN
PARENT_ORIGIN = smoke.PARENT_ORIGIN
EXPECTED_CAPS = ["doc.open", "doc.save", "doc.saved-ack", "doc.dirty", "doc.request-save", "export.drive", "pick.drive",
                 "command", "command.catalog", "font.inject", "storage.memory", "storage.purge", "live.lang", "live.theme",
                 "ui.focus-out"]
SHELL_PREFIXES = ("file.", "app.", "help.", "prefs.", "window.")
ADMITTED = {"app.exportPdf", "app.exportInteractivePdf", "app.exportPng", "app.exportIdml", "app.exportEpub",
            "app.exportFixedEpub", "app.exportHtml", "app.exportPptx", "app.exportXml", "app.exportText",
            "app.placeDialog", "app.loadSwatches"}
LABELS = ["READY", "INIT-V2", "INIT-EXTRA", "INIT-ACK", "STARTED", "CATALOG", "NEW-DOC", "MODIFY", "SAVE-REQ",
          "S-ACK-1", "S-ACK-2", "S-ACK-3", "S-ACK-4", "S-ACK-5", "OPEN-BYTES", "EXPORT-PDF", "EXPORT-PNG", "PICK",
          "SHELL-REFUSED", "PARAMS-REFUSED", "DESTRUCTIVE", "SOURCE-REFUSED", "FILTER-NEW", "FOCUS-OUT", "LANG",
          "THEME", "FONT", "FONT-INVALID", "PURGE", "NO-DOWNLOAD", "NO-EXCEPTION"]
PNG_1PX = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg=="
STORAGE_NS = "a" * 64
STEP = 60.0  # secondes : borne d'instrument par étape (SwiftShader), pas un budget produit
QUIET = 2.5  # secondes d'observation pour un message qui ne doit PAS arriver
LIMITS = {"maxDocumentBytes": 64 * 1024 * 1024, "maxExportBytes": 64 * 1024 * 1024}

PARENT = """<!DOCTYPE html><html lang="fr"><head><meta charset="utf-8"><title>Evora : coquille de banc</title>
<link rel="icon" href="data:,"></head><body style="margin:0">
<iframe id="engine" src="%(engine)s/index.html?host=evora&lang=fr" width="1280" height="800" style="border:0;display:block"
 sandbox="allow-scripts allow-same-origin" allow="clipboard-read; clipboard-write; fullscreen"></iframe>
<iframe id="witness" style="display:none" srcdoc="<script>addEventListener('message', function (e) {
 if (e.source !== parent) return; parent.document.getElementById('engine').contentWindow.postMessage(e.data.forward, '%(engine)s'); });</script>"></iframe>
<script>
(function () {
  const ENGINE = '%(engine)s';
  const CROCK = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
  window.__log = []; window.__buffers = {}; window.__foreign = 0;
  window.__ulid = function () { let t = Date.now(), s = '';
    for (let i = 0; i < 10; i++) { s = CROCK[t %% 32] + s; t = Math.floor(t / 32); }
    for (let i = 0; i < 16; i++) s += CROCK[Math.floor(Math.random() * 32)]; return s; };
  function head(buf) { return Array.from(new Uint8Array(buf.slice(0, 8))).map(b => b.toString(16).padStart(2, '0')).join(''); }
  function summary(v) {
    if (v instanceof ArrayBuffer) return { __bytes: v.byteLength, head: head(v) };
    if (Array.isArray(v)) return v.map(summary);
    if (v && typeof v === 'object') { const o = {}; for (const k of Object.keys(v)) o[k] = summary(v[k]); return o; }
    return v; }
  addEventListener('message', function (e) {
    const frame = document.getElementById('engine');
    if (e.origin !== ENGINE || e.source !== frame.contentWindow) { window.__foreign++; return; }
    const m = e.data || {};
    if (m.payload && m.payload.bytes instanceof ArrayBuffer) window.__buffers[m.id] = m.payload.bytes;
    window.__log.push({ type: m.type, id: m.id, re: m.re, keys: Object.keys(m).sort(), payload: summary(m.payload), t: Date.now() });
  });
  window.__b64 = function (s) { return Uint8Array.from(atob(s), c => c.charCodeAt(0)).buffer; };
  window.__send = function (type, payload, re, opts) {
    opts = opts || {}; const id = window.__ulid();
    const msg = { v: opts.v || 1, type: type, id: id, payload: payload };
    if (re) msg.re = re;
    if (opts.extra) msg.extra = true;
    const transfer = payload && payload.bytes instanceof ArrayBuffer ? [payload.bytes] : [];
    if (opts.witness) document.getElementById('witness').contentWindow.postMessage({ forward: msg }, '*');
    else document.getElementById('engine').contentWindow.postMessage(msg, ENGINE, transfer);
    return id; };
  window.__copy = function (id) { const b = window.__buffers[id]; return b ? b.slice(0) : null; };
})();
</script></body></html>"""

ENGINE_PROBE = """(() => { const c = document.getElementById('designcraft_canvas');
  return JSON.stringify({loading: !!document.getElementById('designcraft_loading'), w: c ? c.width : 0}); })()"""

STORAGE_PROBE = """(async () => {
  const out = {local: localStorage.length, session: sessionStorage.length, idb: -1, caches: -1, opfs: -1};
  try { out.idb = (await indexedDB.databases()).length; } catch (e) { out.idb = 'erreur'; }
  try { out.caches = (await caches.keys()).length; } catch (e) { out.caches = 'erreur'; }
  try { const d = await navigator.storage.getDirectory(); let n = 0; for await (const _ of d.keys()) n++; out.opfs = n; }
  catch (e) { out.opfs = 'erreur'; }
  return JSON.stringify(out); })()"""

SEED_STORAGE = """(async () => { localStorage.setItem('reste-autre-page', '1'); sessionStorage.setItem('reste', '1');
  await new Promise((ok) => { const r = indexedDB.open('reste-autre-page'); r.onsuccess = () => { r.result.close(); ok(); }; r.onerror = ok; });
  try { await (await caches.open('reste-autre-page')).put('/reste', new Response('x')); } catch (e) {}
  return 'ok'; })()"""


def start_engine(site, log):
    """Le site du moteur, plus evora-host.json (seule l'origine parente admise), comme le provisionnement."""
    host_json = json.dumps({"parentOrigins": [PARENT_ORIGIN]}).encode()

    class EngineHandler(smoke.Handler):
        def do_GET(self):
            if self.path.split("?")[0] == "/evora-host.json":
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(host_json)))
                self.end_headers()
                self.wfile.write(host_json)
                return
            smoke.Handler.do_GET(self)

    EngineHandler.log = log
    httpd = http.server.ThreadingHTTPServer(("127.0.0.1", smoke.PORT_ENGINE), functools.partial(EngineHandler, directory=site))
    httpd.daemon_threads = True
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


class Banc:
    def __init__(self, site, fonts_dir, out_dir, chrome_bin):
        self.site, self.fonts_dir, self.out_dir = site, fonts_dir, out_dir
        self.res = {label: None for label in LABELS}
        self.notes = {}
        self.downloads, self.exceptions = 0, []
        self.requests = []
        self.engine = start_engine(site, self.requests)
        self.parent = smoke.start_server("localhost", smoke.PORT_PARENT, page=PARENT % {"engine": ENGINE_ORIGIN}, log=[])
        self.profile = tempfile.mkdtemp(prefix="evora-chrome-ehp-")
        self.chrome = smoke.Chrome(chrome_bin, self.profile, os.path.join(out_dir, "chrome-pont-ehp.log"))
        self.sessions = {}
        self.chrome.handlers.append(self.on_event)

    def on_event(self, msg):
        method, params = msg.get("method"), msg.get("params", {})
        if method == "Target.attachedToTarget":
            child, info = params["sessionId"], params.get("targetInfo", {})
            if info.get("type") == "iframe":
                self.sessions[child] = "moteur"
                for m in ("Runtime.enable", "Log.enable", "Page.enable"):
                    self.chrome.send(m, {}, child)
            self.chrome.send("Runtime.runIfWaitingForDebugger", {}, child)
        elif method == "Runtime.exceptionThrown":
            d = params.get("exceptionDetails", {})
            self.exceptions.append(f"{d.get('text', '')} {smoke.remote_text([d.get('exception', {})])}".strip()[:300])
        elif method in ("Browser.downloadWillBegin", "Page.downloadWillBegin"):
            self.downloads += 1

    # ── outils ─────────────────────────────────────────────────────────────
    def js(self, expr, session=None, timeout=20.0):
        r = self.chrome.call("Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True},
                             session or self.page, timeout)
        return r.get("result", {}).get("value")

    def engine_session(self):
        eng = [s for s, w in self.sessions.items() if w == "moteur"]
        return eng[0] if eng else None

    def log(self):
        return json.loads(self.js("JSON.stringify(window.__log)") or "[]")

    def send(self, kind, payload, re=None, **opts):
        return self.js(f"window.__send({json.dumps(kind)}, {payload}, {json.dumps(re)}, {json.dumps(opts)})")

    def wait(self, pred, timeout=STEP, start=0):
        """Le premier message reçu depuis l'indice `start` qui satisfait `pred`, ou None à la borne."""
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            self.chrome.pump(0.2)
            found = [m for m in self.log()[start:] if pred(m)]
            if found:
                return found[0]
        return None

    def quiet(self, pred, seconds=QUIET, start=0):
        """Vrai si aucun message reçu depuis `start` ne satisfait `pred` pendant `seconds`."""
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            self.chrome.pump(0.2)
        return not [m for m in self.log()[start:] if pred(m)]

    def answer(self, re, timeout=STEP, kind="evora:ack"):
        return self.wait(lambda m: m.get("type") == kind and m.get("re") == re, timeout)

    def command(self, name, params="{}", confirmed=None):
        extra = "" if confirmed is None else f", confirmed: {json.dumps(confirmed)}"
        return self.send("evora:command", f"{{command: {json.dumps(name)}, params: {params}{extra}}}")

    def diag(self):
        i = self.command("diag.stats")
        a = self.answer(i)
        return (a or {}).get("payload", {}).get("result") or {}

    def set(self, label, ok, note=None):
        if self.res.get(label) is None or not ok:
            self.res[label] = bool(ok)
        if note is not None:
            self.notes[label] = note

    def count(self):
        return len(self.log())

    def key(self, key, code, vk, modifiers=0, text=None):
        for kind in ("keyDown", "keyUp"):
            p = {"type": kind, "key": key, "code": code, "windowsVirtualKeyCode": vk, "nativeVirtualKeyCode": vk,
                 "modifiers": modifiers}
            if text and kind == "keyDown":
                p["text"] = text
            self.chrome.call("Input.dispatchKeyEvent", p, self.page)
            self.chrome.pump(0.05)

    def click_canvas(self):
        for kind in ("mousePressed", "mouseReleased"):
            self.chrome.call("Input.dispatchMouseEvent", {"type": kind, "x": 900, "y": 700, "button": "left", "clickCount": 1}, self.page)
        self.chrome.pump(0.3)

    def modify(self):
        """Une modification par le catalogue : Gabarit › Nouveau gabarit… puis Entrée dans son dialogue."""
        before = self.diag().get("revision")
        i = self.command("layout.parents.new")
        a = self.answer(i)
        if not (a and a["payload"].get("ok")):
            return False, f"commande : {a and a['payload']}"
        self.key("Enter", "Enter", 13, text="\r")
        end = time.monotonic() + STEP
        while time.monotonic() < end:
            d = self.diag()
            if d.get("revision") != before and d.get("dialog") is None:
                return True, None
            self.chrome.pump(0.3)
        return False, f"aucune modification (dialogue {self.diag().get('dialog')})"

    # ── scénario ───────────────────────────────────────────────────────────
    def run(self):
        c = self.chrome
        c.call("Browser.setDownloadBehavior", {"behavior": "deny", "eventsEnabled": True})
        target = c.call("Target.createTarget", {"url": "about:blank"})["targetId"]
        self.page = c.call("Target.attachToTarget", {"targetId": target, "flatten": True})["sessionId"]
        for m in ("Page.enable", "Runtime.enable"):
            c.call(m, {}, self.page)
        c.call("Target.setAutoAttach", {"autoAttach": True, "waitForDebuggerOnStart": True, "flatten": True}, self.page)
        c.call("Page.navigate", {"url": f"{PARENT_ORIGIN}/"}, self.page)

        ready = self.wait(lambda m: m.get("type") == "evora:ready", 120.0)
        readies = [m for m in self.log() if m.get("type") == "evora:ready"]
        p = (ready or {}).get("payload", {})
        self.set("READY", ready is not None and len(readies) == 1 and p.get("app") == "layout" and str(p.get("ehp", "")).startswith("1.")
                 and sorted(p.get("capabilities", [])) == sorted(EXPECTED_CAPS) and ready.get("keys") == ["id", "payload", "type", "v"],
                 p)
        if not ready:
            return
        init = (f"{{lang: 'fr', engineLang: 'fr', theme: 'dark', accent: '#4F46E5', readOnly: false, storageNs: '{STORAGE_NS}', "
                f"capabilities: {json.dumps(EXPECTED_CAPS)}, prefs: {{}}, limits: {json.dumps(LIMITS)}}}")
        n = self.count()
        self.send("evora:init", init, ready["id"], v=2)
        self.set("INIT-V2", self.quiet(lambda m: m.get("type") == "evora:ack"))
        self.send("evora:init", init, ready["id"], extra=True)
        self.set("INIT-EXTRA", self.quiet(lambda m: m.get("type") == "evora:ack"))
        i = self.send("evora:init", init, ready["id"])
        a = self.answer(i, 150.0)
        self.set("INIT-ACK", bool(a and a["payload"].get("ok") is True), a and a["payload"])
        started = False
        end = time.monotonic() + 150.0
        while time.monotonic() < end and not started:
            c.pump(0.3)
            s = self.engine_session()
            if not s:
                continue
            try:
                pr = json.loads(self.js(ENGINE_PROBE, s, 10) or "{}")
            except (RuntimeError, TimeoutError, ValueError):
                continue
            started = not pr.get("loading") and pr.get("w", 300) != 300
        self.set("STARTED", started)
        if not started:
            return

        # Document neuf de la coquille.
        doc1 = "01JD0000000000000000000001"
        i = self.send("evora:open", f"{{docRef: '{doc1}', name: 'Plaquette.designcraft', new: true}}")
        a = self.answer(i)
        d = self.diag()
        dirty0 = self.wait(lambda m: m.get("type") == "evora:dirty" and m["payload"].get("docRef") == doc1)
        self.set("NEW-DOC", bool(a and a["payload"].get("ok")) and d.get("documents") == 1 and d.get("docRef") == doc1
                 and dirty0 is not None and dirty0["payload"].get("dirty") is False, {"ack": a and a["payload"], "diag": d})

        cat = self.wait(lambda m: m.get("type") == "evora:commands")
        cp = (cat or {}).get("payload", {})
        cmds = cp.get("commands", [])
        bad = [x["id"] for x in cmds if x["id"].startswith(SHELL_PREFIXES) and x["id"] not in ADMITTED]
        ids = {x["id"] for x in cmds}
        self.set("CATALOG", bool(cmds) and not bad and len(str(cp.get("hash", ""))) == 16
                 and cp.get("params") == {"type": "object", "maxProperties": 0} and "layout.parents.new" in ids
                 and cp.get("count") == len(cmds), {"count": len(cmds), "shell": bad[:10], "lang": cp.get("lang")})

        # Modification, demande d'enregistrement, accusés (S-ACK).
        self.click_canvas()
        n = self.count()
        ok, why = self.modify()
        dirt = self.wait(lambda m: m.get("type") == "evora:dirty" and m["payload"].get("dirty") is True, start=n)
        self.set("MODIFY", ok and dirt is not None, why)
        n = self.count()
        rs = self.send("evora:request-save", "{reason: 'user'}")
        save = self.wait(lambda m: m.get("type") == "evora:save" and m.get("re") == rs)
        sp = (save or {}).get("payload", {})
        self.set("SAVE-REQ", save is not None and sp.get("docRef") == doc1 and str(sp.get("name", "")).endswith(".designcraft")
                 and sp.get("saveAs") is False and (sp.get("bytes") or {}).get("head", "").startswith("504b0304"), sp)
        if not save:
            return
        self.set("S-ACK-1", self.quiet(lambda m: m.get("type") == "evora:dirty" and m["payload"].get("dirty") is False, start=n))
        n = self.count()
        self.send("evora:saved", "{ok: true, version: 2}", save["id"])
        clean = self.wait(lambda m: m.get("type") == "evora:dirty" and m["payload"].get("dirty") is False, 20.0, start=n)
        self.set("S-ACK-2", clean is not None and self.diag().get("dirty") is False)
        rs2 = self.send("evora:request-save", "{reason: 'autosave'}")
        a = self.answer(rs2)
        self.set("S-ACK-3", bool(a and a["payload"] == {"ok": True, "code": "clean"}), a and a["payload"])
        saved_bytes_id = save["id"]

        ok, why = self.modify()
        rs3 = self.send("evora:request-save", "{reason: 'user'}")
        save3 = self.wait(lambda m: m.get("type") == "evora:save" and m.get("re") == rs3)
        n = self.count()
        if save3:
            self.send("evora:saved", "{ok: false, error: 'conflict'}", save3["id"])
        still = self.quiet(lambda m: m.get("type") == "evora:dirty" and m["payload"].get("dirty") is False, start=n)
        rs4 = self.send("evora:request-save", "{reason: 'user'}")
        save4 = self.wait(lambda m: m.get("type") == "evora:save" and m.get("re") == rs4)
        self.set("S-ACK-4", ok and save3 is not None and still and self.diag().get("dirty") is True and save4 is not None, why)

        ok, why = self.modify()  # modifié pendant l'envoi de save4
        n = self.count()
        if save4:
            self.send("evora:saved", "{ok: true, version: 3}", save4["id"])
        still = self.quiet(lambda m: m.get("type") == "evora:dirty" and m["payload"].get("dirty") is False, start=n)
        self.set("S-ACK-5", ok and save4 is not None and still and self.diag().get("dirty") is True, why)

        # Ouverture d'octets : remplace le document (un seul par éditeur).
        doc2 = "01JD0000000000000000000002"
        i = self.js(f"(() => {{ const b = window.__copy({json.dumps(saved_bytes_id)}); return b ? window.__send('evora:open', "
                    f"{{docRef: '{doc2}', name: 'Plaquette copie.designcraft', bytes: b}}) : null; }})()")
        a = self.answer(i) if i else None
        d = self.diag()
        dirty2 = self.wait(lambda m: m.get("type") == "evora:dirty" and m["payload"].get("docRef") == doc2)
        self.set("OPEN-BYTES", bool(a and a["payload"].get("ok")) and d.get("documents") == 1 and d.get("docRef") == doc2
                 and dirty2 is not None and dirty2["payload"].get("dirty") is False, {"ack": a and a["payload"], "diag": d})

        # Exports vers la coquille.
        for label, command, mime, magic in (("EXPORT-PDF", "app.exportInteractivePdf", "application/pdf", "25504446"),
                                            ("EXPORT-PNG", "app.exportPng", "image/png", "89504e47")):
            n = self.count()
            i = self.command(command)
            a = self.answer(i)
            ex = self.wait(lambda m: m.get("type") == "evora:export", start=n)
            ep = (ex or {}).get("payload", {})
            if ex:
                self.send("evora:ack", "{ok: true}", ex["id"])
            self.set(label, bool(a and a["payload"].get("ok")) and ep.get("mime") == mime and ep.get("docRef") == doc2
                     and (ep.get("bytes") or {}).get("head", "").startswith(magic), {"ack": a and a["payload"], "export": ep})

        # Sélecteur de la coquille.
        n = self.count()
        i = self.command("app.placeDialog")
        a = self.answer(i)
        pk = self.wait(lambda m: m.get("type") == "evora:pick", start=n)
        pp = (pk or {}).get("payload", {})
        if pk:
            self.js(f"window.__send('evora:file', {{path: '/Drive/pixel.png', name: 'pixel.png', bytes: window.__b64({json.dumps(PNG_1PX)})}}, "
                    f"{json.dumps(pk['id'])})")
            self.send("evora:ack", "{ok: true}", pk["id"])
        self.quiet(lambda m: False, 1.0)
        self.set("PICK", pk is not None and pp.get("purpose") == "import" and pp.get("multiple") is False and ".png" in pp.get("accept", [])
                 and self.diag().get("pickPending") is False, pp)
        self.key("Escape", "Escape", 27)

        # Refus : coquille, paramètres libres, destructives, source.
        codes = []
        for name in ("file.revert", "help.about", "file.newSample"):
            a = self.answer(self.command(name))
            codes.append((name, a and a["payload"]))
        self.set("SHELL-REFUSED", all(p == {"ok": False, "code": "capability"} for _, p in codes), codes)
        before = self.diag()
        i = self.command("layout.parents.new", params="{name: '/opfs/evora/x'}")
        quiet = self.quiet(lambda m: m.get("re") == i)
        after = self.diag()
        self.set("PARAMS-REFUSED", quiet and after.get("dialog") is None and after.get("revision") == before.get("revision"),
                 {"dialogue": after.get("dialog")})
        if after.get("dialog"):
            self.key("Escape", "Escape", 27)
        dm = self.answer(self.command("diag.menus"))
        destr = ((dm or {}).get("payload", {}).get("result") or {}).get("destructive", [])
        if destr:
            a = self.answer(self.command(destr[0]))
            self.set("DESTRUCTIVE", bool(a and a["payload"] == {"ok": False, "code": "forbidden"}), {"command": destr[0], "ack": a and a["payload"]})
        else:
            self.set("DESTRUCTIVE", True, "n/a : aucune commande destructive au catalogue")
        rsw = self.send("evora:request-save", "{reason: 'user'}", witness=True)
        self.set("SOURCE-REFUSED", self.quiet(lambda m: m.get("re") == rsw or (m.get("type") == "evora:save" and m.get("re") == rsw)))
        # La fenêtre témoin compte aussi pour la coquille (une demande ordinaire n'a pas été rejouée).

        # Filtre de l'hôte au clavier, sortie clavier.
        self.click_canvas()
        self.key("n", "KeyN", 78, modifiers=2)
        self.quiet(lambda m: False, 1.0)
        d = self.diag()
        self.set("FILTER-NEW", d.get("dialog") is None and d.get("documents") == 1, d.get("dialog"))
        n = self.count()
        self.key("F6", "F6", 117)
        fo = self.wait(lambda m: m.get("type") == "evora:focus-out", 10.0, start=n)
        self.quiet(lambda m: False, 1.0)
        fos = [m for m in self.log()[n:] if m.get("type") == "evora:focus-out"]
        self.set("FOCUS-OUT", fo is not None and len(fos) == 1)

        # Langue, thème.
        n = self.count()
        a = self.answer(self.send("evora:lang", "{lang: 'de', engineLang: 'de'}"))
        cat_de = self.wait(lambda m: m.get("type") == "evora:commands" and m["payload"].get("lang") == "de", start=n)
        self.set("LANG", bool(a and a["payload"].get("ok")) and cat_de is not None and self.diag().get("language") == "de")
        a = self.answer(self.send("evora:theme", "{theme: 'light', accent: '#4F46E5'}"))
        self.set("THEME", bool(a and a["payload"].get("ok")))

        # Polices.
        fonts = sorted(glob.glob(os.path.join(self.fonts_dir, "fonts", "*", "*.ttf")) + glob.glob(os.path.join(self.fonts_dir, "fonts", "*", "*.otf")))
        if fonts:
            path = fonts[0]
            family = os.path.splitext(os.path.basename(path))[0]
            fmt = "otf" if path.endswith(".otf") else "ttf"
            data = base64.b64encode(open(path, "rb").read()).decode()
            i = self.js(f"window.__send('evora:font', {{family: {json.dumps(family)}, format: '{fmt}', bytes: window.__b64({json.dumps(data)})}})")
            a = self.answer(i)
            self.set("FONT", bool(a and a["payload"].get("ok")) and family in self.diag().get("fonts", []), {"file": os.path.basename(path), "ack": a and a["payload"]})
        else:
            self.set("FONT", False, f"aucune police dans {self.fonts_dir}")
        i = self.js("window.__send('evora:font', {family: 'Pas une police', format: 'ttf', bytes: window.__b64('bm90IGEgZm9udA==')})")
        a = self.answer(i)
        self.set("FONT-INVALID", bool(a and a["payload"] == {"ok": False, "code": "invalid_content"}), a and a["payload"])

        # Purge : des restes posés par « une autre page » de l'origine du moteur.
        s = self.engine_session()
        seeded = self.js(SEED_STORAGE, s, 20)
        before = json.loads(self.js(STORAGE_PROBE, s, 20) or "{}")
        a = self.answer(self.send("evora:purge", "{scope: 'all'}"))
        self.quiet(lambda m: False, 1.0)
        after = json.loads(self.js(STORAGE_PROBE, s, 20) or "{}")
        empty = all(after.get(k) == 0 for k in ("local", "session", "idb", "caches", "opfs"))
        self.set("PURGE", seeded == "ok" and before.get("local", 0) >= 1 and bool(a and a["payload"].get("ok")) and empty,
                 {"avant": before, "apres": after, "ack": a and a["payload"]})

    def finish(self):
        self.set("NO-DOWNLOAD", self.downloads == 0, self.downloads)
        self.set("NO-EXCEPTION", not self.exceptions, self.exceptions[:5])
        for label in LABELS:
            if self.res[label] is None:
                self.res[label] = False
                self.notes.setdefault(label, "non joué")
        try:
            shot = self.chrome.call("Page.captureScreenshot", {"format": "png"}, self.page)
            with open(os.path.join(self.out_dir, "capture-pont-ehp.png"), "wb") as fh:
                fh.write(base64.b64decode(shot["data"]))
            with open(os.path.join(self.out_dir, "journal-pont-ehp.json"), "w", encoding="utf-8") as fh:
                json.dump(self.log(), fh, ensure_ascii=False, indent=1)
        except (RuntimeError, TimeoutError, OSError, ValueError):
            pass

    def close(self):
        self.chrome.close()
        for srv in (self.engine, self.parent):
            srv.shutdown()
            srv.server_close()
        shutil.rmtree(self.profile, ignore_errors=True)


def main(argv):
    if len(argv) != 5:
        print(__doc__)
        return 2
    site, fonts_dir, out_dir, report_path = argv[1:]
    os.makedirs(out_dir, exist_ok=True)
    chrome_bin = smoke.find_chrome()
    if not chrome_bin:
        print("::error::fumée pont-ehp impossible : Chrome introuvable")
        return 1
    banc = Banc(site, fonts_dir, out_dir, chrome_bin)
    try:
        try:
            banc.run()
        except (RuntimeError, TimeoutError, OSError, ValueError, KeyError) as e:
            banc.notes["INSTRUMENT"] = f"{type(e).__name__} : {e}"
        banc.finish()
    finally:
        banc.close()
    red = [label for label in LABELS if not banc.res[label]]
    banc.notes["REQUETES-MOTEUR"] = len(banc.requests)
    with open(os.path.join(out_dir, "result-pont-ehp.json"), "w", encoding="utf-8") as fh:
        json.dump({"labels": banc.res, "notes": banc.notes, "red": red}, fh, ensure_ascii=False, indent=1, default=str)
    with open(report_path, "a", encoding="utf-8") as rep:
        rep.write("### Fumée « pont-ehp » (pont hôte natif, bloquante)\n\n| Étiquette | Verdict | Note |\n|---|---|---|\n")
        for label in LABELS:
            note = json.dumps(banc.notes.get(label), ensure_ascii=False, default=str)[:300] if label in banc.notes else ""
            rep.write(f"| {label} | {'vert' if banc.res[label] else 'ROUGE'} | `{note}` |\n")
        if "INSTRUMENT" in banc.notes:
            rep.write(f"\nInstrument : `{banc.notes['INSTRUMENT']}`\n")
        rep.write(f"\nROUGES : {', '.join(red) or 'aucun'}\n\n")
    print("ROUGES : " + (", ".join(red) or "aucun"))
    for label in LABELS:
        print(f"{'vert ' if banc.res[label] else 'ROUGE'} {label} {json.dumps(banc.notes.get(label), ensure_ascii=False, default=str)[:400]}")
    if red:
        print("::error::fumée pont-ehp : " + ", ".join(red))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
