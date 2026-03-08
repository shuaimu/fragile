cmake_minimum_required(VERSION 3.22)

set(FRAGILEC_LOCAL_BIN "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/fragilec")
if(DEFINED ENV{FRAGILEC_BIN} AND NOT "$ENV{FRAGILEC_BIN}" STREQUAL "")
  set(FRAGILEC_BIN "$ENV{FRAGILEC_BIN}")
elseif(EXISTS "${FRAGILEC_LOCAL_BIN}")
  set(FRAGILEC_BIN "${FRAGILEC_LOCAL_BIN}")
else()
  find_program(FRAGILEC_BIN fragilec REQUIRED)
endif()

# Use fragilec for C++ compilation.
set(CMAKE_CXX_COMPILER "${FRAGILEC_BIN}" CACHE FILEPATH "C++ compiler" FORCE)

project(cmake_rust_object_demo LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_program(RUSTC_BIN rustc REQUIRED)
find_program(CLANGXX_BIN clang++ REQUIRED)

set(RUST_SRC "${CMAKE_CURRENT_SOURCE_DIR}/src/math.rs")
set(RUST_OBJ "${CMAKE_CURRENT_BINARY_DIR}/rust/math.rs.o")

add_custom_command(
  OUTPUT "${RUST_OBJ}"
  COMMAND "${CMAKE_COMMAND}" -E make_directory "${CMAKE_CURRENT_BINARY_DIR}/rust"
  COMMAND "${RUSTC_BIN}"
          --edition=2021
          --crate-type=lib
          --emit=obj
          --crate-name cmake_rust_object_demo_math
          -C panic=abort
          -C overflow-checks=off
          "${RUST_SRC}"
          -o "${RUST_OBJ}"
  DEPENDS "${RUST_SRC}"
  VERBATIM
  COMMENT "Compiling Rust object ${RUST_SRC}"
)

add_custom_target(rust_math_obj DEPENDS "${RUST_OBJ}")

set_source_files_properties("${RUST_OBJ}" PROPERTIES
  GENERATED TRUE
  EXTERNAL_OBJECT TRUE
)

add_executable(demo src/main.cpp "${RUST_OBJ}")
add_dependencies(demo rust_math_obj)
set_property(TARGET demo PROPERTY
  RULE_LAUNCH_LINK "${CMAKE_COMMAND} -E env FRAGILEC_LINKER=${CLANGXX_BIN}"
)
