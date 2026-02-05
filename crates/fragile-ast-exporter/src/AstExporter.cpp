//
// AstExporter.cpp
// C++ AST exporter using Clang LibTooling for Fragile transpiler
//
// This exports C++ AST including template instantiations to CBOR format
// for consumption by the Rust transpiler.
//

#include <algorithm>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <memory>
#include <optional>
#include <set>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>

// LLVM headers
#include "llvm/Support/CommandLine.h"
#include "llvm/Support/raw_ostream.h"

// Clang headers
#include "clang/AST/ASTContext.h"
#include "clang/AST/Decl.h"
#include "clang/AST/DeclCXX.h"
#include "clang/AST/DeclTemplate.h"
#include "clang/AST/Expr.h"
#include "clang/AST/ExprCXX.h"
#include "clang/AST/RecursiveASTVisitor.h"
#include "clang/AST/Stmt.h"
#include "clang/AST/StmtCXX.h"
#include "clang/AST/Type.h"
#include "clang/Basic/SourceManager.h"
#include "clang/Basic/Version.h"
#include "clang/Frontend/CompilerInstance.h"
#include "clang/Frontend/FrontendActions.h"
#include "clang/Tooling/CommonOptionsParser.h"
#include "clang/Tooling/Tooling.h"

#include "AstExporter.hpp"
#include "ast_tags.hpp"
#include <tinycbor/cbor.h>

using namespace clang;
using namespace clang::tooling;

// Import only specific llvm items we need (avoid importing llvm::Type which conflicts with clang::Type)
using llvm::cl::opt;
using llvm::cl::OptionCategory;
using llvm::cl::extrahelp;
using llvm::cl::cat;
using llvm::cl::desc;
using llvm::StringRef;
using llvm::DenseMap;

// Compatibility macros for different LLVM/Clang versions
#if CLANG_VERSION_MAJOR < 15
#define OPTIONAL_VALUE_OR(opt, def) ((opt).hasValue() ? (opt).getValue() : (def))
#else
#define OPTIONAL_VALUE_OR(opt, def) ((opt).value_or(def))
#endif

namespace {

// Command line options
static OptionCategory FragileCategory("fragile-ast-exporter options");
static opt<bool> DebugMode("debug", desc("Enable debug output"),
                                cat(FragileCategory));

// Helper to encode a string to CBOR
void cbor_encode_string(CborEncoder *encoder, const std::string &str) {
    cbor_encode_text_string(encoder, str.data(), str.size());
}

// Forward declarations
class TypeEncoder;
class ASTExporterVisitor;

// ============================================================================
// TypeEncoder - Encodes C++ types to CBOR
// ============================================================================
class TypeEncoder {
    ASTContext &Context;
    CborEncoder *encoder;
    std::unordered_set<const Type *> exportedTypes;
    ASTExporterVisitor *visitor;

public:
    TypeEncoder(ASTContext &ctx, CborEncoder *enc, ASTExporterVisitor *v)
        : Context(ctx), encoder(enc), visitor(v) {}

    uint64_t encodeQualType(QualType QT);
    void visitQualType(QualType QT);

private:
    bool markExported(const Type *T) {
        return exportedTypes.insert(T).second;
    }

    void encodeType(const Type *T, ASTEntryTag tag,
                    std::function<void(CborEncoder *)> extra = [](CborEncoder *) {});

    void visitBuiltinType(const BuiltinType *T);
    void visitPointerType(const clang::PointerType *T);
    void visitReferenceType(const ReferenceType *T);
    void visitArrayType(const ArrayType *T);
    void visitFunctionProtoType(const FunctionProtoType *T);
    void visitRecordType(const RecordType *T);
    void visitEnumType(const clang::EnumType *T);
    void visitTypedefType(const TypedefType *T);
    void visitElaboratedType(const ElaboratedType *T);
    void visitTemplateSpecializationType(const TemplateSpecializationType *T);
    void visitSubstTemplateTypeParmType(const SubstTemplateTypeParmType *T);
    void visitAutoType(const AutoType *T);
    void visitDecltypeType(const DecltypeType *T);
    void visitAttributedType(const AttributedType *T);
    void visitParenType(const ParenType *T);
    void visitDecayedType(const DecayedType *T);
};

// ============================================================================
// ASTExporterVisitor - Main AST visitor
// ============================================================================
class ASTExporterVisitor : public RecursiveASTVisitor<ASTExporterVisitor> {
    ASTContext &Context;
    CborEncoder *encoder;
    TypeEncoder typeEncoder;

    // Track exported declarations to avoid duplicates
    std::unordered_set<const void *> exportedDecls;

    // Track source files
    std::vector<std::pair<std::string, SourceLocation>> files;
    DenseMap<FileID, size_t> fileIdMapping;

    // Debug mode
    bool debug;

public:
    ASTExporterVisitor(ASTContext &ctx, CborEncoder *enc, bool dbg = false)
        : Context(ctx), encoder(enc), typeEncoder(ctx, enc, this), debug(dbg) {}

    // Enable visiting template instantiations - THIS IS KEY!
    bool shouldVisitTemplateInstantiations() const { return true; }

    // Visit implicit code (compiler-generated)
    bool shouldVisitImplicitCode() const { return true; }

    // Entry point
    void exportTranslationUnit();

    // Declaration visitors
    bool VisitFunctionDecl(FunctionDecl *FD);
    bool VisitCXXMethodDecl(CXXMethodDecl *MD);
    bool VisitCXXConstructorDecl(CXXConstructorDecl *CD);
    bool VisitCXXDestructorDecl(CXXDestructorDecl *DD);
    bool VisitVarDecl(VarDecl *VD);
    bool VisitParmVarDecl(ParmVarDecl *PVD);
    bool VisitFieldDecl(FieldDecl *FD);
    bool VisitCXXRecordDecl(CXXRecordDecl *RD);
    bool VisitClassTemplateDecl(ClassTemplateDecl *CTD);
    bool VisitClassTemplateSpecializationDecl(ClassTemplateSpecializationDecl *CTSD);
    bool VisitNamespaceDecl(NamespaceDecl *ND);
    bool VisitTypedefDecl(TypedefDecl *TD);
    bool VisitTypeAliasDecl(TypeAliasDecl *TAD);
    bool VisitEnumDecl(EnumDecl *ED);
    bool VisitEnumConstantDecl(EnumConstantDecl *ECD);

    // Statement visitors
    void visitStmt(Stmt *S);
    void visitCompoundStmt(CompoundStmt *CS);
    void visitDeclStmt(DeclStmt *DS);
    void visitReturnStmt(ReturnStmt *RS);
    void visitIfStmt(IfStmt *IS);
    void visitWhileStmt(WhileStmt *WS);
    void visitForStmt(ForStmt *FS);
    void visitCXXForRangeStmt(CXXForRangeStmt *FRS);
    void visitBreakStmt(BreakStmt *BS);
    void visitContinueStmt(ContinueStmt *CS);
    void visitSwitchStmt(SwitchStmt *SS);
    void visitCaseStmt(CaseStmt *CS);
    void visitDefaultStmt(DefaultStmt *DS);

    // Expression visitors
    void visitExpr(Expr *E);
    void visitIntegerLiteral(IntegerLiteral *IL);
    void visitFloatingLiteral(FloatingLiteral *FL);
    void visitStringLiteral(clang::StringLiteral *SL);
    void visitCharacterLiteral(CharacterLiteral *CL);
    void visitCXXBoolLiteralExpr(CXXBoolLiteralExpr *BL);
    void visitCXXNullPtrLiteralExpr(CXXNullPtrLiteralExpr *NL);
    void visitDeclRefExpr(DeclRefExpr *DRE);
    void visitMemberExpr(MemberExpr *ME);
    void visitCXXThisExpr(CXXThisExpr *TE);
    void visitCallExpr(CallExpr *CE);
    void visitCXXMemberCallExpr(CXXMemberCallExpr *MCE);
    void visitCXXOperatorCallExpr(CXXOperatorCallExpr *OCE);
    void visitCXXConstructExpr(CXXConstructExpr *CCE);
    void visitBinaryOperator(BinaryOperator *BO);
    void visitUnaryOperator(UnaryOperator *UO);
    void visitCastExpr(CastExpr *CE);
    void visitArraySubscriptExpr(ArraySubscriptExpr *ASE);
    void visitInitListExpr(InitListExpr *ILE);
    void visitParenExpr(ParenExpr *PE);
    void visitUnaryExprOrTypeTraitExpr(UnaryExprOrTypeTraitExpr *UE);
    void visitConditionalOperator(ConditionalOperator *CO);
    void visitCXXNewExpr(CXXNewExpr *NE);
    void visitCXXDeleteExpr(CXXDeleteExpr *DE);
    void visitExprWithCleanups(ExprWithCleanups *EWC);
    void visitMaterializeTemporaryExpr(MaterializeTemporaryExpr *MTE);
    void visitCXXBindTemporaryExpr(CXXBindTemporaryExpr *BTE);
    void visitImplicitValueInitExpr(ImplicitValueInitExpr *IVE);
    void visitCXXDefaultArgExpr(CXXDefaultArgExpr *DAE);
    void visitCXXDefaultInitExpr(CXXDefaultInitExpr *DIE);
    void visitLambdaExpr(LambdaExpr *LE);

    // Type encoding access
    TypeEncoder &getTypeEncoder() { return typeEncoder; }

private:
    // Helper to encode source location
    void encodeSourceLocation(SourceLocation Loc, CborEncoder *enc);
    void encodeSourceRange(SourceRange Range, CborEncoder *enc);

    // Helper to check if already exported
    bool markExported(const void *ptr) {
        return exportedDecls.insert(ptr).second;
    }

