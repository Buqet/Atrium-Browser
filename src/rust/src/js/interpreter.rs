use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;


#[derive(Debug, Clone)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(Rc<RefCell<JsObject>>),
    Function(Rc<JsFunction>),
    BuiltinFunction(BuiltinFn),
}

impl PartialEq for JsValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (JsValue::Undefined, JsValue::Undefined) => true,
            (JsValue::Null, JsValue::Null) => true,
            (JsValue::Boolean(a), JsValue::Boolean(b)) => a == b,
            (JsValue::Number(a), JsValue::Number(b)) => a == b,
            (JsValue::String(a), JsValue::String(b)) => a == b,
            (JsValue::Object(a), JsValue::Object(b)) => Rc::ptr_eq(a, b),
            (JsValue::Function(a), JsValue::Function(b)) => Rc::ptr_eq(a, b),
            (JsValue::BuiltinFunction(a), JsValue::BuiltinFunction(b)) => a.name == b.name,
            _ => false,
        }
    }
}

impl JsValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            JsValue::Boolean(b) => *b,
            JsValue::Number(n) => *n != 0.0 && !n.is_nan(),
            JsValue::String(s) => !s.is_empty(),
            JsValue::Null | JsValue::Undefined => false,
            JsValue::Object(_) | JsValue::Function(_) | JsValue::BuiltinFunction(_) => true,
        }
    }

    pub fn to_number(&self) -> f64 {
        match self {
            JsValue::Number(n) => *n,
            JsValue::Boolean(true) => 1.0,
            JsValue::Boolean(false) => 0.0,
            JsValue::String(s) => s.parse::<f64>().unwrap_or(f64::NAN),
            JsValue::Null => 0.0,
            _ => f64::NAN,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            JsValue::Undefined => "undefined".to_string(),
            JsValue::Null => "null".to_string(),
            JsValue::Boolean(b) => b.to_string(),
            JsValue::Number(n) => n.to_string(),
            JsValue::String(s) => s.clone(),
            JsValue::Object(_) => "[object Object]".to_string(),
            JsValue::Function(f) => format!("[Function: {}]", f.name),
            JsValue::BuiltinFunction(f) => format!("[Builtin: {}]", f.name()),
        }
    }
}

impl fmt::Display for JsValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}


#[derive(Debug, Clone)]
pub struct JsObject {
    pub properties: HashMap<String, JsValue>,
    pub proto: Option<Rc<RefCell<JsObject>>>,
}

impl JsObject {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            proto: None,
        }
    }

    pub fn get_property(&self, name: &str) -> Option<JsValue> {
        if let Some(val) = self.properties.get(name) {
            return Some(val.clone());
        }
        if let Some(proto) = &self.proto {
            return proto.borrow().get_property(name);
        }
        None
    }

    pub fn set_property(&mut self, name: String, value: JsValue) {
        self.properties.insert(name, value);
    }
}


#[derive(Debug)]
pub struct JsFunction {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<crate::js::parser::JsNode>,
    pub closure: Environment,
}


#[derive(Debug, Clone)]
pub struct BuiltinFn {
    pub name: String,
    pub handler: fn(&[JsValue]) -> JsValue,
}

impl BuiltinFn {
    pub fn name(&self) -> &str {
        &self.name
    }
}


#[derive(Debug, Clone)]
pub struct Environment {
    pub bindings: HashMap<String, JsValue>,
    pub outer: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new(outer: Option<Rc<RefCell<Environment>>>) -> Self {
        Self {
            bindings: HashMap::new(),
            outer,
        }
    }

    pub fn get(&self, name: &str) -> Option<JsValue> {
        if let Some(val) = self.bindings.get(name) {
            return Some(val.clone());
        }
        if let Some(outer) = &self.outer {
            return outer.borrow().get(name);
        }
        None
    }

    pub fn set(&mut self, name: String, value: JsValue) {
        self.bindings.insert(name, value);
    }

    
    pub fn define(&mut self, name: String, value: JsValue) {
        self.bindings.insert(name, value);
    }
}


pub struct JsInterpreter {
    pub global_env: Rc<RefCell<Environment>>,
    pub console_output: Vec<String>,
}

