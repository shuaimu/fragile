//
// ast_tags.hpp
// C++ AST tags for Fragile transpiler
//

#ifndef FRAGILE_AST_TAGS_H
#define FRAGILE_AST_TAGS_H

// ============================================================================
// Declaration Tags (0-99)
// ============================================================================
enum ASTEntryTag {
    // Functions
    TagFunctionDecl = 0,
    TagCXXMethodDecl = 1,
    TagCXXConstructorDecl = 2,
    TagCXXDestructorDecl = 3,
    TagParmVarDecl = 4,

    // Variables
    TagVarDecl = 10,
    TagFieldDecl = 11,

    // Types
    TagCXXRecordDecl = 20,
    TagClassTemplateDecl = 21,
    TagClassTemplateSpecializationDecl = 22,
    TagTypedefDecl = 23,
    TagTypeAliasDecl = 24,
    TagEnumDecl = 25,
    TagEnumConstantDecl = 26,

    // Namespaces
    TagNamespaceDecl = 30,
    TagUsingDecl = 31,
    TagUsingDirectiveDecl = 32,

    // Templates
    TagTemplateTypeParmDecl = 40,
    TagNonTypeTemplateParmDecl = 41,
    TagTemplateTemplateParmDecl = 42,
    TagFunctionTemplateDecl = 43,

    // Access
    TagAccessSpecDecl = 50,

    // Static assert
    TagStaticAssertDecl = 60,

    // ============================================================================
    // Statement Tags (100-199)
    // ============================================================================
    TagCompoundStmt = 100,
    TagDeclStmt = 101,
    TagReturnStmt = 102,
    TagIfStmt = 103,
    TagWhileStmt = 104,
    TagDoStmt = 105,
    TagForStmt = 106,
    TagCXXForRangeStmt = 107,
    TagBreakStmt = 108,
    TagContinueStmt = 109,
    TagSwitchStmt = 110,
    TagCaseStmt = 111,
    TagDefaultStmt = 112,
    TagGotoStmt = 113,
    TagLabelStmt = 114,
    TagNullStmt = 115,
    TagCXXTryStmt = 116,
    TagCXXCatchStmt = 117,
    TagCXXThrowExpr = 118,

    // ============================================================================
    // Expression Tags (200-399)
    // ============================================================================

    // Literals
    TagIntegerLiteral = 200,
    TagFloatingLiteral = 201,
    TagCharacterLiteral = 202,
    TagStringLiteral = 203,
    TagCXXBoolLiteralExpr = 204,
    TagCXXNullPtrLiteralExpr = 205,

    // Operators
    TagBinaryOperator = 210,
    TagUnaryOperator = 211,
    TagCompoundAssignOperator = 212,
    TagConditionalOperator = 213,

    // References
    TagDeclRefExpr = 220,
    TagMemberExpr = 221,
    TagCXXThisExpr = 222,

    // Calls
    TagCallExpr = 230,
    TagCXXMemberCallExpr = 231,
    TagCXXOperatorCallExpr = 232,
    TagCXXConstructExpr = 233,
    TagCXXTemporaryObjectExpr = 234,
    TagCXXNewExpr = 235,
    TagCXXDeleteExpr = 236,

    // Casts
    TagImplicitCastExpr = 240,
    TagCStyleCastExpr = 241,
    TagCXXStaticCastExpr = 242,
    TagCXXDynamicCastExpr = 243,
    TagCXXReinterpretCastExpr = 244,
    TagCXXConstCastExpr = 245,
    TagCXXFunctionalCastExpr = 246,

    // Array/Subscript
    TagArraySubscriptExpr = 250,
    TagInitListExpr = 251,
    TagCXXStdInitializerListExpr = 252,

    // Sizeof/Alignof
    TagUnaryExprOrTypeTraitExpr = 260,

    // Parentheses and cleanup
    TagParenExpr = 270,
    TagExprWithCleanups = 271,
    TagMaterializeTemporaryExpr = 272,
    TagCXXBindTemporaryExpr = 273,

    // Default argument
    TagCXXDefaultArgExpr = 280,
    TagCXXDefaultInitExpr = 281,

    // Lambda
    TagLambdaExpr = 290,

    // Type traits
    TagTypeTraitExpr = 295,

    // Misc expressions
    TagImplicitValueInitExpr = 296,
    TagCXXScalarValueInitExpr = 297,

    // ============================================================================
    // Type Tags (400-599)
    // ============================================================================
    TagTypeUnknown = 400,

    // Builtin types
    TagVoid = 500,
    TagBool = 501,
    TagChar = 502,
    TagSChar = 503,
    TagUChar = 504,
    TagWChar = 505,
    TagChar16 = 506,
    TagChar32 = 507,
    TagShort = 508,
    TagUShort = 509,
    TagInt = 510,
    TagUInt = 511,
    TagLong = 512,
    TagULong = 513,
    TagLongLong = 514,
    TagULongLong = 515,
    TagInt128 = 516,
    TagUInt128 = 517,
    TagFloat = 518,
    TagDouble = 519,
    TagLongDouble = 520,
    TagFloat128 = 521,

    // Pointer/Reference types
    TagPointerType = 530,
    TagLValueReferenceType = 531,
    TagRValueReferenceType = 532,

    // Array types
    TagConstantArrayType = 540,
    TagIncompleteArrayType = 541,
    TagVariableArrayType = 542,
    TagDependentSizedArrayType = 543,

    // Function types
    TagFunctionProtoType = 550,
    TagFunctionNoProtoType = 551,

