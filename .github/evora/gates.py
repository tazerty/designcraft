#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Portes Evora du build web (fork tazerty/designcraft, branche evora/<tag>) ; bibliothèque standard.

Écrit par Evora (patch EVORA-CIC-00). Sous-commandes :
  hosting PKG REPORT             BLOQUANTE : aucun fichier d'hébergement dans l'artefact.
  lock-sources LOCK EXPECTED REPORT
                                 BLOQUANTE : les sources git de Cargo.lock sont EXACTEMENT celles de
                                 EXPECTED (source complète, révision comprise).
  brands PKG REPORT [--strict]   marques ArtCraft (artcraft, getartcraft, discord.gg, casse ignorée) ET nom
                                 amont « DesignCraft » (casse exacte : les crates « designcraft_* » du .wasm
                                 restent hors champ) dans tout le bundle hors LICENSES/ (rapport en F0 ;
                                 --strict dès EVORA-CIC-01 pour les marques, dès EVORA-CIC-18-g pour le nom
                                 amont, couches moteur L0 à L5 comprises : exceptions X-EXP-01 et X-NOM-01).
  brands-ui REPO PKG REPORT      BLOQUANTE (EVORA-CIC-01) : 0 « DesignCraft » dans les chaînes littérales des
                                 couches d'affichage (L6, L7, hors tests) et dans index.html et le chargeur
                                 (garde à la source, avant la construction du bundle).
  links REPO REPORT              BLOQUANTE (EVORA-CIC-05) : aucune URL http(s) dans les chaînes littérales des
                                 couches d'affichage, hors schéma nu (« https:// », champ de saisie vide).
  storage-guard MANIFEST WORKSPACE REPORT
                                 BLOQUANTE (EVORA-CIC-04) : ni fonction de stockage de web-sys (Storage,
                                 IndexedDB, Cache Storage, gestionnaire de stockage, système de fichiers) ni
                                 « persistence » d'eframe pour la crate web.
  terms PKG TERMS REPORT [--strict]
                                 termes de la liste TERMS (mot entier, casse ignorée) dans TOUT le bundle, .wasm
                                 compris : rapport (identifiants fonctionnels des formats : types MIME, espaces de
                                 noms XMP, en-têtes PostScript, jamais affichés).
  terms-ui REPO TERMS ADMITTED REPORT
                                 BLOQUANTE (EVORA-CIC-11) : 0 terme dans les chaînes littérales des couches
                                 d'affichage (L6, L7, hors tests et hors tables de langue, que la fumée juge
                                 telles que servies), sauf un littéral admis un par un (ADMITTED) ou une clé du
                                 catalogue de surcharge qui a sa ligne anglaise.
  terms-page PKG TERMS REPORT     BLOQUANTE (EVORA-CIC-11) : 0 terme dans la page servie hors .wasm (index.html,
                                 chargeur, feuilles, données).
  emojis REPO REPORT [--strict]  émojis des chaînes littérales des sources d'interface (rapport en F0).
  icons REPO EXPECTED REPORT     BLOQUANTE (EVORA-CIC-22) : les pictogrammes d'Evora (crates/ui-egui/src/evora/icons.rs)
                                 sont exactement ceux d'EXPECTED, chacun un nom que les dessins amont connaissent, et
                                 icons::paint les demande à Evora avant tout dessin amont (crochet en tête).
  sizes PKG REPORT               taille du .wasm (brut, .br, .gz) et du site.
  compress PKG                   .br (brotli -q 11) et .gz (gzip -9 -n) à côté de chaque ressource web.
  csp PKG OUT                    empreintes sha256 des scripts inline d'index.html (CSP script-src).
  witness                        chaque détecteur est montré sur une matière fabriquée : il doit voir
                                 ce qu'on y a planté et se taire sur la matière propre.
