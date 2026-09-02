// Flux AST - defines the tree structure representing Flux programs.

use crate::lexer::Span;

/// A complete Flux program: a sequence of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

/// A single statement in a Flux program.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// An expression used as a statement (typically a function call).
    Expression(Expression),
    /// A variable declaration: `let name = expression`
    Let(LetStatement),
    /// A conditional: `if condition { ... } else { ... }`
    If(IfStatement),
    /// An assignment: `target = expression`
    Assignment(AssignmentStatement),
    /// A while loop: `while condition { ... }`
    While(WhileStatement),
    /// A function declaration: `fn name(params) { ... }`
    Function(FunctionDecl),
    /// A return statement: `return expression?`
    Return(ReturnStatement),
    /// An import statement: `import module_name`
    Import(ImportStatement),
    /// A for loop: `for variable in iterable { body }`
    For(ForStatement),
    /// A break statement: `break`
    Break(Span),
    /// A continue statement: `continue`
    Continue(Span),
    /// An after statement: `after duration { body }`
    After(AfterStatement),
    /// An every statement: `every interval { body }`
    Every(EveryStatement),
    /// A calendar every statement: `every day at time(...) { body }`
    EveryCalendar(EveryCalendarStatement),
    /// An at statement: `at target { body }`
    At(AtStatement),
    /// An until loop: `until condition { body }`
    Until(UntilStatement),
    /// A wait until: `wait until condition` or `wait until condition timeout duration`
    WaitUntil(WaitUntilStatement),
    /// A throw statement: `throw expression`
    Throw(ThrowStatement),
    /// A try/catch/finally statement
    TryCatch(TryCatchStatement),
    /// An event handler: `on "type" |e| { body }`
    On(OnStatement),
    /// A spawn statement: `spawn { body }`
    Spawn(SpawnStatement),
    /// A type alias: `type Name = Type`
    TypeAlias(TypeAliasStatement),
    /// A struct type definition: `type Name { field: Type, ... }`
    StructDef(StructDefStatement),
}

/// A type alias: `type Name = Type`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasStatement {
    pub name: String,
    pub target: TypeAnnotation,
    pub span: Span,
}

/// A struct type definition: `type Name { field: Type, ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct StructDefStatement {
    pub name: String,
    pub fields: Vec<(String, TypeAnnotation)>,
    pub span: Span,
}

/// A throw statement: `throw expression`
#[derive(Debug, Clone, PartialEq)]
pub struct ThrowStatement {
    pub value: Expression,
    pub span: Span,
}

/// A try/catch/finally statement.
#[derive(Debug, Clone, PartialEq)]
pub struct TryCatchStatement {
    pub try_body: Block,
    pub catch_var: Option<String>,
    pub catch_body: Option<Block>,
    pub finally_body: Option<Block>,
    pub span: Span,
}

/// A for loop: `for variable in iterable { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct ForStatement {
    pub pattern: Pattern,
    pub iterable: Expression,
    pub body: Block,
    pub span: Span,
}

/// An after statement: `after duration { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct AfterStatement {
    pub delay: Expression,
    pub body: Block,
    pub span: Span,
}

/// An every statement: `every interval { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct EveryStatement {
    pub interval: Expression,
    pub body: Block,
    pub span: Span,
}

/// A calendar-based every statement: `every day at time(...) { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct EveryCalendarStatement {
    /// The recurrence pattern (Daily, Weekly, Monthly, Yearly).
    pub recurrence: crate::time::CalendarRecurrence,
    /// The time-of-day expression.
    pub time_expr: Expression,
    /// The body block.
    pub body: Block,
    /// Source location.
    pub span: Span,
}

/// An at statement: `at target { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct AtStatement {
    pub target: Expression,
    pub body: Block,
    pub span: Span,
}

/// An event handler statement: `on "type" as e where condition { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct OnStatement {
    /// The event type to match (must evaluate to String).
    pub event_type: Expression,
    /// Optional parameter name to bind the event value.
    pub param: Option<String>,
    /// Optional filter expression (evaluated with event bound).
    pub filter: Option<Expression>,
    /// The handler body.
    pub body: Block,
    pub span: Span,
}

/// A spawn statement: `spawn { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnStatement {
    pub body: Block,
    pub span: Span,
}

/// An until loop: `until condition { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct UntilStatement {
    pub condition: Expression,
    pub body: Block,
    pub span: Span,
}

/// A wait until statement: `wait until condition` or `wait until condition timeout duration`
#[derive(Debug, Clone, PartialEq)]
pub struct WaitUntilStatement {
    pub condition: Expression,
    pub timeout: Option<Expression>,
    pub span: Span,
}

/// A block of statements: `{ statement* }`
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
    /// Source location of the opening `{`.
    pub span: Span,
}

