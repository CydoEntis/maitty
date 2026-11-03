#!/usr/bin/env bash
# Runs every scenario and experiment, 3 at a time (20-core box), logs to results/log/.
set -u
cd "$(dirname "$0")"
mkdir -p results/log
{
  for s in a b c d e; do for g in gpu cpu; do echo "--scenario $s --gpu $g"; done; done
  for e in noanim nocursorblink hidesidebar hidetabbar hidestatusbar hidepanehead hidetitlebar hidechrome hideterminal noshadow nogradient flatstyle reducemotion stop1s stop80ms stop1500 stopalltimers nostream bare; do
    echo "--scenario b --gpu gpu --exp $e"
  done
  for e in noanim nocursorblink hidechrome hidesidebar flatstyle stopalltimers bare; do
    echo "--scenario a --gpu gpu --exp $e"
  done
  for e in noanim hidechrome bare; do echo "--scenario b --gpu cpu --exp $e"; done
  echo "--scenario b --gpu gpu --dsf 2"
  echo "--scenario a --gpu gpu --dsf 2"
  echo "--scenario b --gpu gpu --poll --measure 60"
  echo "--scenario a --gpu gpu --poll --measure 60"
  echo "--scenario c --gpu gpu --poll --measure 60"
} | xargs -P 3 -I{} bash -c 'name=$(echo "{}" | tr -d "-" | tr " " "_"); timeout 200 node harness.mjs {} > results/log/$name.log 2>&1; echo "done {}"'
echo ALL DONE
