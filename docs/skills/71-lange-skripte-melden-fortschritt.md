# Skill: Lang laufende Skripte geben laufend Status aus

**Wann.** Ein Skript/Kommando läuft absehbar länger als ein paar Minuten im
Hintergrund — Trainings-/Eval-Batterien, Kampagnen-Loops, Massen-Sims,
Suite-Läufe, Build-Ketten.

**Regel.**

1. **Jedes selbstgeschriebene Lang-Skript emittiert laufend Fortschritt**
   auf stdout/Logfile: pro Etappe eine Zeile mit Zeitstempel
   (`echo "[$(date +%H:%M)] schritt 3/12 — eval tutorial-v4"`), bei inneren
   Loops zusätzlich Zähler (`i/N`). Nie minutenlang stumm arbeiten.
2. **Fremd-Tools, die puffern** (vitest schreibt den Report erst am Ende),
   nicht nackt in einen Stunden-Job stecken: Ausgabe entpuffern oder
   segmentieren — pro Testdatei/Etappe ein eigener Aufruf mit Echo davor,
   oder ein Reporter/`tee` ins Logfile, damit `tail -f` Lebenszeichen zeigt.
3. **Heartbeat-Kriterium:** Ein Beobachter muss aus dem Log allein
   entscheiden können „läuft noch" vs. „hängt" — letzte Zeile älter als das
   längste erwartbare Etappen-Intervall = Verdacht. Ohne laufende Ausgabe
   bleibt nur die CPU-Last-Raterei (`ps`), und die unterscheidet nicht
   zwischen „rechnet sinnvoll" und „Endlosschleife".

**Warum.** Owner-Anweisung 2026-08-23 nach dem Sim-Suite-Vorfall: ein
pauschal gestarteter Suite-Lauf blieb >4 h stumm (Vitest puffert; der
TPE-Treiber `optimize.test.ts` lief unbemerkt mit). Ob der Job hing oder
arbeitete, war nur über CPU-Forensik zu erkennen. Mit Etappen-Echos wäre
nach zwei Minuten klar gewesen, WAS da läuft und dass es der falsche
Umfang ist.

4. **Aktiv überwachen, nicht nur warten (Owner 2026-08-23):** Wer einen
   Lang-Läufer startet, prüft den Log-Output **alle ~5 min** — per
   Stall-Watchdog (Alarm, wenn das Log >5 min nicht wächst, s. Beispiel 2)
   oder direktem `tail`-Check im Status-Takt. Ein Completion-Callback
   ersetzt das nicht: ein HÄNGENDER Prozess feuert ihn nie, und genau der
   soll auffallen.

**Beispiel.**

```bash
for RUN in bot-curriculum tutorial-v4 challenge-v4; do
  echo "[$(date +%H:%M)] eval $RUN startet"
  RL_MANAGER=1 RL_MODEL=... npx tsx scripts/calibration.ts "$RUN" 12 \
    > "/tmp/rl-diag/eval-$RUN.log" 2>&1
  echo "[$(date +%H:%M)] eval $RUN fertig: $(grep -m1 Bosse /tmp/rl-diag/eval-$RUN.log)"
done
```

Verwandt: [61 — Hintergrund-Prozesse sauber killen](61-hintergrund-prozesse-sauber-killen.md)
(Abbruch-Seite desselben Problems), [05 — Updates bei langen Tasks](05-updates-bei-langen-tasks.md)
(User-Kommunikation; hier geht es um die Skript-Ausgabe selbst).

**Beispiel 2 — Stall-Watchdog (meldet NUR Stillstand):**

```bash
LAST=0; SILENT=0
while true; do
  SZ=$(stat -c %s "$LOG" 2>/dev/null || echo 0)
  if [ "$SZ" -eq "$LAST" ]; then SILENT=$((SILENT+60)); else SILENT=0; LAST=$SZ; fi
  [ $SILENT -ge 300 ] && { echo "STALL: $LOG seit ${SILENT}s unveraendert"; SILENT=0; }
  sleep 60
done
```
