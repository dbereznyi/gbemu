#!/usr/bin/bash
rgbasm -L -o sprites.o sprites.rgbds
rgblink -o ../sprites.gb sprites.o
rgbfix -v -p 0xFF ../sprites.gb

