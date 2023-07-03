#!/bin/bash
set -e

docker run --rm --name daicon-example-web-cdn -v ./:/usr/share/nginx/html:ro -p 8080:80 nginx
