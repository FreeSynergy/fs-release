#!/usr/bin/env bash
# Self test of scripts/installimage-config. Local only: no server, no network, no AI. Temp dir, removed at the end.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
tool="$here/installimage-config"
t="$(mktemp -d)"
trap 'rm -rf "$t"' EXIT
pass=0
fail=0
ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; }

# expect <name> <expected exit> <grep pattern or -> -- command...
expect() {
  local name="$1" rc="$2" pat="$3"
  shift 3
  local out
  out="$("$@" 2>&1)"
  local got=$?
  if [ "$got" -ne "$rc" ]; then bad "$name (exit $got, want $rc): $out"; return; fi
  if [ "$pat" != "-" ] && ! grep -q -- "$pat" <<<"$out"; then bad "$name (no match for $pat): $out"; return; fi
  ok "$name"
}

vars() { # name boot drives... -> file
  local f="$t/$1.vars"
  shift
  printf '%s\n' "$@" >"$f"
  echo "$f"
}

u2=$(vars u2 HOSTNAME=node2 IMAGE=Debian-test.tar.gz BOOT_MODE=uefi DRIVE1=/dev/nvme0n1 DRIVE2=/dev/nvme1n1 DISK_SIZE_GIB=476)
b2=$(vars b2 HOSTNAME=node2 IMAGE=Debian-test.tar.gz BOOT_MODE=bios DRIVE1=/dev/sda DRIVE2=/dev/sdb DISK_SIZE_GIB=476)
u1=$(vars u1 HOSTNAME=node3 IMAGE=Debian-test.tar.gz BOOT_MODE=uefi DRIVE1=/dev/sda DISK_SIZE_GIB=200)

# 1 two disks UEFI
expect "2 disks uefi generate" 0 "PART /boot/efi esp 256M" "$tool" --vars "$u2"
expect "2 disks uefi has RAID1" 0 "SWRAIDLEVEL 1" "$tool" --vars "$u2"
"$tool" --vars "$u2" --out "$t/u2.cfg"
expect "2 disks uefi config checks" 0 "CHECK OK" "$tool" --check "$t/u2.cfg" --vars "$u2"
expect "2 disks uefi plan warns" 0 "ALLE DATEN GEHEN VERLOREN" "$tool" --vars "$u2" --plan
expect "2 disks uefi plan has btrfs raid1 step" 0 "mkfs.btrfs -L data -d raid1 -m raid1" "$tool" --vars "$u2" --plan
# 2 two disks BIOS
expect "2 disks bios no esp" 0 "PART / ext4 64G" "$tool" --vars "$b2"
"$tool" --vars "$b2" --out "$t/b2.cfg"
if grep -q esp "$t/b2.cfg"; then bad "bios config has no esp"; else ok "bios config has no esp"; fi
expect "2 disks bios config checks" 0 "CHECK OK" "$tool" --check "$t/b2.cfg" --vars "$b2"
# 3 one disk
expect "1 disk generate SWRAID 0" 0 "SWRAID 0" "$tool" --vars "$u1"
expect "1 disk plan single btrfs" 0 "mkfs.btrfs -L data /dev/sda" "$tool" --vars "$u1" --plan
# determinism
"$tool" --vars "$u2" >"$t/a1"; "$tool" --vars "$u2" >"$t/a2"
cmp -s "$t/a1" "$t/a2" && ok "deterministic output" || bad "deterministic output"
# bestand JSON gives boot mode and drives
printf '{"boot_mode":"uefi","disks":[{"name":"nvme0n1","size_gib":476.9},{"name":"nvme1n1","size_gib":476.9}]}\n' >"$t/best.json"
vb=$(vars vb HOSTNAME=node2 IMAGE=Debian-test.tar.gz)
expect "bestand supplies drives and mode" 0 "DRIVE2 /dev/nvme1n1" "$tool" --vars "$vb" --bestand "$t/best.json"
va=$(vars va HOSTNAME=node2 IMAGE=Debian-test.tar.gz BOOT_MODE=bios)
expect "bestand contradicts BOOT_MODE" 2 "contradicts" "$tool" --vars "$va" --bestand "$t/best.json"

