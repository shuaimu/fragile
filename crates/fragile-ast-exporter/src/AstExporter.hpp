//
// AstExporter.hpp
// C++ AST exporter for Fragile transpiler
//

#ifndef FRAGILE_AST_EXPORTER_H
#define FRAGILE_AST_EXPORTER_H

#include <cstddef>
#include <cstdint>

// Result structure for FFI
struct ExportResult {
    // Number of exported translation units
    size_t entries;
    // Array of names (file paths)
    char **names;
    // Array of CBOR byte arrays
    uint8_t **bytes;
    // Array of sizes for each byte array
    size_t *sizes;
};

extern "C" {
    // Main entry point for AST export
    // Returns a pointer to ExportResult containing CBOR-encoded AST
    ExportResult *ast_exporter(int argc, const char **argv, int debug, int *result);

    // Free the export result
    void drop_export_result(ExportResult *result);

    // Get clang version string
    const char *clang_version();
}

#endif // FRAGILE_AST_EXPORTER_H
