# Skill: Vor aufwändigem Selbst-Bauen kurz fragen

**Wann.** Ein Arbeitsschritt läuft darauf hinaus, ein **Artefakt mühsam
von Hand zusammenzubauen**, das der User interaktiv (im Spiel, im Tool,
aus seinem Kopf) schneller und zuverlässiger erzeugen könnte. Typische
Fälle:
- **Test-Fixtures / Save-Stände** mit komplexem inneren State
  (z.B. ein Infiltrations-Save: `activeBossBody` + Energie-Netz +
  geladener Infiltrator — von Hand fehleranfällig, mehrere Fehlversuche).
- **Datensätze / Setups**, die im UI in Sekunden zusammenklickbar sind.
- Konkrete **Inhalte**, deren „richtige" Form nur der User kennt
  (Beispiel-Eingaben, realistische Konfigurationen).

**Regel.** Bevor mit dem mühsamen Handbau begonnen wird, **kurz über den
aktiven Kanal fragen**: „Kannst du X schneller vorbereiten/exportieren?"
Dann entscheidet der User, ob er etwas vorbereitet oder ob ich es selbst
baue. Nicht stillschweigend lostüfteln.

**Warum.** User-Spec msg 11713 (2026-06-08). Auslöser: ich wollte einen
Infiltrations-Save für e2e-Tests synthetisch erzeugen (Bot-Sim-Dump,
Hand-Edit der JSON, mehrere Proben). Der User hat denselben Aufbau im
Spiel in Sekunden gebaut und als Datei geschickt — exakt korrekt, ohne
die ganze Tüftelei. Eine Frage vorab hätte den Umweg gespart.

**User-Artefakte 1:1 übernehmen, nicht nachbauen oder umlayouten
(2026-06-13).** Wenn der User ein Setup geschickt hat (Save/Export), dessen
Daten EXAKT übernehmen — nicht neu verteilen, clampen oder „verbessern".
Auslöser: ich hatte einen User-Export beim Wrappen in einen Boss über eine
handgemachte „place-map" auf andere Zonen umverteilt → stimmte nicht mit seinem
Build überein; korrekt ist die positionsgetreue Übernahme (`placeFaithful` im
Wrap-Script, Export-Koordinaten 1:1). Verwandter Reflex: zeigt der User per
Screenshot „so soll es aussehen", NICHT die Werte mühsam aus dem Bild
zurückrechnen — nach der **Quell-Datei** fragen (der Export lag bereits vor und
enthielt die exakten Positionen). Memory `feedback_user_items_all_or_fresh`.

**Wann _nicht_ fragen.** Reine Code-Artefakte, triviale Fixtures, Dinge
die schneller selbst gebaut als erklärt sind, oder wenn der User offline
ist und Nacht-Autonomie gilt (dann den günstigsten Eigen-Weg wählen und
das Ergebnis später zur Review stellen).

**Wie konkret.**
1. Erkennen: „Das baue ich jetzt aufwändig von Hand" — besonders bei
   Spielständen/Setups, die im Tool entstehen.
2. Eine **kurze, konkrete** Frage stellen, was gebraucht wird
   („brauche einen Save im Boss-Sektor mit aktivem Infiltrator — kannst
   du so einen Aufbau exportieren?").
3. Sagen, **wo** ich das Ergebnis ablege/verwende, damit der User weiß
   wofür.
4. Auf Antwort warten; bei „bau du" den Eigen-Weg gehen.

**Verwandt.**
- [01-clarify-with-options.md](01-clarify-with-options.md) (Rückfragen
  statt Annahmen).
- `50-e2e-tests-erstellen.md` (nur Chimera) (Fixtures sind
  ein Haupt-Anwendungsfall — echte exportierte Saves schlagen
  Synthetik).
- Memory `feedback_ask_before_laborious_build`,
  `feedback_prefer_correct_over_quick`.
