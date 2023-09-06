#!/usr/bin/env bash
printf '{"public_key": "%s"}\n' "$(ssh-keygen -yf "${1}")"