impl JsInterpreter {
    pub fn new() -> Self {
        let global_env = Rc::new(RefCell::new(Environment::new(None)));
        let mut interp = Self {
            global_env,
            console_output: Vec::new(),
        };
        interp.install_builtins();
        interp
    }

    
    fn install_builtins(&mut self) {
        
        let log_fn = JsValue::BuiltinFunction(BuiltinFn {
            name: "log".to_string(),
            handler: |args: &[JsValue]| {
                let msg = args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(" ");
                println!("{}", msg);
                JsValue::Undefined
            },
        });

        
        let mut console_obj = JsObject::new();
        console_obj.set_property("log".to_string(), log_fn);
        self.global_env.borrow_mut().set(
            "console".to_string(),
            JsValue::Object(Rc::new(RefCell::new(console_obj))),
        );
    }

    
    pub fn eval(&mut self, source: &str) -> Result<JsValue, String> {
        let mut parser = crate::js::parser::JsParser::new();
        let ast = parser.parse(source)?;
        self.eval_program(&ast)
    }

    
    pub fn eval_program(&mut self, program: &crate::js::parser::JsNode) -> Result<JsValue, String> {
        match program {
            crate::js::parser::JsNode::Program(stmts) => {
                let mut result = JsValue::Undefined;
                for stmt in stmts {
                    result = self.eval_node(stmt)?;
                }
                Ok(result)
            }
            _ => self.eval_node(program),
        }
    }

    
    pub fn eval_node(&mut self, node: &crate::js::parser::JsNode) -> Result<JsValue, String> {
        match node {
            crate::js::parser::JsNode::Program(stmts) => {
                let mut result = JsValue::Undefined;
                for stmt in stmts {
                    result = self.eval_node(stmt)?;
                }
                Ok(result)
            }

            crate::js::parser::JsNode::VariableDeclaration { kind: _, name, init } => {
                let value = if let Some(init_node) = init {
                    self.eval_node(init_node)?
                } else {
                    JsValue::Undefined
                };
                self.global_env.borrow_mut().define(name.clone(), value.clone());
                Ok(value)
            }

            crate::js::parser::JsNode::FunctionDeclaration { name, params, body } => {
                let func = JsFunction {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: self.global_env.borrow().clone(),
                };
                let func_val = JsValue::Function(Rc::new(func));
                self.global_env.borrow_mut().define(name.clone(), func_val.clone());
                Ok(func_val)
            }

            crate::js::parser::JsNode::ExpressionStatement(expr) => {
                self.eval_node(expr)
            }

            crate::js::parser::JsNode::Identifier(name) => {
                self.global_env.borrow().get(name)
                    .ok_or_else(|| format!("ReferenceError: {} is not defined", name))
            }

            crate::js::parser::JsNode::Literal(lit) => {
                Ok(match lit {
                    crate::js::parser::LiteralValue::String(s) => JsValue::String(s.clone()),
                    crate::js::parser::LiteralValue::Number(n) => JsValue::Number(n.parse().unwrap_or(0.0)),
                    crate::js::parser::LiteralValue::Boolean(b) => JsValue::Boolean(*b),
                    crate::js::parser::LiteralValue::Null => JsValue::Null,
                })
            }

            crate::js::parser::JsNode::BinaryExpression { operator, left, right } => {
                let l = self.eval_node(left)?;
                let r = self.eval_node(right)?;
                self.eval_binary_op(operator, &l, &r)
            }

            crate::js::parser::JsNode::LogicalExpression { operator, left, right } => {
                let l = self.eval_node(left)?;
                match operator.as_str() {
                    "&&" => {
                        if l.is_truthy() {
                            self.eval_node(right)
                        } else {
                            Ok(l)
                        }
                    }
                    "||" => {
                        if l.is_truthy() {
                            Ok(l)
                        } else {
                            self.eval_node(right)
                        }
                    }
                    _ => Err(format!("Unknown logical operator: {}", operator)),
                }
            }

            crate::js::parser::JsNode::UnaryExpression { operator, argument } => {
                let val = self.eval_node(argument)?;
                match operator.as_str() {
                    "!" => Ok(JsValue::Boolean(!val.is_truthy())),
                    "-" => Ok(JsValue::Number(-val.to_number())),
                    "+" => Ok(JsValue::Number(val.to_number())),
                    _ => Err(format!("Unknown unary operator: {}", operator)),
                }
            }

            crate::js::parser::JsNode::AssignmentExpression { operator: _, left, right } => {
                let value = self.eval_node(right)?;
                if let crate::js::parser::JsNode::Identifier(name) = left.as_ref() {
                    self.global_env.borrow_mut().set(name.clone(), value.clone());
                    Ok(value)
                } else {
                    Err("Invalid assignment target".to_string())
                }
            }

            crate::js::parser::JsNode::CallExpression { callee, arguments } => {
                let func = self.eval_node(callee)?;
                let args: Vec<JsValue> = arguments.iter()
                    .map(|a| self.eval_node(a))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_function(&func, &args)
            }

            crate::js::parser::JsNode::IfStatement { test, consequent, alternate } => {
                let test_val = self.eval_node(test)?;
                if test_val.is_truthy() {
                    self.eval_block(consequent)
                } else if let Some(alt) = alternate {
                    self.eval_block(alt)
                } else {
                    Ok(JsValue::Undefined)
                }
            }

            crate::js::parser::JsNode::ForStatement { init, test, update, body } => {
                
                if let Some(init_node) = init {
                    self.eval_node(init_node)?;
                }

                
                loop {
                    
                    if let Some(test_node) = test {
                        let test_val = self.eval_node(test_node)?;
                        if !test_val.is_truthy() {
                            break;
                        }
                    }

                    
                    self.eval_block(body)?;

                    
                    if let Some(update_node) = update {
                        self.eval_node(update_node)?;
                    }
                }

                Ok(JsValue::Undefined)
            }

            crate::js::parser::JsNode::WhileStatement { test, body } => {
                loop {
                    let test_val = self.eval_node(test)?;
                    if !test_val.is_truthy() {
                        break;
                    }
                    self.eval_block(body)?;
                }
                Ok(JsValue::Undefined)
            }

            crate::js::parser::JsNode::BlockStatement(stmts) => {
                self.eval_block(stmts)
            }
    crate::js::parser::JsNode::ReturnStatement(value) => {
        let result = if let Some(expr) = value {
            self.eval_node(expr)?
        } else {
            JsValue::Undefined
        };
        return Err(format!("__RETURN__{:?}", result));
}

            crate::js::parser::JsNode::ArrayExpression(items) => {
                let elements: Vec<JsValue> = items.iter()
                    .map(|i| self.eval_node(i))
                    .collect::<Result<Vec<_>, _>>()?;

                let mut obj = JsObject::new();
                for (i, elem) in elements.into_iter().enumerate() {
                    obj.set_property(i.to_string(), elem);
                }
                obj.set_property("length".to_string(), JsValue::Number(obj.properties.len() as f64 - 1.0));
                Ok(JsValue::Object(Rc::new(RefCell::new(obj))))
            }

            crate::js::parser::JsNode::ObjectExpression(pairs) => {
                let mut obj = JsObject::new();
                for (key, value_node) in pairs {
                    let value = self.eval_node(value_node)?;
                    obj.set_property(key.clone(), value);
                }
                Ok(JsValue::Object(Rc::new(RefCell::new(obj))))
            }

            crate::js::parser::JsNode::MemberExpression { object, property } => {
                let obj_val = self.eval_node(object)?;
                if let JsValue::Object(obj_rc) = obj_val {
                    let obj = obj_rc.borrow();
                    obj.get_property(property)
                        .ok_or_else(|| format!("Property '{}' not found", property))
                } else {
                    Err("Cannot read property of non-object".to_string())
                }
            }

            _ => Ok(JsValue::Undefined),
        }
    }

    
    fn eval_binary_op(&self, op: &str, left: &JsValue, right: &JsValue) -> Result<JsValue, String> {
        match op {
            "+" => {
                
                if let (JsValue::String(a), JsValue::String(b)) = (left, right) {
                    Ok(JsValue::String(format!("{}{}", a, b)))
                } else if let (JsValue::String(s), _) = (left, right) {
                    Ok(JsValue::String(format!("{}{}", s, right.to_string())))
                } else if let (_, JsValue::String(s)) = (left, right) {
                    Ok(JsValue::String(format!("{}{}", left.to_string(), s)))
                } else {
                    Ok(JsValue::Number(left.to_number() + right.to_number()))
                }
            }
            "-" => Ok(JsValue::Number(left.to_number() - right.to_number())),
            "*" => Ok(JsValue::Number(left.to_number() * right.to_number())),
            "/" => {
                let r = right.to_number();
                if r == 0.0 {
                    Ok(JsValue::Number(f64::INFINITY))
                } else {
                    Ok(JsValue::Number(left.to_number() / r))
                }
            }
            "%" => Ok(JsValue::Number(left.to_number() % right.to_number())),
            "==" => Ok(JsValue::Boolean(left.to_number() == right.to_number())),
            "===" => Ok(JsValue::Boolean(left == right)),
            "!=" => Ok(JsValue::Boolean(left.to_number() != right.to_number())),
            "!==" => Ok(JsValue::Boolean(left != right)),
            "<" => Ok(JsValue::Boolean(left.to_number() < right.to_number())),
            ">" => Ok(JsValue::Boolean(left.to_number() > right.to_number())),
            "<=" => Ok(JsValue::Boolean(left.to_number() <= right.to_number())),
            ">=" => Ok(JsValue::Boolean(left.to_number() >= right.to_number())),
            _ => Err(format!("Unknown binary operator: {}", op)),
        }
    }

    
    fn call_function(&mut self, func: &JsValue, args: &[JsValue]) -> Result<JsValue, String> {
        match func {
            JsValue::Function(f) => {
                
                let mut func_env = Environment::new(Some(Rc::new(RefCell::new(f.closure.clone()))));

                
                for (i, param) in f.params.iter().enumerate() {
                    let val = args.get(i).cloned().unwrap_or(JsValue::Undefined);
                    func_env.define(param.clone(), val);
                }

                
                for stmt in &f.body {
                    let result = self.eval_node_in_env(stmt, &func_env)?;
                    
                    
                    if matches!(result, JsValue::Undefined) {
                        
                    }
                }
                
                
                Ok(JsValue::Undefined)
            }
            JsValue::BuiltinFunction(bf) => {
                Ok((bf.handler)(args))
            }
            _ => Err(format!("{} is not a function", func.to_string())),
        }
    }

    
    fn eval_node_in_env(&mut self, node: &crate::js::parser::JsNode, env: &Environment) -> Result<JsValue, String> {
        match node {
            crate::js::parser::JsNode::Identifier(name) => {
                env.get(name)
                    .ok_or_else(|| format!("ReferenceError: {} is not defined", name))
            }
            crate::js::parser::JsNode::BinaryExpression { operator, left, right } => {
                let l = self.eval_node_in_env(left, env)?;
                let r = self.eval_node_in_env(right, env)?;
                self.eval_binary_op(operator, &l, &r)
            }
            crate::js::parser::JsNode::CallExpression { callee, arguments } => {
                let func = self.eval_node_in_env(callee, env)?;
                let args: Vec<JsValue> = arguments.iter()
                    .map(|a| self.eval_node_in_env(a, env))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_function(&func, &args)
            }
            crate::js::parser::JsNode::Literal(lit) => {
                Ok(match lit {
                    crate::js::parser::LiteralValue::String(s) => JsValue::String(s.clone()),
                    crate::js::parser::LiteralValue::Number(n) => JsValue::Number(n.parse().unwrap_or(0.0)),
                    crate::js::parser::LiteralValue::Boolean(b) => JsValue::Boolean(*b),
                    crate::js::parser::LiteralValue::Null => JsValue::Null,
                })
            }
            _ => self.eval_node(node),
        }
    }

    
    fn eval_block(&mut self, stmts: &[crate::js::parser::JsNode]) -> Result<JsValue, String> {
        let mut result = JsValue::Undefined;
        for stmt in stmts {
            result = self.eval_node(stmt)?;
        }
        Ok(result)
    }
}


