use std::collections::BTreeMap;

use crate::ast::*;
use crate::diagnostics::{Category, Diagnostic};

pub fn check_program(program: &Program) -> Result<(), Diagnostic> {
    StaticChecker::from_program(program)?.check(program)
}

#[derive(Clone, Debug)]
struct StaticChecker {
    dens: BTreeMap<String, DenInfo>,
    masks: BTreeMap<String, MaskInfo>,
}

#[derive(Clone, Debug)]
struct DenInfo {
    name: String,
    parent: Option<String>,
    masks: Vec<String>,
    fields: BTreeMap<String, FieldInfo>,
    methods: BTreeMap<String, MethodInfo>,
}

#[derive(Clone, Debug)]
struct MaskInfo {
    methods: BTreeMap<String, MaskMethod>,
}

#[derive(Clone, Debug)]
struct FieldInfo {
    type_name: Option<String>,
    access: Access,
    owner: String,
}

#[derive(Clone, Debug)]
struct MethodInfo {
    params: Vec<Param>,
    return_type: Option<String>,
}

impl StaticChecker {
    fn from_program(program: &Program) -> Result<Self, Diagnostic> {
        let mut checker = Self {
            dens: BTreeMap::new(),
            masks: BTreeMap::new(),
        };
        for item in &program.items {
            match item {
                Item::Mask(mask) => checker.register_mask(mask)?,
                Item::Den(den) => checker.register_den(den)?,
                _ => {}
            }
        }
        checker.validate_dens()?;
        Ok(checker)
    }

    fn register_mask(&mut self, item: &MaskDef) -> Result<(), Diagnostic> {
        if self.masks.contains_key(&item.name) || self.dens.contains_key(&item.name) {
            return Err(static_error(format!(
                "type {} is already defined",
                item.name
            )));
        }
        let mut methods = BTreeMap::new();
        for method in &item.methods {
            if methods
                .insert(method.name.clone(), method.clone())
                .is_some()
            {
                return Err(static_error(format!(
                    "duplicate method in mask {}",
                    item.name
                )));
            }
        }
        self.masks.insert(item.name.clone(), MaskInfo { methods });
        Ok(())
    }

    fn register_den(&mut self, item: &DenDef) -> Result<(), Diagnostic> {
        if self.dens.contains_key(&item.name) || self.masks.contains_key(&item.name) {
            return Err(static_error(format!(
                "type {} is already defined",
                item.name
            )));
        }
        let mut fields = BTreeMap::new();
        let mut methods = BTreeMap::new();
        for member in &item.members {
            match member {
                DenMember::Field(field) => {
                    if fields
                        .insert(
                            field.name.clone(),
                            FieldInfo {
                                type_name: field.type_name.clone(),
                                access: field.access,
                                owner: item.name.clone(),
                            },
                        )
                        .is_some()
                    {
                        return Err(static_error(format!(
                            "duplicate field {}.{}",
                            item.name, field.name
                        )));
                    }
                }
                DenMember::Method(method) => {
                    if method.name == "init" && method.return_type.is_some() {
                        return Err(static_error(format!(
                            "{}.init cannot declare a return type",
                            item.name
                        )));
                    }
                    if methods
                        .insert(
                            method.name.clone(),
                            MethodInfo {
                                params: method.params.clone(),
                                return_type: method.return_type.clone(),
                            },
                        )
                        .is_some()
                    {
                        return Err(static_error(format!(
                            "duplicate method {}.{}",
                            item.name, method.name
                        )));
                    }
                }
            }
        }
        self.dens.insert(
            item.name.clone(),
            DenInfo {
                name: item.name.clone(),
                parent: item.parent.clone(),
                masks: item.masks.clone(),
                fields,
                methods,
            },
        );
        Ok(())
    }

