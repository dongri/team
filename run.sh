#!/bin/bash

docker build -t team:0.1.0 .

docker run -i -t -d -p 80:3000 team:0.1.0