/// A conditional statement: `if condition { ... } else { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    /// The condition expression.
    pub condition: Expression,
    /// The block to execute if the condition is truthy.
    pub then_branch: Block,
    /// The optional block to execute if the condition is falsy.
    pub else_branch: Option<Block>,
    /// Source location of the `if` keyword.
    pub span: Span,
}

/// An assignment statement: `target = expression` or `target op= expression`
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentStatement {
    /// The assignment target (variable or indexed expression).
    pub target: AssignTarget,
    /// The value expression.
    pub value: Expression,
    /// Compound assignment operator (None for plain `=`).
    pub compound_op: Option<BinaryOp>,
    /// Source location.
    pub span: Span,
}

/// An assignment target — the left-hand side of `=`.
#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    /// A simple variable: `x = ...`
    Variable(String),
    /// An indexed target: `expr[index] = ...`
    Index {
        object: Box<AssignTarget>,
        index: Expression,
    },
    /// A destructuring pattern: `[a, b] = ...` or `{"k": v} = ...`
    Pattern(Pattern),
}

/// A while loop: `while condition { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct WhileStatement {
    /// The loop condition expression.
    pub condition: Expression,
    /// The loop body.
    pub body: Block,
    /// Source location of the `while` keyword.
    pub span: Span,
}

/// A type annotation in Flux source code.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    /// A simple named type: `Int`, `String`, `Bool`, etc.
    Named(String, Span),
    /// A generic type: `Array<Int>`, `Map<String, Int>`, `Task<Int>`
    Generic(String, Vec<TypeAnnotation>, Span),
    /// A function type: `(Int, Int) -> Int`
    FunctionType(Vec<TypeAnnotation>, Box<TypeAnnotation>, Span),
    /// An optional type: `Int?`
    Optional(Box<TypeAnnotation>, Span),
    /// A union type: `Int | String`
    Union(Vec<TypeAnnotation>, Span),
}

impl TypeAnnotation {
    pub fn span(&self) -> &Span {
        match self {
            TypeAnnotation::Named(_, s) => s,
            TypeAnnotation::Generic(_, _, s) => s,
            TypeAnnotation::FunctionType(_, _, s) => s,
            TypeAnnotation::Optional(_, s) => s,
            TypeAnnotation::Union(_, s) => s,
        }
    }
}

/// A function declaration: `fn name(params) { body }` or `fn name(params) -> Type { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    /// The function name.
    pub name: String,
    /// Generic type parameters: `<T, U>`
    pub generic_params: Vec<String>,
    /// Parameter patterns.
    pub params: Vec<Pattern>,
    /// Optional return type annotation.
    pub return_type: Option<TypeAnnotation>,
    /// The function body.
    pub body: Block,
    /// Source location of the `fn` keyword.
    pub span: Span,
}

/// A return statement: `return expression?`
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
    pub span: Span,
}

/// A function call expression: `callee(arguments)`
#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
    pub span: Span,
}

/// A variable declaration: `let name: Type = initializer` or `let pattern = initializer`
#[derive(Debug, Clone, PartialEq)]
pub struct LetStatement {
    pub pattern: Pattern,
    /// Optional type annotation.
    pub type_annotation: Option<TypeAnnotation>,
    pub initializer: Expression,
    pub span: Span,
}

/// An expression that evaluates to a value.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A string literal: `"hello"`
    StringLiteral(StringLit),
    /// An integer literal: `42`
    IntegerLiteral(IntegerLit),
    /// A floating-point literal: `3.14`
    FloatLiteral(FloatLit),
    /// A boolean literal: `true` or `false`
    BooleanLiteral(BooleanLit),
    /// A nil literal: `nil`
    NilLiteral(Span),
    /// A duration literal: `5s`, `100ms`, `2h`
    DurationLiteral(DurationLit),
    /// A variable reference: `x`
    Identifier(IdentifierExpr),
    /// A binary operation: `left op right`
    Binary(BinaryExpr),
    /// A unary operation: `op operand`
    Unary(UnaryExpr),
    /// A function call: `callee(args)` where callee is any expression
    Call(CallExpr),
    /// An array literal: `[a, b, c]`
    Array(ArrayExpr),
    /// A map literal: `{"key": value, ...}`
    Map(MapExpr),
    /// An index operation: `expr[index]`
    Index(IndexExpr),
    /// A member call: `module.func(args)`
    MemberCall(MemberCallExpr),
    /// A member access: `module.variable`
    MemberAccess(MemberAccessExpr),
    /// An anonymous function: `fn(params) { body }`
    FunctionExpr(FunctionExprNode),
    /// A range expression: `start..end` or `start..<end`
    Range(RangeExpr),
    /// An after expression: `after duration { body }` — returns Task
    After(Box<AfterStatement>),
    /// An every expression: `every interval { body }` — returns Task
    Every(Box<EveryStatement>),
    /// A calendar every expression: `every day at time(...) { body }` — returns Task
    EveryCalendar(Box<EveryCalendarStatement>),
    /// An at expression: `at target { body }` — returns Task
    At(Box<AtStatement>),
    /// An await expression: `await task`
    Await(Box<AwaitExpr>),
    /// A spawn expression: `spawn { body }` — returns Task
    Spawn(Box<SpawnStatement>),
}

