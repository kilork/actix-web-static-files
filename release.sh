#!/bin/sh

set -e

RELEASE_TYPE=${RELEASE_TYPE:-minor}
if [ "${RELEASE_TYPE}" != "current" ]; then
  cargo set-version --bump ${RELEASE_TYPE}
fi
VERSION=`cargo pkgid | cut -d"#" -f2`
export CRATE_RUST_MAJOR_VERSION=`echo ${VERSION} | cut -d"." -f1,2`
handlebars-magic templates .
git add .
git commit -m"Release ${VERSION}"
git tag v${VERSION}
git push && git push --tag