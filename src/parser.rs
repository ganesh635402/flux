// Flux parser - transforms tokens into an abstract syntax tree.

use crate::ast::{
    AfterStatement, ArrayExpr, AssignTarget, AssignmentStatement, AtStatement, BinaryExpr,
    BinaryOp, Block, BooleanLit, CallExpr, DurationLit, EveryCalendarStatement, EveryStatement,
    Expression, FloatLit, ForStatement, FunctionDecl, FunctionExprNode, IdentifierExpr,
    IfStatement, ImportStatement, IndexExpr, IntegerLit, LetStatement, MapExpr, MemberAccessExpr,
    MemberCallExpr, Pattern, Program, RangeExpr, ReturnStatement, Statement, StringLit, UnaryExpr,
    UnaryOp, UntilStatement, WaitUntilStatement, WhileStatement,
};
use crate::lexer::{Span, Token, TokenKind};

/// An error encountered during parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// A human-readable description of the error.
    pub message: String,
    /// Where in the source the error was detected.
    pub span: Span,
}

/// The result of parsing a token stream.
pub struct ParseResult {
    /// The parsed program (may be partial if errors occurred).
    pub program: Program,
    /// Any errors encountered during parsing.
    pub errors: Vec<ParseError>,
}

/// The Flux parser. Consumes tokens and builds an AST.
pub struct Parser {
    /// The token stream to parse.
    tokens: Vec<Token>,
    /// Current position in the token stream.
    current: usize,
}

