#!/usr/bin/bash
rgbasm -L -o hilo.o hilo.rgbds
rgblink -o ../hilo.gb hilo.o
rgbfix -v -p 0xFF ../hilo.gb

