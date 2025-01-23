#!/bin/bash
if [[ "${VERCEL_ENV}" == "production" ]] ; then
    # Proceed with the build
    echo "✅ - Build can proceed on production branch"
    exit 1;
elif [[ "${VERCEL_GIT_COMMIT_REF}" == "release" ]] ; then
    # Proceed with the build
    echo "✅ - Build can proceed on release branch"
    exit 1;
else
    # Only build if the dashboard or dashboard-common has changed.
    git diff HEAD^ HEAD --quiet . ../dashboard-common && echo "🛑 - Build canceled"
fi
