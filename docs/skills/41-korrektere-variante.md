# Skill: Korrektere Variante statt Quick-and-Dirty

**Wann.** Bei einer Refactor-/Implementierungs-Entscheidung stehen zwei
Varianten zur Auswahl:
- (a) macht es **substantiell richtig** — neue Felder werden ordentlich
  in der zustaendigen Pipeline integriert, alle Regeln greifen, Tests
  decken die Pfade einheitlich ab.
- (b) **Scaffolding-Minimum** — Sonderbehandlung am Caller, Workaround
  am Edge, „spaeter saubere Variante".

**Regel.** **(a) ist Default.** Den substantiellen Pfad waehlen, sofern
Aufwand und Risiko vertretbar sind.

**Quick-Fixes brauchen explizite User-Freigabe** (User-Spec msg 10982,
2026-05-31, nach pulseMod-Display-Layer-Quick-Fix der eine korrigierende
Bag-Level-Iteration nach sich zog). Wenn (b) wirklich gerechtfertigt
scheint, ist das eine Entscheidung, die zurueck zum Auftraggeber
gehoert — nicht eine, die der Assistent stillschweigend trifft.
Konkret: vor jedem Quick-Fix-Commit explizit fragen ("hier waere ein
Quick-Fix moeglich der X umgeht — soll ich, oder soll ich die richtige
Variante umsetzen?") und auf Freigabe warten.

**Warum.** Scaffolding-Loesungen haben die Eigenschaft, dauerhaft zu
bleiben. „Quick-and-Dirty jetzt, sauber spaeter" wird in der Praxis
selten nachgezogen, weil der naechste Druck aus einer anderen Richtung
kommt. Ein bisschen mehr Zeit jetzt erspart einen Refactor in 6 Monaten
plus die Begleiterscheinungen (mehrere Stellen muessen umgezogen werden,
Tests anders, Konsumenten geaendert). Die Freigabe-Pflicht zwingt den
Assistenten dazu, die echte Kosten/Nutzen-Rechnung offen zu legen,
statt sie wegzukapseln.

**Wann _doch_ Minimum.** Diese Ausnahmen sind die einzigen Faelle, in
denen (b) ohne neue Freigabe gewaehlt werden darf — User-Aussage muss
trotzdem mit „mach Quick-Fix" oder analog bestaetigen, dass einer
dieser Faelle vorliegt:
- **Spike-Code**, explizit als Throwaway markiert und auch wieder
  geloescht (nicht in main gemergt).
- **Hot-Fix unter Zeitdruck** — mit ausdruecklicher Folge-Story, die
  innerhalb weniger Tage abgearbeitet wird.
- **Reine Daten-Migration**, wo der „richtige" Pfad keinen Mehrwert hat.

**Beispiel.**
Property-Bag-Erweiterung mit zwei Varianten:
- (a) Neues Feld in `VALUE_KEYS` aufnehmen, im Resolver alle Regeln
  (Multiplikatoren, Inverter, Clamp) integrieren.
- (b) Sonderbehandlung im Caller, der das Feld manuell zusammenrechnet.

(a) waehlen. (b) nur, wenn ein expliziter Grund (z.B. Feld ist
strukturell anders) das rechtfertigt.

Konkret aus Chimera: bei der Damping-Generalisierung wurde Phase C
direkt auf das Count-Modell umgestellt (substantielle Variante), statt
am Caller pro Aufrufer zu zaehlen — das spart Konsumenten-Aenderungen
und macht die Regel an einer Stelle definiert.

**Anti-Pattern.**
- Sonderfall-Branch am Caller einbauen mit `// TODO: in Resolver
  integrieren` — der TODO bleibt 6 Monate.
- Boolean-Flag „zeroDemand" anhaengen, das nur ein einzelner Caller
  versteht, weil der Resolver-Integration „zuviel Aufwand" waere.
- Adapter-Schicht um eine neue API legen, statt die Aufrufer auf die
  neue API umzustellen.

**Verwandt.**
- [09-scope-vor-impl.md](09-scope-vor-impl.md) (Scope ehrlich
  abschaetzen, dann die richtige Variante waehlen).
- [11-mehrphasen-commits.md](11-mehrphasen-commits.md) (substantielle
  Variante kann mehrphasig sein — das ist OK).
- [20-keine-bw-compat-shims.md](20-keine-bw-compat-shims.md)
  (Scaffolding-Reste sind oft Backward-Compat-Shims).
- Memory `feedback_prefer_correct_over_quick`.