/// An await expression: `await task_expr`
#[derive(Debug, Clone, PartialEq)]
pub struct AwaitExpr {
    pub task_expr: Expression,
    pub span: Span,
}

/// A range expression: `start..end` or `start..<end`
#[derive(Debug, Clone, PartialEq)]
pub struct RangeExpr {
    pub start: Box<Expression>,
    pub end: Box<Expression>,
    pub inclusive: bool,
    pub span: Span,
}

/// An anonymous function expression: `fn(params) { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionExprNode {
    pub params: Vec<Pattern>,
    pub body: Block,
    pub span: Span,
}

/// An import statement.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportStatement {
    /// Module path segments (e.g., ["utils", "math"] for `import utils.math`).
    pub module_path: Vec<String>,
    /// Optional alias (e.g., "m" for `import math as m`).
    pub alias: Option<String>,
    /// Selective imports (e.g., `from math import square, cube`). Empty for regular import.
    pub selective: Vec<ImportName>,
    pub span: Span,
}

/// A single name in a `from ... import ...` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportName {
    pub name: String,
    pub alias: Option<String>,
}

/// A member function call: `object.method(args)`
#[derive(Debug, Clone, PartialEq)]
pub struct MemberCallExpr {
    pub object: String,
    pub member: String,
    pub arguments: Vec<Expression>,
    pub span: Span,
}

/// A member access expression: `object.member`
#[derive(Debug, Clone, PartialEq)]
pub struct MemberAccessExpr {
    pub object: String,
    pub member: String,
    pub span: Span,
}

/// An array literal expression.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayExpr {
    pub elements: Vec<Expression>,
    pub span: Span,
}

/// A map literal expression: `{"key": value, ...}`
#[derive(Debug, Clone, PartialEq)]
pub struct MapExpr {
    pub entries: Vec<(Expression, Expression)>,
    pub span: Span,
}

/// An index expression: `object[index]`
#[derive(Debug, Clone, PartialEq)]
pub struct IndexExpr {
    pub object: Box<Expression>,
    pub index: Box<Expression>,
    pub span: Span,
}

/// A string literal value.
#[derive(Debug, Clone, PartialEq)]
pub struct StringLit {
    /// The string content (without surrounding quotes).
    pub value: String,
    /// Source location of the opening quote.
    pub span: Span,
}

/// An integer literal value.
#[derive(Debug, Clone, PartialEq)]
pub struct IntegerLit {
    pub value: i64,
    pub span: Span,
}

/// A floating-point literal value.
#[derive(Debug, Clone, PartialEq)]
pub struct FloatLit {
    pub value: f64,
    pub span: Span,
}

/// A boolean literal value.
#[derive(Debug, Clone, PartialEq)]
pub struct BooleanLit {
    pub value: bool,
    pub span: Span,
}

/// A duration literal: `5s`, `100ms`, `2h`
#[derive(Debug, Clone, PartialEq)]
pub struct DurationLit {
    /// The numeric value portion (e.g. 5 in `5s`)
    pub value: i64,
    /// The unit suffix (e.g. "s", "ms", "h")
    pub unit: String,
    pub span: Span,
}

/// A variable reference in an expression.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentifierExpr {
    pub name: String,
    pub span: Span,
}

/// A binary expression: `left op right`
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr {
    pub left: Box<Expression>,
    pub operator: BinaryOp,
    pub right: Box<Expression>,
    /// Source location of the operator.
    pub span: Span,
}

/// Binary operators.
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
    NotEqual,
    LogicalAnd,
    LogicalOr,
    LogicalXor,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    In,
    NotIn,
}

/// A unary expression: `op operand`
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr {
    pub operator: UnaryOp,
    pub operand: Box<Expression>,
    /// Source location of the operator.
    pub span: Span,
}

/// Unary operators.
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Negate,
    BitwiseNot,
}

/// A binding pattern used in `let`, function parameters, `for`, and destructuring assignment.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Bind to a single variable: `x`
    Identifier(String, Span),
    /// Bind to a typed variable: `x: Int`
    TypedIdentifier(String, TypeAnnotation, Span),
    /// Discard value: `_`
    Wildcard(Span),
    /// Destructure an array: `[a, b, c]`
    Array(Vec<Pattern>, Span),
    /// Destructure a map: `{"key": pattern, ...}`
    Map(Vec<(String, Pattern)>, Span),
}

impl Pattern {
    /// Get the span of this pattern.
    pub fn span(&self) -> &Span {
        match self {
            Pattern::Identifier(_, s) => s,
            Pattern::TypedIdentifier(_, _, s) => s,
            Pattern::Wildcard(s) => s,
            Pattern::Array(_, s) => s,
            Pattern::Map(_, s) => s,
        }
    }
}
