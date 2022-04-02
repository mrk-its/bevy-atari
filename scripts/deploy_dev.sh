source $(dirname $0)/common.sh

git checkout web --recurse-submodules --

rm -fr docs/test

mv $BUILD_DIR docs/test


git add docs/test

git commit -m "test release"
git push --set-upstream origin web