impl Parser {
    /// Create a new parser for the given token stream.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    /// Parse the token stream into a program AST.
    pub fn parse(mut self) -> ParseResult {
        let mut statements = Vec::new();
        let mut errors = Vec::new();

        while !self.is_at_end() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    errors.push(err);
                    self.synchronize();
                }
            }
        }

        ParseResult {
            program: Program { statements },
            errors,
        }
    }

    /// Parse input for REPL mode. If normal parsing fails and the input might be
    /// a bare expression, try parsing it as an expression statement.
    pub fn parse_repl(mut self) -> ParseResult {
        // First try normal parsing
        let saved_pos = self.current;
        let mut statements = Vec::new();
        let mut errors = Vec::new();

        while !self.is_at_end() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    errors.push(err);
                    self.synchronize();
                }
            }
        }

        // If no errors, return normally
        if errors.is_empty() {
            return ParseResult {
                program: Program { statements },
                errors,
            };
        }

        // If there were errors, try parsing the whole input as an expression
        self.current = saved_pos;
        match self.parse_expression() {
            Ok(expr) => {
                if self.is_at_end() {
                    // Successfully parsed everything as an expression
                    ParseResult {
                        program: Program {
                            statements: vec![Statement::Expression(expr)],
                        },
                        errors: vec![],
                    }
                } else {
                    // Expression parsed but there's more input — return original errors
                    ParseResult {
                        program: Program { statements },
                        errors,
                    }
                }
            }
            Err(_) => {
                // Expression parsing also failed — return original statement errors
                ParseResult {
                    program: Program { statements },
                    errors,
                }
            }
        }
    }

    /// Parse a single statement.
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        // Destructuring assignment: `[a, b] = expr` or `{"k": v} = expr`
        if matches!(
            self.peek_kind(),
            TokenKind::LeftBracket | TokenKind::LeftBrace
        ) {
            let span = self.peek_span();
            let pattern = self.parse_pattern()?;
            check_duplicate_bindings(&pattern)?;
            self.expect(
                TokenKind::Equals,
                "expected '=' after destructuring pattern",
            )?;
            let value = self.parse_expression()?;
            return Ok(Statement::Assignment(AssignmentStatement {
                target: AssignTarget::Pattern(pattern),
                value,
                compound_op: None,
                span,
            }));
        }

        let name_token = self.expect(TokenKind::Identifier, "expected statement")?;
        let name = name_token.lexeme.clone();
        let span = name_token.span.clone();

        // Keywords that cannot start a statement
        if name == "true" || name == "false" || name == "nil" || name == "else" {
            return Err(ParseError {
                message: "expected statement".to_string(),
                span,
            });
        }

        // Dispatch keywords
        if name == "let" {
            return self.parse_let_statement(span);
        }
        if name == "if" {
            return self.parse_if_statement(span);
        }
        if name == "while" {
            return self.parse_while_statement(span);
        }
        if name == "fn" {
            return self.parse_function_declaration(span);
        }
        if name == "return" {
            return self.parse_return_statement(span);
        }
        if name == "import" {
            return self.parse_import_statement(span);
        }
        if name == "from" {
            return self.parse_from_import(span);
        }
        if name == "for" {
            return self.parse_for_statement(span);
        }
        if name == "after" {
            return self.parse_after_statement(span);
        }
        if name == "every" {
            return self.parse_every_statement(span);
        }
        if name == "at" {
            return self.parse_at_statement(span);
        }
        if name == "until" {
            return self.parse_until_statement(span);
        }
        if name == "wait" {
            return self.parse_wait_statement(span);
        }
        if name == "throw" {
            return self.parse_throw_statement(span);
        }
        if name == "try" {
            return self.parse_try_catch(span);
        }
        if name == "on" {
            return self.parse_on_statement(span);
        }
        if name == "spawn" {
            let body = self.parse_block()?;
            return Ok(Statement::Spawn(crate::ast::SpawnStatement { body, span }));
        }
        if name == "type" && self.peek_kind() == &TokenKind::Identifier {
            return self.parse_type_statement(span);
        }
        if name == "break" {
            return Ok(Statement::Break(span));
        }
        if name == "continue" {
            return Ok(Statement::Continue(span));
        }

        // Assignment or indexed assignment: identifier[...][...]... = expression
        // or compound assignment: identifier op= expression
        // or simple: identifier = expression
        if self.peek_kind() == &TokenKind::Equals
            || self.peek_kind() == &TokenKind::LeftBracket
            || self.is_compound_assign_token()
        {
            // Build the assignment target starting from the variable name
            let mut target = AssignTarget::Variable(name.clone());

            // Parse chain of [index] operations
            while self.peek_kind() == &TokenKind::LeftBracket {
                self.advance(); // consume '['
                let index = self.parse_expression()?;
                self.expect(TokenKind::RightBracket, "expected ']' after index")?;
                target = AssignTarget::Index {
                    object: Box::new(target),
                    index,
                };
            }

            // Check for compound assignment
            if let Some(op) = self.try_compound_assign_op() {
                self.advance(); // consume the compound token
                let value = self.parse_expression()?;
                return Ok(Statement::Assignment(AssignmentStatement {
                    target,
                    value,
                    compound_op: Some(op),
                    span,
                }));
            }

            // Now we must have '='
            if self.peek_kind() == &TokenKind::Equals {
                self.advance(); // consume '='
                let value = self.parse_expression()?;
                return Ok(Statement::Assignment(AssignmentStatement {
                    target,
                    value,
                    compound_op: None,
                    span,
                }));
            }

            // No '=' after indexing — not a valid statement
            return Err(ParseError {
                message: "expected '=' after indexed expression".to_string(),
                span: self.peek_span(),
            });
        }

        // Expression statement: identifier(...) — function call as statement
        if self.peek_kind() == &TokenKind::LeftParen {
            self.advance(); // consume '('
            let arguments = self.parse_arguments()?;
            self.expect(TokenKind::RightParen, "expected ')' after arguments")?;
            let call_expr = Expression::Call(CallExpr {
                callee: Box::new(Expression::Identifier(IdentifierExpr {
                    name: name.clone(),
                    span: span.clone(),
                })),
                arguments,
                span,
            });
            return Ok(Statement::Expression(call_expr));
        }

        // Member call statement: identifier.member(args)
        if self.peek_kind() == &TokenKind::Dot {
            self.advance(); // consume '.'
            let member_token =
                self.expect(TokenKind::Identifier, "expected member name after '.'")?;
            let member = member_token.lexeme.clone();

            if self.peek_kind() == &TokenKind::LeftParen {
                self.advance(); // consume '('
                let arguments = self.parse_arguments()?;
                self.expect(TokenKind::RightParen, "expected ')' after arguments")?;
                let call_expr = Expression::MemberCall(MemberCallExpr {
                    object: name,
                    member,
                    arguments,
                    span,
                });
                return Ok(Statement::Expression(call_expr));
            }

            return Err(ParseError {
                message: "expected '(' after member name".to_string(),
                span: self.peek_span(),
            });
        }

        Err(ParseError {
            message: format!("unexpected identifier '{}' in statement position", name),
            span,
        })
    }

    /// Parse an import statement: `import path` or `import path as alias`
    fn parse_import_statement(&mut self, import_span: Span) -> Result<Statement, ParseError> {
        let mut path = Vec::new();
        let name_token =
            self.expect(TokenKind::Identifier, "expected module name after 'import'")?;
        path.push(name_token.lexeme.clone());

        // Parse dotted path: import utils.math
        while self.peek_kind() == &TokenKind::Dot {
            self.advance(); // consume '.'
            let segment = self.expect(TokenKind::Identifier, "expected module name after '.'")?;
            path.push(segment.lexeme.clone());
        }

        // Check for alias: import math as m
        let alias = if self.peek_kind() == &TokenKind::Identifier
            && self.tokens[self.current].lexeme == "as"
        {
            self.advance(); // consume "as"
            let alias_token = self.expect(TokenKind::Identifier, "expected alias after 'as'")?;
            Some(alias_token.lexeme.clone())
        } else {
            None
        };

        Ok(Statement::Import(ImportStatement {
            module_path: path,
            alias,
            selective: Vec::new(),
            span: import_span,
        }))
    }

    /// Parse `from path import name1, name2 as alias, ...`
    fn parse_from_import(&mut self, span: Span) -> Result<Statement, ParseError> {
        let mut path = Vec::new();
        let name_token = self.expect(TokenKind::Identifier, "expected module name after 'from'")?;
        path.push(name_token.lexeme.clone());

        // Parse dotted path
        while self.peek_kind() == &TokenKind::Dot {
            self.advance();
            let segment = self.expect(TokenKind::Identifier, "expected module name after '.'")?;
            path.push(segment.lexeme.clone());
        }

        // Expect "import"
        let import_token =
            self.expect(TokenKind::Identifier, "expected 'import' after module name")?;
        if import_token.lexeme != "import" {
            return Err(ParseError {
                message: "expected 'import' after module name".to_string(),
                span: import_token.span.clone(),
            });
        }

        // Parse name list: name1 as alias1, name2, name3 as alias3
        let mut names = Vec::new();
        loop {
            let name = self.expect(TokenKind::Identifier, "expected name to import")?;
            let import_name = name.lexeme.clone();

            let alias = if self.peek_kind() == &TokenKind::Identifier
                && self.tokens[self.current].lexeme == "as"
            {
                self.advance(); // consume "as"
                let alias_token =
                    self.expect(TokenKind::Identifier, "expected alias after 'as'")?;
                Some(alias_token.lexeme.clone())
            } else {
                None
            };

            names.push(crate::ast::ImportName {
                name: import_name,
                alias,
            });

            if self.peek_kind() != &TokenKind::Comma {
                break;
            }
            self.advance(); // consume ','
        }

        Ok(Statement::Import(ImportStatement {
            module_path: path,
            alias: None,
            selective: names,
            span,
        }))
    }

    /// Parse a for statement: `for pattern in expression { body }`
    fn parse_for_statement(&mut self, for_span: Span) -> Result<Statement, ParseError> {
        let pattern = if matches!(
            self.peek_kind(),
            TokenKind::LeftBracket | TokenKind::LeftBrace
        ) {
            self.parse_pattern()?
        } else {
            let var_token =
                self.expect(TokenKind::Identifier, "expected variable name after 'for'")?;
            Pattern::Identifier(var_token.lexeme.clone(), var_token.span.clone())
        };
        check_duplicate_bindings(&pattern)?;

        // Expect 'in' keyword
        let in_token = self.expect(TokenKind::Identifier, "expected 'in' after for variable")?;
        if in_token.lexeme != "in" {
            return Err(ParseError {
                message: "expected 'in' after for variable".to_string(),
                span: in_token.span.clone(),
            });
        }

        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Statement::For(ForStatement {
            pattern,
            iterable,
            body,
            span: for_span,
        }))
    }

    /// Parse an after statement: `after expression { body }`
    fn parse_after_statement(&mut self, span: Span) -> Result<Statement, ParseError> {
        let delay = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Statement::After(AfterStatement { delay, body, span }))
    }

    /// Parse an on statement: `on "type" { body }` or `on "type" as param { body }`
    fn parse_on_statement(&mut self, span: Span) -> Result<Statement, ParseError> {
        let event_type = self.parse_expression()?;

        // Check for optional parameter: `as param`
        let param = if self.peek_kind() == &TokenKind::Identifier
            && self.tokens[self.current].lexeme == "as"
        {
            self.advance(); // consume 'as'
            let param_token =
                self.expect(TokenKind::Identifier, "expected parameter name after 'as'")?;
            Some(param_token.lexeme.clone())
        } else {
            None
        };

        // Check for optional filter: `where expression`
        let filter = if self.peek_kind() == &TokenKind::Identifier
            && self.tokens[self.current].lexeme == "where"
        {
            self.advance(); // consume 'where'
            Some(self.parse_expression()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        Ok(Statement::On(crate::ast::OnStatement {
            event_type,
            param,
            filter,
            body,
            span,
        }))
    }

    /// Parse an every statement: `every expression { body }` or calendar forms.
    fn parse_every_statement(&mut self, span: Span) -> Result<Statement, ParseError> {
        if let Some(result) = self.try_parse_every_calendar(span.clone())? {
            return Ok(result);
        }
        let interval = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Statement::Every(EveryStatement {
            interval,
            body,
            span,
        }))
    }

    /// Try to parse calendar forms of `every`. Returns None if not a calendar form.
    fn try_parse_every_calendar(&mut self, span: Span) -> Result<Option<Statement>, ParseError> {
        use crate::time::CalendarRecurrence;

        if self.peek_kind() != &TokenKind::Identifier {
            return Ok(None);
        }

        let name = self.tokens[self.current].lexeme.as_str();

        // every day at time(...) { ... }
        if name == "day" {
            self.advance(); // consume "day"
            let at_token = self.expect(TokenKind::Identifier, "expected 'at' after 'day'")?;
            if at_token.lexeme != "at" {
                return Err(ParseError {
                    message: "expected 'at' after 'day'".to_string(),
                    span: at_token.span.clone(),
                });
            }
            let time_expr = self.parse_expression()?;
            let body = self.parse_block()?;
            return Ok(Some(Statement::EveryCalendar(EveryCalendarStatement {
                recurrence: CalendarRecurrence::Daily,
                time_expr,
                body,
                span,
            })));
        }

        // every Monday/Tuesday/.../Sunday at time(...) { ... }
        if let Some(wd) = CalendarRecurrence::weekday_number(name) {
            self.advance(); // consume weekday name
            let at_token = self.expect(TokenKind::Identifier, "expected 'at' after weekday")?;
            if at_token.lexeme != "at" {
                return Err(ParseError {
                    message: "expected 'at' after weekday name".to_string(),
                    span: at_token.span.clone(),
                });
            }
            let time_expr = self.parse_expression()?;
            let body = self.parse_block()?;
            return Ok(Some(Statement::EveryCalendar(EveryCalendarStatement {
                recurrence: CalendarRecurrence::Weekly(wd),
                time_expr,
                body,
                span,
            })));
        }

        // every month on <day> at time(...) { ... }
        if name == "month" {
            self.advance(); // consume "month"
            let on_token = self.expect(TokenKind::Identifier, "expected 'on' after 'month'")?;
            if on_token.lexeme != "on" {
                return Err(ParseError {
                    message: "expected 'on' after 'month'".to_string(),
                    span: on_token.span.clone(),
                });
            }
            let day_token =
                self.expect(TokenKind::IntegerLiteral, "expected day number after 'on'")?;
            let day: u32 = day_token.lexeme.parse().map_err(|_| ParseError {
                message: "invalid day number".to_string(),
                span: day_token.span.clone(),
            })?;
            let at_token = self.expect(TokenKind::Identifier, "expected 'at' after day")?;
            if at_token.lexeme != "at" {
                return Err(ParseError {
                    message: "expected 'at' after day number".to_string(),
                    span: at_token.span.clone(),
                });
            }
            let time_expr = self.parse_expression()?;
            let body = self.parse_block()?;
            return Ok(Some(Statement::EveryCalendar(EveryCalendarStatement {
                recurrence: CalendarRecurrence::Monthly(day),
                time_expr,
                body,
                span,
            })));
        }

        // every year on <month>/<day> at time(...) { ... }
        if name == "year" {
            self.advance(); // consume "year"
            let on_token = self.expect(TokenKind::Identifier, "expected 'on' after 'year'")?;
            if on_token.lexeme != "on" {
                return Err(ParseError {
                    message: "expected 'on' after 'year'".to_string(),
                    span: on_token.span.clone(),
                });
            }
            let month_token = self.expect(TokenKind::IntegerLiteral, "expected month number")?;
            let month: u32 = month_token.lexeme.parse().map_err(|_| ParseError {
                message: "invalid month number".to_string(),
                span: month_token.span.clone(),
            })?;
            self.expect(TokenKind::Slash, "expected '/' between month and day")?;
            let day_token =
                self.expect(TokenKind::IntegerLiteral, "expected day number after '/'")?;
            let day: u32 = day_token.lexeme.parse().map_err(|_| ParseError {
                message: "invalid day number".to_string(),
                span: day_token.span.clone(),
            })?;
            let at_token = self.expect(TokenKind::Identifier, "expected 'at' after date")?;
            if at_token.lexeme != "at" {
                return Err(ParseError {
                    message: "expected 'at' after date".to_string(),
                    span: at_token.span.clone(),
                });
            }
            let time_expr = self.parse_expression()?;
            let body = self.parse_block()?;
            return Ok(Some(Statement::EveryCalendar(EveryCalendarStatement {
                recurrence: CalendarRecurrence::Yearly(month, day),
                time_expr,
                body,
                span,
            })));
        }

        Ok(None)
    }

    /// Parse an at statement: `at expression { body }`
    fn parse_at_statement(&mut self, span: Span) -> Result<Statement, ParseError> {
        let target = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Statement::At(AtStatement { target, body, span }))
    }

    /// Parse an until loop: `until condition { body }`
    fn parse_until_statement(&mut self, span: Span) -> Result<Statement, ParseError> {
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Statement::Until(UntilStatement {
            condition,
            body,
            span,
        }))
    }

    /// Parse a wait statement: `wait until condition` or `wait until condition timeout duration`
    fn parse_wait_statement(&mut self, span: Span) -> Result<Statement, ParseError> {
        // Expect "until" keyword after "wait"
        let until_token = self.expect(TokenKind::Identifier, "expected 'until' after 'wait'")?;
        if until_token.lexeme != "until" {
            return Err(ParseError {
                message: "expected 'until' after 'wait'".to_string(),
                span: until_token.span.clone(),
            });
        }

        let condition = self.parse_expression()?;

        // Check for optional "timeout" keyword
        let timeout = if self.peek_kind() == &TokenKind::Identifier
            && self.tokens[self.current].lexeme == "timeout"
        {
            self.advance(); // consume "timeout"
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::WaitUntil(WaitUntilStatement {
            condition,
            timeout,
            span,
        }))
    }

    /// Parse a throw statement: `throw expression`
    fn parse_throw_statement(&mut self, span: Span) -> Result<Statement, ParseError> {
        let value = self.parse_expression()?;
        Ok(Statement::Throw(crate::ast::ThrowStatement { value, span }))
    }

    /// Parse a try/catch/finally statement.
    fn parse_try_catch(&mut self, span: Span) -> Result<Statement, ParseError> {
        let try_body = self.parse_block()?;

        let mut catch_var = None;
        let mut catch_body = None;
        let mut finally_body = None;

        // Check for catch
        if self.peek_kind() == &TokenKind::Identifier && self.tokens[self.current].lexeme == "catch"
        {
            self.advance(); // consume "catch"
            let var_token = self.expect(
                TokenKind::Identifier,
                "expected variable name after 'catch'",
            )?;
            catch_var = Some(var_token.lexeme.clone());
            catch_body = Some(self.parse_block()?);
        }

        // Check for finally
        if self.peek_kind() == &TokenKind::Identifier
            && self.tokens[self.current].lexeme == "finally"
        {
            self.advance(); // consume "finally"
            finally_body = Some(self.parse_block()?);
        }

        if catch_body.is_none() && finally_body.is_none() {
            return Err(ParseError {
                message: "try requires at least 'catch' or 'finally'".to_string(),
                span,
            });
        }

        Ok(Statement::TryCatch(crate::ast::TryCatchStatement {
            try_body,
            catch_var,
            catch_body,
            finally_body,
            span,
        }))
    }

    /// Parse a let declaration: `let identifier = expression` or `let pattern = expression`
    /// Called after `let` has already been consumed.
    fn parse_let_statement(&mut self, let_span: Span) -> Result<Statement, ParseError> {
        // Check for destructuring patterns
        if matches!(
            self.peek_kind(),
            TokenKind::LeftBracket | TokenKind::LeftBrace
        ) {
            let pattern = self.parse_pattern()?;
            check_duplicate_bindings(&pattern)?;
            self.expect(TokenKind::Equals, "expected '=' after pattern")?;
            let initializer = self.parse_expression()?;
            return Ok(Statement::Let(LetStatement {
                pattern,
                type_annotation: None,
                initializer,
                span: let_span,
            }));
        }

        // Expect the variable name
        let name_token =
            self.expect(TokenKind::Identifier, "expected variable name after 'let'")?;
        let var_name = name_token.lexeme.clone();

        // Keywords cannot be used as variable names
        if var_name == "true"
            || var_name == "false"
            || var_name == "nil"
            || var_name == "let"
            || var_name == "if"
            || var_name == "else"
            || var_name == "while"
            || var_name == "fn"
            || var_name == "return"
            || var_name == "import"
            || var_name == "for"
            || var_name == "in"
            || var_name == "break"
            || var_name == "continue"
        {
            return Err(ParseError {
                message: format!("'{}' cannot be used as a variable name", var_name),
                span: self.tokens[self.current - 1].span.clone(),
            });
        }

        // Check for optional type annotation: `let x: Type = ...`
        let type_annotation = if self.peek_kind() == &TokenKind::Colon {
            self.advance(); // consume ':'
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // Expect `=`
        self.expect(TokenKind::Equals, "expected '=' after variable name")?;

        // Parse the initializer expression
        let initializer = self.parse_expression()?;

        Ok(Statement::Let(LetStatement {
            pattern: Pattern::Identifier(var_name, let_span.clone()),
            type_annotation,
            initializer,
            span: let_span,
        }))
    }

    /// Parse an if statement: `if expression { ... } else { ... }`
    /// Called after `if` has already been consumed.
    fn parse_if_statement(&mut self, if_span: Span) -> Result<Statement, ParseError> {
        // Parse the condition expression
        let condition = self.parse_expression()?;

        // Parse the then block
        let then_branch = self.parse_block()?;

        // Check for optional else branch
        let else_branch = if self.peek_kind() == &TokenKind::Identifier
            && self.tokens[self.current].lexeme == "else"
        {
            self.advance(); // consume 'else'
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Statement::If(IfStatement {
            condition,
            then_branch,
            else_branch,
            span: if_span,
        }))
    }

    /// Parse a while statement: `while expression { ... }`
    /// Called after `while` has already been consumed.
    fn parse_while_statement(&mut self, while_span: Span) -> Result<Statement, ParseError> {
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Statement::While(WhileStatement {
            condition,
            body,
            span: while_span,
        }))
    }

    /// Parse a function declaration: `fn name(params) { body }`
    /// Called after `fn` has already been consumed.
    fn parse_function_declaration(&mut self, fn_span: Span) -> Result<Statement, ParseError> {
        let name_token = self.expect(TokenKind::Identifier, "expected function name after 'fn'")?;
        let name = name_token.lexeme.clone();

        // Keywords cannot be function names
        if matches!(
            name.as_str(),
            "true" | "false" | "nil" | "let" | "if" | "else" | "while" | "fn" | "return"
        ) {
            return Err(ParseError {
                message: format!("'{}' cannot be used as a function name", name),
                span: self.tokens[self.current - 1].span.clone(),
            });
        }

        // Check for generic parameters: `<T, U>`
        let generic_params = if self.peek_kind() == &TokenKind::Less {
            self.advance(); // consume '<'
            let mut params = Vec::new();
            loop {
                let param =
                    self.expect(TokenKind::Identifier, "expected generic type parameter")?;
                params.push(param.lexeme.clone());
                if self.peek_kind() == &TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::Greater, "expected '>' after generic parameters")?;
            params
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LeftParen, "expected '(' after function name")?;

        let params = self.parse_param_patterns()?;
        check_duplicate_bindings_across(&params)?;

        self.expect(TokenKind::RightParen, "expected ')' after parameters")?;

        // Check for optional return type: `-> Type`
        let return_type = if self.peek_kind() == &TokenKind::Arrow {
            self.advance(); // consume '->'
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Statement::Function(FunctionDecl {
            name,
            generic_params,
            params,
            return_type,
            body,
            span: fn_span,
        }))
    }

    /// Parse a return statement: `return expression?`
    /// Called after `return` has already been consumed.
    fn parse_return_statement(&mut self, return_span: Span) -> Result<Statement, ParseError> {
        // Check if there's a value to return (not at end, not at `}`, not at next statement keyword)
        let value = if !self.is_at_end()
            && self.peek_kind() != &TokenKind::RightBrace
            && !(self.peek_kind() == &TokenKind::Identifier
                && matches!(
                    self.tokens[self.current].lexeme.as_str(),
                    "let" | "if" | "while" | "return" | "else" | "for" | "break" | "continue"
                )) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Return(ReturnStatement {
            value,
            span: return_span,
        }))
    }

    /// Parse comma-separated arguments: `expression ("," expression)*`
    fn parse_arguments(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut args = Vec::new();
        if self.peek_kind() == &TokenKind::RightParen {
            return Ok(args);
        }

        args.push(self.parse_expression()?);
        while self.peek_kind() == &TokenKind::Comma {
            self.advance(); // consume ','
            args.push(self.parse_expression()?);
        }

        Ok(args)
    }

    /// Parse a type statement: `type Name = Type` or `type Name { fields }`
    fn parse_type_statement(&mut self, span: Span) -> Result<Statement, ParseError> {
        let name_token = self.expect(TokenKind::Identifier, "expected type name after 'type'")?;
        let name = name_token.lexeme.clone();

        if self.peek_kind() == &TokenKind::Equals {
            // Type alias: `type Name = Type`
            self.advance(); // consume '='
            let target = self.parse_type_annotation()?;
            Ok(Statement::TypeAlias(crate::ast::TypeAliasStatement {
                name,
                target,
                span,
            }))
        } else if self.peek_kind() == &TokenKind::LeftBrace {
            // Struct definition: `type Name { field: Type, ... }`
            self.advance(); // consume '{'
            let mut fields = Vec::new();
            while !self.is_at_end() && self.peek_kind() != &TokenKind::RightBrace {
                let field_token = self.expect(TokenKind::Identifier, "expected field name")?;
                let field_name = field_token.lexeme.clone();
                self.expect(TokenKind::Colon, "expected ':' after field name")?;
                let field_type = self.parse_type_annotation()?;
                fields.push((field_name, field_type));
                // Allow optional comma between fields
                if self.peek_kind() == &TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(TokenKind::RightBrace, "expected '}'")?;
            Ok(Statement::StructDef(crate::ast::StructDefStatement {
                name,
                fields,
                span,
            }))
        } else {
            Err(ParseError {
                message: "expected '=' or '{' after type name".to_string(),
                span: self.peek_span(),
            })
        }
    }

    /// Parse a type annotation: `Int`, `Array<Int>`, `Map<String, Int>`, `Int?`, etc.
    fn parse_type_annotation(&mut self) -> Result<crate::ast::TypeAnnotation, ParseError> {
        let name_token = self.expect(TokenKind::Identifier, "expected type name")?;
        let name = name_token.lexeme.clone();
        let span = name_token.span.clone();

        // Check for generic parameters: `<...>`
        if self.peek_kind() == &TokenKind::Less {
            self.advance(); // consume '<'
            let mut type_params = Vec::new();
            loop {
                type_params.push(self.parse_type_annotation()?);
                if self.peek_kind() == &TokenKind::Comma {
                    self.advance(); // consume ','
                } else {
                    break;
                }
            }
            self.expect(TokenKind::Greater, "expected '>' after type parameters")?;

            // Check for optional: `Array<Int>?`
            if self.peek_kind() == &TokenKind::Identifier && !self.is_at_end() {
                // Not optional suffix via identifier
            }

            return Ok(crate::ast::TypeAnnotation::Generic(name, type_params, span));
        }

        // Check for optional suffix: `Int?`
        // We check if the next char is `?` — but `?` is not a token.
        // For simplicity, we use `!` + check, or just skip optional for now.
        // Optional types can be written as `Int | Nil` using union syntax.

        Ok(crate::ast::TypeAnnotation::Named(name, span))
    }

    /// Parse a block: `{ statement* }`
    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let span = self.peek_span();
        self.expect(TokenKind::LeftBrace, "expected '{'")?;

        let mut statements = Vec::new();

        while !self.is_at_end() && self.peek_kind() != &TokenKind::RightBrace {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    return Err(err);
                }
            }
        }

        self.expect(TokenKind::RightBrace, "expected '}'")?;

        Ok(Block { statements, span })
    }

    // --- Expression parsing (precedence climbing) ---
    //
    // Precedence levels (lowest to highest):
    //   1.  Logical OR:     ||
    //   2.  Logical XOR:    ^^
    //   3.  Logical AND:    &&
    //   4.  Equality:       == !=
    //   5.  Comparison:     > < >= <=
    //   6.  Range:          .. ..<
    //   7.  Bitwise OR:     |
    //   8.  Bitwise XOR:    ^
    //   9.  Bitwise AND:    &
    //   10. Shift:          << >>
    //   11. Additive:       + -
    //   12. Multiplicative: * / %
    //   13. Power:          ** (right-assoc)
    //   14. Unary:          ! - ~
    //   15. Postfix:        calls, indexing
    //   16. Primary:        literals, identifiers, grouped expressions

    /// Parse an expression (entry point — lowest precedence).
    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_logical_or()
    }

    /// Parse logical OR: `expr ('||') expr`
    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_logical_xor()?;

        while matches!(self.peek_kind(), TokenKind::PipePipe) {
            let op_span = self.peek_span();
            self.advance();
            let right = self.parse_logical_xor()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator: BinaryOp::LogicalOr,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse logical XOR: `expr ('^^') expr`
    fn parse_logical_xor(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_logical_and()?;

        while matches!(self.peek_kind(), TokenKind::CaretCaret) {
            let op_span = self.peek_span();
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator: BinaryOp::LogicalXor,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse logical AND: `expr ('&&') expr`
    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_equality()?;

        while matches!(self.peek_kind(), TokenKind::AmpAmp) {
            let op_span = self.peek_span();
            self.advance();
            let right = self.parse_equality()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator: BinaryOp::LogicalAnd,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse equality: `expr ('==' | '!=') expr`
    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison()?;

        while matches!(
            self.peek_kind(),
            TokenKind::EqualEqual | TokenKind::BangEqual
        ) {
            let op_span = self.peek_span();
            let operator = match self.peek_kind() {
                TokenKind::EqualEqual => BinaryOp::Equal,
                TokenKind::BangEqual => BinaryOp::NotEqual,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse comparison: `expr ('>' | '<' | '>=' | '<=') expr`
    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_range()?;

        while matches!(
            self.peek_kind(),
            TokenKind::Greater | TokenKind::GreaterEqual | TokenKind::Less | TokenKind::LessEqual
        ) || (self.peek_kind() == &TokenKind::Identifier
            && (self.tokens[self.current].lexeme == "in"
                || self.tokens[self.current].lexeme == "not"))
        {
            // Handle "in" and "not in"
            if self.peek_kind() == &TokenKind::Identifier {
                let lexeme = self.tokens[self.current].lexeme.as_str();
                if lexeme == "in" {
                    let op_span = self.peek_span();
                    self.advance();
                    let right = self.parse_range()?;
                    left = Expression::Binary(BinaryExpr {
                        left: Box::new(left),
                        operator: BinaryOp::In,
                        right: Box::new(right),
                        span: op_span,
                    });
                    continue;
                } else if lexeme == "not" {
                    // Check for "not in"
                    if self.current + 1 < self.tokens.len()
                        && self.tokens[self.current + 1].kind == TokenKind::Identifier
                        && self.tokens[self.current + 1].lexeme == "in"
                    {
                        let op_span = self.peek_span();
                        self.advance(); // consume "not"
                        self.advance(); // consume "in"
                        let right = self.parse_range()?;
                        left = Expression::Binary(BinaryExpr {
                            left: Box::new(left),
                            operator: BinaryOp::NotIn,
                            right: Box::new(right),
                            span: op_span,
                        });
                        continue;
                    } else {
                        break; // "not" without "in" — not our operator
                    }
                }
            }

            let op_span = self.peek_span();
            let operator = match self.peek_kind() {
                TokenKind::Greater => BinaryOp::Greater,
                TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
                TokenKind::Less => BinaryOp::Less,
                TokenKind::LessEqual => BinaryOp::LessEqual,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_range()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse range expressions: `expr '..' expr` or `expr '..<' expr`
    fn parse_range(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_bitwise_or()?;

        if matches!(self.peek_kind(), TokenKind::DotDot | TokenKind::DotDotLess) {
            let span = self.peek_span();
            let inclusive = *self.peek_kind() == TokenKind::DotDot;
            self.advance();
            let right = self.parse_bitwise_or()?;
            Ok(Expression::Range(RangeExpr {
                start: Box::new(left),
                end: Box::new(right),
                inclusive,
                span,
            }))
        } else {
            Ok(left)
        }
    }

    /// Parse bitwise OR: `expr '|' expr`
    fn parse_bitwise_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_bitwise_xor()?;

        while matches!(self.peek_kind(), TokenKind::Pipe) {
            let op_span = self.peek_span();
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator: BinaryOp::BitwiseOr,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse bitwise XOR: `expr '^' expr`
    fn parse_bitwise_xor(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_bitwise_and()?;

        while matches!(self.peek_kind(), TokenKind::Caret) {
            let op_span = self.peek_span();
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator: BinaryOp::BitwiseXor,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse bitwise AND: `expr '&' expr`
    fn parse_bitwise_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_shift()?;

        while matches!(self.peek_kind(), TokenKind::Amp) {
            let op_span = self.peek_span();
            self.advance();
            let right = self.parse_shift()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator: BinaryOp::BitwiseAnd,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse shift expressions: `expr ('<<' | '>>') expr`
    fn parse_shift(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_additive()?;

        while matches!(
            self.peek_kind(),
            TokenKind::LessLess | TokenKind::GreaterGreater
        ) {
            let op_span = self.peek_span();
            let operator = match self.peek_kind() {
                TokenKind::LessLess => BinaryOp::ShiftLeft,
                TokenKind::GreaterGreater => BinaryOp::ShiftRight,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse additive expressions: `expr ('+' | '-') expr`
    fn parse_additive(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_multiplicative()?;

        while matches!(self.peek_kind(), TokenKind::Plus | TokenKind::Minus) {
            let op_span = self.peek_span();
            let operator = match self.peek_kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Subtract,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse multiplicative expressions: `expr ('*' | '/' | '%') expr`
    fn parse_multiplicative(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_power()?;

        while matches!(
            self.peek_kind(),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent
        ) {
            let op_span = self.peek_span();
            let operator = match self.peek_kind() {
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Slash => BinaryOp::Divide,
                TokenKind::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_power()?;
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                span: op_span,
            });
        }

        Ok(left)
    }

    /// Parse exponentiation: `expr '**' expr` (right-associative)
    fn parse_power(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_unary()?;

        if matches!(self.peek_kind(), TokenKind::StarStar) {
            let op_span = self.peek_span();
            self.advance();
            // Right-associative: recurse into parse_power, not parse_unary
            let right = self.parse_power()?;
            Ok(Expression::Binary(BinaryExpr {
                left: Box::new(left),
                operator: BinaryOp::Power,
                right: Box::new(right),
                span: op_span,
            }))
        } else {
            Ok(left)
        }
    }

    /// Parse unary expressions: `'!' expr` or `'-' expr` or `'~' expr`
    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if matches!(
            self.peek_kind(),
            TokenKind::Bang | TokenKind::Minus | TokenKind::Tilde
        ) {
            let span = self.peek_span();
            let operator = match self.peek_kind() {
                TokenKind::Bang => UnaryOp::Not,
                TokenKind::Minus => UnaryOp::Negate,
                TokenKind::Tilde => UnaryOp::BitwiseNot,
                _ => unreachable!(),
            };
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expression::Unary(UnaryExpr {
                operator,
                operand: Box::new(operand),
                span,
            }));
        }
        // Handle `await` as a unary-level expression
        if self.peek_kind() == &TokenKind::Identifier && self.tokens[self.current].lexeme == "await"
        {
            let span = self.peek_span();
            self.advance(); // consume "await"
            let task_expr = self.parse_unary()?;
            return Ok(Expression::Await(Box::new(crate::ast::AwaitExpr {
                task_expr,
                span,
            })));
        }
        self.parse_postfix()
    }

    /// Parse postfix operations: indexing `expr[index]` and calls `expr(args)`
    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.peek_kind() == &TokenKind::LeftBracket {
                // Only treat '[' as postfix indexing if it's on the same line
                // as the preceding token. This avoids ambiguity with
                // destructuring assignment `[a, b] = ...` on a new line.
                let prev_line = self.tokens[self.current - 1].span.line;
                let bracket_line = self.peek_span().line;
                if bracket_line != prev_line {
                    break;
                }
                let span = self.peek_span();
                self.advance(); // consume '['
                let index = self.parse_expression()?;
                self.expect(TokenKind::RightBracket, "expected ']' after index")?;
                expr = Expression::Index(IndexExpr {
                    object: Box::new(expr),
                    index: Box::new(index),
                    span,
                });
            } else if self.peek_kind() == &TokenKind::LeftParen {
                let span = self.peek_span();
                self.advance(); // consume '('
                let arguments = self.parse_arguments()?;
                self.expect(TokenKind::RightParen, "expected ')' after arguments")?;
                expr = Expression::Call(CallExpr {
                    callee: Box::new(expr),
                    arguments,
                    span,
                });
            } else if self.peek_kind() == &TokenKind::Dot {
                let span = self.peek_span();
                self.advance(); // consume '.'
                let member_token =
                    self.expect(TokenKind::Identifier, "expected member name after '.'")?;
                let member = member_token.lexeme.clone();
                // Member call requires '(' immediately
                if self.peek_kind() == &TokenKind::LeftParen {
                    self.advance(); // consume '('
                    let arguments = self.parse_arguments()?;
                    self.expect(TokenKind::RightParen, "expected ')' after arguments")?;
                    // Extract the object name from the expression
                    let object = match &expr {
                        Expression::Identifier(id) => id.name.clone(),
                        _ => {
                            return Err(ParseError {
                                message: "member access requires an identifier".to_string(),
                                span,
                            });
                        }
                    };
                    expr = Expression::MemberCall(MemberCallExpr {
                        object,
                        member,
                        arguments,
                        span,
                    });
                } else {
                    // Member access without call: module.variable
                    let object = match &expr {
                        Expression::Identifier(id) => id.name.clone(),
                        _ => {
                            return Err(ParseError {
                                message: "member access requires an identifier".to_string(),
                                span,
                            });
                        }
                    };
                    expr = Expression::MemberAccess(MemberAccessExpr {
                        object,
                        member,
                        span,
                    });
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parse a primary expression: literal, identifier, or `(` expression `)`.
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match self.peek_kind() {
            TokenKind::StringLiteral => {
                let token = self.consume();
                let value = strip_quotes(&token.lexeme);
                Ok(Expression::StringLiteral(StringLit {
                    value,
                    span: token.span.clone(),
                }))
            }
            TokenKind::IntegerLiteral => {
                let token = self.consume();
                let value = token.lexeme.parse::<i64>().map_err(|_| ParseError {
                    message: format!("invalid integer literal '{}'", token.lexeme),
                    span: token.span.clone(),
                })?;
                Ok(Expression::IntegerLiteral(IntegerLit {
                    value,
                    span: token.span.clone(),
                }))
            }
            TokenKind::FloatLiteral => {
                let token = self.consume();
                let value = token.lexeme.parse::<f64>().map_err(|_| ParseError {
                    message: format!("invalid float literal '{}'", token.lexeme),
                    span: token.span.clone(),
                })?;
                Ok(Expression::FloatLiteral(FloatLit {
                    value,
                    span: token.span.clone(),
                }))
            }
            TokenKind::DurationLiteral => {
                let token = self.consume();
                // Split lexeme into numeric part and unit suffix
                let pos = token
                    .lexeme
                    .find(|c: char| c.is_ascii_alphabetic())
                    .unwrap();
                let num_str = &token.lexeme[..pos];
                let unit = token.lexeme[pos..].to_string();
                let value = num_str.parse::<i64>().map_err(|_| ParseError {
                    message: format!("invalid duration literal '{}'", token.lexeme),
                    span: token.span.clone(),
                })?;
                Ok(Expression::DurationLiteral(DurationLit {
                    value,
                    unit,
                    span: token.span.clone(),
                }))
            }
            TokenKind::Identifier => {
                let lexeme = self.tokens[self.current].lexeme.as_str();
                match lexeme {
                    "true" => {
                        let token = self.consume();
                        Ok(Expression::BooleanLiteral(BooleanLit {
                            value: true,
                            span: token.span.clone(),
                        }))
                    }
                    "false" => {
                        let token = self.consume();
                        Ok(Expression::BooleanLiteral(BooleanLit {
                            value: false,
                            span: token.span.clone(),
                        }))
                    }
                    "nil" => {
                        let token = self.consume();
                        Ok(Expression::NilLiteral(token.span.clone()))
                    }
                    // Keywords that cannot appear as expressions
                    "let" | "if" | "else" | "while" | "return" | "import" | "for" | "in"
                    | "break" | "continue" | "until" | "wait" | "throw" | "try" | "catch"
                    | "finally" | "await" => Err(ParseError {
                        message: "expected expression".to_string(),
                        span: self.peek_span(),
                    }),
                    // Scheduling expressions: after/every/at return Task values
                    "after" | "at" | "spawn" => {
                        let token = self.consume();
                        let span = token.span.clone();
                        let keyword = token.lexeme.clone();
                        match keyword.as_str() {
                            "spawn" => {
                                let body = self.parse_block()?;
                                Ok(Expression::Spawn(Box::new(crate::ast::SpawnStatement {
                                    body,
                                    span,
                                })))
                            }
                            "after" => {
                                let expr = self.parse_expression()?;
                                let body = self.parse_block()?;
                                Ok(Expression::After(Box::new(crate::ast::AfterStatement {
                                    delay: expr,
                                    body,
                                    span,
                                })))
                            }
                            "at" => {
                                let expr = self.parse_expression()?;
                                let body = self.parse_block()?;
                                Ok(Expression::At(Box::new(crate::ast::AtStatement {
                                    target: expr,
                                    body,
                                    span,
                                })))
                            }
                            _ => unreachable!(),
                        }
                    }
                    "every" => {
                        let token = self.consume();
                        let span = token.span.clone();
                        // Try calendar form first
                        if let Some(stmt) = self.try_parse_every_calendar(span.clone())? {
                            match stmt {
                                Statement::EveryCalendar(ec) => {
                                    Ok(Expression::EveryCalendar(Box::new(ec)))
                                }
                                _ => unreachable!(),
                            }
                        } else {
                            let expr = self.parse_expression()?;
                            let body = self.parse_block()?;
                            Ok(Expression::Every(Box::new(crate::ast::EveryStatement {
                                interval: expr,
                                body,
                                span,
                            })))
                        }
                    }
                    // fn as expression: anonymous function
                    "fn" => {
                        let token = self.consume();
                        let span = token.span.clone();
                        self.expect(TokenKind::LeftParen, "expected '(' after 'fn'")?;
                        let params = self.parse_param_patterns()?;
                        self.expect(TokenKind::RightParen, "expected ')' after parameters")?;
                        let body = self.parse_block()?;
                        Ok(Expression::FunctionExpr(FunctionExprNode {
                            params,
                            body,
                            span,
                        }))
                    }
                    // Any other identifier — variable reference
                    // Calls and member access are handled by parse_postfix
                    _ => {
                        let token = self.consume();
                        Ok(Expression::Identifier(IdentifierExpr {
                            name: token.lexeme.clone(),
                            span: token.span.clone(),
                        }))
                    }
                }
            }
            TokenKind::LeftParen => {
                self.advance(); // consume `(`
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RightParen, "expected ')' after expression")?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                let span = self.peek_span();
                self.advance(); // consume '['
                let mut elements = Vec::new();
                if self.peek_kind() != &TokenKind::RightBracket {
                    elements.push(self.parse_expression()?);
                    while self.peek_kind() == &TokenKind::Comma {
                        self.advance(); // consume ','
                        elements.push(self.parse_expression()?);
                    }
                }
                self.expect(TokenKind::RightBracket, "expected ']' after array elements")?;
                Ok(Expression::Array(ArrayExpr { elements, span }))
            }
            TokenKind::LeftBrace => {
                // Map literal: { key: value, ... }
                let span = self.peek_span();
                self.advance(); // consume '{'
                let mut entries = Vec::new();
                if self.peek_kind() != &TokenKind::RightBrace {
                    let key = self.parse_expression()?;
                    self.expect(TokenKind::Colon, "expected ':' after map key")?;
                    let value = self.parse_expression()?;
                    entries.push((key, value));
                    while self.peek_kind() == &TokenKind::Comma {
                        self.advance(); // consume ','
                        // Allow trailing comma before }
                        if self.peek_kind() == &TokenKind::RightBrace {
                            break;
                        }
                        let key = self.parse_expression()?;
                        self.expect(TokenKind::Colon, "expected ':' after map key")?;
                        let value = self.parse_expression()?;
                        entries.push((key, value));
                    }
                }
                self.expect(TokenKind::RightBrace, "expected '}' after map entries")?;
                Ok(Expression::Map(MapExpr { entries, span }))
            }
            _ => Err(ParseError {
                message: "expected expression".to_string(),
                span: self.peek_span(),
            }),
        }
    }

    // --- Compound assignment helpers ---

    /// Check if the current token is a compound assignment operator.
    fn is_compound_assign_token(&self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::PlusEqual
                | TokenKind::MinusEqual
                | TokenKind::StarEqual
                | TokenKind::SlashEqual
                | TokenKind::PercentEqual
                | TokenKind::StarStarEqual
                | TokenKind::AmpEqual
                | TokenKind::PipeEqual
                | TokenKind::CaretEqual
                | TokenKind::LessLessEqual
                | TokenKind::GreaterGreaterEqual
        )
    }

    /// If the current token is a compound assignment, return the corresponding BinaryOp.
    fn try_compound_assign_op(&self) -> Option<BinaryOp> {
        match self.peek_kind() {
            TokenKind::PlusEqual => Some(BinaryOp::Add),
            TokenKind::MinusEqual => Some(BinaryOp::Subtract),
            TokenKind::StarEqual => Some(BinaryOp::Multiply),
            TokenKind::SlashEqual => Some(BinaryOp::Divide),
            TokenKind::PercentEqual => Some(BinaryOp::Modulo),
            TokenKind::StarStarEqual => Some(BinaryOp::Power),
            TokenKind::AmpEqual => Some(BinaryOp::BitwiseAnd),
            TokenKind::PipeEqual => Some(BinaryOp::BitwiseOr),
            TokenKind::CaretEqual => Some(BinaryOp::BitwiseXor),
            TokenKind::LessLessEqual => Some(BinaryOp::ShiftLeft),
            TokenKind::GreaterGreaterEqual => Some(BinaryOp::ShiftRight),
            _ => None,
        }
    }

    // --- Pattern parsing ---

    /// Parse a binding pattern: identifier, `_`, `[...]`, or `{...}`.
    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        match self.peek_kind() {
            TokenKind::LeftBracket => self.parse_array_pattern(),
            TokenKind::LeftBrace => self.parse_map_pattern(),
            TokenKind::Identifier => {
                let token = self.consume();
                let name = token.lexeme.clone();
                let span = token.span.clone();
                if name == "_" {
                    Ok(Pattern::Wildcard(span))
                } else if self.peek_kind() == &TokenKind::Colon {
                    self.advance(); // consume ':'
                    let type_ann = self.parse_type_annotation()?;
                    Ok(Pattern::TypedIdentifier(name, type_ann, span))
                } else {
                    Ok(Pattern::Identifier(name, span))
                }
            }
            _ => Err(ParseError {
                message: "expected pattern".to_string(),
                span: self.peek_span(),
            }),
        }
    }

    /// Parse an array pattern: `[pattern, pattern, ...]`
    fn parse_array_pattern(&mut self) -> Result<Pattern, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume '['
        let mut elements = Vec::new();
        if self.peek_kind() != &TokenKind::RightBracket {
            elements.push(self.parse_pattern()?);
            while self.peek_kind() == &TokenKind::Comma {
                self.advance(); // consume ','
                if self.peek_kind() == &TokenKind::RightBracket {
                    break; // trailing comma
                }
                elements.push(self.parse_pattern()?);
            }
        }
        self.expect(TokenKind::RightBracket, "expected ']' after array pattern")?;
        Ok(Pattern::Array(elements, span))
    }

    /// Parse a map pattern: `{"key": pattern, ...}`
    fn parse_map_pattern(&mut self) -> Result<Pattern, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume '{'
        let mut entries = Vec::new();
        if self.peek_kind() != &TokenKind::RightBrace {
            // Parse key (must be string literal)
            let key = self.parse_map_pattern_key()?;
            self.expect(TokenKind::Colon, "expected ':' after map pattern key")?;
            let pat = self.parse_pattern()?;
            entries.push((key, pat));
            while self.peek_kind() == &TokenKind::Comma {
                self.advance(); // consume ','
                if self.peek_kind() == &TokenKind::RightBrace {
                    break; // trailing comma
                }
                let key = self.parse_map_pattern_key()?;
                self.expect(TokenKind::Colon, "expected ':' after map pattern key")?;
                let pat = self.parse_pattern()?;
                entries.push((key, pat));
            }
        }
        self.expect(TokenKind::RightBrace, "expected '}' after map pattern")?;
        Ok(Pattern::Map(entries, span))
    }

    /// Parse a map pattern key — must be a string literal.
    fn parse_map_pattern_key(&mut self) -> Result<String, ParseError> {
        if self.peek_kind() == &TokenKind::StringLiteral {
            let token = self.consume();
            Ok(strip_quotes(&token.lexeme))
        } else {
            Err(ParseError {
                message: "map pattern key must be a string literal".to_string(),
                span: self.peek_span(),
            })
        }
    }

    /// Parse function parameter patterns (comma-separated patterns).
    fn parse_param_patterns(&mut self) -> Result<Vec<Pattern>, ParseError> {
        let mut params = Vec::new();
        if self.peek_kind() == &TokenKind::RightParen {
            return Ok(params);
        }
        loop {
            let pat = self.parse_pattern()?;
            params.push(pat);
            if self.peek_kind() != &TokenKind::Comma {
                break;
            }
            self.advance(); // consume ','
        }
        Ok(params)
    }

    // --- Utility methods ---

    /// Expect the current token to be of the given kind. If so, advance and
    /// return a reference to it. Otherwise, return an error.
    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<&Token, ParseError> {
        if self.peek_kind() == &kind {
            self.advance();
            Ok(&self.tokens[self.current - 1])
        } else {
            Err(ParseError {
                message: message.to_string(),
                span: self.peek_span(),
            })
        }
    }

    /// Consume the current token and return a reference to it.
    fn consume(&mut self) -> &Token {
        self.advance();
        &self.tokens[self.current - 1]
    }

    /// Advance past tokens until we reach a point that looks like the start
    /// of the next statement (an identifier) or EOF.
    fn synchronize(&mut self) {
        if self.peek_kind() == &TokenKind::Identifier {
            return;
        }

        while !self.is_at_end() {
            self.advance();
            if self.peek_kind() == &TokenKind::Identifier {
                return;
            }
        }
    }

    /// Get the kind of the current token.
    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.current].kind
    }

    /// Get the span of the current token.
    fn peek_span(&self) -> Span {
        self.tokens[self.current].span.clone()
    }

    /// Check if we've reached the EOF token.
    fn is_at_end(&self) -> bool {
        self.tokens[self.current].kind == TokenKind::Eof
    }

    /// Move to the next token.
    fn advance(&mut self) {
        if !self.is_at_end() {
            self.current += 1;
        }
    }
}

/// Strip the surrounding double quotes from a string literal lexeme.
fn strip_quotes(lexeme: &str) -> String {
    if lexeme.len() >= 2 && lexeme.starts_with('"') && lexeme.ends_with('"') {
        let raw = &lexeme[1..lexeme.len() - 1];
        let mut out = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some('0') => out.push('\0'),
                    Some(other) => {
                        // Shouldn't happen — lexer rejects unknown escapes
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(ch);
            }
        }
        out
    } else {
        lexeme.to_string()
    }
}

/// Collect all identifier names from a pattern into `names`.
fn collect_pattern_names(pattern: &Pattern, names: &mut Vec<(String, Span)>) {
    match pattern {
        Pattern::Identifier(name, span) => {
            names.push((name.clone(), span.clone()));
        }
        Pattern::TypedIdentifier(name, _, span) => {
            names.push((name.clone(), span.clone()));
        }
        Pattern::Wildcard(_) => {}
        Pattern::Array(patterns, _) => {
            for p in patterns {
                collect_pattern_names(p, names);
            }
        }
        Pattern::Map(entries, _) => {
            for (_, p) in entries {
                collect_pattern_names(p, names);
            }
        }
    }
}

/// Check for duplicate binding names in a pattern.
fn check_duplicate_bindings(pattern: &Pattern) -> Result<(), ParseError> {
    let mut names = Vec::new();
    collect_pattern_names(pattern, &mut names);
    let mut seen = std::collections::HashSet::new();
    for (name, span) in &names {
        if !seen.insert(name.clone()) {
            return Err(ParseError {
                message: format!("duplicate binding '{}' in pattern", name),
                span: span.clone(),
            });
        }
    }
    Ok(())
}

/// Check for duplicate binding names across multiple patterns (function params).
fn check_duplicate_bindings_across(patterns: &[Pattern]) -> Result<(), ParseError> {
    let mut names = Vec::new();
    for p in patterns {
        collect_pattern_names(p, &mut names);
    }
    let mut seen = std::collections::HashSet::new();
    for (name, span) in &names {
        if !seen.insert(name.clone()) {
            return Err(ParseError {
                message: format!("duplicate parameter name '{}'", name),
                span: span.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{AssignTarget, BinaryOp, LetStatement, UnaryOp};
    use crate::lexer::Lexer;

    // Helper: lex and parse, assert no errors
    fn parse_ok(source: &str) -> Program {
        let lex_result = Lexer::new(source).tokenize();
        assert!(
            lex_result.errors.is_empty(),
            "unexpected lexer errors: {:?}",
            lex_result.errors
        );
        let parse_result = Parser::new(lex_result.tokens).parse();
        assert!(
            parse_result.errors.is_empty(),
            "unexpected parse errors: {:?}",
            parse_result.errors
        );
        parse_result.program
    }

    // Helper: lex and parse, return both program and errors
    fn parse_with_errors(source: &str) -> (Program, Vec<ParseError>) {
        let lex_result = Lexer::new(source).tokenize();
        let parse_result = Parser::new(lex_result.tokens).parse();
        (parse_result.program, parse_result.errors)
    }

    // Helper: extract CallExpr from a Statement (expression-statement wrapping a call)
    fn as_call(stmt: &Statement) -> &CallExpr {
        match stmt {
            Statement::Expression(Expression::Call(call)) => call,
            _ => panic!("expected Expression(Call), got {:?}", stmt),
        }
    }

    // Helper: get the single argument of a call as an expression ref
    fn call_arg(call: &CallExpr) -> &Expression {
        assert_eq!(
            call.arguments.len(),
            1,
            "expected 1 argument, got {}",
            call.arguments.len()
        );
        &call.arguments[0]
    }

    // Helper: get the name from a call's callee (assumes Identifier callee)
    fn call_name(call: &CallExpr) -> &str {
        match call.callee.as_ref() {
            Expression::Identifier(id) => &id.name,
            _ => panic!("expected Identifier callee, got {:?}", call.callee),
        }
    }

    // Helper: extract LetStatement from a Statement
    fn as_let(stmt: &Statement) -> &LetStatement {
        match stmt {
            Statement::Let(let_stmt) => let_stmt,
            _ => panic!("expected Let, got {:?}", stmt),
        }
    }

    // Helper: extract the string value from an expression
    fn as_string(expr: &Expression) -> &str {
        match expr {
            Expression::StringLiteral(s) => &s.value,
            _ => panic!("expected StringLiteral, got {:?}", expr),
        }
    }

    // Helper: extract the integer value from an expression
    fn as_integer(expr: &Expression) -> i64 {
        match expr {
            Expression::IntegerLiteral(i) => i.value,
            _ => panic!("expected IntegerLiteral, got {:?}", expr),
        }
    }

    // --- Statement tests ---

    #[test]
    fn empty_program() {
        let program = parse_ok("");
        assert_eq!(program.statements.len(), 0);
    }

    #[test]
    fn single_print_string() {
        let program = parse_ok("print(\"Hello, Flux!\")");
        assert_eq!(program.statements.len(), 1);
        let call = as_call(&program.statements[0]);
        assert_eq!(call_name(call), "print");
        assert_eq!(as_string(call_arg(call)), "Hello, Flux!");
        assert_eq!(call.span, Span { line: 1, column: 1 });
    }

    #[test]
    fn multiple_statements() {
        let program = parse_ok("print(\"One\")\nprint(\"Two\")\nprint(\"Three\")");
        assert_eq!(program.statements.len(), 3);
        assert_eq!(
            as_string(&as_call(&program.statements[0]).arguments[0]),
            "One"
        );
        assert_eq!(
            as_string(&as_call(&program.statements[1]).arguments[0]),
            "Two"
        );
        assert_eq!(
            as_string(&as_call(&program.statements[2]).arguments[0]),
            "Three"
        );
    }

    #[test]
    fn empty_string_argument() {
        let program = parse_ok("print(\"\")");
        let call = as_call(&program.statements[0]);
        assert_eq!(as_string(call_arg(call)), "");
    }

    #[test]
    fn string_with_spaces() {
        let program = parse_ok("print(\"hello world\")");
        let call = as_call(&program.statements[0]);
        assert_eq!(as_string(call_arg(call)), "hello world");
    }

    #[test]
    fn string_with_parentheses() {
        let program = parse_ok("print(\"Hello (world)\")");
        let call = as_call(&program.statements[0]);
        assert_eq!(as_string(call_arg(call)), "Hello (world)");
    }

    // --- Expression tests ---

    #[test]
    fn parse_integer() {
        let program = parse_ok("print(42)");
        let call = as_call(&program.statements[0]);
        assert_eq!(as_integer(call_arg(call)), 42);
    }

    #[test]
    fn parse_float() {
        let program = parse_ok("print(3.14)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::FloatLiteral(f) => assert_eq!(f.value, 3.14),
            _ => panic!("expected FloatLiteral"),
        }
    }

    #[test]
    fn parse_true() {
        let program = parse_ok("print(true)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::BooleanLiteral(b) => assert!(b.value),
            _ => panic!("expected BooleanLiteral"),
        }
    }

    #[test]
    fn parse_false() {
        let program = parse_ok("print(false)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::BooleanLiteral(b) => assert!(!b.value),
            _ => panic!("expected BooleanLiteral"),
        }
    }

    #[test]
    fn parse_addition() {
        let program = parse_ok("print(10 + 20)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => {
                assert_eq!(as_integer(&bin.left), 10);
                assert_eq!(bin.operator, BinaryOp::Add);
                assert_eq!(as_integer(&bin.right), 20);
            }
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn parse_precedence_mul_over_add() {
        // 10 + 20 * 3 should parse as 10 + (20 * 3)
        let program = parse_ok("print(10 + 20 * 3)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => {
                assert_eq!(bin.operator, BinaryOp::Add);
                assert_eq!(as_integer(&bin.left), 10);
                // Right should be 20 * 3
                match bin.right.as_ref() {
                    Expression::Binary(right_bin) => {
                        assert_eq!(right_bin.operator, BinaryOp::Multiply);
                        assert_eq!(as_integer(&right_bin.left), 20);
                        assert_eq!(as_integer(&right_bin.right), 3);
                    }
                    _ => panic!("expected nested Binary"),
                }
            }
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn parse_grouped_expression() {
        // (10 + 20) * 3 should parse as (10 + 20) * 3
        let program = parse_ok("print((10 + 20) * 3)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => {
                assert_eq!(bin.operator, BinaryOp::Multiply);
                // Left should be 10 + 20
                match bin.left.as_ref() {
                    Expression::Binary(left_bin) => {
                        assert_eq!(left_bin.operator, BinaryOp::Add);
                        assert_eq!(as_integer(&left_bin.left), 10);
                        assert_eq!(as_integer(&left_bin.right), 20);
                    }
                    _ => panic!("expected nested Binary"),
                }
                assert_eq!(as_integer(&bin.right), 3);
            }
            _ => panic!("expected Binary"),
        }
    }

    // --- Error tests ---

    #[test]
    fn missing_left_paren() {
        let (_, errors) = parse_with_errors("print \"hello\")");
        assert!(!errors.is_empty());
    }

    #[test]
    fn missing_argument() {
        // print() is now valid — zero-arg call
        let program = parse_ok("print()");
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn missing_right_paren() {
        let (_, errors) = parse_with_errors("print(\"hello\"");
        assert!(!errors.is_empty());
    }

    #[test]
    fn missing_identifier_at_start() {
        let (_, errors) = parse_with_errors("(\"hello\")");
        assert!(!errors.is_empty());
    }

    #[test]
    fn malformed_then_valid_statement() {
        let source = "print(\"Good\")\nprint(\nprint(\"Also good\")";
        let (program, errors) = parse_with_errors(source);
        assert!(!errors.is_empty());
        // First statement should have parsed successfully
        assert!(program.statements.len() >= 1);
        assert_eq!(
            as_string(&as_call(&program.statements[0]).arguments[0]),
            "Good"
        );
    }

    #[test]
    fn eof_after_identifier() {
        let (_, errors) = parse_with_errors("print");
        assert!(!errors.is_empty());
    }

    #[test]
    fn eof_after_left_paren() {
        let (_, errors) = parse_with_errors("print(");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn preserves_span_on_call() {
        let program = parse_ok("  print(\"hi\")");
        let call = as_call(&program.statements[0]);
        assert_eq!(call.span, Span { line: 1, column: 3 });
    }

    #[test]
    fn different_function_name() {
        let program = parse_ok("say(\"hello\")");
        let call = as_call(&program.statements[0]);
        assert_eq!(call_name(call), "say");
        assert_eq!(as_string(call_arg(call)), "hello");
    }

    #[test]
    fn multiple_errors_multiple_statements() {
        let source = "bad(\nprint(\"ok\")\nalso_bad(";
        let (_program, errors) = parse_with_errors(source);
        assert!(!errors.is_empty());
    }

    // --- Let statement tests ---

    #[test]
    fn parse_let_integer() {
        let program = parse_ok("let x = 10");
        assert_eq!(program.statements.len(), 1);
        let let_stmt = as_let(&program.statements[0]);
        assert!(matches!(&let_stmt.pattern, Pattern::Identifier(n, _) if n == "x"));
        assert_eq!(as_integer(&let_stmt.initializer), 10);
    }

    #[test]
    fn parse_let_string() {
        let program = parse_ok("let message = \"Hello\"");
        let let_stmt = as_let(&program.statements[0]);
        assert!(matches!(&let_stmt.pattern, Pattern::Identifier(n, _) if n == "message"));
        assert_eq!(as_string(&let_stmt.initializer), "Hello");
    }

    #[test]
    fn parse_let_expression() {
        let program = parse_ok("let x = 10 + 20 * 3");
        let let_stmt = as_let(&program.statements[0]);
        assert!(matches!(&let_stmt.pattern, Pattern::Identifier(n, _) if n == "x"));
        match &let_stmt.initializer {
            Expression::Binary(bin) => assert_eq!(bin.operator, BinaryOp::Add),
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn parse_let_then_print() {
        let program = parse_ok("let x = 10\nprint(x)");
        assert_eq!(program.statements.len(), 2);
        assert!(matches!(&program.statements[0], Statement::Let(_)));
        assert!(matches!(
            &program.statements[1],
            Statement::Expression(Expression::Call(_))
        ));
    }

    #[test]
    fn parse_identifier_in_expression() {
        let program = parse_ok("print(x)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Identifier(id) => assert_eq!(id.name, "x"),
            _ => panic!("expected Identifier"),
        }
    }

    #[test]
    fn parse_identifier_in_arithmetic() {
        let program = parse_ok("print(x + 5)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => {
                match bin.left.as_ref() {
                    Expression::Identifier(id) => assert_eq!(id.name, "x"),
                    _ => panic!("expected Identifier on left"),
                }
                assert_eq!(as_integer(&bin.right), 5);
            }
            _ => panic!("expected Binary"),
        }
    }

    // --- Let error tests ---

    #[test]
    fn let_missing_name() {
        let (_, errors) = parse_with_errors("let = 10");
        assert!(!errors.is_empty());
    }

    #[test]
    fn let_missing_equals() {
        let (_, errors) = parse_with_errors("let x 10");
        assert!(!errors.is_empty());
    }

    #[test]
    fn let_missing_initializer() {
        let (_, errors) = parse_with_errors("let x =");
        assert!(!errors.is_empty());
    }

    #[test]
    fn let_only_keyword() {
        let (_, errors) = parse_with_errors("let");
        assert!(!errors.is_empty());
    }

    #[test]
    fn malformed_let_then_valid_print() {
        let (program, errors) = parse_with_errors("let = 10\nprint(\"ok\")");
        assert!(!errors.is_empty());
        assert!(program.statements.len() >= 1);
    }

    // --- Comparison and logical operator parser tests ---

    #[test]
    fn parse_greater_than() {
        let program = parse_ok("print(10 > 5)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => {
                assert_eq!(bin.operator, BinaryOp::Greater);
                assert_eq!(as_integer(&bin.left), 10);
                assert_eq!(as_integer(&bin.right), 5);
            }
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn parse_equal_equal() {
        let program = parse_ok("print(10 == 10)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => assert_eq!(bin.operator, BinaryOp::Equal),
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn parse_logical_and() {
        let program = parse_ok("print(true && false)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => assert_eq!(bin.operator, BinaryOp::LogicalAnd),
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn parse_not() {
        let program = parse_ok("print(!true)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Unary(un) => {
                assert_eq!(un.operator, UnaryOp::Not);
                match un.operand.as_ref() {
                    Expression::BooleanLiteral(b) => assert!(b.value),
                    _ => panic!("expected BooleanLiteral"),
                }
            }
            _ => panic!("expected Unary"),
        }
    }

    #[test]
    fn parse_precedence_add_over_comparison() {
        // 2 + 3 > 4 should parse as (2 + 3) > 4
        let program = parse_ok("print(2 + 3 > 4)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => {
                assert_eq!(bin.operator, BinaryOp::Greater);
                match bin.left.as_ref() {
                    Expression::Binary(left_bin) => assert_eq!(left_bin.operator, BinaryOp::Add),
                    _ => panic!("expected Binary on left"),
                }
                assert_eq!(as_integer(&bin.right), 4);
            }
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn parse_precedence_and_over_or() {
        // true || false && false should parse as true || (false && false)
        let program = parse_ok("print(true || false && false)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => {
                assert_eq!(bin.operator, BinaryOp::LogicalOr);
                match bin.right.as_ref() {
                    Expression::Binary(right_bin) => {
                        assert_eq!(right_bin.operator, BinaryOp::LogicalAnd);
                    }
                    _ => panic!("expected Binary on right"),
                }
            }
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn parse_precedence_not_over_and() {
        // !false && true should parse as (!false) && true
        let program = parse_ok("print(!false && true)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => {
                assert_eq!(bin.operator, BinaryOp::LogicalAnd);
                match bin.left.as_ref() {
                    Expression::Unary(un) => assert_eq!(un.operator, UnaryOp::Not),
                    _ => panic!("expected Unary on left"),
                }
            }
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn parse_complex_precedence() {
        // 10 + 2 * 3 >= 15 || false should parse as ((10 + (2 * 3)) >= 15) || false
        let program = parse_ok("print(10 + 2 * 3 >= 15 || false)");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Binary(bin) => {
                assert_eq!(bin.operator, BinaryOp::LogicalOr);
                match bin.left.as_ref() {
                    Expression::Binary(left_bin) => {
                        assert_eq!(left_bin.operator, BinaryOp::GreaterEqual);
                    }
                    _ => panic!("expected Binary on left"),
                }
            }
            _ => panic!("expected Binary"),
        }
    }

    // --- If/else parser tests ---

    #[test]
    fn parse_simple_if() {
        let program = parse_ok("if true {\n    print(\"yes\")\n}");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::If(if_stmt) => {
                assert!(if_stmt.else_branch.is_none());
                assert_eq!(if_stmt.then_branch.statements.len(), 1);
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_if_else() {
        let program = parse_ok("if true {\n    print(\"yes\")\n} else {\n    print(\"no\")\n}");
        match &program.statements[0] {
            Statement::If(if_stmt) => {
                assert!(if_stmt.else_branch.is_some());
                assert_eq!(if_stmt.then_branch.statements.len(), 1);
                assert_eq!(if_stmt.else_branch.as_ref().unwrap().statements.len(), 1);
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_empty_block() {
        let program = parse_ok("if true {\n}");
        match &program.statements[0] {
            Statement::If(if_stmt) => {
                assert_eq!(if_stmt.then_branch.statements.len(), 0);
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_multiple_statements_in_block() {
        let program = parse_ok("if true {\n    print(\"one\")\n    print(\"two\")\n}");
        match &program.statements[0] {
            Statement::If(if_stmt) => {
                assert_eq!(if_stmt.then_branch.statements.len(), 2);
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_if_with_expression_condition() {
        let program = parse_ok("if 10 + 20 > 25 {\n    print(\"yes\")\n}");
        match &program.statements[0] {
            Statement::If(if_stmt) => match &if_stmt.condition {
                Expression::Binary(bin) => assert_eq!(bin.operator, BinaryOp::Greater),
                _ => panic!("expected Binary condition"),
            },
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_nested_if() {
        let source = "if true {\n    if false {\n        print(\"inner\")\n    }\n}";
        let program = parse_ok(source);
        match &program.statements[0] {
            Statement::If(outer) => {
                assert_eq!(outer.then_branch.statements.len(), 1);
                match &outer.then_branch.statements[0] {
                    Statement::If(inner) => {
                        assert_eq!(inner.then_branch.statements.len(), 1);
                    }
                    _ => panic!("expected nested If"),
                }
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_if_missing_brace() {
        let (_, errors) = parse_with_errors("if true\n    print(\"hello\")\n}");
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_if_missing_closing_brace() {
        let (_, errors) = parse_with_errors("if true {\n    print(\"hello\")");
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_if_missing_condition() {
        let (_, errors) = parse_with_errors("if {\n    print(\"hello\")\n}");
        assert!(!errors.is_empty());
    }

    // --- Assignment parser tests ---

    #[test]
    fn parse_assignment() {
        let program = parse_ok("let x = 10\nx = 20");
        assert_eq!(program.statements.len(), 2);
        match &program.statements[1] {
            Statement::Assignment(assign) => {
                match &assign.target {
                    AssignTarget::Variable(name) => assert_eq!(name, "x"),
                    _ => panic!("expected Variable target"),
                }
                assert_eq!(as_integer(&assign.value), 20);
            }
            _ => panic!("expected Assignment"),
        }
    }

    #[test]
    fn parse_assignment_expression() {
        let program = parse_ok("let x = 0\nx = 10 + 20 * 3");
        match &program.statements[1] {
            Statement::Assignment(assign) => match &assign.value {
                Expression::Binary(bin) => assert_eq!(bin.operator, BinaryOp::Add),
                _ => panic!("expected Binary"),
            },
            _ => panic!("expected Assignment"),
        }
    }

    #[test]
    fn parse_malformed_assignment() {
        let (_, errors) = parse_with_errors("x =");
        assert!(!errors.is_empty());
    }

    // --- While parser tests ---

    #[test]
    fn parse_while() {
        let program = parse_ok("while true {\n    print(\"yes\")\n}");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::While(w) => {
                assert_eq!(w.body.statements.len(), 1);
            }
            _ => panic!("expected While"),
        }
    }

    #[test]
    fn parse_while_empty_body() {
        let program = parse_ok("while false {\n}");
        match &program.statements[0] {
            Statement::While(w) => {
                assert_eq!(w.body.statements.len(), 0);
            }
            _ => panic!("expected While"),
        }
    }

    #[test]
    fn parse_while_with_condition() {
        let program = parse_ok("while x < 10 {\n    print(x)\n}");
        match &program.statements[0] {
            Statement::While(w) => match &w.condition {
                Expression::Binary(bin) => assert_eq!(bin.operator, BinaryOp::Less),
                _ => panic!("expected Binary condition"),
            },
            _ => panic!("expected While"),
        }
    }

    #[test]
    fn parse_while_missing_brace() {
        let (_, errors) = parse_with_errors("while true\n    print(\"hello\")\n}");
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_while_missing_closing_brace() {
        let (_, errors) = parse_with_errors("while true {\n    print(\"hello\")");
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_nested_while() {
        let program = parse_ok("while true {\n    while false {\n    }\n}");
        match &program.statements[0] {
            Statement::While(w) => {
                assert_eq!(w.body.statements.len(), 1);
                match &w.body.statements[0] {
                    Statement::While(inner) => {
                        assert_eq!(inner.body.statements.len(), 0);
                    }
                    _ => panic!("expected inner While"),
                }
            }
            _ => panic!("expected While"),
        }
    }

    // --- Function parser tests ---

    #[test]
    fn parse_zero_arg_function() {
        let program = parse_ok("fn hello() {\n    print(\"hello\")\n}");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Function(func) => {
                assert_eq!(func.name, "hello");
                assert_eq!(func.params.len(), 0);
                assert_eq!(func.body.statements.len(), 1);
            }
            _ => panic!("expected Function"),
        }
    }

    #[test]
    fn parse_function_with_params() {
        let program = parse_ok("fn add(a, b) {\n    return a + b\n}");
        match &program.statements[0] {
            Statement::Function(func) => {
                assert_eq!(func.name, "add");
                assert_eq!(func.params.len(), 2);
                assert!(matches!(&func.params[0], Pattern::Identifier(n, _) if n == "a"));
                assert!(matches!(&func.params[1], Pattern::Identifier(n, _) if n == "b"));
            }
            _ => panic!("expected Function"),
        }
    }

    #[test]
    fn parse_call_expression() {
        let program = parse_ok("let x = add(10, 20)");
        match &program.statements[0] {
            Statement::Let(let_stmt) => match &let_stmt.initializer {
                Expression::Call(call) => {
                    assert_eq!(call_name(call), "add");
                    assert_eq!(call.arguments.len(), 2);
                }
                _ => panic!("expected Call expression"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_return_with_value() {
        let program = parse_ok("fn test() {\n    return 42\n}");
        match &program.statements[0] {
            Statement::Function(func) => match &func.body.statements[0] {
                Statement::Return(ret) => assert!(ret.value.is_some()),
                _ => panic!("expected Return"),
            },
            _ => panic!("expected Function"),
        }
    }

    #[test]
    fn parse_bare_return() {
        let program = parse_ok("fn test() {\n    return\n}");
        match &program.statements[0] {
            Statement::Function(func) => match &func.body.statements[0] {
                Statement::Return(ret) => assert!(ret.value.is_none()),
                _ => panic!("expected Return"),
            },
            _ => panic!("expected Function"),
        }
    }

    #[test]
    fn parse_duplicate_params() {
        let (_, errors) = parse_with_errors("fn bad(a, a) {\n}");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate parameter"));
    }

    #[test]
    fn parse_nested_call() {
        let program = parse_ok("print(add(2, 3))");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Call(inner) => {
                assert_eq!(call_name(inner), "add");
                assert_eq!(inner.arguments.len(), 2);
            }
            _ => panic!("expected nested Call"),
        }
    }

    // --- Array parser tests ---

    #[test]
    fn parse_empty_array() {
        let program = parse_ok("let x = []");
        match &program.statements[0] {
            Statement::Let(l) => match &l.initializer {
                Expression::Array(arr) => assert_eq!(arr.elements.len(), 0),
                _ => panic!("expected Array"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_array_literal() {
        let program = parse_ok("let x = [1, 2, 3]");
        match &program.statements[0] {
            Statement::Let(l) => match &l.initializer {
                Expression::Array(arr) => assert_eq!(arr.elements.len(), 3),
                _ => panic!("expected Array"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_index_expression() {
        let program = parse_ok("print(x[0])");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Index(idx) => {
                match idx.object.as_ref() {
                    Expression::Identifier(id) => assert_eq!(id.name, "x"),
                    _ => panic!("expected Identifier"),
                }
                match idx.index.as_ref() {
                    Expression::IntegerLiteral(i) => assert_eq!(i.value, 0),
                    _ => panic!("expected IntegerLiteral"),
                }
            }
            _ => panic!("expected Index"),
        }
    }

    #[test]
    fn parse_nested_index() {
        let program = parse_ok("print(x[0][1])");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Index(outer) => {
                match outer.object.as_ref() {
                    Expression::Index(_) => {} // nested index — correct
                    _ => panic!("expected nested Index"),
                }
            }
            _ => panic!("expected Index"),
        }
    }

    #[test]
    fn parse_inline_array_index() {
        let program = parse_ok("print([10, 20, 30][1])");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Index(idx) => match idx.object.as_ref() {
                Expression::Array(arr) => assert_eq!(arr.elements.len(), 3),
                _ => panic!("expected Array"),
            },
            _ => panic!("expected Index"),
        }
    }

    // --- Stage 13: Indexed assignment parser tests ---

    #[test]
    fn parse_indexed_assignment() {
        let program = parse_ok("let a = [1, 2]\na[0] = 99");
        assert_eq!(program.statements.len(), 2);
        match &program.statements[1] {
            Statement::Assignment(assign) => match &assign.target {
                AssignTarget::Index { index, .. } => match index {
                    Expression::IntegerLiteral(i) => assert_eq!(i.value, 0),
                    _ => panic!("expected IntegerLiteral index"),
                },
                _ => panic!("expected Index target"),
            },
            _ => panic!("expected Assignment"),
        }
    }

    #[test]
    fn parse_indexed_assignment_computed() {
        let program = parse_ok("let a = [1]\na[1 + 1] = 10 + 20");
        match &program.statements[1] {
            Statement::Assignment(assign) => match &assign.target {
                AssignTarget::Index { index, .. } => match index {
                    Expression::Binary(_) => {}
                    _ => panic!("expected Binary index"),
                },
                _ => panic!("expected Index target"),
            },
            _ => panic!("expected Assignment"),
        }
    }

    // --- Nested indexed assignment parser tests ---

    #[test]
    fn parse_nested_index_assignment() {
        let program = parse_ok("let m = [[1, 2]]\nm[0][1] = 99");
        match &program.statements[1] {
            Statement::Assignment(assign) => match &assign.target {
                AssignTarget::Index { object, .. } => match object.as_ref() {
                    AssignTarget::Index { object: inner, .. } => match inner.as_ref() {
                        AssignTarget::Variable(name) => assert_eq!(name, "m"),
                        _ => panic!("expected Variable at root"),
                    },
                    _ => panic!("expected nested Index"),
                },
                _ => panic!("expected Index target"),
            },
            _ => panic!("expected Assignment"),
        }
    }

    #[test]
    fn parse_triple_nested_index_assignment() {
        let program = parse_ok("let a = [[[1]]]\na[0][0][0] = 99");
        match &program.statements[1] {
            Statement::Assignment(assign) => {
                // Should be Index(Index(Index(Variable("a"))))
                let mut depth = 0;
                let mut t = &assign.target;
                while let AssignTarget::Index { object, .. } = t {
                    depth += 1;
                    t = object.as_ref();
                }
                assert_eq!(depth, 3);
                match t {
                    AssignTarget::Variable(name) => assert_eq!(name, "a"),
                    _ => panic!("expected Variable at root"),
                }
            }
            _ => panic!("expected Assignment"),
        }
    }

    // --- Map parser tests ---

    #[test]
    fn parse_empty_map() {
        let program = parse_ok("let m = {}");
        match &program.statements[0] {
            Statement::Let(l) => match &l.initializer {
                Expression::Map(map) => assert_eq!(map.entries.len(), 0),
                _ => panic!("expected Map"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_map_literal() {
        let program = parse_ok("let m = {\"a\": 1, \"b\": 2}");
        match &program.statements[0] {
            Statement::Let(l) => match &l.initializer {
                Expression::Map(map) => assert_eq!(map.entries.len(), 2),
                _ => panic!("expected Map"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_map_access() {
        let program = parse_ok("print(m[\"key\"])");
        let call = as_call(&program.statements[0]);
        match call_arg(call) {
            Expression::Index(_) => {}
            _ => panic!("expected Index"),
        }
    }

    #[test]
    fn parse_map_string_assignment() {
        let program = parse_ok("m[\"key\"] = 42");
        match &program.statements[0] {
            Statement::Assignment(assign) => match &assign.target {
                AssignTarget::Index { .. } => {}
                _ => panic!("expected Index target"),
            },
            _ => panic!("expected Assignment"),
        }
    }

    // --- Module parser tests ---

    #[test]
    fn parse_import() {
        let program = parse_ok("import math");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Import(imp) => assert_eq!(imp.module_path, vec!["math"]),
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_import_dotted() {
        let program = parse_ok("import utils.math");
        match &program.statements[0] {
            Statement::Import(imp) => {
                assert_eq!(imp.module_path, vec!["utils", "math"]);
                assert!(imp.alias.is_none());
            }
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_import_alias() {
        let program = parse_ok("import math as m");
        match &program.statements[0] {
            Statement::Import(imp) => {
                assert_eq!(imp.module_path, vec!["math"]);
                assert_eq!(imp.alias, Some("m".to_string()));
            }
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_from_import() {
        let program = parse_ok("from math import square");
        match &program.statements[0] {
            Statement::Import(imp) => {
                assert_eq!(imp.module_path, vec!["math"]);
                assert_eq!(imp.selective.len(), 1);
                assert_eq!(imp.selective[0].name, "square");
                assert!(imp.selective[0].alias.is_none());
            }
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_from_import_multiple() {
        let program = parse_ok("from math import square, cube");
        match &program.statements[0] {
            Statement::Import(imp) => {
                assert_eq!(imp.selective.len(), 2);
                assert_eq!(imp.selective[0].name, "square");
                assert_eq!(imp.selective[1].name, "cube");
            }
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_from_import_alias() {
        let program = parse_ok("from math import square as sq");
        match &program.statements[0] {
            Statement::Import(imp) => {
                assert_eq!(imp.selective[0].name, "square");
                assert_eq!(imp.selective[0].alias, Some("sq".to_string()));
            }
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_member_call_statement() {
        let program = parse_ok("math.add(1, 2)");
        match &program.statements[0] {
            Statement::Expression(Expression::MemberCall(mc)) => {
                assert_eq!(mc.object, "math");
                assert_eq!(mc.member, "add");
                assert_eq!(mc.arguments.len(), 2);
            }
            _ => panic!("expected MemberCall expression statement"),
        }
    }

    #[test]
    fn parse_member_call_in_expression() {
        let program = parse_ok("let x = math.add(1, 2)");
        match &program.statements[0] {
            Statement::Let(l) => match &l.initializer {
                Expression::MemberCall(mc) => {
                    assert_eq!(mc.object, "math");
                    assert_eq!(mc.member, "add");
                }
                _ => panic!("expected MemberCall"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_import_missing_name() {
        let (_, errors) = parse_with_errors("import");
        assert!(!errors.is_empty());
    }

    // === Stage 21: Destructuring Pattern Tests ===

    #[test]
    fn parse_let_array_pattern() {
        let program = parse_ok("let [a, b] = [1, 2]");
        match &program.statements[0] {
            Statement::Let(let_stmt) => match &let_stmt.pattern {
                Pattern::Array(pats, _) => {
                    assert_eq!(pats.len(), 2);
                    assert!(matches!(&pats[0], Pattern::Identifier(n, _) if n == "a"));
                    assert!(matches!(&pats[1], Pattern::Identifier(n, _) if n == "b"));
                }
                _ => panic!("expected Array pattern"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_let_map_pattern() {
        let program = parse_ok("let {\"name\": name} = x");
        match &program.statements[0] {
            Statement::Let(let_stmt) => match &let_stmt.pattern {
                Pattern::Map(entries, _) => {
                    assert_eq!(entries.len(), 1);
                    assert_eq!(entries[0].0, "name");
                    assert!(matches!(&entries[0].1, Pattern::Identifier(n, _) if n == "name"));
                }
                _ => panic!("expected Map pattern"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_nested_array_pattern() {
        let program = parse_ok("let [a, [b, c]] = x");
        match &program.statements[0] {
            Statement::Let(let_stmt) => match &let_stmt.pattern {
                Pattern::Array(pats, _) => {
                    assert_eq!(pats.len(), 2);
                    assert!(matches!(&pats[0], Pattern::Identifier(n, _) if n == "a"));
                    match &pats[1] {
                        Pattern::Array(inner, _) => {
                            assert_eq!(inner.len(), 2);
                        }
                        _ => panic!("expected nested Array pattern"),
                    }
                }
                _ => panic!("expected Array pattern"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_wildcard_pattern() {
        let program = parse_ok("let [_, x] = [1, 2]");
        match &program.statements[0] {
            Statement::Let(let_stmt) => match &let_stmt.pattern {
                Pattern::Array(pats, _) => {
                    assert!(matches!(&pats[0], Pattern::Wildcard(_)));
                    assert!(matches!(&pats[1], Pattern::Identifier(n, _) if n == "x"));
                }
                _ => panic!("expected Array pattern"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_destr_assignment() {
        let program = parse_ok("[a, b] = [1, 2]");
        match &program.statements[0] {
            Statement::Assignment(assign) => match &assign.target {
                AssignTarget::Pattern(Pattern::Array(pats, _)) => {
                    assert_eq!(pats.len(), 2);
                }
                _ => panic!("expected Pattern target"),
            },
            _ => panic!("expected Assignment"),
        }
    }

    #[test]
    fn parse_map_destr_assignment() {
        let program = parse_ok("{\"x\": x} = val");
        match &program.statements[0] {
            Statement::Assignment(assign) => match &assign.target {
                AssignTarget::Pattern(Pattern::Map(entries, _)) => {
                    assert_eq!(entries.len(), 1);
                }
                _ => panic!("expected Pattern target"),
            },
            _ => panic!("expected Assignment"),
        }
    }

    #[test]
    fn parse_fn_with_pattern_param() {
        let program = parse_ok("fn f([a, b]) { print(a) }");
        match &program.statements[0] {
            Statement::Function(func) => {
                assert_eq!(func.params.len(), 1);
                assert!(matches!(&func.params[0], Pattern::Array(_, _)));
            }
            _ => panic!("expected Function"),
        }
    }

    #[test]
    fn parse_fn_with_map_param() {
        let program = parse_ok("fn f({\"x\": x}) { print(x) }");
        match &program.statements[0] {
            Statement::Function(func) => {
                assert_eq!(func.params.len(), 1);
                assert!(matches!(&func.params[0], Pattern::Map(_, _)));
            }
            _ => panic!("expected Function"),
        }
    }

    #[test]
    fn parse_for_with_pattern() {
        let program = parse_ok("for [a, b] in values { print(a) }");
        match &program.statements[0] {
            Statement::For(for_stmt) => {
                assert!(matches!(&for_stmt.pattern, Pattern::Array(pats, _) if pats.len() == 2));
            }
            _ => panic!("expected For"),
        }
    }

    #[test]
    fn parse_duplicate_binding_error() {
        let (_, errors) = parse_with_errors("let [x, x] = [1, 2]");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate"));
    }

    #[test]
    fn parse_duplicate_map_binding_error() {
        let (_, errors) = parse_with_errors("let {\"a\": x, \"b\": x} = data");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate"));
    }

    #[test]
    fn parse_duplicate_param_error() {
        let (_, errors) = parse_with_errors("fn f(x, x) {}");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate"));
    }

    #[test]
    fn parse_nested_duplicate_binding_error() {
        let (_, errors) = parse_with_errors("let [a, [a, b]] = [1, [2, 3]]");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate"));
    }
}
