#!/bin/sh
# exits 1 (and prints) if free space on /System/Volumes/Data < 40 GiB
free_kb=$(df -k /System/Volumes/Data | tail -1 | awk '{print $4}')
if [ "$free_kb" -lt $((40*1024*1024)) ]; then echo "DISK_LOW free_kb=$free_kb"; exit 1; fi
exit 0