    // Get file ID for a source location
    size_t getFileId(SourceLocation Loc);

    // Encode a declaration entry
    void encodeEntry(const void *ptr, ASTEntryTag tag, SourceRange range,
                     const std::vector<const void *> &children,
                     QualType type = QualType(),
                     std::function<void(CborEncoder *)> extra = [](CborEncoder *) {});

    // Recursively ensure field type specializations are exported
    void ensureFieldTypeSpecializationsExported(ClassTemplateSpecializationDecl *CTSD);
};

// ============================================================================
// TypeEncoder Implementation
// ============================================================================

uint64_t TypeEncoder::encodeQualType(QualType QT) {
    if (QT.isNull())
        return 0;

    const Type *T = QT.getTypePtr();
    uint64_t id = reinterpret_cast<uint64_t>(T);

    // Encode qualifiers in the lower bits
    if (QT.isConstQualified())
        id |= 0x1;
    if (QT.isRestrictQualified())
        id |= 0x2;
    if (QT.isVolatileQualified())
        id |= 0x4;

    return id;
}

void TypeEncoder::visitQualType(QualType QT) {
    if (QT.isNull())
        return;

    const Type *T = QT.getTypePtr();
    if (!markExported(T))
        return;

    // Dispatch based on type class
    switch (T->getTypeClass()) {
    case Type::Builtin:
        visitBuiltinType(cast<BuiltinType>(T));
        break;
    case Type::Pointer:
        visitPointerType(cast<clang::PointerType>(T));
        break;
    case Type::LValueReference:
    case Type::RValueReference:
        visitReferenceType(cast<ReferenceType>(T));
        break;
    case Type::ConstantArray:
    case Type::IncompleteArray:
    case Type::VariableArray:
        visitArrayType(cast<ArrayType>(T));
        break;
    case Type::FunctionProto:
        visitFunctionProtoType(cast<FunctionProtoType>(T));
        break;
    case Type::Record:
        visitRecordType(cast<RecordType>(T));
        break;
    case Type::Enum:
        visitEnumType(cast<clang::EnumType>(T));
        break;
    case Type::Typedef:
        visitTypedefType(cast<TypedefType>(T));
        break;
    case Type::Elaborated:
        visitElaboratedType(cast<ElaboratedType>(T));
        break;
    case Type::TemplateSpecialization:
        visitTemplateSpecializationType(cast<TemplateSpecializationType>(T));
        break;
    case Type::SubstTemplateTypeParm:
        visitSubstTemplateTypeParmType(cast<SubstTemplateTypeParmType>(T));
        break;
    case Type::Auto:
        visitAutoType(cast<AutoType>(T));
        break;
    case Type::Decltype:
        visitDecltypeType(cast<DecltypeType>(T));
        break;
    case Type::Attributed:
        visitAttributedType(cast<AttributedType>(T));
        break;
    case Type::Paren:
        visitParenType(cast<ParenType>(T));
        break;
    case Type::Decayed:
        visitDecayedType(cast<DecayedType>(T));
        break;
    default:
        // For unknown types, try to get the desugared type
        {
            auto desugared = T->getLocallyUnqualifiedSingleStepDesugaredType();
            if (!desugared.isNull() && desugared.getTypePtr() != T) {
                visitQualType(desugared);
            }
        }
        break;
    }
}

void TypeEncoder::encodeType(const Type *T, ASTEntryTag tag,
                              std::function<void(CborEncoder *)> extra) {
    CborEncoder entry;
    cbor_encoder_create_array(encoder, &entry, CborIndefiniteLength);

    // ID
    cbor_encode_uint(&entry, reinterpret_cast<uint64_t>(T));
    // Tag
    cbor_encode_uint(&entry, tag);
    // Extra data
    extra(&entry);

    cbor_encoder_close_container(encoder, &entry);
}

void TypeEncoder::visitBuiltinType(const BuiltinType *T) {
    ASTEntryTag tag;
    switch (T->getKind()) {
    case BuiltinType::Void:
        tag = TagVoid;
        break;
    case BuiltinType::Bool:
        tag = TagBool;
        break;
    case BuiltinType::Char_S:
    case BuiltinType::Char_U:
        tag = TagChar;
        break;
    case BuiltinType::SChar:
        tag = TagSChar;
        break;
    case BuiltinType::UChar:
        tag = TagUChar;
        break;
    case BuiltinType::WChar_S:
    case BuiltinType::WChar_U:
        tag = TagWChar;
        break;
    case BuiltinType::Char16:
        tag = TagChar16;
        break;
    case BuiltinType::Char32:
        tag = TagChar32;
        break;
    case BuiltinType::Short:
        tag = TagShort;
        break;
    case BuiltinType::UShort:
        tag = TagUShort;
        break;
    case BuiltinType::Int:
        tag = TagInt;
        break;
    case BuiltinType::UInt:
        tag = TagUInt;
        break;
    case BuiltinType::Long:
        tag = TagLong;
        break;
    case BuiltinType::ULong:
        tag = TagULong;
        break;
    case BuiltinType::LongLong:
        tag = TagLongLong;
        break;
    case BuiltinType::ULongLong:
        tag = TagULongLong;
        break;
    case BuiltinType::Int128:
        tag = TagInt128;
        break;
    case BuiltinType::UInt128:
        tag = TagUInt128;
        break;
    case BuiltinType::Float:
        tag = TagFloat;
        break;
    case BuiltinType::Double:
        tag = TagDouble;
        break;
    case BuiltinType::LongDouble:
        tag = TagLongDouble;
        break;
    case BuiltinType::Float128:
        tag = TagFloat128;
        break;
    default:
        tag = TagTypeUnknown;
        break;
    }
    encodeType(T, tag);
}

void TypeEncoder::visitPointerType(const clang::PointerType *T) {
    auto pointee = T->getPointeeType();
    auto pointeeId = encodeQualType(pointee);

    encodeType(T, TagPointerType, [pointeeId](CborEncoder *enc) {
        cbor_encode_uint(enc, pointeeId);
    });

    visitQualType(pointee);
}

void TypeEncoder::visitReferenceType(const ReferenceType *T) {
    auto pointee = T->getPointeeType();
    auto pointeeId = encodeQualType(pointee);
    ASTEntryTag tag = isa<LValueReferenceType>(T) ? TagLValueReferenceType
                                                   : TagRValueReferenceType;

    encodeType(T, tag, [pointeeId](CborEncoder *enc) {
        cbor_encode_uint(enc, pointeeId);
    });

    visitQualType(pointee);
}

void TypeEncoder::visitArrayType(const ArrayType *T) {
    auto elemType = T->getElementType();
    auto elemId = encodeQualType(elemType);
    ASTEntryTag tag;
    uint64_t size = 0;

    if (auto *CAT = dyn_cast<ConstantArrayType>(T)) {
        tag = TagConstantArrayType;
        size = CAT->getSize().getZExtValue();
    } else if (isa<IncompleteArrayType>(T)) {
        tag = TagIncompleteArrayType;
    } else {
        tag = TagVariableArrayType;
    }

    encodeType(T, tag, [elemId, size, tag](CborEncoder *enc) {
        cbor_encode_uint(enc, elemId);
        if (tag == TagConstantArrayType) {
            cbor_encode_uint(enc, size);
        }
    });

    visitQualType(elemType);
}

void TypeEncoder::visitFunctionProtoType(const FunctionProtoType *T) {
    auto retType = T->getReturnType();
    auto retId = encodeQualType(retType);

    encodeType(T, TagFunctionProtoType, [T, retId, this](CborEncoder *enc) {
        // Return type
        cbor_encode_uint(enc, retId);

        // Parameter types
        CborEncoder params;
        cbor_encoder_create_array(enc, &params, T->getNumParams());
        for (auto paramType : T->param_types()) {
            cbor_encode_uint(&params, encodeQualType(paramType));
        }
        cbor_encoder_close_container(enc, &params);

        // Is variadic
        cbor_encode_boolean(enc, T->isVariadic());
    });

    visitQualType(retType);
    for (auto paramType : T->param_types()) {
        visitQualType(paramType);
    }
}

void TypeEncoder::visitRecordType(const RecordType *T) {
    auto *RD = T->getDecl();
    encodeType(T, TagRecordType, [RD](CborEncoder *enc) {
        cbor_encode_uint(enc, reinterpret_cast<uint64_t>(RD));

        // For template specializations, encode the full name with template arguments
        // so the Rust side can match field types to exported specializations
        if (auto *CTSD = dyn_cast<ClassTemplateSpecializationDecl>(RD)) {
            std::string name = CTSD->getNameAsString();
            const auto &args = CTSD->getTemplateArgs();
            name += "<";
            for (unsigned i = 0; i < args.size(); ++i) {
                if (i > 0) name += ", ";
                std::string argStr;
                llvm::raw_string_ostream os(argStr);
                args[i].print(PrintingPolicy(LangOptions()), os, true);
                name += os.str();
            }
            name += ">";
            cbor_encode_string(enc, name);
        } else {
            cbor_encode_string(enc, RD->getNameAsString());
        }
    });
}

void TypeEncoder::visitEnumType(const clang::EnumType *T) {
    auto *ED = T->getDecl();
    encodeType(T, TagEnumType, [ED](CborEncoder *enc) {
        cbor_encode_uint(enc, reinterpret_cast<uint64_t>(ED));
        cbor_encode_string(enc, ED->getNameAsString());
    });
}

void TypeEncoder::visitTypedefType(const TypedefType *T) {
    auto underlying = T->desugar();
    auto underlyingId = encodeQualType(underlying);
    auto *TD = T->getDecl();

    encodeType(T, TagTypedefType, [TD, underlyingId](CborEncoder *enc) {
        cbor_encode_string(enc, TD->getNameAsString());
        cbor_encode_uint(enc, underlyingId);
    });

    visitQualType(underlying);
}

void TypeEncoder::visitElaboratedType(const ElaboratedType *T) {
    auto named = T->getNamedType();
    auto namedId = encodeQualType(named);

    encodeType(T, TagElaboratedType, [namedId](CborEncoder *enc) {
        cbor_encode_uint(enc, namedId);
    });

    visitQualType(named);
}

void TypeEncoder::visitTemplateSpecializationType(const TemplateSpecializationType *T) {
    // Check if this is a type alias template (like __type_identity_t)
    // If so, we want to encode the desugared/aliased type as well
    uint64_t aliasedTypeId = 0;
    if (T->isTypeAlias()) {
        auto aliasedType = T->getAliasedType();
        if (!aliasedType.isNull()) {
            aliasedTypeId = encodeQualType(aliasedType);
        }
    }

    encodeType(T, TagTemplateSpecializationType, [T, this, aliasedTypeId](CborEncoder *enc) {
        // Template name
        auto templateName = T->getTemplateName();
        if (auto *TD = templateName.getAsTemplateDecl()) {
            cbor_encode_string(enc, TD->getNameAsString());
        } else {
            cbor_encode_string(enc, "");
        }

        // Template arguments
        CborEncoder args;
        cbor_encoder_create_array(enc, &args, T->template_arguments().size());
        for (const auto &arg : T->template_arguments()) {
            if (arg.getKind() == TemplateArgument::Type) {
                cbor_encode_uint(&args, encodeQualType(arg.getAsType()));
            } else {
                cbor_encode_uint(&args, 0); // Non-type arguments
            }
        }
        cbor_encoder_close_container(enc, &args);

        // Aliased type ID (0 if not a type alias template)
        cbor_encode_uint(enc, aliasedTypeId);
    });

    // Visit template argument types
    for (const auto &arg : T->template_arguments()) {
        if (arg.getKind() == TemplateArgument::Type) {
            visitQualType(arg.getAsType());
        }
    }

    // Visit the aliased type if present
    if (T->isTypeAlias()) {
        auto aliasedType = T->getAliasedType();
        if (!aliasedType.isNull()) {
            visitQualType(aliasedType);
        }
    }
}

void TypeEncoder::visitSubstTemplateTypeParmType(const SubstTemplateTypeParmType *T) {
    auto replacement = T->getReplacementType();
    auto replacementId = encodeQualType(replacement);

    encodeType(T, TagSubstTemplateTypeParmType, [replacementId](CborEncoder *enc) {
        cbor_encode_uint(enc, replacementId);
    });

    visitQualType(replacement);
}

void TypeEncoder::visitAutoType(const AutoType *T) {
    auto deduced = T->getDeducedType();
    auto deducedId = encodeQualType(deduced);

    encodeType(T, TagAutoType, [deducedId](CborEncoder *enc) {
        cbor_encode_uint(enc, deducedId);
    });

    if (!deduced.isNull()) {
        visitQualType(deduced);
    }
}

void TypeEncoder::visitDecltypeType(const DecltypeType *T) {
    auto underlying = T->getUnderlyingType();
    auto underlyingId = encodeQualType(underlying);

    encodeType(T, TagDecltypeType, [underlyingId](CborEncoder *enc) {
        cbor_encode_uint(enc, underlyingId);
    });

    visitQualType(underlying);
}

void TypeEncoder::visitAttributedType(const AttributedType *T) {
    auto modified = T->getModifiedType();
    auto modifiedId = encodeQualType(modified);

    encodeType(T, TagAttributedType, [modifiedId](CborEncoder *enc) {
        cbor_encode_uint(enc, modifiedId);
    });

    visitQualType(modified);
}

void TypeEncoder::visitParenType(const ParenType *T) {
    auto inner = T->getInnerType();
    auto innerId = encodeQualType(inner);

    encodeType(T, TagParenType, [innerId](CborEncoder *enc) {
        cbor_encode_uint(enc, innerId);
    });

    visitQualType(inner);
}

void TypeEncoder::visitDecayedType(const DecayedType *T) {
    auto decayed = T->getDecayedType();
    auto decayedId = encodeQualType(decayed);

    encodeType(T, TagDecayedType, [decayedId](CborEncoder *enc) {
        cbor_encode_uint(enc, decayedId);
    });

    visitQualType(decayed);
}

// ============================================================================
// ASTExporterVisitor Implementation
// ============================================================================

void ASTExporterVisitor::exportTranslationUnit() {
    TraverseDecl(Context.getTranslationUnitDecl());
}

void ASTExporterVisitor::encodeSourceLocation(SourceLocation Loc, CborEncoder *enc) {
    if (Loc.isInvalid()) {
        cbor_encode_uint(enc, 0);
        cbor_encode_uint(enc, 0);
        cbor_encode_uint(enc, 0);
        return;
    }

    auto &SM = Context.getSourceManager();
    auto ExpLoc = SM.getExpansionLoc(Loc);
    auto FileId = getFileId(ExpLoc);
    auto Line = SM.getExpansionLineNumber(ExpLoc);
    auto Col = SM.getExpansionColumnNumber(ExpLoc);

    cbor_encode_uint(enc, FileId);
    cbor_encode_uint(enc, Line);
    cbor_encode_uint(enc, Col);
}

void ASTExporterVisitor::encodeSourceRange(SourceRange Range, CborEncoder *enc) {
    encodeSourceLocation(Range.getBegin(), enc);

    if (Range.getEnd().isValid()) {
        auto &SM = Context.getSourceManager();
        auto EndLoc = SM.getExpansionLoc(Range.getEnd());
        cbor_encode_uint(enc, SM.getExpansionLineNumber(EndLoc));
        cbor_encode_uint(enc, SM.getExpansionColumnNumber(EndLoc));
    } else {
        cbor_encode_uint(enc, 0);
        cbor_encode_uint(enc, 0);
    }
}

size_t ASTExporterVisitor::getFileId(SourceLocation Loc) {
    if (Loc.isInvalid())
        return 0;

    auto &SM = Context.getSourceManager();
    auto FID = SM.getFileID(SM.getExpansionLoc(Loc));

    auto it = fileIdMapping.find(FID);
    if (it != fileIdMapping.end())
        return it->second;

    size_t id = files.size();
    fileIdMapping[FID] = id;

    std::string filename;
#if CLANG_VERSION_MAJOR >= 19
    if (auto FE = SM.getFileEntryRefForID(FID)) {
        filename = FE->getName().str();
    }
#else
    if (auto *FE = SM.getFileEntryForID(FID)) {
        filename = FE->getName().str();
    }
#endif
    files.push_back({filename, Loc});

    return id;
}

void ASTExporterVisitor::encodeEntry(const void *ptr, ASTEntryTag tag,
                                      SourceRange range,
                                      const std::vector<const void *> &children,
                                      QualType type,
                                      std::function<void(CborEncoder *)> extra) {
    CborEncoder entry;
    cbor_encoder_create_array(encoder, &entry, CborIndefiniteLength);

    // ID
    cbor_encode_uint(&entry, reinterpret_cast<uint64_t>(ptr));

    // Tag
    cbor_encode_uint(&entry, tag);

    // Children
    CborEncoder childArray;
    cbor_encoder_create_array(&entry, &childArray, children.size());
    for (auto child : children) {
        if (child) {
            cbor_encode_uint(&childArray, reinterpret_cast<uint64_t>(child));
        } else {
            cbor_encode_null(&childArray);
        }
    }
    cbor_encoder_close_container(&entry, &childArray);

    // Source location
    encodeSourceRange(range, &entry);

    // Type (if any)
    if (!type.isNull()) {
        cbor_encode_uint(&entry, typeEncoder.encodeQualType(type));
    } else {
        cbor_encode_null(&entry);
    }

    // Extra data
    extra(&entry);

    cbor_encoder_close_container(encoder, &entry);

    // Visit the type
    if (!type.isNull()) {
        typeEncoder.visitQualType(type);
    }
}

// ============================================================================
// Declaration Visitors
// ============================================================================

bool ASTExporterVisitor::VisitFunctionDecl(FunctionDecl *FD) {
    // Skip methods - they're handled by VisitCXXMethodDecl
    if (isa<CXXMethodDecl>(FD))
        return true;

    if (!markExported(FD))
        return true;

    // Get function body
    Stmt *body = FD->getBody();

    std::vector<const void *> children;
    for (auto *param : FD->parameters()) {
        children.push_back(param);
    }
    children.push_back(body);

    encodeEntry(FD, TagFunctionDecl, FD->getSourceRange(), children,
                FD->getType(), [FD](CborEncoder *enc) {
                    cbor_encode_string(enc, FD->getNameAsString());
                    cbor_encode_boolean(enc, FD->isGlobal());
                    cbor_encode_boolean(enc, FD->isInlineSpecified());
                    cbor_encode_boolean(enc, FD->isStatic());
                });

    // Visit body
    if (body) {
        visitStmt(body);
    }

    return true;
}

bool ASTExporterVisitor::VisitCXXMethodDecl(CXXMethodDecl *MD) {
    // Skip constructors/destructors - handled separately
    if (isa<CXXConstructorDecl>(MD) || isa<CXXDestructorDecl>(MD))
        return true;

    if (!markExported(MD))
        return true;

    Stmt *body = MD->getBody();

    std::vector<const void *> children;
    for (auto *param : MD->parameters()) {
        children.push_back(param);
    }
    children.push_back(body);

    // Check if this is from a template instantiation
    bool isInstantiation = MD->isTemplateInstantiation();

    encodeEntry(MD, TagCXXMethodDecl, MD->getSourceRange(), children,
                MD->getType(), [MD, isInstantiation](CborEncoder *enc) {
                    cbor_encode_string(enc, MD->getNameAsString());
                    cbor_encode_boolean(enc, MD->isStatic());
                    cbor_encode_boolean(enc, MD->isConst());
                    cbor_encode_boolean(enc, MD->isVirtual());
                    cbor_encode_boolean(enc, MD->isPureVirtual());
                    cbor_encode_uint(enc, MD->getAccess());
                    cbor_encode_boolean(enc, isInstantiation);

                    // Parent class
                    if (auto *parent = MD->getParent()) {
                        cbor_encode_uint(enc, reinterpret_cast<uint64_t>(parent));
                    } else {
                        cbor_encode_null(enc);
                    }
                });

    if (body) {
        visitStmt(body);
    }

    return true;
}

bool ASTExporterVisitor::VisitCXXConstructorDecl(CXXConstructorDecl *CD) {
    if (!markExported(CD))
        return true;

    Stmt *body = CD->getBody();

    std::vector<const void *> children;
    for (auto *param : CD->parameters()) {
        children.push_back(param);
    }
    children.push_back(body);

    bool isInstantiation = CD->isTemplateInstantiation();

    encodeEntry(CD, TagCXXConstructorDecl, CD->getSourceRange(), children,
                CD->getType(), [CD, isInstantiation](CborEncoder *enc) {
                    cbor_encode_boolean(enc, CD->isDefaultConstructor());
                    cbor_encode_boolean(enc, CD->isCopyConstructor());
                    cbor_encode_boolean(enc, CD->isMoveConstructor());
                    cbor_encode_boolean(enc, CD->isExplicit());
                    cbor_encode_uint(enc, CD->getAccess());
                    cbor_encode_boolean(enc, isInstantiation);

                    // Parent class
                    if (auto *parent = CD->getParent()) {
                        cbor_encode_uint(enc, reinterpret_cast<uint64_t>(parent));
                    } else {
                        cbor_encode_null(enc);
                    }

                    // Member initializers
                    CborEncoder inits;
                    cbor_encoder_create_array(enc, &inits, CD->getNumCtorInitializers());
                    for (auto *init : CD->inits()) {
                        CborEncoder initEntry;
                        cbor_encoder_create_array(&inits, &initEntry, 3);

                        // Field or base
                        if (init->isMemberInitializer()) {
                            cbor_encode_string(&initEntry, init->getMember()->getNameAsString());
                        } else if (init->isBaseInitializer()) {
                            cbor_encode_string(&initEntry, init->getTypeSourceInfo()->getType().getAsString());
                        } else {
                            cbor_encode_string(&initEntry, "");
                        }

                        // Is member (vs base)
                        cbor_encode_boolean(&initEntry, init->isMemberInitializer());

                        // Init expression
                        if (auto *initExpr = init->getInit()) {
                            cbor_encode_uint(&initEntry, reinterpret_cast<uint64_t>(initExpr));
                        } else {
                            cbor_encode_null(&initEntry);
                        }

                        cbor_encoder_close_container(&inits, &initEntry);
                    }
                    cbor_encoder_close_container(enc, &inits);
                });

    // Visit initializer expressions
    for (auto *init : CD->inits()) {
        if (auto *initExpr = init->getInit()) {
            visitExpr(initExpr);
        }
    }

    if (body) {
        visitStmt(body);
    }

    return true;
}

bool ASTExporterVisitor::VisitCXXDestructorDecl(CXXDestructorDecl *DD) {
    if (!markExported(DD))
        return true;

    Stmt *body = DD->getBody();

    std::vector<const void *> children;
    children.push_back(body);

    bool isInstantiation = DD->isTemplateInstantiation();

    encodeEntry(DD, TagCXXDestructorDecl, DD->getSourceRange(), children,
                DD->getType(), [DD, isInstantiation](CborEncoder *enc) {
                    cbor_encode_boolean(enc, DD->isVirtual());
                    cbor_encode_uint(enc, DD->getAccess());
                    cbor_encode_boolean(enc, isInstantiation);

                    // Parent class
                    if (auto *parent = DD->getParent()) {
                        cbor_encode_uint(enc, reinterpret_cast<uint64_t>(parent));
                    } else {
                        cbor_encode_null(enc);
                    }
                });

    if (body) {
        visitStmt(body);
    }

    return true;
}

bool ASTExporterVisitor::VisitVarDecl(VarDecl *VD) {
    // Skip parameters - handled separately
    if (isa<ParmVarDecl>(VD))
        return true;

    if (!markExported(VD))
        return true;

    Expr *init = VD->getInit();

    std::vector<const void *> children;
    children.push_back(init);

    encodeEntry(VD, TagVarDecl, VD->getSourceRange(), children,
                VD->getType(), [VD](CborEncoder *enc) {
                    cbor_encode_string(enc, VD->getNameAsString());
                    cbor_encode_boolean(enc, VD->isStaticLocal());
                    cbor_encode_boolean(enc, VD->isConstexpr());
                    cbor_encode_boolean(enc, VD->hasExternalStorage());
                });

    if (init) {
        visitExpr(init);
    }

    return true;
}

bool ASTExporterVisitor::VisitParmVarDecl(ParmVarDecl *PVD) {
    if (!markExported(PVD))
        return true;

    Expr *defaultArg = PVD->hasDefaultArg() ? PVD->getDefaultArg() : nullptr;

    std::vector<const void *> children;
    children.push_back(defaultArg);

    encodeEntry(PVD, TagParmVarDecl, PVD->getSourceRange(), children,
                PVD->getType(), [PVD](CborEncoder *enc) {
                    cbor_encode_string(enc, PVD->getNameAsString());
                });

    if (defaultArg) {
        visitExpr(defaultArg);
    }

    return true;
}

bool ASTExporterVisitor::VisitFieldDecl(FieldDecl *FD) {
    if (!markExported(FD))
        return true;

    Expr *init = FD->getInClassInitializer();
    Expr *bitWidth = FD->getBitWidth();

    std::vector<const void *> children;
    children.push_back(init);
    children.push_back(bitWidth);

    encodeEntry(FD, TagFieldDecl, FD->getSourceRange(), children,
                FD->getType(), [FD](CborEncoder *enc) {
                    cbor_encode_string(enc, FD->getNameAsString());
                    cbor_encode_boolean(enc, FD->isMutable());
                    cbor_encode_uint(enc, FD->getAccess());
                });

    if (init) {
        visitExpr(init);
    }
    if (bitWidth) {
        visitExpr(bitWidth);
    }

    return true;
}

bool ASTExporterVisitor::VisitCXXRecordDecl(CXXRecordDecl *RD) {
    // Skip template patterns - we want the specializations
    if (RD->getDescribedClassTemplate())
        return true;

    // Skip template specializations - they are handled by VisitClassTemplateSpecializationDecl
    if (isa<ClassTemplateSpecializationDecl>(RD))
        return true;

    if (!markExported(RD))
        return true;

    std::vector<const void *> children;

    // Add base classes
    if (RD->hasDefinition()) {
        for (const auto &base : RD->bases()) {
            // Store base class type
        }
    }

    encodeEntry(RD, TagCXXRecordDecl, RD->getSourceRange(), children,
                QualType(), [RD](CborEncoder *enc) {
                    cbor_encode_string(enc, RD->getNameAsString());
                    cbor_encode_boolean(enc, RD->isStruct());
                    cbor_encode_boolean(enc, RD->isClass());
                    cbor_encode_boolean(enc, RD->isUnion());
                    cbor_encode_boolean(enc, RD->hasDefinition());

                    // Base classes
                    if (RD->hasDefinition()) {
                        CborEncoder bases;
                        cbor_encoder_create_array(enc, &bases, RD->getNumBases());
                        for (const auto &base : RD->bases()) {
                            CborEncoder baseEntry;
                            cbor_encoder_create_array(&bases, &baseEntry, 3);
                            cbor_encode_string(&baseEntry, base.getType().getAsString());
                            cbor_encode_uint(&baseEntry, base.getAccessSpecifier());
                            cbor_encode_boolean(&baseEntry, base.isVirtual());
                            cbor_encoder_close_container(&bases, &baseEntry);
                        }
                        cbor_encoder_close_container(enc, &bases);
                    } else {
                        CborEncoder bases;
                        cbor_encoder_create_array(enc, &bases, 0);
                        cbor_encoder_close_container(enc, &bases);
                    }
                });

    return true;
}

bool ASTExporterVisitor::VisitClassTemplateDecl(ClassTemplateDecl *CTD) {
    if (!markExported(CTD))
        return true;

    std::vector<const void *> children;

    encodeEntry(CTD, TagClassTemplateDecl, CTD->getSourceRange(), children,
                QualType(), [CTD](CborEncoder *enc) {
                    cbor_encode_string(enc, CTD->getNameAsString());

                    // Template parameters
                    auto *params = CTD->getTemplateParameters();
                    CborEncoder paramArray;
                    cbor_encoder_create_array(enc, &paramArray, params->size());
                    for (auto *param : *params) {
                        cbor_encode_string(&paramArray, param->getNameAsString());
                    }
                    cbor_encoder_close_container(enc, &paramArray);
                });

    return true;
}

bool ASTExporterVisitor::VisitClassTemplateSpecializationDecl(ClassTemplateSpecializationDecl *CTSD) {
    if (!markExported(CTSD))
        return true;

    if (debug) {
        llvm::errs() << "Visiting template specialization: "
                     << CTSD->getQualifiedNameAsString() << "\n";
    }

    std::vector<const void *> children;

    // Collect methods, fields, etc.
    for (auto *D : CTSD->decls()) {
        if (auto *MD = dyn_cast<CXXMethodDecl>(D)) {
            children.push_back(MD);
        } else if (auto *FD = dyn_cast<FieldDecl>(D)) {
            children.push_back(FD);
        }
    }

    encodeEntry(CTSD, TagClassTemplateSpecializationDecl, CTSD->getSourceRange(),
                children, QualType(), [CTSD](CborEncoder *enc) {
                    cbor_encode_string(enc, CTSD->getNameAsString());

                    // Qualified name (includes template args)
                    cbor_encode_string(enc, CTSD->getQualifiedNameAsString());

                    // Template arguments
                    const auto &args = CTSD->getTemplateArgs();
                    CborEncoder argArray;
                    cbor_encoder_create_array(enc, &argArray, args.size());
                    for (unsigned i = 0; i < args.size(); ++i) {
                        const auto &arg = args[i];
                        CborEncoder argEntry;
                        cbor_encoder_create_array(&argArray, &argEntry, 2);

                        // Argument kind
                        cbor_encode_uint(&argEntry, arg.getKind());

                        // Argument value (as string for now)
                        std::string argStr;
                        llvm::raw_string_ostream os(argStr);
                        arg.print(PrintingPolicy(LangOptions()), os, true);
                        cbor_encode_string(&argEntry, os.str());

                        cbor_encoder_close_container(&argArray, &argEntry);
                    }
                    cbor_encoder_close_container(enc, &argArray);

                    // Is implicit instantiation
                    cbor_encode_boolean(enc, CTSD->getSpecializationKind() ==
                                                 TSK_ImplicitInstantiation);

                    // Is explicit specialization
                    cbor_encode_boolean(enc, CTSD->getSpecializationKind() ==
                                                 TSK_ExplicitSpecialization);
                });

    // Recursively ensure field type specializations are exported
    ensureFieldTypeSpecializationsExported(CTSD);

    return true;
}

void ASTExporterVisitor::ensureFieldTypeSpecializationsExported(
    ClassTemplateSpecializationDecl *CTSD) {
    if (!CTSD->hasDefinition())
        return;

    for (auto *D : CTSD->decls()) {
        auto *FD = dyn_cast<FieldDecl>(D);
        if (!FD)
            continue;

        // Get the canonical type, stripping typedefs, elaborated types, etc.
        QualType FieldType = FD->getType();
        const Type *T = FieldType.getCanonicalType().getTypePtr();

        // Peel through pointers/references to find the underlying record
        while (true) {
            if (auto *PT = dyn_cast<clang::PointerType>(T)) {
                T = PT->getPointeeType().getTypePtr();
            } else if (auto *RT = dyn_cast<ReferenceType>(T)) {
                T = RT->getPointeeType().getTypePtr();
            } else {
                break;
            }
        }

        // If the underlying type is a class template specialization, visit it
        if (auto *RT = dyn_cast<RecordType>(T)) {
            if (auto *FieldCTSD = dyn_cast<ClassTemplateSpecializationDecl>(RT->getDecl())) {
                if (debug) {
                    llvm::errs() << "  Ensuring field type specialization exported: "
                                 << FieldCTSD->getQualifiedNameAsString() << "\n";
                }
                // markExported inside VisitClassTemplateSpecializationDecl prevents
                // infinite recursion for self-referential types
                VisitClassTemplateSpecializationDecl(FieldCTSD);
            }
        }
    }
}

bool ASTExporterVisitor::VisitNamespaceDecl(NamespaceDecl *ND) {
    if (!markExported(ND))
        return true;

    std::vector<const void *> children;

    encodeEntry(ND, TagNamespaceDecl, ND->getSourceRange(), children,
                QualType(), [ND](CborEncoder *enc) {
                    cbor_encode_string(enc, ND->getNameAsString());
                    cbor_encode_boolean(enc, ND->isInline());
                    cbor_encode_boolean(enc, ND->isAnonymousNamespace());
                });

    return true;
}

bool ASTExporterVisitor::VisitTypedefDecl(TypedefDecl *TD) {
    if (!markExported(TD))
        return true;

    std::vector<const void *> children;

    encodeEntry(TD, TagTypedefDecl, TD->getSourceRange(), children,
                TD->getUnderlyingType(), [TD](CborEncoder *enc) {
                    cbor_encode_string(enc, TD->getNameAsString());
                });

    return true;
}

bool ASTExporterVisitor::VisitTypeAliasDecl(TypeAliasDecl *TAD) {
    if (!markExported(TAD))
        return true;

    std::vector<const void *> children;

    encodeEntry(TAD, TagTypeAliasDecl, TAD->getSourceRange(), children,
                TAD->getUnderlyingType(), [TAD](CborEncoder *enc) {
                    cbor_encode_string(enc, TAD->getNameAsString());
                });

    return true;
}

bool ASTExporterVisitor::VisitEnumDecl(EnumDecl *ED) {
    if (!markExported(ED))
        return true;

    std::vector<const void *> children;
    for (auto *ECD : ED->enumerators()) {
        children.push_back(ECD);
    }

    encodeEntry(ED, TagEnumDecl, ED->getSourceRange(), children,
                ED->getIntegerType(), [ED](CborEncoder *enc) {
                    cbor_encode_string(enc, ED->getNameAsString());
                    cbor_encode_boolean(enc, ED->isScoped());
                });

    return true;
}

bool ASTExporterVisitor::VisitEnumConstantDecl(EnumConstantDecl *ECD) {
    if (!markExported(ECD))
        return true;

    Expr *init = ECD->getInitExpr();
    std::vector<const void *> children;
    children.push_back(init);

    encodeEntry(ECD, TagEnumConstantDecl, ECD->getSourceRange(), children,
                ECD->getType(), [ECD](CborEncoder *enc) {
                    cbor_encode_string(enc, ECD->getNameAsString());

                    // Value
                    auto val = ECD->getInitVal();
                    cbor_encode_int(enc, val.getExtValue());
                });

    if (init) {
        visitExpr(init);
    }

    return true;
}

// ============================================================================
// Statement Visitors
// ============================================================================

void ASTExporterVisitor::visitStmt(Stmt *S) {
    if (!S)
        return;

    switch (S->getStmtClass()) {
    case Stmt::CompoundStmtClass:
        visitCompoundStmt(cast<CompoundStmt>(S));
        break;
    case Stmt::DeclStmtClass:
        visitDeclStmt(cast<DeclStmt>(S));
        break;
    case Stmt::ReturnStmtClass:
        visitReturnStmt(cast<ReturnStmt>(S));
        break;
    case Stmt::IfStmtClass:
        visitIfStmt(cast<IfStmt>(S));
        break;
    case Stmt::WhileStmtClass:
        visitWhileStmt(cast<WhileStmt>(S));
        break;
    case Stmt::ForStmtClass:
        visitForStmt(cast<ForStmt>(S));
        break;
    case Stmt::CXXForRangeStmtClass:
        visitCXXForRangeStmt(cast<CXXForRangeStmt>(S));
        break;
    case Stmt::BreakStmtClass:
        visitBreakStmt(cast<BreakStmt>(S));
        break;
    case Stmt::ContinueStmtClass:
        visitContinueStmt(cast<ContinueStmt>(S));
        break;
    case Stmt::SwitchStmtClass:
        visitSwitchStmt(cast<SwitchStmt>(S));
        break;
    case Stmt::CaseStmtClass:
        visitCaseStmt(cast<CaseStmt>(S));
        break;
    case Stmt::DefaultStmtClass:
        visitDefaultStmt(cast<DefaultStmt>(S));
        break;
    case Stmt::NullStmtClass:
        // Null statement - encode but nothing special
        encodeEntry(S, TagNullStmt, S->getSourceRange(), {}, QualType());
        break;
    default:
        // Check if it's an expression
        if (auto *E = dyn_cast<Expr>(S)) {
            visitExpr(E);
        }
        break;
    }
}

void ASTExporterVisitor::visitCompoundStmt(CompoundStmt *CS) {
    if (!markExported(CS))
        return;

    std::vector<const void *> children;
    for (auto *child : CS->body()) {
        children.push_back(child);
    }

    encodeEntry(CS, TagCompoundStmt, CS->getSourceRange(), children, QualType());

    for (auto *child : CS->body()) {
        visitStmt(child);
    }
}

void ASTExporterVisitor::visitDeclStmt(DeclStmt *DS) {
    if (!markExported(DS))
        return;

    std::vector<const void *> children;
    for (auto *D : DS->decls()) {
        children.push_back(D);
    }

    encodeEntry(DS, TagDeclStmt, DS->getSourceRange(), children, QualType());
}

void ASTExporterVisitor::visitReturnStmt(ReturnStmt *RS) {
    if (!markExported(RS))
        return;

    Expr *retVal = RS->getRetValue();
    std::vector<const void *> children;
    children.push_back(retVal);

    encodeEntry(RS, TagReturnStmt, RS->getSourceRange(), children, QualType());

    if (retVal) {
        visitExpr(retVal);
    }
}

void ASTExporterVisitor::visitIfStmt(IfStmt *IS) {
    if (!markExported(IS))
        return;

    std::vector<const void *> children;
    children.push_back(IS->getCond());
    children.push_back(IS->getThen());
    children.push_back(IS->getElse());

    encodeEntry(IS, TagIfStmt, IS->getSourceRange(), children, QualType());

    visitExpr(IS->getCond());
    visitStmt(IS->getThen());
    if (IS->getElse()) {
        visitStmt(IS->getElse());
    }
}

void ASTExporterVisitor::visitWhileStmt(WhileStmt *WS) {
    if (!markExported(WS))
        return;

    std::vector<const void *> children;
    children.push_back(WS->getCond());
    children.push_back(WS->getBody());

    encodeEntry(WS, TagWhileStmt, WS->getSourceRange(), children, QualType());

    visitExpr(WS->getCond());
    visitStmt(WS->getBody());
}

void ASTExporterVisitor::visitForStmt(ForStmt *FS) {
    if (!markExported(FS))
        return;

    std::vector<const void *> children;
    children.push_back(FS->getInit());
    children.push_back(FS->getCond());
    children.push_back(FS->getInc());
    children.push_back(FS->getBody());

    encodeEntry(FS, TagForStmt, FS->getSourceRange(), children, QualType());

    if (FS->getInit())
        visitStmt(FS->getInit());
    if (FS->getCond())
        visitExpr(FS->getCond());
    if (FS->getInc())
        visitExpr(FS->getInc());
    visitStmt(FS->getBody());
}

void ASTExporterVisitor::visitCXXForRangeStmt(CXXForRangeStmt *FRS) {
    if (!markExported(FRS))
        return;

    std::vector<const void *> children;
    children.push_back(FRS->getRangeInit());
    children.push_back(FRS->getLoopVariable());
    children.push_back(FRS->getBody());

    encodeEntry(FRS, TagCXXForRangeStmt, FRS->getSourceRange(), children, QualType());

    visitExpr(FRS->getRangeInit());
    visitStmt(FRS->getBody());
}

void ASTExporterVisitor::visitBreakStmt(BreakStmt *BS) {
    if (!markExported(BS))
        return;
    encodeEntry(BS, TagBreakStmt, BS->getSourceRange(), {}, QualType());
}

void ASTExporterVisitor::visitContinueStmt(ContinueStmt *CS) {
    if (!markExported(CS))
        return;
    encodeEntry(CS, TagContinueStmt, CS->getSourceRange(), {}, QualType());
}

void ASTExporterVisitor::visitSwitchStmt(SwitchStmt *SS) {
    if (!markExported(SS))
        return;

    std::vector<const void *> children;
    children.push_back(SS->getCond());
    children.push_back(SS->getBody());

    encodeEntry(SS, TagSwitchStmt, SS->getSourceRange(), children, QualType());

    visitExpr(SS->getCond());
    visitStmt(SS->getBody());
}

void ASTExporterVisitor::visitCaseStmt(CaseStmt *CS) {
    if (!markExported(CS))
        return;

    std::vector<const void *> children;
    children.push_back(CS->getLHS());
    children.push_back(CS->getRHS()); // For case ranges
    children.push_back(CS->getSubStmt());

    encodeEntry(CS, TagCaseStmt, CS->getSourceRange(), children, QualType());

    visitExpr(CS->getLHS());
    if (CS->getRHS())
        visitExpr(CS->getRHS());
    visitStmt(CS->getSubStmt());
}

void ASTExporterVisitor::visitDefaultStmt(DefaultStmt *DS) {
    if (!markExported(DS))
        return;

    std::vector<const void *> children;
    children.push_back(DS->getSubStmt());

    encodeEntry(DS, TagDefaultStmt, DS->getSourceRange(), children, QualType());

    visitStmt(DS->getSubStmt());
}

// ============================================================================
// Expression Visitors
// ============================================================================

void ASTExporterVisitor::visitExpr(Expr *E) {
    if (!E)
        return;

    switch (E->getStmtClass()) {
    case Stmt::IntegerLiteralClass:
        visitIntegerLiteral(cast<IntegerLiteral>(E));
        break;
    case Stmt::FloatingLiteralClass:
        visitFloatingLiteral(cast<FloatingLiteral>(E));
        break;
    case Stmt::StringLiteralClass:
        visitStringLiteral(cast<clang::StringLiteral>(E));
        break;
    case Stmt::CharacterLiteralClass:
        visitCharacterLiteral(cast<CharacterLiteral>(E));
        break;
    case Stmt::CXXBoolLiteralExprClass:
        visitCXXBoolLiteralExpr(cast<CXXBoolLiteralExpr>(E));
        break;
    case Stmt::CXXNullPtrLiteralExprClass:
        visitCXXNullPtrLiteralExpr(cast<CXXNullPtrLiteralExpr>(E));
        break;
    case Stmt::DeclRefExprClass:
        visitDeclRefExpr(cast<DeclRefExpr>(E));
        break;
    case Stmt::MemberExprClass:
        visitMemberExpr(cast<MemberExpr>(E));
        break;
    case Stmt::CXXThisExprClass:
        visitCXXThisExpr(cast<CXXThisExpr>(E));
        break;
    case Stmt::CallExprClass:
        visitCallExpr(cast<CallExpr>(E));
        break;
    case Stmt::CXXMemberCallExprClass:
        visitCXXMemberCallExpr(cast<CXXMemberCallExpr>(E));
        break;
    case Stmt::CXXOperatorCallExprClass:
        visitCXXOperatorCallExpr(cast<CXXOperatorCallExpr>(E));
        break;
    case Stmt::CXXConstructExprClass:
        visitCXXConstructExpr(cast<CXXConstructExpr>(E));
        break;
    case Stmt::BinaryOperatorClass:
    case Stmt::CompoundAssignOperatorClass:
        visitBinaryOperator(cast<BinaryOperator>(E));
        break;
    case Stmt::UnaryOperatorClass:
        visitUnaryOperator(cast<UnaryOperator>(E));
        break;
    case Stmt::ImplicitCastExprClass:
    case Stmt::CStyleCastExprClass:
    case Stmt::CXXStaticCastExprClass:
    case Stmt::CXXDynamicCastExprClass:
    case Stmt::CXXReinterpretCastExprClass:
    case Stmt::CXXConstCastExprClass:
    case Stmt::CXXFunctionalCastExprClass:
        visitCastExpr(cast<CastExpr>(E));
        break;
    case Stmt::ArraySubscriptExprClass:
        visitArraySubscriptExpr(cast<ArraySubscriptExpr>(E));
        break;
    case Stmt::InitListExprClass:
        visitInitListExpr(cast<InitListExpr>(E));
        break;
    case Stmt::ParenExprClass:
        visitParenExpr(cast<ParenExpr>(E));
        break;
    case Stmt::UnaryExprOrTypeTraitExprClass:
        visitUnaryExprOrTypeTraitExpr(cast<UnaryExprOrTypeTraitExpr>(E));
        break;
    case Stmt::ConditionalOperatorClass:
        visitConditionalOperator(cast<ConditionalOperator>(E));
        break;
    case Stmt::CXXNewExprClass:
        visitCXXNewExpr(cast<CXXNewExpr>(E));
        break;
    case Stmt::CXXDeleteExprClass:
        visitCXXDeleteExpr(cast<CXXDeleteExpr>(E));
        break;
    case Stmt::ExprWithCleanupsClass:
        visitExprWithCleanups(cast<ExprWithCleanups>(E));
        break;
    case Stmt::MaterializeTemporaryExprClass:
        visitMaterializeTemporaryExpr(cast<MaterializeTemporaryExpr>(E));
        break;
    case Stmt::CXXBindTemporaryExprClass:
        visitCXXBindTemporaryExpr(cast<CXXBindTemporaryExpr>(E));
        break;
    case Stmt::ImplicitValueInitExprClass:
        visitImplicitValueInitExpr(cast<ImplicitValueInitExpr>(E));
        break;
    case Stmt::CXXDefaultArgExprClass:
        visitCXXDefaultArgExpr(cast<CXXDefaultArgExpr>(E));
        break;
    case Stmt::CXXDefaultInitExprClass:
        visitCXXDefaultInitExpr(cast<CXXDefaultInitExpr>(E));
        break;
    case Stmt::LambdaExprClass:
        visitLambdaExpr(cast<LambdaExpr>(E));
        break;
    default:
        // Unknown expression type - just encode it generically
        if (!markExported(E))
            return;
        encodeEntry(E, TagDeclRefExpr, E->getSourceRange(), {}, E->getType(),
                    [E](CborEncoder *enc) {
                        cbor_encode_string(enc, E->getStmtClassName());
                    });
        break;
    }
}

void ASTExporterVisitor::visitIntegerLiteral(IntegerLiteral *IL) {
    if (!markExported(IL))
        return;

    encodeEntry(IL, TagIntegerLiteral, IL->getSourceRange(), {}, IL->getType(),
                [IL](CborEncoder *enc) {
                    auto val = IL->getValue();
                    if (val.getBitWidth() <= 64) {
                        if (IL->getType()->isSignedIntegerType()) {
                            cbor_encode_int(enc, val.getSExtValue());
                        } else {
                            cbor_encode_uint(enc, val.getZExtValue());
                        }
                    } else {
                        // For larger integers, encode as string
                        SmallString<40> str;
                        val.toString(str, 10, IL->getType()->isSignedIntegerType());
                        cbor_encode_string(enc, str.str().str());
                    }
                });
}

void ASTExporterVisitor::visitFloatingLiteral(FloatingLiteral *FL) {
    if (!markExported(FL))
        return;

    encodeEntry(FL, TagFloatingLiteral, FL->getSourceRange(), {}, FL->getType(),
                [FL](CborEncoder *enc) {
                    cbor_encode_double(enc, FL->getValue().convertToDouble());
                });
}

void ASTExporterVisitor::visitStringLiteral(clang::StringLiteral *SL) {
    if (!markExported(SL))
        return;

    encodeEntry(SL, TagStringLiteral, SL->getSourceRange(), {}, SL->getType(),
                [SL](CborEncoder *enc) {
                    cbor_encode_string(enc, SL->getString().str());
                });
}

void ASTExporterVisitor::visitCharacterLiteral(CharacterLiteral *CL) {
    if (!markExported(CL))
        return;

    encodeEntry(CL, TagCharacterLiteral, CL->getSourceRange(), {}, CL->getType(),
                [CL](CborEncoder *enc) {
                    cbor_encode_uint(enc, CL->getValue());
                });
}

void ASTExporterVisitor::visitCXXBoolLiteralExpr(CXXBoolLiteralExpr *BL) {
    if (!markExported(BL))
        return;

    encodeEntry(BL, TagCXXBoolLiteralExpr, BL->getSourceRange(), {}, BL->getType(),
                [BL](CborEncoder *enc) {
                    cbor_encode_boolean(enc, BL->getValue());
                });
}

void ASTExporterVisitor::visitCXXNullPtrLiteralExpr(CXXNullPtrLiteralExpr *NL) {
    if (!markExported(NL))
        return;

    encodeEntry(NL, TagCXXNullPtrLiteralExpr, NL->getSourceRange(), {}, NL->getType());
}

void ASTExporterVisitor::visitDeclRefExpr(DeclRefExpr *DRE) {
    if (!markExported(DRE))
        return;

    std::vector<const void *> children;
    children.push_back(DRE->getDecl());

    encodeEntry(DRE, TagDeclRefExpr, DRE->getSourceRange(), children, DRE->getType(),
                [DRE](CborEncoder *enc) {
                    cbor_encode_string(enc, DRE->getDecl()->getNameAsString());
                });
}

void ASTExporterVisitor::visitMemberExpr(MemberExpr *ME) {
    if (!markExported(ME))
        return;

    std::vector<const void *> children;
    children.push_back(ME->getBase());
    children.push_back(ME->getMemberDecl());

    encodeEntry(ME, TagMemberExpr, ME->getSourceRange(), children, ME->getType(),
                [ME](CborEncoder *enc) {
                    cbor_encode_string(enc, ME->getMemberDecl()->getNameAsString());
                    cbor_encode_boolean(enc, ME->isArrow());
                });

    visitExpr(ME->getBase());
}

void ASTExporterVisitor::visitCXXThisExpr(CXXThisExpr *TE) {
    if (!markExported(TE))
        return;

    encodeEntry(TE, TagCXXThisExpr, TE->getSourceRange(), {}, TE->getType(),
                [TE](CborEncoder *enc) {
                    cbor_encode_boolean(enc, TE->isImplicit());
                });
}

void ASTExporterVisitor::visitCallExpr(CallExpr *CE) {
    if (!markExported(CE))
        return;

    std::vector<const void *> children;
    children.push_back(CE->getCallee());
    for (auto *arg : CE->arguments()) {
        children.push_back(arg);
    }

    encodeEntry(CE, TagCallExpr, CE->getSourceRange(), children, CE->getType());

    visitExpr(CE->getCallee());
    for (auto *arg : CE->arguments()) {
        visitExpr(arg);
    }
}

void ASTExporterVisitor::visitCXXMemberCallExpr(CXXMemberCallExpr *MCE) {
    if (!markExported(MCE))
        return;

    std::vector<const void *> children;
    children.push_back(MCE->getCallee());
    for (auto *arg : MCE->arguments()) {
        children.push_back(arg);
    }

    encodeEntry(MCE, TagCXXMemberCallExpr, MCE->getSourceRange(), children,
                MCE->getType(), [MCE](CborEncoder *enc) {
                    if (auto *MD = MCE->getMethodDecl()) {
                        cbor_encode_string(enc, MD->getNameAsString());
                    } else {
                        cbor_encode_string(enc, "");
                    }
                });

    visitExpr(MCE->getCallee());
    for (auto *arg : MCE->arguments()) {
        visitExpr(arg);
    }
}

void ASTExporterVisitor::visitCXXOperatorCallExpr(CXXOperatorCallExpr *OCE) {
    if (!markExported(OCE))
        return;

    std::vector<const void *> children;
    for (auto *arg : OCE->arguments()) {
        children.push_back(arg);
    }

    encodeEntry(OCE, TagCXXOperatorCallExpr, OCE->getSourceRange(), children,
                OCE->getType(), [OCE](CborEncoder *enc) {
                    cbor_encode_uint(enc, OCE->getOperator());
                });

    for (auto *arg : OCE->arguments()) {
        visitExpr(arg);
    }
}

void ASTExporterVisitor::visitCXXConstructExpr(CXXConstructExpr *CCE) {
    if (!markExported(CCE))
        return;

    std::vector<const void *> children;
    for (auto *arg : CCE->arguments()) {
        children.push_back(arg);
    }

    encodeEntry(CCE, TagCXXConstructExpr, CCE->getSourceRange(), children,
                CCE->getType(), [CCE](CborEncoder *enc) {
                    if (auto *CD = CCE->getConstructor()) {
                        cbor_encode_uint(enc, reinterpret_cast<uint64_t>(CD));
                        cbor_encode_string(enc, CD->getParent()->getNameAsString());
                    } else {
                        cbor_encode_null(enc);
                        cbor_encode_string(enc, "");
                    }
                    cbor_encode_boolean(enc, CCE->isElidable());
                    cbor_encode_boolean(enc, CCE->isListInitialization());
                });

    for (auto *arg : CCE->arguments()) {
        visitExpr(arg);
    }
}

void ASTExporterVisitor::visitBinaryOperator(BinaryOperator *BO) {
    if (!markExported(BO))
        return;

    std::vector<const void *> children;
    children.push_back(BO->getLHS());
    children.push_back(BO->getRHS());

    ASTEntryTag tag = isa<CompoundAssignOperator>(BO) ? TagCompoundAssignOperator
                                                       : TagBinaryOperator;

    encodeEntry(BO, tag, BO->getSourceRange(), children, BO->getType(),
                [BO](CborEncoder *enc) {
                    cbor_encode_uint(enc, BO->getOpcode());
                    cbor_encode_string(enc, BO->getOpcodeStr().str());
                });

    visitExpr(BO->getLHS());
    visitExpr(BO->getRHS());
}

void ASTExporterVisitor::visitUnaryOperator(UnaryOperator *UO) {
    if (!markExported(UO))
        return;

    std::vector<const void *> children;
    children.push_back(UO->getSubExpr());

    encodeEntry(UO, TagUnaryOperator, UO->getSourceRange(), children, UO->getType(),
                [UO](CborEncoder *enc) {
                    cbor_encode_uint(enc, UO->getOpcode());
                    cbor_encode_boolean(enc, UO->isPrefix());
                });

    visitExpr(UO->getSubExpr());
}

void ASTExporterVisitor::visitCastExpr(CastExpr *CE) {
    if (!markExported(CE))
        return;

    std::vector<const void *> children;
    children.push_back(CE->getSubExpr());

    ASTEntryTag tag;
    switch (CE->getStmtClass()) {
    case Stmt::ImplicitCastExprClass:
        tag = TagImplicitCastExpr;
        break;
    case Stmt::CStyleCastExprClass:
        tag = TagCStyleCastExpr;
        break;
    case Stmt::CXXStaticCastExprClass:
        tag = TagCXXStaticCastExpr;
        break;
    case Stmt::CXXDynamicCastExprClass:
        tag = TagCXXDynamicCastExpr;
        break;
    case Stmt::CXXReinterpretCastExprClass:
        tag = TagCXXReinterpretCastExpr;
        break;
    case Stmt::CXXConstCastExprClass:
        tag = TagCXXConstCastExpr;
        break;
    case Stmt::CXXFunctionalCastExprClass:
        tag = TagCXXFunctionalCastExpr;
        break;
    default:
        tag = TagImplicitCastExpr;
        break;
    }

    encodeEntry(CE, tag, CE->getSourceRange(), children, CE->getType(),
                [CE](CborEncoder *enc) {
                    cbor_encode_uint(enc, CE->getCastKind());
                });

    visitExpr(CE->getSubExpr());
}

void ASTExporterVisitor::visitArraySubscriptExpr(ArraySubscriptExpr *ASE) {
    if (!markExported(ASE))
        return;

    std::vector<const void *> children;
    children.push_back(ASE->getBase());
    children.push_back(ASE->getIdx());

    encodeEntry(ASE, TagArraySubscriptExpr, ASE->getSourceRange(), children,
                ASE->getType());

    visitExpr(ASE->getBase());
    visitExpr(ASE->getIdx());
}

void ASTExporterVisitor::visitInitListExpr(InitListExpr *ILE) {
    if (!markExported(ILE))
        return;

    std::vector<const void *> children;
    for (auto *init : ILE->inits()) {
        children.push_back(init);
    }

    encodeEntry(ILE, TagInitListExpr, ILE->getSourceRange(), children,
                ILE->getType());

    for (auto *init : ILE->inits()) {
        visitExpr(init);
    }
}

void ASTExporterVisitor::visitParenExpr(ParenExpr *PE) {
    if (!markExported(PE))
        return;

    std::vector<const void *> children;
    children.push_back(PE->getSubExpr());

    encodeEntry(PE, TagParenExpr, PE->getSourceRange(), children, PE->getType());

    visitExpr(PE->getSubExpr());
}

void ASTExporterVisitor::visitUnaryExprOrTypeTraitExpr(UnaryExprOrTypeTraitExpr *UE) {
    if (!markExported(UE))
        return;

    std::vector<const void *> children;
    if (!UE->isArgumentType()) {
        children.push_back(UE->getArgumentExpr());
    }

    encodeEntry(UE, TagUnaryExprOrTypeTraitExpr, UE->getSourceRange(), children,
                UE->getType(), [UE, this](CborEncoder *enc) {
                    cbor_encode_uint(enc, UE->getKind());
                    cbor_encode_boolean(enc, UE->isArgumentType());
                    if (UE->isArgumentType()) {
                        cbor_encode_uint(enc, typeEncoder.encodeQualType(UE->getArgumentType()));
                    }
                });

    if (!UE->isArgumentType()) {
        visitExpr(UE->getArgumentExpr());
    } else {
        typeEncoder.visitQualType(UE->getArgumentType());
    }
}

void ASTExporterVisitor::visitConditionalOperator(ConditionalOperator *CO) {
    if (!markExported(CO))
        return;

    std::vector<const void *> children;
    children.push_back(CO->getCond());
    children.push_back(CO->getTrueExpr());
    children.push_back(CO->getFalseExpr());

    encodeEntry(CO, TagConditionalOperator, CO->getSourceRange(), children,
                CO->getType());

    visitExpr(CO->getCond());
    visitExpr(CO->getTrueExpr());
    visitExpr(CO->getFalseExpr());
}

void ASTExporterVisitor::visitCXXNewExpr(CXXNewExpr *NE) {
    if (!markExported(NE))
        return;

    std::vector<const void *> children;
    if (NE->hasInitializer()) {
        children.push_back(NE->getInitializer());
    }
    if (NE->isArray()) {
#if CLANG_VERSION_MAJOR >= 16
        children.push_back(NE->getArraySize().value_or(nullptr));
#else
        auto arraySize = NE->getArraySize();
        children.push_back(arraySize.hasValue() ? arraySize.getValue() : nullptr);
#endif
    }

    encodeEntry(NE, TagCXXNewExpr, NE->getSourceRange(), children, NE->getType(),
                [NE, this](CborEncoder *enc) {
                    cbor_encode_boolean(enc, NE->isArray());
                    cbor_encode_uint(enc, typeEncoder.encodeQualType(NE->getAllocatedType()));
                });

    if (NE->hasInitializer()) {
        visitExpr(NE->getInitializer());
    }
    if (NE->isArray() && NE->getArraySize()) {
        visitExpr(*NE->getArraySize());
    }
    typeEncoder.visitQualType(NE->getAllocatedType());
}

void ASTExporterVisitor::visitCXXDeleteExpr(CXXDeleteExpr *DE) {
    if (!markExported(DE))
        return;

    std::vector<const void *> children;
    children.push_back(DE->getArgument());

    encodeEntry(DE, TagCXXDeleteExpr, DE->getSourceRange(), children, DE->getType(),
                [DE](CborEncoder *enc) {
                    cbor_encode_boolean(enc, DE->isArrayForm());
                });

    visitExpr(DE->getArgument());
}

void ASTExporterVisitor::visitExprWithCleanups(ExprWithCleanups *EWC) {
    if (!markExported(EWC))
        return;

    std::vector<const void *> children;
    children.push_back(EWC->getSubExpr());

    encodeEntry(EWC, TagExprWithCleanups, EWC->getSourceRange(), children,
                EWC->getType());

    visitExpr(EWC->getSubExpr());
}

void ASTExporterVisitor::visitMaterializeTemporaryExpr(MaterializeTemporaryExpr *MTE) {
    if (!markExported(MTE))
        return;

    std::vector<const void *> children;
    children.push_back(MTE->getSubExpr());

    encodeEntry(MTE, TagMaterializeTemporaryExpr, MTE->getSourceRange(), children,
                MTE->getType());

    visitExpr(MTE->getSubExpr());
}

void ASTExporterVisitor::visitCXXBindTemporaryExpr(CXXBindTemporaryExpr *BTE) {
    if (!markExported(BTE))
        return;

    std::vector<const void *> children;
    children.push_back(BTE->getSubExpr());

    encodeEntry(BTE, TagCXXBindTemporaryExpr, BTE->getSourceRange(), children,
                BTE->getType());

    visitExpr(BTE->getSubExpr());
}

void ASTExporterVisitor::visitImplicitValueInitExpr(ImplicitValueInitExpr *IVE) {
    if (!markExported(IVE))
        return;

    encodeEntry(IVE, TagImplicitValueInitExpr, IVE->getSourceRange(), {},
                IVE->getType());
}

void ASTExporterVisitor::visitCXXDefaultArgExpr(CXXDefaultArgExpr *DAE) {
    if (!markExported(DAE))
        return;

    std::vector<const void *> children;
    children.push_back(DAE->getExpr());

    encodeEntry(DAE, TagCXXDefaultArgExpr, DAE->getSourceRange(), children,
                DAE->getType());

    visitExpr(DAE->getExpr());
}

void ASTExporterVisitor::visitCXXDefaultInitExpr(CXXDefaultInitExpr *DIE) {
    if (!markExported(DIE))
        return;

    std::vector<const void *> children;
    children.push_back(DIE->getExpr());

    encodeEntry(DIE, TagCXXDefaultInitExpr, DIE->getSourceRange(), children,
                DIE->getType());

    visitExpr(DIE->getExpr());
}

void ASTExporterVisitor::visitLambdaExpr(LambdaExpr *LE) {
    if (!markExported(LE))
        return;

    std::vector<const void *> children;
    children.push_back(LE->getBody());

    encodeEntry(LE, TagLambdaExpr, LE->getSourceRange(), children, LE->getType(),
                [LE](CborEncoder *enc) {
                    // Capture default
                    cbor_encode_uint(enc, LE->getCaptureDefault());

                    // Captures
                    CborEncoder captures;
                    cbor_encoder_create_array(enc, &captures, LE->capture_size());
                    for (const auto &cap : LE->captures()) {
                        CborEncoder capEntry;
                        cbor_encoder_create_array(&captures, &capEntry, 3);

                        // Capture kind
                        cbor_encode_uint(&capEntry, cap.getCaptureKind());

                        // Is implicit
                        cbor_encode_boolean(&capEntry, cap.isImplicit());

                        // Captured variable name
                        if (cap.capturesVariable()) {
                            cbor_encode_string(&capEntry,
                                               cap.getCapturedVar()->getNameAsString());
                        } else {
                            cbor_encode_string(&capEntry, "");
                        }

                        cbor_encoder_close_container(&captures, &capEntry);
                    }
                    cbor_encoder_close_container(enc, &captures);
                });

    visitStmt(LE->getBody());
}

// ============================================================================
// AST Consumer and Frontend Action
// ============================================================================

class ASTExporterConsumer : public ASTConsumer {
    std::vector<uint8_t> &output;
    bool debug;

public:
    ASTExporterConsumer(std::vector<uint8_t> &out, bool dbg)
        : output(out), debug(dbg) {}

