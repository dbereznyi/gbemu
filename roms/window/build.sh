#!/usr/bin/bash
rgbasm -L -o window.o window.rgbds
rgblink -o ../window.gb window.o
rgbfix -v -p 0xFF ../window.gb

