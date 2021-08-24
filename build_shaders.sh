#!/bin/sh

build() {
  /b/lib/glslang/bin/glslangValidator -V ./src/shaders/$1.glsl # -o ./target/shaders/$1.spv
}

build geometry.vert
build geometry.frag
build background.vert
build background.frag

