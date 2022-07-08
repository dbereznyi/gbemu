#!/usr/bin/bash
rgbasm -L -o controller.o controller.rgbds
rgblink -o ../controller.gb controller.o
rgbfix -v -p 0xFF ../controller.gb