Statut 1 quand une porte bloquante (ou --strict) échoue, 2 sur un usage invalide.
"""

import base64
import hashlib
import os
import re
import subprocess
import sys
import tempfile

LICENCES_DIR = "LICENSES"
COMPRESSED = (".br", ".gz")
WEB_SUFFIXES = (".wasm", ".js", ".html", ".css", ".svg", ".json", ".map")
HOSTING_NAMES = {".htaccess", "_headers", "_redirects", "web.config", ".user.ini", "HOSTING.md",
                 ".nojekyll", "netlify.toml", "vercel.json", "Caddyfile", "staticwebapp.config.json"}
HOSTING_SUFFIXES = (".conf", ".php", ".phtml", ".cgi")
# Marques à retirer par E1 (casse ignorée) ; le nom amont affiché est compté à part, casse exacte,
# car les noms de crates « designcraft_* » du .wasm sont hors champ (UPSTREAM.md, CIC-01).
BRANDS = ("artcraft", "getartcraft", "discord.gg")
UPSTREAM_NAME = "DesignCraft"
UI_SOURCE_DIRS = ("crates/ui-egui/src", "apps/designcraft-web/src")
STRING_LITERAL = re.compile(r'"((?:[^"\\\n]|\\.)*)"')
# Tables de langue (EVORA-CIC-11) : jugées telles que servies, dans chaque langue, par la fumée (diag.labels).
I18N_TABLES = ("crates/ui-egui/src/i18n.rs", "crates/ui-egui/src/i18n/")
OVERRIDES = "crates/ui-egui/src/i18n/evora/overrides.rs"
OVERRIDE_ROW = re.compile(r'\(\s*"((?:[^"\\]|\\.)*)",\s*"([a-z-]*)",\s*"((?:[^"\\]|\\.)*)",?\s*\)', re.S)
INLINE_SCRIPT = re.compile(r"<script\b([^>]*)>(.*?)</script\s*>", re.S | re.I)
LOCK_SOURCE = re.compile(r'^source = "(git\+[^"]+)"\s*$', re.M)
URL = re.compile(r"https?://[^\s\"]+")
# Pictogrammes d'Evora (EVORA-CIC-22) : la table d'Evora, et le crochet en tête des dessins amont.
ICONS_UPSTREAM = "crates/ui-egui/src/icons.rs"
ICONS_EVORA = "crates/ui-egui/src/evora/icons.rs"
ICONS_SIGNATURE = "pub fn paint(p: &Painter, r: Rect, name: &str, c: Color32) {\n"
ICONS_HOOK = "    if crate::evora::icons::paint(p, r, name, c) {\n        return;\n    }\n"
EVORA_ICON_ROW = re.compile(r'^    \("([a-z0-9-]+)", &\[$', re.M)
UPSTREAM_ICON_NAME = re.compile(r'"([a-z0-9-]+)"(?=\s*(?:\||=>))')
# Fonctions de web-sys qui donnent un stockage au moteur (EVORA-CIC-04).
STORAGE_FEATURES = ("Storage", "StorageManager", "StorageEstimate", "CacheStorage", "Cache", "IdbFactory", "IdbDatabase",
                    "IdbOpenDbRequest", "IdbTransaction", "IdbObjectStore", "FileSystemDirectoryHandle",
                    "FileSystemFileHandle", "FileSystemHandle", "FileSystemWritableFileStream", "FileSystemSyncAccessHandle")


def is_emoji_like(cp):
    """Émojis et pictogrammes (directive « zéro émoji d'interface »), même règle que les autres forks.

    Blocs entiers U+1F000-1FAFF, U+2600-27BF (dont U+26A0 et U+2713), U+2B00-2BFF, sélecteur U+FE0F,
    cabochon U+20E3, et quelques Extended_Pictographic isolés. Les signes typographiques restent permis.
    """
    if 0x1F000 <= cp <= 0x1FAFF or 0x2600 <= cp <= 0x27BF or 0x2B00 <= cp <= 0x2BFF:
        return True
    if cp in (0xFE0F, 0x20E3, 0x203C, 0x2049, 0x2139, 0x24C2, 0x231A, 0x231B, 0x2328, 0x23CF,
              0x25B6, 0x25C0, 0x2934, 0x2935, 0x3030, 0x303D, 0x3297, 0x3299):
        return True
    return 0x23E9 <= cp <= 0x23F3 or 0x23F8 <= cp <= 0x23FA or 0x25FB <= cp <= 0x25FE


class Report:
    def __init__(self, path):
        self.fh = open(path, "a", encoding="utf-8") if path else None

    def line(self, text=""):
        print(text)
        if self.fh:
            self.fh.write(text + "\n")

    def close(self):
        if self.fh:
            self.fh.close()


def walk_files(root):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for name in sorted(filenames):
            yield os.path.join(dirpath, name)


def rel(path, root):
    return os.path.relpath(path, root).replace(os.sep, "/")


def bundle_files(pkg):
    """Fichiers servis, hors dérivés compressés (leur clair est compté) et hors LICENSES/."""
    for path in walk_files(pkg):
        r = rel(path, pkg)
        # Un lien n'est jamais suivi (il compterait deux fois sa cible) : la porte hébergement le refuse.
        if os.path.islink(path) or r.endswith(COMPRESSED) or r.split("/", 1)[0] == LICENCES_DIR:
            continue
        yield path, r


def find_hosting(pkg):
    found = []
    for dirpath, dirnames, filenames in os.walk(pkg):
        for name in sorted(dirnames + filenames):
            full = os.path.join(dirpath, name)
            if os.path.islink(full):
                found.append(rel(full, pkg) + " (lien)")
            elif name in HOSTING_NAMES or name.endswith(HOSTING_SUFFIXES):
                found.append(rel(full, pkg))
    return sorted(found)


def cmd_hosting(pkg, report_path):
    rep = Report(report_path)
    if not os.path.isfile(os.path.join(pkg, "index.html")):
        rep.line(f"::error::porte hébergement : {pkg}/index.html absent, rien à juger")
        rep.close()
        return 1
    found = find_hosting(pkg)
    rep.line("### Porte des fichiers d'hébergement (bloquante)")
    rep.line()
    rep.line(f"Noms interdits ({', '.join(sorted(HOSTING_NAMES))}, *{', *'.join(HOSTING_SUFFIXES)}, liens) : "
             f"**{len(found)}** (attendu : 0) sur {sum(1 for _ in walk_files(pkg))} fichiers.")
    for r in found:
        rep.line(f"- `{r}`")
    rep.line()
    rep.close()
    if found:
        print("::error::fichier d'hébergement dans l'artefact : " + ", ".join(found))
        return 1
    print("::notice::fichiers d'hébergement : 0")
    return 0


def lock_sources(lock_path):
    with open(lock_path, encoding="utf-8") as fh:
        return sorted(set(LOCK_SOURCE.findall(fh.read())))


def read_expected(path):
    with open(path, encoding="utf-8") as fh:
        return sorted({l.strip() for l in fh if l.strip() and not l.lstrip().startswith("#")})


def cmd_lock_sources(lock_path, expected_path, report_path):
    have, want = lock_sources(lock_path), read_expected(expected_path)
    extra = [s for s in have if s not in want]
    missing = [s for s in want if s not in have]
    rep = Report(report_path)
    rep.line("### Sources git de Cargo.lock (bloquante)")
    rep.line()
    rep.line(f"Sources git verrouillées : **{len(have)}** ; attendues (`{os.path.basename(expected_path)}`) : "
             f"**{len(want)}** ; en trop : **{len(extra)}** ; manquantes : **{len(missing)}**.")
    for s in extra:
        rep.line(f"- en trop : `{s}`")
    for s in missing:
        rep.line(f"- manquante : `{s}`")
    rep.line()
    rep.close()
    if not want:
        print("::error::liste des sources attendues vide : rien à juger")
        return 1
    if extra or missing:
        print(f"::error::sources git du verrou différentes de la liste admise ({len(extra)} en trop, {len(missing)} manquantes)")
        return 1
    print(f"::notice::sources git du verrou : {len(have)}, toutes admises à leur révision")
    return 0


def count_in_bundle(pkg, needles, fold):
    per_file, total = {}, {n: 0 for n in needles}
    for path, r in bundle_files(pkg):
        with open(path, "rb") as fh:
            data = fh.read()
        if fold:
            data = data.lower()
        counts = {n: data.count((n.lower() if fold else n).encode("utf-8")) for n in needles}
        if any(counts.values()):
            per_file[r] = counts
        for n in needles:
            total[n] += counts[n]
    return per_file, total


def table(rep, per_file, needles):
    if not per_file:
        rep.line("Aucune occurrence.")
        return
    rep.line("| Fichier | " + " | ".join(f"`{n}`" for n in needles) + " |")
    rep.line("|---|" + "---|" * len(needles))
    for r, counts in per_file.items():
        rep.line(f"| `{r}` | " + " | ".join(str(counts[n]) for n in needles) + " |")


def gate_verdict(name, total, strict, rep, scope=f"hors {LICENCES_DIR}/ du bundle"):
    n = sum(total.values())
    mode = "bloquante" if strict else "rapport (non bloquante)"
    rep.line()
    rep.line(f"Total {scope} : **{n}** (porte en mode {mode}).")
    rep.line()
    if n and strict:
        print(f"::error::porte {name} : {n} occurrence(s)")
        return 1
    if n:
        print(f"::warning::porte {name} (rapport) : {n} occurrence(s)")
    else:
        print(f"::notice::porte {name} : 0 occurrence")
    return 0


def cmd_brands(pkg, report_path, strict):
    rep = Report(report_path)
    rep.line("### Marques et nom amont dans le bundle (" + ("bloquante" if strict else "rapport : E1 n'est pas encore posé") + ")")
    rep.line()
    per_file, total = count_in_bundle(pkg, BRANDS, fold=True)
    table(rep, per_file, BRANDS)
    named_files, named = count_in_bundle(pkg, (UPSTREAM_NAME,), fold=False)
    rep.line()
    rep.line(f"Nom amont `{UPSTREAM_NAME}` (casse exacte, hors {LICENCES_DIR}/) : {named[UPSTREAM_NAME]} occurrence(s) dans "
             f"{len(named_files)} fichier(s) ; il compte dans la porte (EVORA-CIC-18-g).")
    for r, counts in named_files.items():
        rep.line(f"- `{r}` : {counts[UPSTREAM_NAME]}")
    status = gate_verdict("marques", {**total, **named}, strict, rep)
    rep.close()
    return status


def ui_literals(repo):
    """(fichier, littéral) des chaînes des sources d'affichage, hors tests et commentaires de ligne."""
    for base in UI_SOURCE_DIRS:
        for path in walk_files(os.path.join(repo, base)):
            name = os.path.basename(path)
            if not name.endswith(".rs") or name.startswith("tests") or "/tests/" in path.replace(os.sep, "/"):
                continue
            with open(path, encoding="utf-8") as fh:
                text = strip_test_modules(fh.read())
            code = "\n".join(l for l in text.splitlines() if not l.lstrip().startswith("//"))
            for lit in STRING_LITERAL.findall(code):
                yield rel(path, repo), lit


def cmd_brands_ui(repo, pkg, report_path):
    rep = Report(report_path)
    rep.line("### Nom amont dans les couches d'affichage (bloquante, EVORA-CIC-01)")
    rep.line()
    hits = {}
    for r, lit in ui_literals(repo):
        n = lit.count(UPSTREAM_NAME)
        if n:
            hits[r] = hits.get(r, 0) + n
    served = {}
    for path, r in bundle_files(pkg):
        if r.endswith((".html", ".js")):
            with open(path, "rb") as fh:
                n = fh.read().count(UPSTREAM_NAME.encode())
            if n:
                served[r] = n
    for r, n in sorted(hits.items()):
        rep.line(f"- source `{r}` : {n}")
    for r, n in sorted(served.items()):
        rep.line(f"- servi `{r}` : {n}")
    total = sum(hits.values()) + sum(served.values())
    rep.line()
    rep.line(f"`{UPSTREAM_NAME}` dans les chaînes d'affichage, index.html et le chargeur : **{total}** (attendu : 0).")
    rep.line()
    rep.close()
    if total:
        print(f"::error::porte nom amont (affichage) : {total} occurrence(s)")
        return 1
    print("::notice::porte nom amont (affichage) : 0 occurrence")
    return 0


def outside_links(repo):
    found = []
    for r, lit in ui_literals(repo):
        for url in URL.findall(lit):
            if url not in ("http://", "https://"):
                found.append((r, url[:80]))
    return found


def cmd_links(repo, report_path):
    rep = Report(report_path)
    rep.line("### Liens sortants des couches d'affichage (bloquante, EVORA-CIC-05)")
    rep.line()
    found = outside_links(repo)
    for r, url in found:
        rep.line(f"- `{r}` : `{url}`")
    rep.line(f"URL dans les chaînes d'affichage (hors schéma nu) : **{len(found)}** (attendu : 0).")
    rep.line()
    rep.close()
    if found:
        print(f"::error::porte liens sortants : {len(found)} URL")
        return 1
    print("::notice::porte liens sortants : 0 URL")
    return 0


def toml_list(text, key):
    """Les chaînes de la liste `key = [ … ]` d'une ligne de dépendance TOML (approché, suffisant ici)."""
    out = []
    for m in re.finditer(r"%s\s*=\s*\[([^\]]*)\]" % re.escape(key), text):
        out += re.findall(r'"([^"]+)"', m.group(1))
    return out


def storage_findings(manifest_text, workspace_text):
    found = []
    for line in manifest_text.splitlines():
        if line.lstrip().startswith("web-sys"):
            found += [f"web-sys:{f}" for f in toml_list(line, "features") if f in STORAGE_FEATURES]
        if line.lstrip().startswith("eframe") and "persistence" in toml_list(line, "features"):
            found.append("eframe:persistence (crate web)")
    for line in workspace_text.splitlines():
        if line.lstrip().startswith("eframe") and "persistence" in toml_list(line, "features"):
            found.append("eframe:persistence (espace de travail)")
    return found


def cmd_storage_guard(manifest, workspace, report_path):
    with open(manifest, encoding="utf-8") as fh:
        mt = fh.read()
    with open(workspace, encoding="utf-8") as fh:
        wt = fh.read()
    found = storage_findings(mt, wt)
    rep = Report(report_path)
    rep.line("### Absence de stockage du moteur (bloquante, EVORA-CIC-04)")
    rep.line()
    rep.line(f"Fonctions de stockage de la crate web : **{len(found)}** (attendu : 0) ; " + (", ".join(found) or "aucune") + ".")
    rep.line()
    rep.close()
    if "web-sys" not in mt:
        print("::error::porte stockage : aucune ligne web-sys dans le manifeste, rien à juger")
        return 1
    if found:
        print("::error::porte stockage : " + ", ".join(found))
        return 1
    print("::notice::porte stockage : aucune fonction de stockage")
    return 0


def read_terms(path):
    with open(path, encoding="utf-8") as fh:
        terms = [l.rstrip("\n") for l in fh if l.strip() and not l.lstrip().startswith("#")]
    return [t.strip() for t in terms]


def term_patterns(terms):
    """Un motif par terme : mot entier (ni lettre, ni chiffre, ni _ autour), casse ignorée."""
    return [(t, re.compile(r"(?<!\w)" + re.escape(t) + r"(?!\w)", re.I)) for t in terms]


def term_hits(text, patterns):
    return [t for t, p in patterns if p.search(text)]


def override_keys(repo):
    """Clés du catalogue de surcharge qui ont leur ligne anglaise (« »), ou None sans catalogue."""
    path = os.path.join(repo, OVERRIDES)
    if not os.path.isfile(path):
        return None
    with open(path, encoding="utf-8") as fh:
        rows = OVERRIDE_ROW.findall(fh.read())
    return {key for key, lang, _ in rows if lang == ""}


def read_admitted(path):
    with open(path, encoding="utf-8") as fh:
        return {l.split("\t", 1)[0] for l in fh if l.strip() and not l.startswith("#") and "\t" in l}


def terms_in_ui(repo, patterns, admitted, keys):
    """(fichier, termes) des littéraux d'affichage qui portent un terme, hors tables, admis et clés surchargées."""
    found = []
    for r, lit in ui_literals(repo):
        if r == I18N_TABLES[0] or r.startswith(I18N_TABLES[1]) or lit in admitted or lit in keys:
            continue
        hits = term_hits(lit, patterns)
        if hits:
            found.append((r, hits))
    return found


def cmd_terms_ui(repo, terms_path, admitted_path, report_path):
    rep = Report(report_path)
    rep.line("### Termes dans les couches d'affichage (bloquante, EVORA-CIC-11)")
    rep.line()
    terms, keys = read_terms(terms_path), override_keys(repo)
    if not terms or keys is None or not keys:
        rep.line(f"::error::liste des termes vide ou catalogue de surcharge absent ({OVERRIDES}) : rien à juger")
        rep.close()
        print("::error::porte termes (affichage) : rien à juger")
        return 1
    admitted = read_admitted(admitted_path)
    found = terms_in_ui(repo, term_patterns(terms), admitted, keys)
    rep.line(f"Liste : `{os.path.basename(terms_path)}`, {len(terms)} terme(s), mot entier, casse ignorée ; "
             f"{len(keys)} clé(s) du catalogue de surcharge ; {len(admitted)} littéral(aux) admis un par un.")
    for r, hits in found:
        rep.line(f"- `{r}` : " + ", ".join(f"`{t}`" for t in hits))
    rep.line(f"Littéraux d'affichage porteurs d'un terme : **{len(found)}** (attendu : 0).")
    rep.line()
    rep.close()
    if found:
        print(f"::error::porte termes (affichage) : {len(found)} littéral(aux)")
        return 1
    print("::notice::porte termes (affichage) : 0 littéral")
    return 0


def terms_in_page(pkg, patterns):
    found = []
    for path, r in bundle_files(pkg):
        if r.endswith(".wasm"):
            continue
        with open(path, "rb") as fh:
            hits = term_hits(fh.read().decode("utf-8", "replace"), patterns)
        if hits:
            found.append((r, hits))
    return found


def cmd_terms_page(pkg, terms_path, report_path):
    rep = Report(report_path)
    rep.line("### Termes dans la page servie hors .wasm (bloquante, EVORA-CIC-11)")
    rep.line()
    terms = read_terms(terms_path)
    if not terms or not os.path.isfile(os.path.join(pkg, "index.html")):
        rep.line("::error::liste des termes vide ou index.html absent : rien à juger")
        rep.close()
        print("::error::porte termes (page) : rien à juger")
        return 1
    found = terms_in_page(pkg, term_patterns(terms))
    for r, hits in found:
        rep.line(f"- `{r}` : " + ", ".join(f"`{t}`" for t in hits))
    rep.line(f"Fichiers servis hors .wasm porteurs d'un terme : **{len(found)}** (attendu : 0).")
    rep.line()
    rep.close()
    if found:
        print(f"::error::porte termes (page) : {len(found)} fichier(s)")
        return 1
    print("::notice::porte termes (page) : 0 fichier")
    return 0


def cmd_terms(pkg, terms_path, report_path, strict):
    terms = read_terms(terms_path)
    rep = Report(report_path)
    rep.line("### Termes dans tout le bundle (" + ("bloquante" if strict else "rapport : identifiants fonctionnels du .wasm") + ")")
    rep.line()
    if not terms:
        rep.line(f"::error::liste des termes vide ({terms_path}) : rien à juger")
        rep.close()
        return 1
    rep.line(f"Liste : `{os.path.basename(terms_path)}`, {len(terms)} terme(s), sous-chaîne, casse ignorée.")
    rep.line()
    per_file, total = count_in_bundle(pkg, terms, fold=True)
    table(rep, per_file, terms)
    status = gate_verdict("termes", total, strict, rep)
    rep.close()
    return status


def strip_test_modules(text):
    """Retire les modules `#[cfg(test)] mod x { … }` (accolades comptées hors chaînes, approché)."""
    out, i = [], 0
    for m in re.finditer(r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{", text):
        if m.start() < i:
            continue
        out.append(text[i:m.start()])
        depth, j = 1, m.end()
        while j < len(text) and depth:
            depth += {"{": 1, "}": -1}.get(text[j], 0)
            j += 1
        i = j
    out.append(text[i:])
    return "".join(out)


def emoji_in_sources(repo):
    per_file = {}
    for base in UI_SOURCE_DIRS:
        root = os.path.join(repo, base)
        for path in walk_files(root):
            name = os.path.basename(path)
            if not name.endswith(".rs") or name.startswith("tests") or "/tests/" in path.replace(os.sep, "/"):
                continue
            with open(path, encoding="utf-8") as fh:
                text = strip_test_modules(fh.read())
            code = "\n".join(l for l in text.splitlines() if not l.lstrip().startswith("//"))
            n = sum(1 for lit in STRING_LITERAL.findall(code) for ch in lit if is_emoji_like(ord(ch)))
            if n:
                per_file[rel(path, repo)] = n
    return per_file


def cmd_emojis(repo, report_path, strict):
    roots = [os.path.join(repo, b) for b in UI_SOURCE_DIRS]
    rep = Report(report_path)
    rep.line("### Émojis des chaînes d'interface (" + ("bloquante" if strict else "rapport : E10 n'est pas encore posé") + ")")
    rep.line()
    if not all(os.path.isdir(r) for r in roots):
        rep.line("::error::dossiers de sources d'interface introuvables : " + ", ".join(UI_SOURCE_DIRS))
        rep.close()
        return 1
    per_file = emoji_in_sources(repo)
    rep.line("Chaînes littérales des sources (`" + "`, `".join(UI_SOURCE_DIRS) + "`), hors tests et commentaires "
             "de ligne ; mesure approchée par le texte, le rendu fera foi (E10).")
    rep.line()
    for r, n in sorted(per_file.items()):
        rep.line(f"- `{r}` : {n}")
    status = gate_verdict("émojis", {"emoji": sum(per_file.values())}, strict, rep, scope="des chaînes littérales")
    rep.close()
    return status


def icon_findings(upstream_text, evora_text, expected):
    """Constats de la porte des pictogrammes (liste vide : conforme)."""
    found = []
    if not expected:
        found.append("liste attendue vide")
    start = upstream_text.find(ICONS_SIGNATURE)
    if start < 0:
        found.append("signature de icons::paint introuvable")
    else:
        lines = upstream_text[start + len(ICONS_SIGNATURE):].splitlines(keepends=True)
        k = 0
        while k < len(lines) and lines[k].lstrip().startswith("//"):
            k += 1
        if "".join(lines[k:k + 3]) != ICONS_HOOK:
            found.append("crochet d'Evora absent en tête de icons::paint")
    names = EVORA_ICON_ROW.findall(evora_text)
    doubles = sorted({n for n in names if names.count(n) > 1})
    if doubles:
        found.append("en double : " + ", ".join(doubles))
    missing = [n for n in expected if n not in names]
    if missing:
        found.append("manquants dans la table d'Evora : " + ", ".join(missing))
    extra = sorted(n for n in set(names) if n not in expected)
    if extra:
        found.append("hors liste : " + ", ".join(extra))
    upstream = set(UPSTREAM_ICON_NAME.findall(strip_test_modules(upstream_text)))
    unknown = [n for n in expected if n not in upstream]
    if unknown:
        found.append("inconnus des dessins amont : " + ", ".join(unknown))
    return found


def cmd_icons(repo, expected_path, report_path):
    rep = Report(report_path)
    rep.line("### Pictogrammes d'Evora (bloquante, EVORA-CIC-22)")
    rep.line()
    expected = read_terms(expected_path)
    texts = []
    for r in (ICONS_UPSTREAM, ICONS_EVORA):
        path = os.path.join(repo, r)
        texts.append(open(path, encoding="utf-8").read() if os.path.isfile(path) else "")
    found = icon_findings(texts[0], texts[1], expected)
    for f in found:
        rep.line(f"- {f}")
    rep.line(f"Pictogrammes attendus : {len(expected)} ; dessinés par Evora : {len(EVORA_ICON_ROW.findall(texts[1]))} ; "
             f"constats : **{len(found)}** (attendu : 0).")
    rep.line()
    rep.close()
    if found:
        print(f"::error::porte pictogrammes : {len(found)} constat(s) : " + " ; ".join(found)[:400])
        return 1
    print(f"::notice::porte pictogrammes : {len(expected)} pictogrammes d'Evora, crochet en tête")
    return 0


def cmd_sizes(pkg, report_path):
    rep = Report(report_path)
    rep.line("### Taille (rapport ; ligne de base, aucun budget arbitré)")
    rep.line()
    rep.line("| Fichier | Brut (octets) | .br (brotli -q 11) | .gz (gzip -9) |")
    rep.line("|---|---|---|---|")
    wasm = 0
    for path in walk_files(pkg):
        if path.endswith((".wasm", ".js", ".html")):
            br = os.path.getsize(path + ".br") if os.path.exists(path + ".br") else "n/d"
            gz = os.path.getsize(path + ".gz") if os.path.exists(path + ".gz") else "n/d"
            rep.line(f"| `{rel(path, pkg)}` | {os.path.getsize(path)} | {br} | {gz} |")
            wasm += path.endswith(".wasm")
            if path.endswith(".wasm"):
                print(f"::notice::{rel(path, pkg)} : {os.path.getsize(path)} octets bruts, {br} en .br, {gz} en .gz")
    files = list(walk_files(pkg))
    plain = sum(os.path.getsize(p) for p in files if not p.endswith(COMPRESSED))
    rep.line()
    rep.line(f"Site : {plain} octets sans les précompressés, {sum(os.path.getsize(p) for p in files)} avec ; "
             f"{len(files)} fichiers ; {wasm} module(s) .wasm.")
    rep.line()
    rep.close()
    return 0 if wasm == 1 else 1


def cmd_compress(pkg):
    n = 0
    for path in list(walk_files(pkg)):
        r = rel(path, pkg)
        if r.split("/", 1)[0] == LICENCES_DIR or not r.endswith(WEB_SUFFIXES):
            continue
        subprocess.run(["brotli", "-q", "11", "-f", "-o", path + ".br", path], check=True)
        with open(path + ".gz", "wb") as out:
            subprocess.run(["gzip", "-9", "-n", "-c", path], check=True, stdout=out)
        n += 1
    print(f"::notice::{n} ressource(s) précompressée(s) en .br et .gz")
    return 0 if n else 1


def csp_hashes(html):
    hashes = []
    for attrs, body in INLINE_SCRIPT.findall(html):
        if re.search(r"\bsrc\s*=", attrs, re.I):
            continue
        hashes.append("sha256-" + base64.b64encode(hashlib.sha256(body.encode("utf-8")).digest()).decode())
    return hashes


def cmd_csp(pkg, out_path):
    with open(os.path.join(pkg, "index.html"), encoding="utf-8") as fh:
        hashes = csp_hashes(fh.read())
    if not hashes:
        print("::error::aucun script inline dans index.html : chargeur Trunk inattendu")
        return 1
    with open(out_path, "w", encoding="utf-8") as fh:
        fh.write("".join(h + "\n" for h in hashes))
    for h in hashes:
        print(f"::notice::CSP script-src '{h}'")
    return 0


def witness():
    """Montre chaque détecteur sur une matière fabriquée ; statut 1 si l'un rate ou parle à tort."""
    results = []

    def expect(label, cond):
        results.append(cond)
        print(("TÉMOIN OK    " if cond else "TÉMOIN RATÉ  ") + label)

    def write(path, text):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text)

    with tempfile.TemporaryDirectory() as tmp:
        pkg = os.path.join(tmp, "pkg")
        write(os.path.join(pkg, "index.html"), '<script type="module">init()</script><script src="a.js"></script>')
        write(os.path.join(pkg, "a.js"), "console.log('ArtCraft Primary Text Frame');")
        write(os.path.join(pkg, LICENCES_DIR, "NOTICE"), "artcraft DesignCraft")
        expect("hébergement : matière propre, 0 trouvé", find_hosting(pkg) == [])
        for name in (".htaccess", "_headers", "sub/site.conf"):
            write(os.path.join(pkg, name), "x")
        os.symlink("a.js", os.path.join(pkg, "lien.js"))
        expect("hébergement : .htaccess, _headers, *.conf et lien plantés, 4 trouvés",
               find_hosting(pkg) == [".htaccess", "_headers", "lien.js (lien)", "sub/site.conf"])
        expect("hébergement : porte rouge sur la matière plantée", cmd_hosting(pkg, None) == 1)

        _, brands = count_in_bundle(pkg, BRANDS, fold=True)
        expect("marques : 1 « ArtCraft » planté hors LICENSES/, celui de LICENSES/ ignoré", brands["artcraft"] == 1)
        _, terms = count_in_bundle(pkg, ("Primary Text Frame",), fold=False)
        expect("termes : « Primary Text Frame » planté, 1 trouvé", terms["Primary Text Frame"] == 1)
        expect("marques --strict : porte rouge", cmd_brands(pkg, None, True) == 1)
        expect("marques en rapport : porte verte malgré l'occurrence", cmd_brands(pkg, None, False) == 0)

        # Nom amont (EVORA-CIC-18-g) : un bundle sans marque, dont seul le nom amont décide.
        pkg_nom = os.path.join(tmp, "pkg-nom")
        write(os.path.join(pkg_nom, "index.html"), "<title>Cicero</title>")
        write(os.path.join(pkg_nom, "c.js"), "import * as designcraft_engine from './x.js'; // Cicero")
        write(os.path.join(pkg_nom, LICENCES_DIR, "NOTICE"), "Cicero, based on DesignCraft")
        expect("nom amont : casse exacte (designcraft_*) et LICENSES/ ignorés, porte stricte verte",
               cmd_brands(pkg_nom, None, True) == 0)
        with open(os.path.join(pkg_nom, "m.wasm"), "wb") as fh:
            fh.write(b"\x00asm\x01\x00\x00\x00 producer DesignCraft \x00")
        _, named = count_in_bundle(pkg_nom, (UPSTREAM_NAME,), fold=False)
        expect("nom amont : 1 « DesignCraft » planté dans un .wasm, 1 trouvé", named[UPSTREAM_NAME] == 1)
        expect("nom amont --strict : porte rouge sur le seul nom amont", cmd_brands(pkg_nom, None, True) == 1)
        expect("CSP : une empreinte pour le seul script inline",
               csp_hashes('<script type="module">init()</script><script src="a.js"></script>')
               == ["sha256-" + base64.b64encode(hashlib.sha256(b"init()").digest()).decode()])

        lock = os.path.join(tmp, "Cargo.lock")
        src_a = "git+https://github.com/o/a?rev=1111111111111111111111111111111111111111#1111111111111111111111111111111111111111"
        src_b = "git+https://github.com/o/b?branch=main#2222222222222222222222222222222222222222"
        write(lock, f'[[package]]\nname = "a"\nsource = "{src_a}"\n\n[[package]]\nname = "b"\nsource = "{src_b}"\n'
                    '\n[[package]]\nname = "c"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n')
        exp = os.path.join(tmp, "expected.txt")
        write(exp, f"# commentaire\n{src_a}\n{src_b}\n")
        expect("sources du verrou : liste identique, porte verte", cmd_lock_sources(lock, exp, None) == 0)
        write(exp, f"{src_a}\n")
        expect("sources du verrou : source non admise, porte rouge", cmd_lock_sources(lock, exp, None) == 1)
        write(exp, f"{src_a}\n{src_b.replace('2222', '3333', 1)}\n")
        expect("sources du verrou : révision changée, porte rouge", cmd_lock_sources(lock, exp, None) == 1)
        write(exp, "# vide\n")
        expect("sources du verrou : liste vide, porte rouge (rien à juger)", cmd_lock_sources(lock, exp, None) == 1)

        repo = os.path.join(tmp, "repo")
        write(os.path.join(repo, UI_SOURCE_DIRS[0], "a.rs"),
              'fn f() -> &str { "\u26a0 x" }\n// "\u2713" commentaire\n'
              '#[cfg(test)]\nmod tests {\n    fn t() { let _ = "\u2713 {"; }\n}\nfn g() -> &str { "ok" }\n')
        write(os.path.join(repo, UI_SOURCE_DIRS[1], "tests_x.rs"), 'fn t() { "\u2705"; }\n')
        expect("émojis : 1 compté (littéral), commentaire, module de test et fichier de test ignorés",
               emoji_in_sources(repo) == {f"{UI_SOURCE_DIRS[0]}/a.rs": 1})
        expect("émojis --strict : porte rouge", cmd_emojis(repo, None, True) == 1)

        write(os.path.join(repo, UI_SOURCE_DIRS[0], "b.rs"),
              'fn f() -> &str { "Quit DesignCraft" }\nfn g() -> &str { "https://example.org/x" }\n'
              'fn h() -> &str { "https://" }\n// "https://commentaire.example"\n')
        expect("nom amont : 1 « DesignCraft » littéral trouvé", cmd_brands_ui(repo, pkg, None) == 1)
        expect("liens : 1 URL trouvée, schéma nu et commentaire ignorés",
               outside_links(repo) == [(f"{UI_SOURCE_DIRS[0]}/b.rs", "https://example.org/x")])
        write(os.path.join(repo, UI_SOURCE_DIRS[0], "b.rs"), 'fn h() -> &str { "https://" }\n')
        expect("liens : matière propre, porte verte", cmd_links(repo, None) == 0)
        clean_ws = 'eframe = { version = "0.36", default-features = false }\n'
        clean = 'web-sys = { version = "0.3", features = ["Window", "MessageEvent"] }\neframe = { workspace = true, features = ["wgpu"] }\n'
        expect("stockage : manifeste propre, 0 trouvé", storage_findings(clean, clean_ws) == [])
        dirty = clean.replace('"MessageEvent"', '"MessageEvent", "Storage", "IdbFactory"')
        expect("stockage : Storage et IdbFactory plantés, 2 trouvés",
               storage_findings(dirty, clean_ws) == ["web-sys:Storage", "web-sys:IdbFactory"])
        # Termes (EVORA-CIC-11) : mot entier, casse ignorée ; tables, tests, admis et clés surchargées à part.
        pats = term_patterns(["Primary Text Frame", "Canva", "Premiere"])
        expect("termes : mot entier (« Canvas », « première » et « PrimaryTextFrame » ignorés)",
               term_hits("Canvas première PrimaryTextFrame", pats) == [])
        expect("termes : casse ignorée (« primary text frame's » trouvé)", term_hits("the primary text frame's story", pats) == ["Primary Text Frame"])
        write(os.path.join(repo, UI_SOURCE_DIRS[0], "c.rs"),
              'fn f() -> &str { "Open in Premiere" }\nfn g() -> &str { "Primary Text Frame" }\n'
              'fn h() -> &str { "application/x-canva" }\n// "Canva" commentaire\n')
        write(os.path.join(repo, OVERRIDES), '    ("Primary Text Frame", "", "Main Text Frame"),\n    ("Open in Premiere", "fr", "x"),\n')
        write(os.path.join(repo, I18N_TABLES[0]), 'const T: &[(&str, &str)] = &[("Premiere", "Premiere")];\n')
        keys = override_keys(repo)
        expect("termes : clés du catalogue avec ligne anglaise seulement", keys == {"Primary Text Frame"})
        expect("termes : 1 littéral trouvé (clé sans ligne anglaise), clé surchargée, admis, table et commentaire ignorés",
               terms_in_ui(repo, pats, {"application/x-canva"}, keys) == [(f"{UI_SOURCE_DIRS[0]}/c.rs", ["Premiere"])])
        expect("termes : le littéral admis retiré de la liste, trouvé",
               len(terms_in_ui(repo, pats, set(), keys)) == 2)
        pkg_t = os.path.join(tmp, "pkg-termes")
        write(os.path.join(pkg_t, "index.html"), "<title>Cicero</title>")
        with open(os.path.join(pkg_t, "m.wasm"), "wb") as fh:
            fh.write(b"\x00asm image/vnd.adobe.photoshop Premiere")
        expect("termes (page) : .wasm ignoré, page propre", terms_in_page(pkg_t, pats) == [])
        write(os.path.join(pkg_t, "a.js"), "label('Premiere')")
        expect("termes (page) : 1 terme planté dans le chargeur, trouvé", terms_in_page(pkg_t, pats) == [("a.js", ["Premiere"])])
        expect("stockage : persistence d'eframe plantée, trouvée",
               storage_findings(clean.replace('"wgpu"', '"wgpu", "persistence"'), clean_ws) == ["eframe:persistence (crate web)"])
        # Pictogrammes (EVORA-CIC-22) : crochet en tête, table d'Evora égale à la liste, noms connus de l'amont.
        up = (ICONS_SIGNATURE + "    // Evora build.\n" + ICONS_HOOK + '    match name {\n        "trash" => {}\n'
              '        "eye" | "lock" => {}\n        _ => {}\n    }\n}\n')
        ev = 'const ICONS: &[(&str, &[Sub])] = &[\n    ("trash", &[\n    ]),\n    ("eye", &[\n    ]),\n];\n'
        expect("pictogrammes : matière propre, 0 constat", icon_findings(up, ev, ["trash", "eye"]) == [])
        expect("pictogrammes : crochet neutralisé, trouvé",
               icon_findings(up.replace("    if crate::", "    if false && crate::"), ev, ["trash", "eye"])
               == ["crochet d'Evora absent en tête de icons::paint"])
        expect("pictogrammes : un pictogramme retiré de la table, trouvé",
               icon_findings(up, ev.replace('    ("eye", &[\n    ]),\n', ""), ["trash", "eye"])
               == ["manquants dans la table d'Evora : eye"])
        expect("pictogrammes : un nom que l'amont ne dessine pas, trouvé",
               icon_findings(up, ev + '    ("brand-logo", &[\n', ["trash", "eye", "brand-logo"])
               == ["inconnus des dessins amont : brand-logo"])
    ok = all(results)
    print(f"Témoins des détecteurs : {sum(results)}/{len(results)} conformes")
    return 0 if ok else 1


def main(argv):
    args = [a for a in argv[1:] if a != "--strict"]
    strict = "--strict" in argv[1:]
    commands = {
        "hosting": (2, lambda a: cmd_hosting(a[0], a[1])),
        "lock-sources": (3, lambda a: cmd_lock_sources(a[0], a[1], a[2])),
        "brands": (2, lambda a: cmd_brands(a[0], a[1], strict)),
        "brands-ui": (3, lambda a: cmd_brands_ui(a[0], a[1], a[2])),
        "links": (2, lambda a: cmd_links(a[0], a[1])),
        "storage-guard": (3, lambda a: cmd_storage_guard(a[0], a[1], a[2])),
        "terms": (3, lambda a: cmd_terms(a[0], a[1], a[2], strict)),
        "terms-ui": (4, lambda a: cmd_terms_ui(a[0], a[1], a[2], a[3])),
        "terms-page": (3, lambda a: cmd_terms_page(a[0], a[1], a[2])),
        "emojis": (2, lambda a: cmd_emojis(a[0], a[1], strict)),
        "icons": (3, lambda a: cmd_icons(a[0], a[1], a[2])),
        "sizes": (2, lambda a: cmd_sizes(a[0], a[1])),
        "compress": (1, lambda a: cmd_compress(a[0])),
        "csp": (2, lambda a: cmd_csp(a[0], a[1])),
        "witness": (0, lambda a: witness()),
    }
    if not args or args[0] not in commands or len(args) - 1 != commands[args[0]][0]:
        print(__doc__)
        return 2
    return commands[args[0]][1](args[1:])


if __name__ == "__main__":
    sys.exit(main(sys.argv))
