cmake_minimum_required(VERSION 3.22)

project(cmake_rust_object_demo LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_program(RUSTC_BIN rustc REQUIRED)

set(RUST_SRC ${CMAKE_CURRENT_SOURCE_DIR}/src/math.rs)
set(RUST_OBJ ${CMAKE_CURRENT_BINARY_DIR}/rust/math.rs.o)

add_custom_command(
  OUTPUT ${RUST_OBJ}
  COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_CURRENT_BINARY_DIR}/rust
  COMMAND ${RUSTC_BIN}
          --edition=2021
          --crate-type=lib
          --emit=obj
          --crate-name cmake_rust_object_demo_math
          -C panic=abort
          -C overflow-checks=off
          ${RUST_SRC}
          -o ${RUST_OBJ}
  DEPENDS ${RUST_SRC}
  VERBATIM
  COMMENT "Compiling Rust object ${RUST_SRC}"
)

add_custom_target(rust_math_obj DEPENDS ${RUST_OBJ})

set_source_files_properties(${RUST_OBJ} PROPERTIES
  GENERATED TRUE
  EXTERNAL_OBJECT TRUE
)

add_executable(demo src/main.cc ${RUST_OBJ})
add_dependencies(demo rust_math_obj)
