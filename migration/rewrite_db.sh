#!/bin/sh
rm ../../main.sqlite
DATABASE_URL="sqlite:./../../main.sqlite?mode=rwc" cargo run