    fn validate_dens(&self) -> Result<(), Diagnostic> {
        for den in self.dens.values() {
            if let Some(parent) = &den.parent {
                if !self.dens.contains_key(parent) {
                    return Err(static_error(format!(
                        "parent den {parent} for {} is not defined",
                        den.name
                    )));
                }
            }
            for mask in &den.masks {
                if !self.masks.contains_key(mask) {
                    return Err(static_error(format!(
                        "mask {mask} for {} is not defined",
                        den.name
                    )));
                }
            }
        }
        for den in self.dens.values() {
            for mask_name in &den.masks {
                let mask = self.masks.get(mask_name).expect("validated mask");
                for (method_name, required) in &mask.methods {
                    let Some(method) = self.find_method(&den.name, method_name) else {
                        return Err(static_error(format!(
                            "{} wears {mask_name} but does not implement {method_name}",
                            den.name
                        )));
                    };
                    if !same_signature_method_mask(&method, required) {
                        return Err(static_error(format!(
                            "{}.{method_name} does not match mask {mask_name}.{method_name}",
                            den.name
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn check(&self, program: &Program) -> Result<(), Diagnostic> {
        let mut env = StaticEnv::default();
        for item in &program.items {
            match item {
                Item::Function(def) | Item::HowlFunction(def) => {
                    env.values.insert(
                        def.name.clone(),
                        StaticBinding {
                            type_name: def.return_type.clone(),
                            is_const: true,
                        },
                    );
                }
                Item::Den(den) => {
                    env.values.insert(
                        den.name.clone(),
                        StaticBinding {
                            type_name: Some(den.name.clone()),
                            is_const: true,
                        },
                    );
                }
                Item::Mask(mask) => {
                    env.values.insert(
                        mask.name.clone(),
                        StaticBinding {
                            type_name: Some(mask.name.clone()),
                            is_const: true,
                        },
                    );
                }
                _ => {}
            }
        }
        for item in &program.items {
            match item {
                Item::Main { body, .. } => {
                    self.check_block(body, &mut env.child(), false, None, None)?
                }
                Item::HowlMain { body, .. } => {
                    self.check_block(body, &mut env.child(), true, None, None)?
                }
                Item::Function(def) => {
                    let mut fn_env = env.child();
                    for param in &def.params {
                        fn_env.values.insert(
                            param.name.clone(),
                            StaticBinding {
                                type_name: param.type_name.clone(),
                                is_const: false,
                            },
                        );
                    }
                    self.check_block(&def.body, &mut fn_env, false, def.return_type.clone(), None)?;
                }
                Item::HowlFunction(def) => {
                    let mut fn_env = env.child();
                    for param in &def.params {
                        fn_env.values.insert(
                            param.name.clone(),
                            StaticBinding {
                                type_name: param.type_name.clone(),
                                is_const: false,
                            },
                        );
                    }
                    self.check_block(&def.body, &mut fn_env, true, def.return_type.clone(), None)?;
                }
                Item::Den(den) => {
                    for member in &den.members {
                        if let DenMember::Method(method) = member {
                            let mut method_env = env.child();
                            method_env.values.insert(
                                "self".to_string(),
                                StaticBinding {
                                    type_name: Some(den.name.clone()),
                                    is_const: true,
                                },
                            );
                            for param in &method.params {
                                method_env.values.insert(
                                    param.name.clone(),
                                    StaticBinding {
                                        type_name: param.type_name.clone(),
                                        is_const: false,
                                    },
                                );
                            }
                            self.check_block(
                                &method.body,
                                &mut method_env,
                                false,
                                method.return_type.clone(),
                                Some(den.name.clone()),
                            )?;
                        }
                    }
                }
                Item::Probe { body, .. } | Item::Stmt(Stmt::InsaneBlock(body)) => {
                    self.check_block(body, &mut env.child(), false, None, None)?;
                }
                Item::Stmt(stmt) => self.check_stmt(stmt, &mut env, false, None, None)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn check_block(
        &self,
        body: &[Stmt],
        env: &mut StaticEnv,
        howl: bool,
        return_type: Option<String>,
        current_den: Option<String>,
    ) -> Result<(), Diagnostic> {
        for stmt in body {
            self.check_stmt(stmt, env, howl, return_type.clone(), current_den.clone())?;
        }
        Ok(())
    }

    fn check_stmt(
        &self,
        stmt: &Stmt,
        env: &mut StaticEnv,
        howl: bool,
        return_type: Option<String>,
        current_den: Option<String>,
    ) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Let {
                name,
                expr,
                type_name,
                is_const,
            } => {
                self.check_expr(expr, env, howl, current_den.clone())?;
                if let Some(type_name) = type_name {
                    if let Some(expr_type) = self.expr_type(expr, env) {
                        self.require_assignable(&expr_type, type_name, name)?;
                    }
                }
                env.values.insert(
                    name.clone(),
                    StaticBinding {
                        type_name: type_name.clone().or_else(|| self.expr_type(expr, env)),
                        is_const: *is_const,
                    },
                );
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.check_expr(condition, env, howl, current_den.clone())?;
                if let Some(kind) = self.expr_type(condition, env) {
                    if kind != "Bool" {
                        return Err(static_error("if condition must be Bool"));
                    }
                }
                self.check_block(
                    then_body,
                    &mut env.child(),
                    howl,
                    return_type.clone(),
                    current_den.clone(),
                )?;
                if let Some(else_body) = else_body {
                    match else_body {
                        ElseBody::If(stmt) => {
                            self.check_stmt(stmt, env, howl, return_type, current_den)?
                        }
                        ElseBody::Block(body) => self.check_block(
                            body,
                            &mut env.child(),
                            howl,
                            return_type,
                            current_den,
                        )?,
                    }
                }
            }
            Stmt::While { condition, body } => {
                self.check_expr(condition, env, howl, current_den.clone())?;
                if let Some(kind) = self.expr_type(condition, env) {
                    if kind != "Bool" {
                        return Err(static_error("while condition must be Bool"));
                    }
                }
                self.check_block(body, &mut env.child(), howl, return_type, current_den)?;
            }
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => {
                self.check_expr(iterable, env, howl, current_den.clone())?;
                let mut loop_env = env.child();
                loop_env.values.insert(
                    name.clone(),
                    StaticBinding {
                        type_name: None,
                        is_const: false,
                    },
                );
                self.check_block(body, &mut loop_env, howl, return_type, current_den)?;
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.check_expr(expr, env, howl, current_den.clone())?;
                    if let (Some(expected), Some(actual)) =
                        (return_type.as_ref(), self.expr_type(expr, env))
                    {
                        self.require_assignable(&actual, expected, "return value")?;
                    }
                } else if return_type.as_deref().is_some_and(|kind| kind != "Void") {
                    return Err(static_error(format!(
                        "return value must be {}",
                        return_type.unwrap()
                    )));
                }
            }
            Stmt::Expr(expr) | Stmt::Panic(expr) | Stmt::Expect(expr) => {
                self.check_expr(expr, env, howl, current_den)?
            }
            Stmt::Squeak(exprs) | Stmt::Trace(exprs) => {
                for expr in exprs {
                    self.check_expr(expr, env, howl, current_den.clone())?;
                }
            }
            Stmt::Try {
                body,
                catch_name,
                catch_body,
                ..
            } => {
                self.check_block(
                    body,
                    &mut env.child(),
                    howl,
                    return_type.clone(),
                    current_den.clone(),
                )?;
                if let Some(catch_body) = catch_body {
                    let mut catch_env = env.child();
                    if let Some(catch_name) = catch_name {
                        catch_env.values.insert(
                            catch_name.clone(),
                            StaticBinding {
                                type_name: Some("String".to_string()),
                                is_const: false,
                            },
                        );
                    }
                    self.check_block(catch_body, &mut catch_env, howl, return_type, current_den)?;
                }
            }
            Stmt::InsaneBlock(body) => {
                self.check_block(body, &mut env.child(), howl, return_type, current_den)?
            }
            Stmt::Break | Stmt::Continue => {}
        }
        Ok(())
    }

    fn check_expr(
        &self,
        expr: &Expr,
        env: &StaticEnv,
        howl: bool,
        current_den: Option<String>,
    ) -> Result<(), Diagnostic> {
        match expr {
            Expr::Wait(inner) => {
                if !howl {
                    return Err(static_error("wait can only be used inside howl context"));
                }
                self.check_expr(inner, env, howl, current_den)
            }
            Expr::Scatter { expr, .. } => {
                if !howl {
                    return Err(static_error("scatter can only be used inside howl context"));
                }
                self.check_expr(expr, env, howl, current_den)
            }
            Expr::Nest(items) => {
                if !howl {
                    return Err(static_error("nest can only be used inside howl context"));
                }
                for item in items {
                    self.check_expr(item, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Assign { target, value } => {
                if let Expr::Var(name) = target.as_ref() {
                    if env.get(name).is_some_and(|binding| binding.is_const) {
                        return Err(static_error(format!("{name} is a stash constant")));
                    }
                }
                self.check_expr(target, env, howl, current_den.clone())?;
                self.check_expr(value, env, howl, current_den)
            }
            Expr::Binary { left, right, .. } => {
                self.check_expr(left, env, howl, current_den.clone())?;
                self.check_expr(right, env, howl, current_den)
            }
            Expr::Unary { expr, .. } | Expr::InsaneChoose(expr) => {
                self.check_expr(expr, env, howl, current_den)
            }
            Expr::Array(items) | Expr::Matrix(items) => {
                for item in items {
                    self.check_expr(item, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Map(pairs) => {
                for (key, value) in pairs {
                    self.check_expr(key, env, howl, current_den.clone())?;
                    self.check_expr(value, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Point { x, y } | Expr::Range { start: x, end: y } => {
                self.check_expr(x, env, howl, current_den.clone())?;
                self.check_expr(y, env, howl, current_den)
            }
            Expr::Call { callee, args } => {
                self.check_expr(callee, env, howl, current_den.clone())?;
                for arg in args {
                    self.check_expr(arg, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Index { target, args } => {
                self.check_expr(target, env, howl, current_den.clone())?;
                for arg in args {
                    self.check_expr(arg, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Member { target, name } => {
                self.check_expr(target, env, howl, current_den.clone())?;
                if let Some(target_type) = self.expr_type(target, env) {
                    if let Some(mask) = self.masks.get(&target_type) {
                        if !mask.methods.contains_key(name) {
                            return Err(static_error(format!(
                                "mask {target_type} has no member {name}"
                            )));
                        }
                    }
                    if self.dens.contains_key(&target_type) {
                        if let Some(field) = self.find_field(&target_type, name) {
                            if field.access == Access::Fang
                                && current_den.as_deref() != Some(field.owner.as_str())
                            {
                                return Err(static_error(format!(
                                    "{target_type}.{name} is private"
                                )));
                            }
                        }
                    }
                }
                Ok(())
            }
            Expr::Lambda { body, .. } => match body {
                LambdaBody::Expr(expr) => self.check_expr(expr, env, howl, current_den),
                LambdaBody::Block(body) => {
                    self.check_block(body, &mut env.child(), howl, None, current_den)
                }
            },
            Expr::Tunnel { left, right } => {
                self.check_expr(left, env, howl, current_den.clone())?;
                self.check_expr(right, env, howl, current_den)
            }
            Expr::Hatch { args, .. } => {
                for arg in args {
                    self.check_expr(arg, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Literal(_) | Expr::Var(_) | Expr::Sniff => Ok(()),
        }
    }

    fn expr_type(&self, expr: &Expr, env: &StaticEnv) -> Option<String> {
        match expr {
            Expr::Literal(Literal::Null) => Some("Null".to_string()),
            Expr::Literal(Literal::Bool(_)) => Some("Bool".to_string()),
            Expr::Literal(Literal::Int(_)) => Some("Int".to_string()),
            Expr::Literal(Literal::Float(_)) => Some("Float".to_string()),
            Expr::Literal(Literal::String(_)) => Some("String".to_string()),
            Expr::Array(items) => {
                if let Some(first) = items.first().and_then(|item| self.expr_type(item, env)) {
                    Some(format!("Array<{first}>"))
                } else {
                    Some("Array".to_string())
                }
            }
            Expr::Matrix(rows) => {
                if let Some(Expr::Array(items)) = rows.first() {
                    if let Some(first) = items.first().and_then(|item| self.expr_type(item, env)) {
                        return Some(format!("Matrix<{first}>"));
                    }
                }
                Some("Matrix".to_string())
            }
            Expr::Point { .. } => Some("Point".to_string()),
            Expr::Hatch { name, .. } => Some(name.clone()),
            Expr::Var(name) => env.get(name).and_then(|binding| binding.type_name.clone()),
            Expr::Binary { op, .. }
                if ["==", "!=", "<", "<=", ">", ">=", "&&", "||"].contains(&op.as_str()) =>
            {
                Some("Bool".to_string())
            }
            Expr::Binary { left, op, right } if op == "+" => {
                let left = self.expr_type(left, env);
                let right = self.expr_type(right, env);
                if left.as_deref() == Some("String") || right.as_deref() == Some("String") {
                    Some("String".to_string())
                } else {
                    left.or(right)
                }
            }
            Expr::Member { target, name } => {
                let target_type = self.expr_type(target, env)?;
                self.find_field(&target_type, name)
                    .and_then(|field| field.type_name)
            }
            _ => None,
        }
    }

    fn require_assignable(
        &self,
        actual: &str,
        expected: &str,
        label: &str,
    ) -> Result<(), Diagnostic> {
        let expected_ref = parse_type_ref(expected)?;
        let actual_base = parse_type_ref(actual)?.name;
        let ok = expected_ref.name == "Any"
            || actual_base == "Null"
            || actual_base == expected_ref.name
            || (expected_ref.name == "Float" && actual_base == "Int")
            || (self.dens.contains_key(&expected_ref.name)
                && self.den_is_a(&actual_base, &expected_ref.name))
            || (self.masks.contains_key(&expected_ref.name)
                && self.den_wears(&actual_base, &expected_ref.name));
        if !ok {
            Err(static_error(format!(
                "{label} must be {}",
                expected_ref.text()
            )))
        } else {
            Ok(())
        }
    }

    fn find_field(&self, den_name: &str, field: &str) -> Option<FieldInfo> {
        let den = self.dens.get(den_name)?;
        if let Some(local) = den.fields.get(field) {
            return Some(local.clone());
        }
        den.parent
            .as_ref()
            .and_then(|parent| self.find_field(parent, field))
    }

    fn find_method(&self, den_name: &str, method: &str) -> Option<MethodInfo> {
        let den = self.dens.get(den_name)?;
        if method != "init" {
            if let Some(local) = den.methods.get(method) {
                return Some(local.clone());
            }
        }
        den.parent
            .as_ref()
            .and_then(|parent| self.find_method(parent, method))
    }

    fn den_is_a(&self, den_name: &str, expected: &str) -> bool {
        if den_name == expected {
            return true;
        }
        self.dens
            .get(den_name)
            .and_then(|den| den.parent.as_ref())
            .is_some_and(|parent| self.den_is_a(parent, expected))
    }

    fn den_wears(&self, den_name: &str, mask: &str) -> bool {
        let Some(den) = self.dens.get(den_name) else {
            return false;
        };
        den.masks.iter().any(|candidate| candidate == mask)
            || den
                .parent
                .as_ref()
                .is_some_and(|parent| self.den_wears(parent, mask))
    }
}

#[derive(Clone, Debug, Default)]
struct StaticEnv {
    parent: Option<Box<StaticEnv>>,
    values: BTreeMap<String, StaticBinding>,
}

#[derive(Clone, Debug)]
struct StaticBinding {
    type_name: Option<String>,
    is_const: bool,
}

impl StaticEnv {
    fn child(&self) -> StaticEnv {
        StaticEnv {
            parent: Some(Box::new(self.clone())),
            values: BTreeMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<&StaticBinding> {
        self.values
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get(name)))
    }
}

#[derive(Clone, Debug)]
struct TypeRef {
    name: String,
    args: Vec<TypeRef>,
}

impl TypeRef {
    fn text(&self) -> String {
        if self.args.is_empty() {
            self.name.clone()
        } else {
            format!(
                "{}<{}>",
                self.name,
                self.args
                    .iter()
                    .map(TypeRef::text)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

fn parse_type_ref(text: &str) -> Result<TypeRef, Diagnostic> {
    let text = text.trim();
    if text.is_empty() {
        return Err(static_error("empty type annotation"));
    }
    let Some(start) = text.find('<') else {
        return Ok(TypeRef {
            name: text.to_string(),
            args: Vec::new(),
        });
    };
    if !text.ends_with('>') {
        return Err(static_error(format!("invalid type annotation {text}")));
    }
    let name = text[..start].trim().to_string();
    let inner = &text[start + 1..text.len() - 1];
    let args = split_type_args(inner)
        .into_iter()
        .map(|arg| parse_type_ref(&arg))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TypeRef { name, args })
}

fn split_type_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut start = 0;
    let mut depth = 0_i32;
    for (index, ch) in text.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                args.push(text[start..index].trim().to_string());
                start = index + 1;
            }
            _ => {}
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        args.push(tail.to_string());
    }
    args
}

fn same_signature_method_mask(method: &MethodInfo, required: &MaskMethod) -> bool {
    method.params.len() == required.params.len()
        && method
            .params
            .iter()
            .zip(required.params.iter())
            .all(|(left, right)| {
                normalize_type_name(left.type_name.as_deref())
                    == normalize_type_name(right.type_name.as_deref())
            })
        && normalize_type_name(method.return_type.as_deref())
            == normalize_type_name(required.return_type.as_deref())
}

fn normalize_type_name(type_name: Option<&str>) -> String {
    type_name.unwrap_or("Void").to_string()
}

fn static_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(Category::Static, message.into())
}
