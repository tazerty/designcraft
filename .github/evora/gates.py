#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Portes Evora du build web (fork tazerty/designcraft, branche evora/<tag>) ; bibliothèque standard.

Écrit par Evora (patch EVORA-CIC-00). Sous-commandes :
  hosting PKG REPORT             BLOQUANTE : aucun fichier d'hébergement dans l'artefact.
  lock-sources LOCK EXPECTED REPORT
                                 BLOQUANTE : les sources git de Cargo.lock sont EXACTEMENT celles de
                                 EXPECTED (source complète, révision comprise).
  brands PKG REPORT [--strict]   marques ArtCraft et nom amont dans le bundle (rapport en F0 ;
                                 --strict à partir du patch E1).
  terms PKG TERMS REPORT [--strict]
                                 termes sensibles de la liste TERMS dans le bundle (rapport en F0).
  emojis REPO REPORT [--strict]  émojis des chaînes littérales des sources d'interface (rapport en F0).
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
INLINE_SCRIPT = re.compile(r"<script\b([^>]*)>(.*?)</script\s*>", re.S | re.I)
LOCK_SOURCE = re.compile(r'^source = "(git\+[^"]+)"\s*$', re.M)


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
    rep.line("### Marques (" + ("bloquante" if strict else "rapport : E1 n'est pas encore posé") + ")")
    rep.line()
    per_file, total = count_in_bundle(pkg, BRANDS, fold=True)
    table(rep, per_file, BRANDS)
    named_files, named = count_in_bundle(pkg, (UPSTREAM_NAME,), fold=False)
    rep.line()
    rep.line(f"Nom amont `{UPSTREAM_NAME}` (casse exacte) : {named[UPSTREAM_NAME]} occurrence(s) dans "
             f"{len(named_files)} fichier(s) ; il compte dans la porte à partir d'E1.")
    status = gate_verdict("marques", total, strict, rep)
    rep.close()
    return status


def read_terms(path):
    with open(path, encoding="utf-8") as fh:
        terms = [l.rstrip("\n") for l in fh if l.strip() and not l.lstrip().startswith("#")]
    return [t.strip() for t in terms]


def cmd_terms(pkg, terms_path, report_path, strict):
    terms = read_terms(terms_path)
    rep = Report(report_path)
    rep.line("### Termes sensibles (" + ("bloquante" if strict else "rapport : catalogue de surcharge E9 non posé") + ")")
    rep.line()
    if not terms:
        rep.line(f"::error::liste des termes vide ({terms_path}) : rien à juger")
        rep.close()
        return 1
    rep.line(f"Liste : `{os.path.basename(terms_path)}`, {len(terms)} terme(s), casse exacte.")
    rep.line()
    per_file, total = count_in_bundle(pkg, terms, fold=False)
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
        "terms": (3, lambda a: cmd_terms(a[0], a[1], a[2], strict)),
        "emojis": (2, lambda a: cmd_emojis(a[0], a[1], strict)),
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
