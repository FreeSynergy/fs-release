# Anbieter-Neuinstallation (Weg C) / Provider reinstall (way C)

Workflow: `workflows/anbieter-neuinstallation.yaml`. Werkzeug / tool: `scripts/installimage-config` (Python 3, ohne KI, ohne Netz).
Selbsttest / self test: `scripts/test_installimage_config.sh`.

## DE - Montagsablauf (Hetzner)

**Vorher gilt:** Kein Agent hat Zugang zum Server. Du führst alles selbst aus. Der Workflow löscht erst im Schritt `install`.
Vor Montag wird nichts gelöscht; die Vorbereitung (Schritte 1 bis 4) ändert nur lokale Dateien.

1. **Variablen** in `server.vars` (KEY=VALUE, keine Geheimnisse): `HOSTNAME`, `IMAGE`, optional `BOOT_MODE=uefi|bios`,
   `DRIVE1`, `DRIVE2`, `BOOT_SIZE_MIB` (1024), `ESP_SIZE_MIB` (256), `SWAP_GIB` (8), `ROOT_GIB` (64), `FS_DATA=btrfs|zfs|none`.
   Den genauen `IMAGE`-Dateinamen kennst du erst im Rescue-System (Verzeichnis der Images); bis dahin Platzhalter, dann neu erzeugen.
2. **Bestand vorher** (auf dem laufenden System): `fs-server-run` mit `fs-server-bestand`, Ausgabe `bestand-vorher.json`
   zurück auf den lokalen Rechner. Er liefert Boot-Modus und Platten (echtes Format des Berichts von `fs-server-bestand`, Branch `unreviewed/bestand`: `summary.boot_mode`, `summary.disks[{name,size_bytes}]`, `section_status`).
3. **Konfig erzeugen:** `scripts/installimage-config --vars server.vars --bestand bestand-vorher.json --out installimage.conf`
   und `... --plan > plan.txt`. Lies `plan.txt` ganz.
4. **Prüfen:** `scripts/installimage-config --check installimage.conf --vars server.vars --bestand bestand-vorher.json` muss `CHECK OK` melden.
5. **Gate "DATEN WERDEN GELÖSCHT":** Backup vorhanden oder Daten dürfen weg? Dann `scripts/installimage-config --vars server.vars --confirm-hostname <Hostname>`;
   nur bei `GATE OK` weiter.
6. **Rescue (Robot):** Server wählen, Reiter Rescue, Rescue aktivieren (Linux, 64 bit, deinen öffentlichen SSH-Schlüssel),
   Reiter Reset: Hardware-Reset oder Reboot auslösen. Warten, dann `ssh root@<IP>` in das Rescue-System (Host-Schlüssel vergleichen).
7. **Konfig hochladen:** `scp installimage.conf root@<IP>:/root/`. Prüfen, dass `IMAGE` zu einer Datei im Images-Verzeichnis passt.
8. **Install:** im Rescue unter `fs-server-run` `installimage` mit der Datei starten (Aufrufform: `installimage -h` lesen, [nicht belegt]).
   Dieser Schritt löscht die Platten. Bei Fehler Ursache beheben und neu starten, nie in ein halb installiertes System booten.
9. **Neustart:** im Rescue `reboot` tippen. Kommt der Server nicht zurück: im Robot Reset, notfalls vKVM oder Rescue erneut (Hilfe, keine Voraussetzung).
10. **Prüfen (nur lesend):** `ssh` auf das neue System, `fs-server-run` mit `fs-server-bestand` -> `bestand-nachher.json`
    (Boot-Modus, `mdadm --detail`, Dateisysteme wie im Plan).
11. **Datenteil:** die Zeilen aus `plan.txt` (`sgdisk`, `mkfs.btrfs -d raid1 -m raid1`, Mount `/srv/freesynergy`) selbst ausführen.
12. **Übergabe:** Workflow `server-einrichtung` (Weg A).
13. **Protokoll zurück:** die eine Protokolldatei (`protocol.log`) plus `bestand-vorher.json`, `installimage.conf`, `plan.txt`,
    `bestand-nachher.json` an die Strategie (Geheimnisse sind geschwärzt; keine Schlüssel mitschicken).

Belegt / nicht belegt: siehe Liste unten (gilt für DE und EN).

## EN - Monday procedure (Hetzner)

