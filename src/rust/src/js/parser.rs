use std::io::{self, Write};

#[derive(Debug, Clone)]
pub enum JsNode {
    Program(Vec<JsNode>),
    VariableDeclaration { kind: String, name: String, init: Option<Box<JsNode>> },
    FunctionDeclaration { name: String, params: Vec<String>, body: Vec<JsNode> },
    ExpressionStatement(Box<JsNode>),
    CallExpression { callee: Box<JsNode>, arguments: Vec<JsNode> },
    Identifier(String),
    Literal(LiteralValue),
    BinaryExpression { operator: String, left: Box<JsNode>, right: Box<JsNode> },
    BlockStatement(Vec<JsNode>),
    ReturnStatement(Option<Box<JsNode>>),
    IfStatement { test: Box<JsNode>, consequent: Vec<JsNode>, alternate: Option<Vec<JsNode>> },
    ForStatement { init: Option<Box<JsNode>>, test: Option<Box<JsNode>>, update: Option<Box<JsNode>>, body: Vec<JsNode> },
    WhileStatement { test: Box<JsNode>, body: Vec<JsNode> },
    AssignmentExpression { operator: String, left: Box<JsNode>, right: Box<JsNode> },
    ArrayExpression(Vec<JsNode>),
    ObjectExpression(Vec<(String, JsNode)>),
    MemberExpression { object: Box<JsNode>, property: String },
    UnaryExpression { operator: String, argument: Box<JsNode> },
    LogicalExpression { operator: String, left: Box<JsNode>, right: Box<JsNode> },
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    String(String),
    Number(String),
    Boolean(bool),
    Null,
}

pub struct JsParser {
    pub unsupported_patterns: Vec<String>,
}

impl JsParser {
    pub fn new() -> Self {
        JsParser {
            unsupported_patterns: Vec::new(),
        }
    }

    pub fn parse(&mut self, source: &str) -> Result<JsNode, String> {
        self.unsupported_patterns.clear();
        let tokens = self.tokenize(source)?;
        self.parse_program(&tokens)
    }

    fn tokenize(&mut self, source: &str) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = source.chars().collect();
        let mut pos = 0;

        while pos < chars.len() {
            if chars[pos].is_whitespace() {
                pos += 1;
                continue;
            }

            
            if pos + 1 < chars.len() && chars[pos] == '/' && chars[pos + 1] == '/' {
                while pos < chars.len() && chars[pos] != '\n' {
                    pos += 1;
                }
                continue;
            }
            if pos + 1 < chars.len() && chars[pos] == '/' && chars[pos + 1] == '*' {
                pos += 2;
                while pos + 1 < chars.len() && !(chars[pos] == '*' && chars[pos + 1] == '/') {
                    pos += 1;
                }
                pos += 2;
                continue;
            }

            
            if chars[pos] == '"' || chars[pos] == '\'' {
                tokens.push(self.parse_string(&chars, &mut pos)?);
                continue;
            }

            
            if chars[pos].is_ascii_digit() || (chars[pos] == '.' && pos + 1 < chars.len() && chars[pos + 1].is_ascii_digit()) {
                tokens.push(self.parse_number(&chars, &mut pos));
                continue;
            }

            
            if chars[pos].is_alphabetic() || chars[pos] == '_' || chars[pos] == '$' {
                tokens.push(self.parse_identifier(&chars, &mut pos));
                continue;
            }

            
            tokens.push(self.parse_operator(&chars, &mut pos)?);
        }