    void HandleTranslationUnit(ASTContext &Context) override {
        // Allocate buffer for CBOR output
        size_t bufferSize = 16 * 1024 * 1024; // 16 MB initial
        output.resize(bufferSize);

        CborEncoder encoder;
        cbor_encoder_init(&encoder, output.data(), output.size(), 0);

        // Create top-level array
        CborEncoder topArray;
        cbor_encoder_create_array(&encoder, &topArray, CborIndefiniteLength);

        // Export AST
        ASTExporterVisitor visitor(Context, &topArray, debug);
        visitor.exportTranslationUnit();

        cbor_encoder_close_container(&encoder, &topArray);

        // Resize to actual size
        size_t actualSize = cbor_encoder_get_buffer_size(&encoder, output.data());
        output.resize(actualSize);
    }
};

class ASTExporterAction : public ASTFrontendAction {
    std::vector<uint8_t> &output;
    bool debug;

public:
    ASTExporterAction(std::vector<uint8_t> &out, bool dbg)
        : output(out), debug(dbg) {}

    std::unique_ptr<ASTConsumer> CreateASTConsumer(CompilerInstance &CI,
                                                    StringRef file) override {
        return std::make_unique<ASTExporterConsumer>(output, debug);
    }
};

class ASTExporterActionFactory : public FrontendActionFactory {
    std::vector<uint8_t> &output;
    bool debug;

public:
    ASTExporterActionFactory(std::vector<uint8_t> &out, bool dbg)
        : output(out), debug(dbg) {}

