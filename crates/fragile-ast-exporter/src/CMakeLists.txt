cmake_minimum_required(VERSION 3.13...4.0)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
project(FragileAstExporter)

#################################################
# TinyCBOR                                      #
#################################################

set(TINYCBOR_REPO "https://github.com/intel/tinycbor.git" CACHE STRING "tinycbor git repo URL")
set(TINYCBOR_TAG "d393c16f3eb30d0c47e6f9d92db62272f0ec4dc7" CACHE STRING "tinycbor git tag (v0.6.3)")
set(TINYCBOR_PREFIX "${CMAKE_BINARY_DIR}/tinycbor" CACHE STRING "tinycbor install prefix")

if(CMAKE_SYSTEM_NAME STREQUAL "FreeBSD")
    set(MAKE "gmake")
else()
    set(MAKE "make")
endif()

include(ExternalProject)
ExternalProject_Add(tinycbor_build
    PREFIX ${TINYCBOR_PREFIX}
    INSTALL_DIR ${CMAKE_BINARY_DIR}
    GIT_REPOSITORY ${TINYCBOR_REPO}
    GIT_TAG ${TINYCBOR_TAG}
    CONFIGURE_COMMAND ${MAKE} .config
    BUILD_COMMAND ${MAKE} --quiet prefix=<INSTALL_DIR> CFLAGS=-fPIC
    INSTALL_COMMAND ${MAKE} --quiet prefix=<INSTALL_DIR> install
    BUILD_IN_SOURCE 1
    BUILD_BYPRODUCTS ${CMAKE_BINARY_DIR}/lib/libtinycbor.a
)

include_directories(${CMAKE_BINARY_DIR}/include)

add_library(tinycbor STATIC IMPORTED)
set_target_properties(tinycbor PROPERTIES IMPORTED_LOCATION ${CMAKE_BINARY_DIR}/lib/libtinycbor.a)
add_dependencies(tinycbor tinycbor_build)

#################################################
# LLVM/Clang                                    #
#################################################

find_package(LLVM REQUIRED CONFIG)

message(STATUS "Found LLVM ${LLVM_PACKAGE_VERSION}")
message(STATUS "Using LLVMConfig.cmake in: ${LLVM_DIR}")
message(STATUS "LLVM include dirs: ${LLVM_INCLUDE_DIRS}")
message(STATUS "LLVM library dir: ${LLVM_LIBRARY_DIR}")

# Get Clang include dirs (usually same as LLVM or in a clang subdirectory)
set(CLANG_INCLUDE_DIRS ${LLVM_INCLUDE_DIRS})

include_directories(${LLVM_INCLUDE_DIRS})
add_definitions(${LLVM_DEFINITIONS})

if (DEFINED LLVM_TOOLS_BINARY_DIR)
  add_definitions(-DCLANG_BIN_PATH="${LLVM_TOOLS_BINARY_DIR}")
elseif(DEFINED LLVM_INSTALL_PREFIX)
  add_definitions(-DCLANG_BIN_PATH="${LLVM_INSTALL_PREFIX}/bin")
else()
  add_definitions(-DCLANG_BIN_PATH="/usr/bin")
endif()

add_definitions(-DCLANG_VERSION_STRING="${LLVM_PACKAGE_VERSION}")
add_definitions(-DCLANG_LIBDIR_SUFFIX="${LLVM_LIBDIR_SUFFIX}")

# LLVM is not always built with RTTI, we don't need it either.
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-rtti")

#################################################
# AST Exporter Library                          #
#################################################

set(AST_EXPORTER_SRCS
    AstExporter.cpp
)

# Static library for linking with Rust
add_library(fragileAstExporter STATIC ${AST_EXPORTER_SRCS})

set_target_properties(fragileAstExporter PROPERTIES
    CXX_STANDARD 17
    CXX_EXTENSIONS OFF
)

add_dependencies(fragileAstExporter tinycbor)

# We only compile to a static library - linking against libclang-cpp happens in Rust
# Just need tinycbor for now
target_link_libraries(fragileAstExporter PRIVATE tinycbor)

# Optional: Build standalone executable for testing
option(BUILD_AST_EXPORTER_BIN "Build standalone AST exporter binary" OFF)

if(BUILD_AST_EXPORTER_BIN)
    add_executable(fragile-ast-exporter-bin ${AST_EXPORTER_SRCS})
    set_target_properties(fragile-ast-exporter-bin PROPERTIES
        OUTPUT_NAME fragile-ast-exporter
        CXX_STANDARD 17
        CXX_EXTENSIONS OFF
    )
    add_dependencies(fragile-ast-exporter-bin tinycbor)

    # For the binary, we need to link against clang-cpp
    target_link_libraries(fragile-ast-exporter-bin PRIVATE
        clang-cpp
        LLVM
        tinycbor
    )
endif()