#[derive(Debug)]
enum _ControlFlow {
    #[allow(dead_code)]
    Normal(JsValue),
    #[allow(dead_code)]
    Return(JsValue),
    #[allow(dead_code)]
    Break,
    #[allow(dead_code)]
    Continue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_literal() {
        let mut interp = JsInterpreter::new();
        let result = interp.eval("42;").unwrap();
        assert_eq!(result, JsValue::Number(42.0));
    }

    #[test]
    fn test_eval_string() {
        let mut interp = JsInterpreter::new();
        let result = interp.eval("'hello';").unwrap();
        assert_eq!(result, JsValue::String("hello".to_string()));
    }

    #[test]
    fn test_eval_binary_expression() {
        let mut interp = JsInterpreter::new();
        let result = interp.eval("1 + 2 * 3;").unwrap();
        
        
        
        assert!(matches!(result, JsValue::Number(_)));
    }

    #[test]
    fn test_eval_variable() {
        let mut interp = JsInterpreter::new();
        let result = interp.eval("var x = 10; x;").unwrap();
        assert_eq!(result, JsValue::Number(10.0));
    }

    #[test]
    fn test_eval_function() {
        let mut interp = JsInterpreter::new();
        
        let result = interp.eval("function greet() { var x = 1; }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_if_statement() {
        let mut interp = JsInterpreter::new();
        let result = interp.eval("var x = 5; if (x > 3) { var y = 10; } y;").unwrap();
        assert_eq!(result, JsValue::Number(10.0));
    }

    #[test]
    fn test_console_log() {
        let mut interp = JsInterpreter::new();
        let result = interp.eval("console.log('test');").unwrap();
        assert_eq!(result, JsValue::Undefined);
    }

    #[test]
    fn test_while_loop() {
        let mut interp = JsInterpreter::new();
        
        let result = interp.eval("var i = 0; while (i < 3) { i = i + 1; }");
        assert!(result.is_ok());
    }
}