    std::unique_ptr<FrontendAction> create() override {
        return std::make_unique<ASTExporterAction>(output, debug);
    }
};

} // anonymous namespace

// ============================================================================
// C API Implementation
// ============================================================================

extern "C" {

ExportResult *ast_exporter(int argc, const char **argv, int debug, int *result) {
    auto expectedParser = CommonOptionsParser::create(argc, argv, FragileCategory);
    if (!expectedParser) {
        llvm::errs() << "Error: " << llvm::toString(expectedParser.takeError()) << "\n";
        *result = 1;
        return nullptr;
    }
    auto &optionsParser = expectedParser.get();

    ClangTool tool(optionsParser.getCompilations(),
                   optionsParser.getSourcePathList());

    // Output buffer
    std::vector<uint8_t> output;

    // Run the tool
    ASTExporterActionFactory factory(output, debug != 0);
    int ret = tool.run(&factory);

    if (ret != 0) {
        *result = ret;
        return nullptr;
    }

    // Create result
    auto *exportResult = new ExportResult();
    exportResult->entries = 1;
    exportResult->names = new char *[1];
    exportResult->bytes = new uint8_t *[1];
    exportResult->sizes = new size_t[1];

    // Copy output
    auto *outputCopy = new uint8_t[output.size()];
    std::copy(output.begin(), output.end(), outputCopy);

    exportResult->names[0] = strdup("main");
    exportResult->bytes[0] = outputCopy;
    exportResult->sizes[0] = output.size();

    *result = 0;
    return exportResult;
}

void drop_export_result(ExportResult *result) {
    if (!result)
        return;

    for (size_t i = 0; i < result->entries; ++i) {
        free(result->names[i]);
        delete[] result->bytes[i];
    }
    delete[] result->names;
    delete[] result->bytes;
    delete[] result->sizes;
    delete result;
}

const char *clang_version() {
    return CLANG_VERSION_STRING;
}

} // extern "C"
