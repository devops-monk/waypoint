#!/usr/bin/env bash
# Publish the website (and optionally a release's installers) to the VPS.
#
#   website/deploy.sh                 # site only
#   website/deploy.sh v0.2.0          # site + that release's installers from GitHub
#
# The server keeps its own nginx config at /etc/nginx/sites-available/waypoint;
# this script only copies files into /var/www/waypoint.
set -euo pipefail
HOST="${WAYPOINT_HOST:-root@168.231.79.163}"
ROOT=/var/www/waypoint
cd "$(dirname "$0")"

rsync -az --delete --exclude downloads ./ --exclude deploy.sh "$HOST:$ROOT/"

if [[ $# -ge 1 ]]; then
  tag="$1"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  gh release download "$tag" -R devops-monk/waypoint -D "$tmp" \
    --pattern '*.dmg' --pattern '*.exe' --pattern '*.msi' --pattern '*.AppImage' --pattern '*.deb' --pattern '*.rpm'
  (cd "$tmp" && shasum -a 256 * > SHA256SUMS.txt)
  rsync -az --partial "$tmp/" "$HOST:$ROOT/downloads/"
  echo "Uploaded $tag installers. Update the version and file names in index.html if they changed."
fi
ssh "$HOST" "chown -R www-data:www-data $ROOT"
echo "Deployed to https://waypoint.devops-monk.com"
