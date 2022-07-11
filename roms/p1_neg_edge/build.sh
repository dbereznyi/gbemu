#!/usr/bin/bash
rgbasm -L -o p1_neg_edge.o p1_neg_edge.rgbds
rgblink -o ../p1_neg_edge.gb p1_neg_edge.o
rgbfix -v -p 0xFF ../p1_neg_edge.gb