        tokens.push(Token::Eof);
        Ok(tokens)
    }

    fn parse_string(&mut self, chars: &[char], pos: &mut usize) -> Result<Token, String> {
        let quote = chars[*pos];
        *pos += 1;
        let mut value = String::new();
        let mut closed = false;

        while *pos < chars.len() {
            if chars[*pos] == quote {
                closed = true;
                *pos += 1;
                break;
            }
            if chars[*pos] == '\\' {
                *pos += 1;
                if *pos < chars.len() {
                    match chars[*pos] {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        '\\' => value.push('\\'),
                        '\'' => value.push('\''),
                        '"' => value.push('"'),
                        _ => value.push(chars[*pos]),
                    }
                    *pos += 1;
                }
            } else {
                value.push(chars[*pos]);
                *pos += 1;
            }
        }

        if !closed {
            self.unsupported_patterns.push("Unclosed string literal".to_string());
        }

        Ok(Token::String(value))
    }

    fn parse_number(&mut self, chars: &[char], pos: &mut usize) -> Token {
        let mut value = String::new();
        let mut has_dot = false;

        while *pos < chars.len() {
            let c = chars[*pos];
            if c.is_ascii_digit() {
                value.push(c);
                *pos += 1;
            } else if c == '.' && !has_dot {
                
                if *pos + 1 < chars.len() && chars[*pos + 1].is_ascii_digit() {
                    has_dot = true;
                    value.push(c);
                    *pos += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        
        if *pos < chars.len() && chars[*pos] == 'n' {
            value.push('n');
            *pos += 1;
            self.unsupported_patterns.push("BigInt literal".to_string());
        }

        Token::Number(value)
    }

    fn parse_identifier(&mut self, chars: &[char], pos: &mut usize) -> Token {
        let mut value = String::new();

        while *pos < chars.len() && (chars[*pos].is_alphanumeric() || chars[*pos] == '_' || chars[*pos] == '$') {
            value.push(chars[*pos]);
            *pos += 1;
        }

        match value.as_str() {
            "var" => Token::Var,
            "let" => Token::Let,
            "const" => Token::Const,
            "function" => Token::Function,
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "while" => Token::While,
            "return" => Token::Return,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            "null" => Token::Null,
            "async" => {
                self.unsupported_patterns.push("async/await".to_string());
                Token::Identifier(value)
            }
            "await" => {
                self.unsupported_patterns.push("async/await".to_string());
                Token::Identifier(value)
            }
            "class" => {
                self.unsupported_patterns.push("class syntax".to_string());
                Token::Identifier(value)
            }
            "import" | "export" | "from" => {
                self.unsupported_patterns.push("ES6 modules".to_string());
                Token::Identifier(value)
            }
            _ => Token::Identifier(value),
        }
    }

    fn parse_operator(&mut self, chars: &[char], pos: &mut usize) -> Result<Token, String> {
        let c = chars[*pos];
        
        match c {
            '+' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '+' {
                    *pos += 1;
                    self.unsupported_patterns.push("++ operator".to_string());
                    Ok(Token::PlusPlus)
                } else if *pos < chars.len() && chars[*pos] == '=' {
                    *pos += 1;
                    Ok(Token::PlusEq)
                } else {
                    Ok(Token::Plus)
                }
            }
            '-' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '-' {
                    *pos += 1;
                    self.unsupported_patterns.push("-- operator".to_string());
                    Ok(Token::MinusMinus)
                } else if *pos < chars.len() && chars[*pos] == '>' {
                    *pos += 1;
                    Ok(Token::Arrow)
                } else if *pos < chars.len() && chars[*pos] == '=' {
                    *pos += 1;
                    Ok(Token::MinusEq)
                } else {
                    Ok(Token::Minus)
                }
            }
            '*' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '*' {
                    *pos += 1;
                    self.unsupported_patterns.push("** operator".to_string());
                    Ok(Token::StarStar)
                } else if *pos < chars.len() && chars[*pos] == '=' {
                    *pos += 1;
                    Ok(Token::StarEq)
                } else {
                    Ok(Token::Star)
                }
            }
            '/' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '=' {
                    *pos += 1;
                    Ok(Token::SlashEq)
                } else {
                    Ok(Token::Slash)
                }
            }
            '%' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '=' {
                    *pos += 1;
                    Ok(Token::PercentEq)
                } else {
                    Ok(Token::Percent)
                }
            }
            '=' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '=' {
                    *pos += 1;
                    if *pos < chars.len() && chars[*pos] == '=' {
                        *pos += 1;
                        Ok(Token::EqEqEq)
                    } else {
                        Ok(Token::EqEq)
                    }
                } else {
                    Ok(Token::Eq)
                }
            }
            '!' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '=' {
                    *pos += 1;
                    if *pos < chars.len() && chars[*pos] == '=' {
                        *pos += 1;
                        Ok(Token::NeEq)
                    } else {
                        Ok(Token::Ne)
                    }
                } else {
                    Ok(Token::Not)
                }
            }
            '<' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '=' {
                    *pos += 1;
                    Ok(Token::LtEq)
                } else {
                    Ok(Token::Lt)
                }
            }
            '>' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '=' {
                    *pos += 1;
                    Ok(Token::GtEq)
                } else {
                    Ok(Token::Gt)
                }
            }
            '&' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '&' {
                    *pos += 1;
                    Ok(Token::And)
                } else {
                    Ok(Token::BitAnd)
                }
            }
            '|' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '|' {
                    *pos += 1;
                    Ok(Token::Or)
                } else {
                    Ok(Token::BitOr)
                }
            }
            '?' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos] == '?' {
                    *pos += 1;
                    self.unsupported_patterns.push("?? operator".to_string());
                    Ok(Token::QuestionQuestion)
                } else if *pos < chars.len() && chars[*pos] == '.' {
                    *pos += 1;
                    self.unsupported_patterns.push("Optional chaining".to_string());
                    Ok(Token::Dot)
                } else {
                    Ok(Token::Question)
                }
            }
            ':' => { *pos += 1; Ok(Token::Colon) }
            ';' => { *pos += 1; Ok(Token::Semicolon) }
            ',' => { *pos += 1; Ok(Token::Comma) }
            '.' => {
                *pos += 1;
                if *pos < chars.len() && chars[*pos].is_ascii_digit() {
                    
                    *pos -= 1;
                    return Ok(self.parse_number(chars, pos));
                }
                Ok(Token::Dot)
            }
            '(' => { *pos += 1; Ok(Token::LParen) }
            ')' => { *pos += 1; Ok(Token::RParen) }
            '{' => { *pos += 1; Ok(Token::LBrace) }
            '}' => { *pos += 1; Ok(Token::RBrace) }
            '[' => { *pos += 1; Ok(Token::LBracket) }
            ']' => { *pos += 1; Ok(Token::RBracket) }
            _ => {
                *pos += 1;
                Ok(Token::Unknown(c))
            }
        }
    }

    fn parse_program(&mut self, tokens: &[Token]) -> Result<JsNode, String> {
        let mut statements = Vec::new();
        let mut pos = 0;

        while pos < tokens.len() && tokens[pos] != Token::Eof {
            if let Some(stmt) = self.parse_statement(tokens, &mut pos)? {
                statements.push(stmt);
            }
        }

        Ok(JsNode::Program(statements))
    }

    fn parse_statement(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        match &tokens[*pos] {
            Token::Var | Token::Let | Token::Const => self.parse_var_decl(tokens, pos),
            Token::Function => self.parse_function(tokens, pos),
            Token::If => self.parse_if(tokens, pos),
            Token::For => self.parse_for(tokens, pos),
            Token::While => self.parse_while(tokens, pos),
            Token::Return => self.parse_return(tokens, pos),
            Token::Break => {
                *pos += 1;
                self.skip_semicolon(tokens, pos);
                Ok(Some(JsNode::BlockStatement(Vec::new())))
            }
            Token::LBrace => self.parse_block(tokens, pos),
            Token::Semicolon => {
                *pos += 1;
                Ok(Some(JsNode::BlockStatement(Vec::new())))
            }
            _ => self.parse_expr_statement(tokens, pos),
        }
    }

    fn parse_var_decl(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        let kind = match &tokens[*pos] {
            Token::Var => "var",
            Token::Let => "let",
            Token::Const => "const",
            _ => return Ok(None),
        };
        *pos += 1;

        let name = if let Token::Identifier(n) = &tokens[*pos] {
            *pos += 1;
            n.clone()
        } else {
            return Err("Expected identifier".to_string());
        };

        let init = if *pos < tokens.len() && tokens[*pos] == Token::Eq {
            *pos += 1;
            Some(Box::new(self.parse_expr(tokens, pos)?))
        } else {
            None
        };

        self.skip_semicolon(tokens, pos);

        Ok(Some(JsNode::VariableDeclaration {
            kind: kind.to_string(),
            name,
            init,
        }))
    }

    fn parse_function(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        *pos += 1; 

        let name = if *pos < tokens.len() {
            if let Token::Identifier(n) = &tokens[*pos] {
                *pos += 1;
                n.clone()
            } else {
                String::new()
            }
        } else {
            return Err("Expected function name".to_string());
        };

        
        let mut params = Vec::new();
        if *pos < tokens.len() && tokens[*pos] == Token::LParen {
            *pos += 1;
            while *pos < tokens.len() && tokens[*pos] != Token::RParen {
                if let Token::Identifier(param_name) = &tokens[*pos] {
                    params.push(param_name.clone());
                }
                *pos += 1;
            }
            if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                *pos += 1;
            }
        }

        let body = self.parse_block(tokens, pos)?;

        Ok(Some(JsNode::FunctionDeclaration {
            name,
            params,
            body: match body {
                Some(JsNode::BlockStatement(stmts)) => stmts,
                _ => Vec::new(),
            },
        }))
    }

    fn parse_if(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        *pos += 1; 

        
        let test = if *pos < tokens.len() && tokens[*pos] == Token::LParen {
            *pos += 1;
            let expr = self.parse_expression(tokens, pos)?;
            if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                *pos += 1;
            }
            expr.unwrap_or(JsNode::Literal(LiteralValue::Boolean(true)))
        } else {
            JsNode::Literal(LiteralValue::Boolean(true))
        };

        let consequent = self.parse_statement(tokens, pos)?.map(|s| vec![s]).unwrap_or_default();

        let alternate = if *pos < tokens.len() && tokens[*pos] == Token::Else {
            *pos += 1;
            Some(self.parse_statement(tokens, pos)?.map(|s| vec![s]).unwrap_or_default())
        } else {
            None
        };

        Ok(Some(JsNode::IfStatement {
            test: Box::new(test),
            consequent,
            alternate,
        }))
    }

    fn parse_for(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        *pos += 1; 

        let (init, test, update) = if *pos < tokens.len() && tokens[*pos] == Token::LParen {
            *pos += 1;
            let init = self.parse_var_decl_or_expr(tokens, pos).ok().flatten();
            
            
            if *pos < tokens.len() && tokens[*pos] == Token::Semicolon {
                *pos += 1;
            }
            
            let test = self.parse_expression(tokens, pos).ok().flatten();
            
            
            if *pos < tokens.len() && tokens[*pos] == Token::Semicolon {
                *pos += 1;
            }
            
            let update = self.parse_expression(tokens, pos).ok().flatten();
            
            if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                *pos += 1;
            }
            
            (
                init.map(|n| Box::new(n)),
                test.map(|n| Box::new(n)),
                update.map(|n| Box::new(n)),
            )
        } else {
            (None, None, None)
        };

        let body = self.parse_statement(tokens, pos)?.map(|s| vec![s]).unwrap_or_default();

        Ok(Some(JsNode::ForStatement { init, test, update, body }))
    }

    fn parse_while(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        *pos += 1; 

        
        let test = if *pos < tokens.len() && tokens[*pos] == Token::LParen {
            *pos += 1;
            let expr = self.parse_expression(tokens, pos).ok().flatten()
                .unwrap_or(JsNode::Literal(LiteralValue::Boolean(true)));
            if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                *pos += 1;
            }
            expr
        } else {
            JsNode::Literal(LiteralValue::Boolean(true))
        };

        let body = self.parse_statement(tokens, pos)?.map(|s| vec![s]).unwrap_or_default();

        Ok(Some(JsNode::WhileStatement {
            test: Box::new(test),
            body,
        }))
    }

    
    fn parse_expression(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        if *pos >= tokens.len() {
            return Ok(None);
        }

        match &tokens[*pos] {
            Token::Identifier(name) => {
                let ident = name.clone();
                *pos += 1;
                Ok(Some(JsNode::Identifier(ident)))
            }
            Token::Number(n) => {
                let num = n.clone();
                *pos += 1;
                Ok(Some(JsNode::Literal(LiteralValue::Number(num))))
            }
            Token::String(s) => {
                let str_val = s.clone();
                *pos += 1;
                Ok(Some(JsNode::Literal(LiteralValue::String(str_val))))
            }
            Token::Boolean(b) => {
                let val = *b;
                *pos += 1;
                Ok(Some(JsNode::Literal(LiteralValue::Boolean(val))))
            }
            Token::Null => {
                *pos += 1;
                Ok(Some(JsNode::Literal(LiteralValue::Null)))
            }
            _ => {
                
                self.parse_expr(tokens, pos).map(Some)
            }
        }
    }

    
    fn parse_var_decl_or_expr(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        if *pos >= tokens.len() {
            return Ok(None);
        }

        
        if matches!(&tokens[*pos], Token::Var | Token::Let | Token::Const) {
            return self.parse_var_decl(tokens, pos);
        }

        self.parse_expression(tokens, pos)
    }

    fn parse_return(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        *pos += 1; 

        let value = if *pos < tokens.len() && tokens[*pos] != Token::Semicolon && tokens[*pos] != Token::RBrace {
            Some(Box::new(self.parse_expr(tokens, pos)?))
        } else {
            None
        };

        self.skip_semicolon(tokens, pos);

        Ok(Some(JsNode::ReturnStatement(value)))
    }

    fn parse_block(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        if *pos >= tokens.len() || tokens[*pos] != Token::LBrace {
            return Ok(None);
        }
        *pos += 1;

        let mut statements = Vec::new();
        while *pos < tokens.len() && tokens[*pos] != Token::RBrace {
            if let Some(stmt) = self.parse_statement(tokens, pos)? {
                statements.push(stmt);
            }
        }

        if *pos < tokens.len() {
            *pos += 1;
        }

        Ok(Some(JsNode::BlockStatement(statements)))
    }

    fn parse_expr_statement(&mut self, tokens: &[Token], pos: &mut usize) -> Result<Option<JsNode>, String> {
        let expr = self.parse_expr(tokens, pos)?;
        self.skip_semicolon(tokens, pos);
        Ok(Some(JsNode::ExpressionStatement(Box::new(expr))))
    }

    fn parse_expr(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        self.parse_assignment(tokens, pos)
    }

    fn parse_assignment(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        let left = self.parse_logical(tokens, pos)?;

        if *pos < tokens.len() {
            let op = match &tokens[*pos] {
                Token::Eq => "=",
                Token::PlusEq => "+=",
                Token::MinusEq => "-=",
                Token::StarEq => "*=",
                Token::SlashEq => "/=",
                _ => return Ok(left),
            };
            *pos += 1;
            let right = self.parse_assignment(tokens, pos)?;

            return Ok(JsNode::AssignmentExpression {
                operator: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    fn parse_logical(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        let mut left = self.parse_equality(tokens, pos)?;

        while *pos < tokens.len() {
            let op = match &tokens[*pos] {
                Token::And => "&&",
                Token::Or => "||",
                _ => break,
            };
            *pos += 1;
            let right = self.parse_equality(tokens, pos)?;

            left = JsNode::LogicalExpression {
                operator: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        let mut left = self.parse_relational(tokens, pos)?;

        loop {
            let op = match &tokens[*pos] {
                Token::EqEq => "==",
                Token::Ne => "!=",
                Token::EqEqEq => "===",
                Token::NeEq => "!==",
                _ => break,
            };
            *pos += 1;
            let right = self.parse_relational(tokens, pos)?;

            left = JsNode::BinaryExpression {
                operator: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_relational(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        let mut left = self.parse_additive(tokens, pos)?;

        loop {
            let op = match &tokens[*pos] {
                Token::Lt => "<",
                Token::Gt => ">",
                Token::LtEq => "<=",
                Token::GtEq => ">=",
                _ => break,
            };
            *pos += 1;
            let right = self.parse_additive(tokens, pos)?;

            left = JsNode::BinaryExpression {
                operator: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        let mut left = self.parse_multiplicative(tokens, pos)?;

        loop {
            let op = match &tokens[*pos] {
                Token::Plus => "+",
                Token::Minus => "-",
                _ => break,
            };
            *pos += 1;
            let right = self.parse_multiplicative(tokens, pos)?;

            left = JsNode::BinaryExpression {
                operator: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        let mut left = self.parse_unary(tokens, pos)?;

        loop {
            let op = match &tokens[*pos] {
                Token::Star => "*",
                Token::Slash => "/",
                Token::Percent => "%",
                _ => break,
            };
            *pos += 1;
            let right = self.parse_unary(tokens, pos)?;

            left = JsNode::BinaryExpression {
                operator: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        match &tokens[*pos] {
            Token::Not | Token::Minus | Token::Plus | Token::Typeof | Token::Void | Token::Delete => {
                let op = match &tokens[*pos] {
                    Token::Not => "!",
                    Token::Minus => "-",
                    Token::Plus => "+",
                    Token::Typeof => "typeof",
                    Token::Void => "void",
                    Token::Delete => "delete",
                    _ => "!",
                };
                *pos += 1;
                let argument = self.parse_unary(tokens, pos)?;

                Ok(JsNode::UnaryExpression {
                    operator: op.to_string(),
                    argument: Box::new(argument),
                })
            }
            _ => self.parse_call_member(tokens, pos),
        }
    }

    fn parse_call_member(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        let mut expr = self.parse_primary(tokens, pos)?;

        loop {
            if *pos < tokens.len() && tokens[*pos] == Token::Dot {
                *pos += 1;
                let property = if let Token::Identifier(p) = &tokens[*pos] {
                    *pos += 1;
                    p.clone()
                } else {
                    String::new()
                };
                expr = JsNode::MemberExpression {
                    object: Box::new(expr),
                    property,
                };
            } else if *pos < tokens.len() && tokens[*pos] == Token::LParen {
                *pos += 1;
                let mut args = Vec::new();
                while *pos < tokens.len() && tokens[*pos] != Token::RParen {
                    if tokens[*pos] != Token::Comma {
                        args.push(self.parse_expr(tokens, pos)?);
                    } else {
                        *pos += 1;
                    }
                }
                if *pos < tokens.len() {
                    *pos += 1;
                }
                expr = JsNode::CallExpression {
                    callee: Box::new(expr),
                    arguments: args,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self, tokens: &[Token], pos: &mut usize) -> Result<JsNode, String> {
        match &tokens[*pos] {
            Token::Identifier(n) => {
                *pos += 1;
                Ok(JsNode::Identifier(n.clone()))
            }
            Token::String(s) => {
                *pos += 1;
                Ok(JsNode::Literal(LiteralValue::String(s.clone())))
            }
            Token::Number(n) => {
                *pos += 1;
                Ok(JsNode::Literal(LiteralValue::Number(n.clone())))
            }
            Token::Boolean(b) => {
                *pos += 1;
                Ok(JsNode::Literal(LiteralValue::Boolean(*b)))
            }
            Token::Null => {
                *pos += 1;
                Ok(JsNode::Literal(LiteralValue::Null))
            }
            Token::LParen => {
                *pos += 1;
                let expr = self.parse_expr(tokens, pos)?;
                if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                    *pos += 1;
                }
                Ok(expr)
            }
            Token::LBracket => {
                *pos += 1;
                let mut elements = Vec::new();
                while *pos < tokens.len() && tokens[*pos] != Token::RBracket {
                    if tokens[*pos] != Token::Comma {
                        elements.push(self.parse_expr(tokens, pos)?);
                    } else {
                        *pos += 1;
                    }
                }
                if *pos < tokens.len() {
                    *pos += 1;
                }
                Ok(JsNode::ArrayExpression(elements))
            }
            Token::LBrace => {
                *pos += 1;
                let mut props = Vec::new();
                while *pos < tokens.len() && tokens[*pos] != Token::RBrace {
                    if let Token::Identifier(key) = &tokens[*pos] {
                        let key_name = key.clone();
                        *pos += 1;
                        if *pos < tokens.len() && tokens[*pos] == Token::Colon {
                            *pos += 1;
                            let value = self.parse_expr(tokens, pos)?;
                            props.push((key_name, value));
                        }
                    }
                    if *pos < tokens.len() && tokens[*pos] == Token::Comma {
                        *pos += 1;
                    }
                }
                if *pos < tokens.len() {
                    *pos += 1;
                }
                Ok(JsNode::ObjectExpression(props))
            }
            _ => {
                *pos += 1;
                Ok(JsNode::Literal(LiteralValue::Null))
            }
        }
    }

    fn skip_semicolon(&mut self, tokens: &[Token], pos: &mut usize) {
        if *pos < tokens.len() && tokens[*pos] == Token::Semicolon {
            *pos += 1;
        }
    }

    pub fn print_unsupported(&self) {
        if self.unsupported_patterns.is_empty() {
            println!("✅ JS Parser: All patterns supported");
            return;
        }

        println!("\n🔴 JS Parser - Unsupported Patterns Found:");
        println!("══════════════════════════════════════════");
        
        let mut stdout = io::stdout();
        for (i, pattern) in self.unsupported_patterns.iter().enumerate() {
            writeln!(stdout, "  [{}] {}", i + 1, pattern).ok();
        }
        writeln!(stdout, "\nTotal: {} unsupported pattern(s)\n", self.unsupported_patterns.len()).ok();
        stdout.flush().ok();
    }
}

impl Default for JsParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Identifier(String),
    String(String),
    Number(String),
    Boolean(bool),
    Null,
    Var,
    Let,
    Const,
    Function,
    If,
    Else,
    For,
    While,
    Return,
    Break,
    Continue,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusPlus,
    MinusMinus,
    StarStar,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    Eq,
    EqEq,
    Ne,
    EqEqEq,
    NeEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    Question,
    QuestionQuestion,
    Colon,
    Semicolon,
    Comma,
    Dot,
    Arrow,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Typeof,
    Void,
    Delete,
    Eof,
    Unknown(char),
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_expression() {
        let mut parser = JsParser::new();
        let result = parser.parse("1 + 2 * 3;");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_variable_declaration() {
        let mut parser = JsParser::new();
        let result = parser.parse("var x = 42;");
        assert!(result.is_ok());
        if let Ok(JsNode::Program(stmts)) = &result {
            assert_eq!(stmts.len(), 1);
        }
    }

    #[test]
    fn test_parse_function_with_params() {
        let mut parser = JsParser::new();
        let result = parser.parse("function foo(a, b, c) { return a + b; }");
        assert!(result.is_ok());
        if let Ok(JsNode::Program(stmts)) = &result {
            assert_eq!(stmts.len(), 1);
            if let JsNode::FunctionDeclaration { name, params, .. } = &stmts[0] {
                assert_eq!(name, "foo");
                assert_eq!(params.len(), 3);
                assert_eq!(params[0], "a");
                assert_eq!(params[1], "b");
                assert_eq!(params[2], "c");
            } else {
                panic!("Expected function declaration");
            }
        }
    }

    #[test]
    fn test_parse_if_with_condition() {
        let mut parser = JsParser::new();
        let result = parser.parse("if (x > 0) { console.log('positive'); }");
        assert!(result.is_ok());
        
    }

    #[test]
    fn test_parse_for_loop() {
        let mut parser = JsParser::new();
        let result = parser.parse("for (var i = 0; i < 10; i++) { sum += i; }");
        assert!(result.is_ok());
        
    }

    #[test]
    fn test_parse_while_loop() {
        let mut parser = JsParser::new();
        let result = parser.parse("while (x > 0) { x--; }");
        assert!(result.is_ok());
        
    }

    #[test]
    fn test_parse_nested_constructs() {
        let mut parser = JsParser::new();
        let js = r#"
            function greet(name) {
                if (name) {
                    for (var i = 0; i < 3; i++) {
                        console.log('Hello ' + name);
                    }
                }
            }
        "#;
        let result = parser.parse(js);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_malformed_js() {
        let mut parser = JsParser::new();
        let js = "var x = ; function () { }";
        
        let _ = parser.parse(js);
    }
}
