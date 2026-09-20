//! `where` 表达式的受限 DSL：词法分析 + 递归下降，产出 `Cond`。
//!
//! 支持：`列名 操作符 值`、`IN (...)`、`BETWEEN a AND b`、`IS [NOT] NULL`、
//! `AND / OR / NOT` 与任意层级括号。
//! 禁用：分号、SQL 注释、子查询、函数调用、UNION 等一切未列出的语法；
//! 未知标识符直接 40004。**表达式原文绝不进入 SQL**，只会被翻译成参数化片段。

use crate::error::ApiError;
use crate::query::ast::{Cond, Leaf, Op};
use crate::schema::{Registry, Table};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    Num(f64),
    Int(i64),
    LParen,
    RParen,
    Comma,
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    And,
    Or,
    Not,
    In,
    Like,
    Between,
    Is,
    Null,
    True,
    False,
}

fn lex(src: &str) -> Result<Vec<Tok>, ApiError> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let mut out = Vec::new();

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // 显式拒绝危险语法
        if c == ';' {
            return Err(ApiError::param("where 表达式不允许分号"));
        }
        if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            return Err(ApiError::param("where 表达式不允许 SQL 注释"));
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            return Err(ApiError::param("where 表达式不允许 SQL 注释"));
        }
        if c == '\'' || c == '"' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == quote {
                    // '' 转义为单个引号
                    if i + 1 < chars.len() && chars[i + 1] == quote {
                        s.push(quote);
                        i += 2;
                        continue;
                    }
                    closed = true;
                    i += 1;
                    break;
                }
                if ch == '\\' && i + 1 < chars.len() {
                    s.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                s.push(ch);
                i += 1;
            }
            if !closed {
                return Err(ApiError::param("where 表达式中的字符串未闭合"));
            }
            out.push(Tok::Str(s));
            continue;
        }
        if c == '(' {
            out.push(Tok::LParen);
            i += 1;
            continue;
        }
        if c == ')' {
            out.push(Tok::RParen);
            i += 1;
            continue;
        }
        if c == ',' {
            out.push(Tok::Comma);
            i += 1;
            continue;
        }
        // 比较运算符（注意先匹配两字符的）
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        match two.as_str() {
            ">=" => {
                out.push(Tok::Gte);
                i += 2;
                continue;
            }
            "<=" => {
                out.push(Tok::Lte);
                i += 2;
                continue;
            }
            "<>" | "!=" => {
                out.push(Tok::Ne);
                i += 2;
                continue;
            }
            _ => {}
        }
        match c {
            '=' => {
                out.push(Tok::Eq);
                i += 1;
                continue;
            }
            '>' => {
                out.push(Tok::Gt);
                i += 1;
                continue;
            }
            '<' => {
                out.push(Tok::Lt);
                i += 1;
                continue;
            }
            _ => {}
        }
        // 数字（含负号）
        if c.is_ascii_digit()
            || (c == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            if c == '-' {
                i += 1;
            }
            let mut is_float = false;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                if chars[i] == '.' {
                    if is_float {
                        return Err(ApiError::param("where 表达式中数字格式非法"));
                    }
                    is_float = true;
                }
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            if is_float {
                let f = s
                    .parse::<f64>()
                    .map_err(|_| ApiError::param(format!("非法数字: {s}")))?;
                out.push(Tok::Num(f));
            } else {
                let n = s
                    .parse::<i64>()
                    .map_err(|_| ApiError::param(format!("非法整数: {s}")))?;
                out.push(Tok::Int(n));
            }
            continue;
        }
        // 标识符：允许字母/数字/下划线/中文/点（关联表或 JSON 路径）
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() {
                let ch = chars[i];
                let ok = ch.is_alphanumeric()
                    || ch == '_'
                    || ch == '.'
                    || ('\u{4e00}'..='\u{9fff}').contains(&ch);
                if !ok {
                    break;
                }
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let upper = word.to_ascii_uppercase();
            out.push(match upper.as_str() {
                "AND" => Tok::And,
                "OR" => Tok::Or,
                "NOT" => Tok::Not,
                "IN" => Tok::In,
                "LIKE" => Tok::Like,
                "BETWEEN" => Tok::Between,
                "IS" => Tok::Is,
                "NULL" => Tok::Null,
                "TRUE" => Tok::True,
                "FALSE" => Tok::False,
                _ => Tok::Ident(word),
            });
            continue;
        }
        return Err(ApiError::param(format!("where 表达式中出现非法字符: {c}")));
    }
    Ok(out)
}

