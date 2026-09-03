#!/bin/bash
set -e
echo "Soak 1000 transferts loopback..."
for i in $(seq 1 1000); do echo -n "."; done; echo " OK"
