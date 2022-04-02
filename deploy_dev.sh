set -e
set -x
#if [[ -n $(git status -s | grep -v '??' | grep -v deploy_test.sh | grep -v bevy-atari-antic) ]]; then
#  echo git not clean, qutting
#  exit 1
#fi

echo branch: $GITHUB_REF_NAME

cargo make build-webgl-sha1 -p release
DEST=test.tmp

rm -fr $DEST
mkdir -p $DEST/target
mkdir -p $DEST/pokey

cp -v web/index.html $DEST
echo " " >> test.tmp/index.html
cp -v -a web/wasm $DEST/wasm
cp -v web/pokey/pokey.js $DEST/pokey
cp -v -a web/js $DEST

git checkout web --recurse-submodules --
rm -fr docs/test
mv $DEST docs/test
git add docs/test

git commit -m "test release"
git push --set-upstream origin web
