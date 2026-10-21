use std::borrow::Cow;
use swc_core::ecma::ast::*;

/// Extract the element name from a JSX element, or `None` for name syntax the
/// plugin doesn't know so the element is left unannotated
#[inline]
pub fn get_element_name(element: &JSXElementName) -> Option<Cow<'_, str>> {
    match element {
        JSXElementName::Ident(ident) => Some(Cow::Borrowed(ident.sym.as_ref())),
        JSXElementName::JSXMemberExpr(member_expr) => {
            get_member_expression_name(member_expr).map(Cow::Owned)
        }
        JSXElementName::JSXNamespacedName(namespaced) => Some(Cow::Owned(format!(
            "{}:{}",
            namespaced.ns.sym, namespaced.name.sym
        ))),
        #[cfg(swc_ast_unknown)]
        _ => None,
    }
}

/// Element names that say nothing about what rendered: intrinsic tags like
/// `<div>` or `<path>`, which the DOM already shows, polymorphic `<Component>`
/// tags, and members of local values like `<field.Switch>`, which the React
/// Compiler can also rename (`field_5.Switch`).
pub fn is_noise_element_name(element: &JSXElementName) -> bool {
    match element {
        // JSX compiles lowercase and namespaced tags to strings, never components
        JSXElementName::Ident(ident) => {
            ident.sym.starts_with(|c: char| c.is_ascii_lowercase()) || &*ident.sym == "Component"
        }
        JSXElementName::JSXNamespacedName(_) => true,
        JSXElementName::JSXMemberExpr(member_expr) => {
            let mut obj = &member_expr.obj;
            loop {
                match obj {
                    JSXObject::Ident(ident) => {
                        break ident.sym.starts_with(|c: char| c.is_ascii_lowercase());
                    }
                    JSXObject::JSXMemberExpr(nested_member) => obj = &nested_member.obj,
                    #[cfg(swc_ast_unknown)]
                    _ => break false,
                }
            }
        }
        #[cfg(swc_ast_unknown)]
        _ => false,
    }
}

/// `<svg>` contents are drawing internals, like paths and groups, that don't say
/// what was clicked
#[inline]
pub fn is_svg_element_name(element: &JSXElementName) -> bool {
    matches!(element, JSXElementName::Ident(ident) if &*ident.sym == "svg")
}

/// Recursively build the name for member expressions (e.g., "Components.UI.Button")
fn get_member_expression_name(member_expr: &JSXMemberExpr) -> Option<String> {
    fn member_expression_name_len(member_expr: &JSXMemberExpr) -> Option<usize> {
        let obj_len = match &member_expr.obj {
            JSXObject::Ident(ident) => ident.sym.len(),
            JSXObject::JSXMemberExpr(nested_member) => member_expression_name_len(nested_member)?,
            #[cfg(swc_ast_unknown)]
            _ => return None,
        };

        Some(obj_len + 1 + member_expr.prop.sym.len())
    }

    fn push_member_expression_name(target: &mut String, member_expr: &JSXMemberExpr) {
        match &member_expr.obj {
            JSXObject::Ident(ident) => target.push_str(ident.sym.as_ref()),
            JSXObject::JSXMemberExpr(nested_member) => {
                push_member_expression_name(target, nested_member);
            }
            // `member_expression_name_len` already rejected unknown objects
            #[cfg(swc_ast_unknown)]
            _ => {}
        }

        target.push('.');
        target.push_str(member_expr.prop.sym.as_ref());
    }

    let mut output = String::with_capacity(member_expression_name_len(member_expr)?);
    push_member_expression_name(&mut output, member_expr);
    Some(output)
}

#[derive(Default)]
pub struct AttributePresence {
    pub component: bool,
    pub element: bool,
    pub source_file: bool,
    pub source_path: bool,
}

pub fn attribute_presence(
    element: &JSXOpeningElement,
    component_attr: &IdentName,
    element_attr: &IdentName,
    source_file_attr: &IdentName,
    source_path_attr: Option<&IdentName>,
) -> AttributePresence {
    let mut presence = AttributePresence::default();

    for attr in &element.attrs {
        let JSXAttrOrSpread::JSXAttr(jsx_attr) = attr else {
            continue;
        };
        let JSXAttrName::Ident(ident) = &jsx_attr.name else {
            continue;
        };

        if ident.sym == component_attr.sym {
            presence.component = true;
        } else if ident.sym == element_attr.sym {
            presence.element = true;
        } else if ident.sym == source_file_attr.sym {
            presence.source_file = true;
        } else if source_path_attr.is_some_and(|source_path_attr| ident.sym == source_path_attr.sym)
        {
            presence.source_path = true;
        }
    }

    presence
}

/// Create a JSX attribute with a string value
#[inline]
pub fn create_jsx_attr(name: &str, value: &str) -> JSXAttrOrSpread {
    JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: Default::default(),
        name: JSXAttrName::Ident(IdentName::new(name.into(), Default::default())),
        value: Some(JSXAttrValue::Str(Str {
            span: Default::default(),
            value: value.into(),
            raw: None,
        })),
    })
}

#[inline]
pub fn create_jsx_attr_with_ident(name: &IdentName, value: &str) -> JSXAttrOrSpread {
    JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: Default::default(),
        name: JSXAttrName::Ident(name.clone()),
        value: Some(JSXAttrValue::Str(Str {
            span: Default::default(),
            value: value.into(),
            raw: None,
        })),
    })
}

#[inline]
pub fn create_jsx_attr_with_ident_and_str(name: &IdentName, value: &Str) -> JSXAttrOrSpread {
    JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: Default::default(),
        name: JSXAttrName::Ident(name.clone()),
        value: Some(JSXAttrValue::Str(value.clone())),
    })
}
