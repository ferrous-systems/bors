#!/bin/bash

# Waits for the ref to be successfully deployed

set -eu
IFS=$'\n\t'

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <subdomain> <git_version>"
    exit 1
fi

subdomain="$1"
git_version="$2"

while true; do
    response="$(curl --silent "https://${subdomain}2.infra.ferrous-systems.net/.internal/git_version")"
    echo "response: $response"
    if [[ "$response" == "$git_version" ]]; then
        echo "successfully deployed"
        exit 0
    fi

    sleep 1
done