    // Record types
    TagRecordType = 560,
    TagEnumType = 561,

    // Typedef types
    TagTypedefType = 570,
    TagElaboratedType = 571,
    TagDecayedType = 572,

    // Template types
    TagTemplateTypeParmType = 580,
    TagSubstTemplateTypeParmType = 581,
    TagTemplateSpecializationType = 582,
    TagDependentNameType = 583,
    TagAutoType = 584,
    TagDecltypeType = 585,

    // Attributed type
    TagAttributedType = 590,

    // Paren type
    TagParenType = 591,

    // ============================================================================
    // Operator kinds (for binary/unary operators)
    // ============================================================================
};

// Binary operator kinds
enum BinaryOperatorKind {
    BO_PtrMemD,   // .*
    BO_PtrMemI,   // ->*
    BO_Mul,       // *
    BO_Div,       // /
    BO_Rem,       // %
    BO_Add,       // +
    BO_Sub,       // -
    BO_Shl,       // <<
    BO_Shr,       // >>
    BO_Cmp,       // <=>
    BO_LT,        // <
    BO_GT,        // >
    BO_LE,        // <=
    BO_GE,        // >=
    BO_EQ,        // ==
    BO_NE,        // !=
    BO_And,       // &
    BO_Xor,       // ^
    BO_Or,        // |
    BO_LAnd,      // &&
    BO_LOr,       // ||
    BO_Assign,    // =
    BO_MulAssign, // *=
    BO_DivAssign, // /=
    BO_RemAssign, // %=
    BO_AddAssign, // +=
    BO_SubAssign, // -=
    BO_ShlAssign, // <<=
    BO_ShrAssign, // >>=
    BO_AndAssign, // &=
    BO_XorAssign, // ^=
    BO_OrAssign,  // |=
    BO_Comma,     // ,
};

// Unary operator kinds
enum UnaryOperatorKind {
    UO_PostInc,   // ++
    UO_PostDec,   // --
    UO_PreInc,    // ++
    UO_PreDec,    // --
    UO_AddrOf,    // &
    UO_Deref,     // *
    UO_Plus,      // +
    UO_Minus,     // -
    UO_Not,       // ~
    UO_LNot,      // !
    UO_Real,      // __real
    UO_Imag,      // __imag
    UO_Extension, // __extension__
    UO_Coawait,   // co_await
};

// Cast kinds
enum CastKind {
    CK_Dependent,
    CK_BitCast,
    CK_LValueBitCast,
    CK_LValueToRValueBitCast,
    CK_LValueToRValue,
    CK_NoOp,
    CK_BaseToDerived,
    CK_DerivedToBase,
    CK_UncheckedDerivedToBase,
    CK_Dynamic,
    CK_ToUnion,
    CK_ArrayToPointerDecay,
    CK_FunctionToPointerDecay,
    CK_NullToPointer,
    CK_NullToMemberPointer,
    CK_BaseToDerivedMemberPointer,
    CK_DerivedToBaseMemberPointer,
    CK_MemberPointerToBoolean,
    CK_ReinterpretMemberPointer,
    CK_UserDefinedConversion,
    CK_ConstructorConversion,
    CK_IntegralToPointer,
    CK_PointerToIntegral,
    CK_PointerToBoolean,
    CK_ToVoid,
    CK_MatrixCast,
    CK_VectorSplat,
    CK_IntegralCast,
    CK_IntegralToBoolean,
    CK_IntegralToFloating,
    CK_FloatingToFixedPoint,
    CK_FixedPointToFloating,
    CK_FixedPointCast,
    CK_FixedPointToIntegral,
    CK_IntegralToFixedPoint,
    CK_FixedPointToBoolean,
    CK_FloatingToIntegral,
    CK_FloatingToBoolean,
    CK_BooleanToSignedIntegral,
    CK_FloatingCast,
    CK_CPointerToObjCPointerCast,
    CK_BlockPointerToObjCPointerCast,
    CK_AnyPointerToBlockPointerCast,
    CK_ObjCObjectLValueCast,
    CK_FloatingRealToComplex,
    CK_FloatingComplexToReal,
    CK_FloatingComplexToBoolean,
    CK_FloatingComplexCast,
    CK_FloatingComplexToIntegralComplex,
    CK_IntegralRealToComplex,
    CK_IntegralComplexToReal,
    CK_IntegralComplexToBoolean,
    CK_IntegralComplexCast,
    CK_IntegralComplexToFloatingComplex,
    CK_ARCProduceObject,
    CK_ARCConsumeObject,
    CK_ARCReclaimReturnedObject,
    CK_ARCExtendBlockObject,
    CK_AtomicToNonAtomic,
    CK_NonAtomicToAtomic,
    CK_CopyAndAutoreleaseBlockObject,
    CK_BuiltinFnToFnPtr,
    CK_ZeroToOCLOpaqueType,
    CK_AddressSpaceConversion,
    CK_IntToOCLSampler,
    CK_HLSLVectorTruncation,
    CK_HLSLArrayRValue,
};

// Access specifier
enum AccessSpecifier {
    AS_public,
    AS_protected,
    AS_private,
    AS_none,
};

// Unary type trait kinds (for sizeof, alignof, etc.)
enum UnaryExprOrTypeTrait {
    UETT_SizeOf,
    UETT_AlignOf,
    UETT_PreferredAlignOf,
    UETT_VecStep,
    UETT_OpenMPRequiredSimdAlign,
    UETT_SizeOfWithSize,
};

#endif // FRAGILE_AST_TAGS_H
