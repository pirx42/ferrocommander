# Skill: Nichts erfinden — verifizieren oder nachfragen

**Wann.** Immer, wenn eine inhaltliche Aussage ueber Spielverhalten, Code
oder Anforderungen gemacht wird (Doku, Handbuch, Telegram-Antworten,
Commit-Beschreibungen) — und besonders, wenn sie „plausibel klingt".

**Regel.**

1. **Keine Aussage aus Annahme.** Jede Verhaltens-Behauptung wird vorher
   gegen die Implementierung verifiziert (Code lesen; docs/features.md
   und die Subsystem-Docs sind gepflegt und zitierfaehig).
2. **Bleibt es danach unklar → NACHFRAGEN statt raten** (Owner msg
   15477/15478: „wenn irgendetwas unklar ist, frage immer nach anstatt zu
   raten") — Rueckfrage per Telegram, idealerweise mit konkreten Optionen
   (Skill [01](01-clarify-with-options.md)).
3. Gilt genauso fuer Anforderungs-Luecken (Skill
   [09](09-scope-vor-impl.md)) und Memory-Inhalte (Skill
   [39](39-memory-verifizieren.md) — Memory beschreibt den Stand von
   damals, nicht notwendig den heutigen).

**Warum.** Vorfall 2026-07-18: das Spieler-Handbuch enthielt ~9 aus
Annahmen geschriebene Fehler (nicht existierender Kabel-Button,
„Munition", Infiltration exakt verkehrt herum, falsche Recycle- und
Run-Ende-Semantik, „Topologie egal" …). Jede einzelne Annahme haette ein
Code-Blick oder eine Rueckfrage verhindert — die Korrektur-Runde kostete
mehr als die Verifikation je gekostet haette. Plausibilitaet ist kein
Beleg.

**How.**
- Vor dem Schreiben: pro Behauptung die Quelle benennen koennen
  (Datei/Zeile, Doku-Abschnitt, Owner-Aussage). Keine Quelle → pruefen
  oder fragen.
- Beim Dokumentieren fremder Subsysteme zuerst die gepflegte Doku lesen
  (features.md, energy-system.md, …), dann gezielt Code-Greps.
- Unklarheits-Signale ernst nehmen: „vermutlich", „sollte eigentlich",
  „typischerweise" im eigenen Kopf = Stopp-Wort.

**Anti-Pattern.**
- „Das wird schon so funktionieren wie in anderen Spielen."
- Eine Mechanik aus dem UI-Augenschein extrapolieren, statt den Handler
  zu lesen.
- Die Rueckfrage sparen wollen und dafuer eine Korrektur-Runde kassieren.

**Verwandt.**
- [01-clarify-with-options.md](01-clarify-with-options.md),
  [09-scope-vor-impl.md](09-scope-vor-impl.md),
  [39-memory-verifizieren.md](39-memory-verifizieren.md),
  `56-spielregel-semantik-ohne-sonderfaelle.md` (nur Chimera).
- Memory `feedback_ask_instead_of_guessing`.
