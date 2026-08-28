# Skill: Wichtige Skripte committen, nicht scratchpad

**Wann.** Immer wenn ein Skript **schwer rekonstruierbare Parameter oder
Konfiguration** trägt — RL-Retrain-Recipes (Staging/Novelty/Entropie-Schedule/
Episodes), Mess-/Kampagnen-Kommandos, Daten-Migrations-Einzeiler mit
ausgetüftelten Flags, Repro-Harnesses die wieder gebraucht werden.

**Regel.**
- Config-tragende Skripte gehören **ins Repo** (`scripts/` bzw. `rl/`),
  parametrisiert + kurz dokumentiert — NICHT ins Scratchpad (gitignored,
  ephemer) und nicht nur als nohup-Kommandozeile im Transkript.
- Wegwerf-Previews/einmalige Debug-Schnipsel dürfen im Scratchpad bleiben;
  die Grenze ist: „Würde der Verlust die Reproduzierbarkeit kosten?"
- Galerie-/Vorschau-Scripts, die bei Design-Tweaks wiederverwendet werden,
  fallen ebenfalls unter „committen" (siehe Skill 55).

**Warum.**
Der `-scope8`-RL-Retrain lag nur in `scratchpad/retrain-b.sh` (gitignored)
und ging verloren; die exakte Config (2-stufig 30+60, novelty 0.05,
episodes 6, ent 0.03→0.015→0.005) musste mühsam aus dem Session-Transkript
geborgen werden (Owner msg 16000, 2026-07-25). Ephemere Scratch-Skripte =
verlorene Reproduzierbarkeit.

**Verwandt.** Skill 55 (Galerie-Script committen), Skill 35 (reproduzierbares
Deploy), `docs/bot-training.md` (Trainings-Recipes).
