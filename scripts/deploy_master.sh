git checkout -b web && git reset --hard master || (echo "cannot reset web branch"; exit 1)

rm -fr docs

cp -a build_dir docs
cp -a build_dir docs/test

git add docs

git commit -m "stable release"
git push -f --set-upstream origin web