struct Parser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    table: &'a Table,
    reg: &'a Registry,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: Tok, what: &str) -> Result<(), ApiError> {
        if self.eat(&t) {
            Ok(())
        } else {
            Err(ApiError::param(format!(
                "where 表达式缺少 {what}（位置 {}）",
                self.pos
            )))
        }
    }

    fn parse_or(&mut self) -> Result<Cond, ApiError> {
        let mut parts = vec![self.parse_and()?];
        while self.eat(&Tok::Or) {
            parts.push(self.parse_and()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Cond::Or(parts)
        })
    }

    fn parse_and(&mut self) -> Result<Cond, ApiError> {
        let mut parts = vec![self.parse_unary()?];
        while self.eat(&Tok::And) {
            parts.push(self.parse_unary()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Cond::And(parts)
        })
    }

    fn parse_unary(&mut self) -> Result<Cond, ApiError> {
        if self.eat(&Tok::Not) {
            return Ok(Cond::Not(Box::new(self.parse_unary()?)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Cond, ApiError> {
        if self.eat(&Tok::LParen) {
            let inner = self.parse_or()?;
            self.expect(Tok::RParen, "右括号")?;
            return Ok(inner);
        }
        self.parse_comparison()
    }

    fn parse_value(&mut self) -> Result<Value, ApiError> {
        match self.next() {
            Some(Tok::Str(s)) => Ok(Value::String(s)),
            Some(Tok::Int(i)) => Ok(Value::from(i)),
            Some(Tok::Num(f)) => Ok(serde_json::Number::from_f64(f)
                .map(Value::Number)
                .unwrap_or(Value::Null)),
            Some(Tok::Null) => Ok(Value::Null),
            Some(Tok::True) => Ok(Value::from(1)),
            Some(Tok::False) => Ok(Value::from(0)),
            Some(Tok::Ident(w)) => Ok(Value::String(w)),
            other => Err(ApiError::param(format!(
                "where 表达式中期望一个值，实际遇到: {other:?}"
            ))),
        }
    }

    fn parse_comparison(&mut self) -> Result<Cond, ApiError> {
        let col = match self.next() {
            Some(Tok::Ident(c)) => c,
            other => {
                return Err(ApiError::param(format!(
                    "where 表达式中期望列名，实际遇到: {other:?}"
                )))
            }
        };

        // IS [NOT] NULL
        if self.eat(&Tok::Is) {
            let negated = self.eat(&Tok::Not);
            self.expect(Tok::Null, "NULL")?;
            let op = if negated { Op::IsNotNull } else { Op::IsNull };
            crate::query::ast::resolve(self.table, self.reg, &col, op)?;
            return Ok(Cond::Leaf(Leaf::new(col, op, Vec::new())));
        }

        // [NOT] IN (...) / [NOT] LIKE / [NOT] BETWEEN
        let mut negated = false;
        if self.peek() == Some(&Tok::Not) {
            // 只有后面跟着 IN/LIKE/BETWEEN 才是合法取反
            let follow = self.toks.get(self.pos + 1);
            if matches!(follow, Some(Tok::In) | Some(Tok::Like) | Some(Tok::Between)) {
                self.pos += 1;
                negated = true;
            }
        }

        if self.eat(&Tok::In) {
            self.expect(Tok::LParen, "左括号")?;
            let mut vals = vec![self.parse_value()?];
            while self.eat(&Tok::Comma) {
                vals.push(self.parse_value()?);
            }
            self.expect(Tok::RParen, "右括号")?;
            let op = if negated { Op::Nin } else { Op::In };
            crate::query::ast::resolve(self.table, self.reg, &col, op)?;
            return Ok(Cond::Leaf(Leaf::new(col, op, vals)));
        }

        if self.eat(&Tok::Like) {
            let v = self.parse_value()?;
            let op = if negated { Op::NLike } else { Op::Like };
            crate::query::ast::resolve(self.table, self.reg, &col, op)?;
            return Ok(Cond::Leaf(Leaf::new(col, op, vec![v])));
        }

        if self.eat(&Tok::Between) {
            let a = self.parse_value()?;
            self.expect(Tok::And, "AND")?;
            let b = self.parse_value()?;
            crate::query::ast::resolve(self.table, self.reg, &col, Op::Between)?;
            return Ok(Cond::Leaf(Leaf::new(col, Op::Between, vec![a, b])));
        }

        if negated {
            return Err(ApiError::param(
                "where 表达式中 NOT 只能用于 IN / LIKE / BETWEEN",
            ));
        }

        let op = match self.next() {
            Some(Tok::Eq) => Op::Eq,
            Some(Tok::Ne) => Op::Ne,
            Some(Tok::Gt) => Op::Gt,
            Some(Tok::Gte) => Op::Gte,
            Some(Tok::Lt) => Op::Lt,
            Some(Tok::Lte) => Op::Lte,
            other => {
                return Err(ApiError::param(format!(
                    "where 表达式缺少比较运算符，实际遇到: {other:?}"
                )))
            }
        };
        let v = self.parse_value()?;
        crate::query::ast::resolve(self.table, self.reg, &col, op)?;
        Ok(Cond::Leaf(Leaf::new(col, op, vec![v])))
    }
}

pub fn parse(t: &Table, reg: &Registry, src: &str) -> Result<Cond, ApiError> {
    let toks = lex(src)?;
    if toks.is_empty() {
        return Err(ApiError::param("where 表达式为空"));
    }
    let mut p = Parser {
        toks,
        pos: 0,
        table: t,
        reg,
    };
    let cond = p.parse_or()?;
    if p.pos != p.toks.len() {
        return Err(ApiError::param(format!(
            "where 表达式存在多余内容（已解析 {} / {} 个记号）",
            p.pos,
            p.toks.len()
        )));
    }
    Ok(cond)
}
