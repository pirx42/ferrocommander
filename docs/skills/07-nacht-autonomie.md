# Skill: Nacht- / Offline-Autonomie

**Wann.** Auftraggeber kuendigt explizit an, offline zu gehen
(„ich gehe schlafen", „bin morgen wieder da", „bin bis 9 weg").

**Regel.** Bis zur angegebenen Wiederkehr-Zeit (typisch ~7:00) autonom an
**entscheidungsfreien** Tasks arbeiten. Nicht idle warten. Tasks die
Rueckfragen brauchen werden **nicht** angefangen — die warten bis morgens.

**Warum.** Die wachen Stunden des Auftraggebers sind die einzige Zeit fuer
Entscheidungen. Decisions-required-Tasks in der Nacht zu blockieren waere
Verschwendung dieser Zeit. Niedrig-Risiko-Tasks (Doku, Test-Nachzug,
klare Bugfixes, Translation, Audit-Folgen) sind ideal — der Auftraggeber
findet morgens Fortschritt vor.

**How.**

1. **Vor Beginn:** vorhandene Memory-Eintraege (Pflichtregeln, Konventionen)
   laden und befolgen — nicht die Gelegenheit nehmen, Hausregeln zu brechen.
2. **Risiko-Filter:**
   - keine Schema-Brueche
   - keine Refactors, die viele Konsumenten beruehren
   - keine destruktiven Git-Operationen ohne ausdrueckliche Vorab-Zustimmung
   - bei aufkommender Entscheidung: vermerken (Memory / Plan-Doku) und liegen
     lassen
3. **Stoppen wenn alle entscheidungsfreien Tasks durch sind** — nicht
   Risiko-Tasks angehen, nur um die Zeit zu fuellen. Ein kurzer
   End-of-Night-Status reicht.

**Geeignete Nacht-Tasks (Beispiele).**
- Doku-Nachzug nach abgeschlossenem Refactor
- Audit-Reports lesen + offene Folge-Tasks identifizieren
- Coverage-Luecken schliessen (erreichbare Pfade)
- i18n-Pflege (fehlende Translation-Keys)
- Tests fuer existierendes Verhalten ergaenzen
- Plan-Dateien archivieren (Status: Umgesetzt → archive/)

**Ungeeignete Nacht-Tasks.**
- Neue Features mit unklarem Spec
- Refactor mit Mehrwege-Tradeoffs („Option A oder B?")
- Major Dependency-Bumps
- Production-Push ohne Vorab-Zustimmung

**Anti-Pattern.**
- 4 Stunden idle, weil „kein Task ohne Rueckfrage da war".
- Risiko-Task starten und morgens vor Augen halten „passt das so?".

**Verwandt.**
- [03-bestaetigung-destruktiv.md](03-bestaetigung-destruktiv.md)
- [06-pausen-ansagen.md](06-pausen-ansagen.md)
- [40-vier-phasen-zyklus.md](40-vier-phasen-zyklus.md) (Phase 2/3-Tasks
  eignen sich besonders).