Nothing is deleted before the `install` step. Steps 1 to 4 only change local files. No agent has access to the server; you run everything.

1. Write `server.vars` (KEY=VALUE, no secrets), see the keys above. The exact `IMAGE` file name is only known inside the rescue system.
2. Inventory on the running system (`fs-server-run`, `fs-server-bestand`), bring `bestand-vorher.json` back.
3. Generate: `scripts/installimage-config --vars server.vars --bestand bestand-vorher.json --out installimage.conf`; `--plan > plan.txt`; read the plan.
4. Check: `--check installimage.conf ...` must print `CHECK OK`.
5. Gate DATA WILL BE DELETED: repeat the host name with `--confirm-hostname`; only `GATE OK` continues.
6. Robot: Rescue tab, activate Rescue (Linux 64 bit, your public key); Reset tab, hardware reset; wait; `ssh root@<IP>`.
7. `scp installimage.conf root@<IP>:/root/`; check that `IMAGE` matches a file in the images directory.
8. Run `installimage` with the file under `fs-server-run` (call line: read `installimage -h`, [nicht belegt]). This deletes the disks.
9. Type `reboot` in the rescue system. If the server does not return: Robot reset, vKVM or rescue again (help, never a precondition).
10. Read-only check on the new system (`bestand-nachher.json`).
11. Data part from `plan.txt` (btrfs RAID1 on the free rest of both disks), mount at `/srv/freesynergy`.
12. Hand over to `server-einrichtung` (way A). 13. Hand back the log and the files listed above.

## Beleg-Stand / Evidence

Belegt auf docs.hetzner.com (installimage-Seite): `DRIVEn` (Nummer anpassen, wenn eine Platte fehlt), `SWRAID`/`SWRAIDLEVEL` (Level 0, 1, 5, 6, 10, gilt für alle Platten),
`HOSTNAME`, Größe `all` = Rest, Einheiten M/G/T, GPT nur mit GRUB2, Syntaxprüfung nach dem Editor, `/autosetup`-Datei im Rescue wird ohne Menü genutzt.
Belegt nur im Quelltext von `github.com/hetzneronline/installimage` (`functions.sh`, `config.sh`, gelesen 2026-11-14): `BOOTLOADER` nur `grub`,
`PART <Mountpunkt> <Dateisystem> <Größe>`, ESP = `PART /boot/efi esp 256M` (mindestens 256M, nicht `all`, genau eine bei UEFI), ESP unter
`SWRAID` immer RAID1 mit Metadaten 1.0, `/boot` bei Level 1 nicht Pflicht, Mountpunkte eindeutig, `/` Pflicht, `IMAGE`-Schlüsselwort,
`SUBVOL`-Zeilen bei `btrfs.N`, kein Multi-Device-btrfs ("Multi-device volumes are not supported").

**[nicht belegt]**
- Die Aufrufzeile von `installimage` (Autosetup-Datei `/autosetup` kopieren oder Option) und die Dateinamen der Images (Debian 13/12, Ubuntu).
- Das vollständige Verzeichnis zulässiger Dateisysteme in `PART` (das Skript kennt ext2/3/4, xfs, btrfs, swap, esp; weitere ungeprüft).
- RAID 5/6/10: bewusst nicht unterstützt (Fehlermeldung); der Generator erzeugt nur RAID1 mit 1 oder 2 Platten.
- btrfs-RAID1 oder btrfs auf md innerhalb der installimage-Konfig: nicht möglich/ungeprüft, daher läuft der Datenteil nach dem ersten Start (Schritt 11).
- ZFS (`FS_DATA=zfs`): weder in der Doku-Seite noch im gelesenen Quelltext; nur der Pool-Befehl als Hinweis im Plan.
- Das Format von `fs-server-bestand` ist jetzt aus dem Quelltext (Kopf f311b4f) übernommen; fehlende oder nicht `ok`-Felder sind ein Fehler, nichts wird geraten. Die Namen im Rescue-System können von denen im laufenden System abweichen (Reihenfolge sda/sdb): `DRIVE1`/`DRIVE2` nach dem Start im Rescue mit `lsblk` prüfen.
- Das Verhalten bei einer Platte mit unterschiedlicher Größe (Generator nimmt die kleinere als Grenze).
- Der Rescue-Ablauf im Robot wurde nicht an einem echten Server geprüft.