# invalid inputs, each with a counter-test (the valid twin above passes)
v=$(vars big HOSTNAME=n IMAGE=i BOOT_MODE=uefi DRIVE1=/dev/sda DRIVE2=/dev/sdb DISK_SIZE_GIB=50 ROOT_GIB=64)
expect "root bigger than disk" 1 "smallest disk" "$tool" --vars "$v"
v=$(vars bigok HOSTNAME=n IMAGE=i BOOT_MODE=uefi DRIVE1=/dev/sda DRIVE2=/dev/sdb DISK_SIZE_GIB=50 ROOT_GIB=20)
expect "root fits (counter)" 0 "PART / ext4 20G" "$tool" --vars "$v"
v=$(vars three HOSTNAME=n IMAGE=i BOOT_MODE=uefi DRIVE1=/dev/sda DRIVE2=/dev/sda DISK_SIZE_GIB=500)
expect "same drive twice (generator)" 1 "appears twice" "$tool" --vars "$v"
cat >"$t/r1.cfg" <<'E'
DRIVE1 /dev/sda
SWRAID 1
SWRAIDLEVEL 1
BOOTLOADER grub
HOSTNAME n
IMAGE i
PART /boot ext3 1024M
PART swap swap 8G
PART / ext4 all
E
expect "RAID1 with one drive" 1 "needs at least 2 drives" "$tool" --check "$t/r1.cfg"
sed -i 's/^SWRAID 1/SWRAID 0/' "$t/r1.cfg"
expect "one drive SWRAID 0 (counter)" 0 "CHECK OK" "$tool" --check "$t/r1.cfg"
cp "$t/r1.cfg" "$t/dm.cfg"; echo "PART /boot ext3 512M" >>"$t/dm.cfg"
expect "duplicate mountpoints" 1 "identical mountpoints: /boot" "$tool" --check "$t/dm.cfg"
sed -e 's#^PART / ext4 all#PART / ext4 all\nPART /srv ext4 5G#' "$t/r1.cfg" >"$t/allfirst.cfg"
expect "size all not last" 1 "all must be the last" "$tool" --check "$t/allfirst.cfg"
expect "UEFI without esp" 1 "uefi but no esp" "$tool" --check "$t/r1.cfg" --vars "$u1"
expect "BIOS with esp" 1 "bios but an esp" "$tool" --check "$t/u2.cfg" --vars "$b2"
expect "UEFI with esp (counter)" 0 "CHECK OK" "$tool" --check "$t/u2.cfg" --vars "$u2"
sed 's#esp 256M#esp 128M#' "$t/u2.cfg" >"$t/smallesp.cfg"
expect "esp too small" 1 "at least 256M" "$tool" --check "$t/smallesp.cfg"
sed 's#^PART / ext4 64G#PART /data ext4 64G#' "$t/u2.cfg" >"$t/noroot.cfg"
expect "no root" 1 "no partition for /" "$tool" --check "$t/noroot.cfg"
sed 's#^SWRAIDLEVEL 1#SWRAIDLEVEL 5#' "$t/u2.cfg" >"$t/r5.cfg"
expect "RAID5 with two drives" 1 "RAID5 needs" "$tool" --check "$t/r5.cfg"
sed 's#^BOOTLOADER grub#BOOTLOADER lilo#' "$t/u2.cfg" >"$t/lilo.cfg"
expect "bootloader not grub" 1 "BOOTLOADER must be grub" "$tool" --check "$t/lilo.cfg"
expect "unknown key in vars" 2 "unknown key" "$tool" --vars "$(vars typo HOSTNAME=n IMAGE=i BOOT_MODE=uefi DRIVE1=/dev/sda ROOTGIB=1)"
expect "missing boot mode" 2 "BOOT_MODE missing" "$tool" --vars "$(vars nomode HOSTNAME=n IMAGE=i DRIVE1=/dev/sda)"
expect "missing image" 2 "IMAGE missing" "$tool" --vars "$(vars noimg HOSTNAME=n BOOT_MODE=bios DRIVE1=/dev/sda)"
expect "bad hostname" 1 "HOSTNAME missing or not a valid" "$tool" --check <(sed 's/^HOSTNAME .*/HOSTNAME Bad_Name/' "$t/u2.cfg")
# confirmation gate
expect "gate with right host name" 0 "GATE OK" "$tool" --vars "$u2" --confirm-hostname node2
expect "gate with wrong host name" 3 "GATE REFUSED" "$tool" --vars "$u2" --confirm-hostname node3
expect "gate with empty text" 3 "GATE REFUSED" "$tool" --vars "$u2" --confirm-hostname ""

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
