#!/bin/bash
set -e

mkdir -p apt-repo/mcping/pool/main/
mv *.deb apt-repo/mcping/pool/main/
for arch in "amd64" "arm64";
do
mkdir -p apt-repo/mcping/dists/stable/main/binary-$arch;
dpkg-scanpackages --arch $arch pool/ > dists/stable/main/binary-$arch/Packages
cat dists/stable/main/binary-$arch/Packages | gzip -9 > dists/stable/main/binary-$arch/Packages.gz
done
cat apt-repo/mcping/dists/stable/Release | gpg -abs > apt-repo/mcping/dists/stable/Release.gpg
cat apt-repo/mcping/dists/stable/Release | gpg -abs --clearsign > apt-repo/mcping/dists/stable/InRelease
gpg --export --armor > apt-repo/mcping/pgp-key.public