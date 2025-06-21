#!/bin/bash
set -e

mkdir -p apt-repo/mcping/pool/main/
mv "$@" apt-repo/mcping/pool/main/

cd apt-repo/mcping/

for arch in "amd64" "arm64";
do
mkdir -p dists/stable/main/binary-$arch;
dpkg-scanpackages --arch $arch pool/ > dists/stable/main/binary-$arch/Packages
cat dists/stable/main/binary-$arch/Packages | gzip -9 > dists/stable/main/binary-$arch/Packages.gz
done
./generate-release.sh > dists/stable/Release
cat dists/stable/Release | gpg -abs > dists/stable/Release.gpg
cat dists/stable/Release | gpg -abs --clearsign > dists/stable/InRelease
gpg --export --armor > pgp-key.public