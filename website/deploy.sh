#!/usr/bin/env bash
# Publish the website's own files (HTML and screenshots) to the VPS.
#
#   website/deploy.sh
#
# Installers and the update feed are published by .github/workflows/publish.yml when a
# GitHub release is published; this script never touches /downloads or /updates.
# The server keeps its own nginx config at /etc/nginx/sites-available/waypoint.
set -euo pipefail
HOST="${WAYPOINT_HOST:-root@168.231.79.163}"
ROOT=/var/www/waypoint
cd "$(dirname "$0")"
rsync -rt --chmod=Du=rwx,Dgo=rx,Fu=rw,Fgo=r --delete --exclude downloads --exclude updates --exclude deploy.sh ./ "$HOST:$ROOT/"
ssh "$HOST" "chown -R waypoint-deploy:www-data $ROOT"
echo "Deployed to https://waypoint.devops-monk.com"
