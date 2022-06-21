#!/bin/sh

set -e

# https://earthly.dev/blog/creating-and-hosting-your-own-deb-packages-and-apt-repo/
cd tauri-src
# cargo tauri build
cd ..
cp tauri-src/target/release/bundle/deb/rl-bot-gui_*.*.*_amd64.deb docs/apt-repo/pool/main/.
cd docs/apt-repo
dpkg-scanpackages --arch amd64 pool/ > dists/stable/main/binary-amd64/Packages
cat dists/stable/main/binary-amd64/Packages | gzip -9 > dists/stable/main/binary-amd64/Packages.gz
cd dists/stable

chmod +x ../../../../generate-deb-release-info.sh
../../../../generate-deb-release-info.sh > Release

cd ../../../../

export GNUPGHOME="$(mktemp -d pgpkeys-XXXXXX)"
gpg --list-keys
cat pgp-key.private | gpg --import
echo "Enter in identifier to be passed to GPG for signing:"
read name
cat ./docs/apt-repo/dists/stable/Release | gpg --default-key $name -abs > ./docs/apt-repo/dists/stable/Release.gpg
cat ./docs/apt-repo/dists/stable/Release | gpg --default-key $name -abs --clearsign > ./docs/apt-repo/dists/stable/InRelease